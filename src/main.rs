#![feature(proc_macro_hygiene, decl_macro)]
use rusqlite::{ Connection, params, NO_PARAMS, Row };
use rocket::State;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::json::Json;
use serde::{ Serialize, Deserialize };
use chrono::offset::Utc;

use std::error::Error;
use std::sync::{ Mutex, Arc };
use std::collections::HashMap;

mod internal_error;

use internal_error::InternalResult;

#[macro_use] extern crate rocket;

#[derive(Serialize, Deserialize, Debug, Clone)]
enum NoteElement {
    Text(String),
    Image(String),
}

enum FragmentTag {
    Text,
    Image
}

#[derive(Serialize, Deserialize, Debug)]
struct Note {
    content: Vec<NoteElement>,
    date: String,
}

type DBConnection = Arc<Mutex<Connection>>;
type NoteID = i64;
type FragmentNum = i64;
type NoteFragment = (NoteID, NoteElement, FragmentNum);

type FragmentMap = HashMap<NoteID, Vec<(NoteElement, FragmentNum)>>;

fn get_fragment_from_row(row: &Row, tag: FragmentTag) -> InternalResult<NoteFragment> {
    let note_id: NoteID = row.get(0)?;
    let content: String = row.get(1)?;
    let fragment_num: FragmentNum = row.get(2)?;

    match tag {
        FragmentTag::Text => Ok((note_id, NoteElement::Text(content), fragment_num)),
        FragmentTag::Image => Ok((note_id, NoteElement::Image(content), fragment_num)),
    }
}

fn add_fragment(fragments: &mut FragmentMap, row: &Row, tag: FragmentTag) {
    match get_fragment_from_row(row, tag) {
        Ok(fragment) => {
                match fragments.get_mut(&fragment.0) {
                    Some(v) => {
                        v.push((fragment.1, fragment.2));
                    },
                    None => {
                        fragments.insert(fragment.0, vec![(fragment.1, fragment.2)]);
                    },
                };
        },
        Err(e) => eprintln!("{}", e),
    }
}

fn construct_notes(note_fragments: &mut FragmentMap, date_map: &HashMap<i64, String>) -> Vec<(NoteID, Note)> {
    let mut notes = vec![];

    for (note_id, fragments) in note_fragments.iter_mut() {
        fragments.sort_by_key(|fragment| fragment.1);

        let mut note = Note {
            content: vec![],
            date: match date_map.get(&note_id) {
                Some(v) => v.clone(),
                None => String::new()
            },
        };

        for fragment in fragments.iter() {
            note.content.push(fragment.0.clone());
        }

        notes.push((*note_id, note));
    }

    notes
}

fn add_note_date(date_map: &mut HashMap<i64, String>, row: &Row) {
    match row.get::<usize, i64>(0) {
        Ok(note_id) => {
            match row.get::<usize, String>(1) {
                Ok(date) => {
                    date_map.insert(note_id, date);
                },
                Err(e) => eprintln!("{}", e),
            }
        },
        Err(e) => eprintln!("{}", e),
    };
}

#[get("/notes")]
fn notes(db_connection: State<DBConnection>) -> InternalResult<Json<Vec<(NoteID, Note)>>> {
    let db_connection = db_connection.lock()?;

    let mut note_statement = db_connection.prepare("SELECT rowid, date FROM notes")?;
    let mut date_map = HashMap::new();

    note_statement.query_map(NO_PARAMS, |row| {
        add_note_date(&mut date_map, &row);
        Ok(())
    })?.next();

    let mut text_statement = db_connection.prepare("SELECT note_id, content, num FROM note_text_elements")?;
    let mut image_statement = db_connection.prepare("SELECT note_id, path, num FROM note_image_elements")?;

    let mut note_fragments = FragmentMap::new();
    let mut rows = text_statement.query(NO_PARAMS)?;

    while let Some(row) = rows.next()? {
        add_fragment(&mut note_fragments, row, FragmentTag::Text);
    }

    let mut rows = image_statement.query(NO_PARAMS)?;

    while let Some(row) = rows.next()? {
        add_fragment(&mut note_fragments, row, FragmentTag::Image);
    }

    Ok(Json(construct_notes(&mut note_fragments, &date_map)))
}

#[derive(Serialize, Debug)]
struct AddNoteResult {
    note_id: NoteID,
}

