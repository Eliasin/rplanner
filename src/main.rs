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

fn construct_notes(note_fragments: &mut FragmentMap, date_map: &HashMap<i64, String>) -> Vec<Note> {
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

        notes.push(note);
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
fn notes(db_connection: State<DBConnection>) -> InternalResult<Json<Vec<Note>>> {
    let db_connection = db_connection.lock()?;

    let mut note_statement = db_connection.prepare("SELECT rowid, date FROM notes")?;
    let mut date_map = HashMap::new();

    note_statement.query_map(NO_PARAMS, |row| {
        add_note_date(&mut date_map, &row);
        Ok(())
    })?.next();

    let mut text_statement = db_connection.prepare("SELECT * FROM note_text_elements")?;
    let mut image_statement = db_connection.prepare("SELECT * FROM note_image_elements")?;

    let mut note_fragments = FragmentMap::new();
    text_statement.query_map(NO_PARAMS , |row| {
        add_fragment(&mut note_fragments, row, FragmentTag::Text);
        Ok(())
    })?.next();

    image_statement.query_map(NO_PARAMS , |row| {
        add_fragment(&mut note_fragments, row, FragmentTag::Image);
        Ok(())
    })?.next();

    Ok(Json(construct_notes(&mut note_fragments, &date_map)))
}

#[post("/add_note", format = "json", data = "<note>")]
fn add_note(note: Json<Note>, db_connection: State<DBConnection>) -> InternalResult<()> {
    let db_connection = db_connection.lock()?;

    db_connection.execute("INSERT INTO notes VALUES (?1)", &[Utc::now().to_rfc2822()])?;
    let id = db_connection.last_insert_rowid().to_string();

    for (num, element) in note.content.iter().enumerate() {
        match element {
            NoteElement::Text(t) => {
                db_connection.execute("INSERT INTO note_text_elements VALUES (?1, ?2, ?3)", &[id.clone(), t.clone(), num.to_string()])?;
            },
            NoteElement::Image(path) => {
                db_connection.execute("INSERT INTO note_image_elements VALUES (?1, ?2, ?3)", &[id.clone(), path.clone(), num.to_string()])?;
            },
        }
    }

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
        .mount("/api", routes![notes, add_note])
        .mount("/images", StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/images")).rank(15))
        .mount("/", StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/web")))
        .launch();

    Ok(())
}