fn add_note_contents_to_db(note_id: NoteID, note_content: impl Iterator<Item = NoteElement>, db_connection: &Connection) -> InternalResult<()> {
    for (num, element) in note_content.enumerate() {
        match element {
            NoteElement::Text(t) => {
                db_connection.execute("INSERT INTO note_text_elements VALUES (?1, ?2, ?3)", params![note_id, t.clone(), num as i64])?;
            },
            NoteElement::Image(path) => {
                db_connection.execute("INSERT INTO note_image_elements VALUES (?1, ?2, ?3)", params![note_id, path.clone(), num as i64])?;
            },
        }
    }

    Ok(())
}

fn add_note_to_db(note: Note, db_connection: &Connection) -> InternalResult<AddNoteResult> {
    db_connection.execute("INSERT INTO notes VALUES (?1)", &[Utc::now().to_rfc2822()])?;
    let id = db_connection.last_insert_rowid();

    add_note_contents_to_db(id, &mut note.content.into_iter(), db_connection)?;

    Ok(AddNoteResult {
        note_id: id
    })
}

#[post("/add_note", format = "json", data = "<note>")]
fn add_note(note: Json<Note>, db_connection: State<DBConnection>) -> InternalResult<Json<AddNoteResult>> {
    let db_connection = db_connection.lock()?;
    add_note_to_db(note.into_inner(), &db_connection).map(|r| Json(r))
}

#[derive(Deserialize, Debug)]
struct SetNoteRequest {
    note_id: i64,
    note: Note,
}

fn delete_note_contents_from_db(note_id: NoteID, db_connection: &Connection) -> InternalResult<()> {
    db_connection.execute("DELETE FROM note_text_elements WHERE note_id = (?1)", params![note_id])?;
    db_connection.execute("DELETE FROM note_image_elements WHERE note_id = (?1)", params![note_id])?;

    Ok(())
}

fn update_note_date(note_id: NoteID, date: String, db_connection: &Connection) -> InternalResult<()> {
    db_connection.execute("UPDATE notes SET date = (?1) WHERE rowid = (?2)", params![date, note_id])?;
    Ok(())
}

#[post("/set_note", format = "json", data = "<set_note_request>")]
fn set_note(set_note_request: Json<SetNoteRequest>, db_connection: State<DBConnection>) -> InternalResult<()> {
    let db_connection = db_connection.lock()?;

    let date = set_note_request.note.date.clone();
    let note_id = set_note_request.note_id;
    update_note_date(note_id, date, &db_connection)?;
    delete_note_contents_from_db(note_id, &db_connection)?;

    let note_contents = set_note_request.into_inner().note.content;
    add_note_contents_to_db(note_id, note_contents.into_iter(), &*db_connection)?;

    Ok(())
}

fn delete_note_from_db(note_id: NoteID, db_connection: &Connection) -> InternalResult<()> {
    db_connection.execute("DELETE FROM notes WHERE rowid = (?1)", params![note_id])?;

    Ok(())
}

#[derive(Deserialize, Debug)]
struct DeleteNoteRequest {
    note_id: i64,
}

#[post("/delete_note", format = "json", data = "<delete_note_request>")]
fn delete_note(delete_note_request: Json<DeleteNoteRequest>, db_connection: State<DBConnection>) -> InternalResult<()> {
    let db_connection = db_connection.lock()?;

    let note_id = delete_note_request.note_id;
    delete_note_contents_from_db(note_id, &db_connection)?;
    delete_note_from_db(note_id, &db_connection)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let connection = Connection::open("rplanner.db")?;

    connection.execute("CREATE TABLE IF NOT EXISTS notes (date TEXT)", params![])?;
    connection.execute("CREATE TABLE IF NOT EXISTS note_text_elements (note_id INTEGER, content TEXT, num INTEGER)", params![])?;
    connection.execute("CREATE TABLE IF NOT EXISTS note_image_elements (note_id INTEGER, path TEXT, num INTEGER)", params![])?;
    let connection = Arc::new(Mutex::new(connection));

    rocket::ignite()
        .manage(connection.clone())
        .mount("/api", routes![notes, add_note, set_note, delete_note])
        .mount("/images", StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/images")).rank(15))
        .mount("/", StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/web")))
        .launch();

    Ok(())
}
