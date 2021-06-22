use chrono::offset::Utc;
use rusqlite::{params, Connection, Row, NO_PARAMS};

use std::collections::HashMap;
use std::fs::read_dir;
use std::path::Path;

use crate::internal_error::{InternalError, InternalResult};

use super::data::*;

pub fn get_fragment_from_row(row: &Row, tag: FragmentTag) -> InternalResult<NoteFragment> {
    let note_id: NoteID = row.get(0)?;
    let content: String = row.get(1)?;
    let fragment_num: FragmentNum = row.get(2)?;

    match tag {
        FragmentTag::Text => Ok((note_id, NoteElement::Text(content), fragment_num)),
        FragmentTag::Image => Ok((note_id, NoteElement::Image(content), fragment_num)),
    }
}

pub fn add_fragment(fragments: &mut FragmentMap, row: &Row, tag: FragmentTag) {
    match get_fragment_from_row(row, tag) {
        Ok(fragment) => {
            match fragments.get_mut(&fragment.0) {
                Some(v) => {
                    v.push((fragment.1, fragment.2));
                }
                None => {
                    fragments.insert(fragment.0, vec![(fragment.1, fragment.2)]);
                }
            };
        }
        Err(e) => eprintln!("{}", e),
    }
}

pub fn construct_notes(
    note_fragments: &mut FragmentMap,
    date_map: &HashMap<i64, String>,
) -> Vec<(NoteID, Note)> {
    let mut notes = vec![];

    for (note_id, fragments) in note_fragments.iter_mut() {
        fragments.sort_by_key(|fragment| fragment.1);

        let mut note = Note {
            content: vec![],
            date: match date_map.get(&note_id) {
                Some(v) => v.clone(),
                None => String::new(),
            },
        };

        for fragment in fragments.iter() {
            note.content.push(fragment.0.clone());
        }

        notes.push((*note_id, note));
    }

    notes
}

pub fn add_note_date(date_map: &mut HashMap<i64, String>, row: &Row) {
    match row.get::<usize, i64>(0) {
        Ok(note_id) => match row.get::<usize, String>(1) {
            Ok(date) => {
                date_map.insert(note_id, date);
            }
            Err(e) => eprintln!("{}", e),
        },
        Err(e) => eprintln!("{}", e),
    };
}

pub fn add_note_contents_to_db(
    note_id: NoteID,
    note_content: impl Iterator<Item = NoteElement>,
    db_connection: &Connection,
) -> InternalResult<()> {
    for (num, element) in note_content.enumerate() {
        match element {
            NoteElement::Text(t) => {
                db_connection.execute(
                    "INSERT INTO note_text_elements VALUES (?1, ?2, ?3)",
                    params![note_id, t.clone(), num as i64],
                )?;
            }
            NoteElement::Image(path) => {
                db_connection.execute(
                    "INSERT INTO note_image_elements VALUES (?1, ?2, ?3)",
                    params![note_id, path.clone(), num as i64],
                )?;
            }
        }
    }

    Ok(())
}

pub fn add_note_to_db(note: Note, db_connection: &Connection) -> InternalResult<AddNoteResult> {
    db_connection.execute("INSERT INTO notes VALUES (?1)", &[Utc::now().to_rfc2822()])?;
    let id = db_connection.last_insert_rowid();

    add_note_contents_to_db(id, &mut note.content.into_iter(), db_connection)?;

    Ok(AddNoteResult { note_id: id })
}

pub fn delete_note_contents_from_db(
    note_id: NoteID,
    db_connection: &Connection,
) -> InternalResult<()> {
    db_connection.execute(
        "DELETE FROM note_text_elements WHERE note_id = (?1)",
        params![note_id],
    )?;
    db_connection.execute(
        "DELETE FROM note_image_elements WHERE note_id = (?1)",
        params![note_id],
    )?;

    Ok(())
}

pub fn update_note_date(
    note_id: NoteID,
    date: String,
    db_connection: &Connection,
) -> InternalResult<()> {
    db_connection.execute(
        "UPDATE notes SET date = (?1) WHERE rowid = (?2)",
        params![date, note_id],
    )?;
    Ok(())
}

pub fn delete_note_from_db(note_id: NoteID, db_connection: &Connection) -> InternalResult<()> {
    db_connection.execute("DELETE FROM notes WHERE rowid = (?1)", params![note_id])?;

    Ok(())
}

pub fn validate_path_is_in_image_folder(path: &Path) -> bool {
    match path.parent() {
        Some(parent) => parent == Path::new("images/"),
        None => false,
    }
}

pub fn get_image_filenames() -> InternalResult<Vec<String>> {
    let image_files: Vec<String> = read_dir(Path::new("images/"))?
        .filter_map(|dir_entry| match dir_entry {
            Ok(v) => Some(v.path().file_name()?.to_string_lossy().to_string()),
            Err(_) => None,
        })
        .collect();
    Ok(image_files)
}

pub fn insert_image_into_note(
    note: &mut Note,
    fragment_num: FragmentNum,
    index: usize,
    image_path: &String,
) -> InternalResult<()> {
    match note.content.get_mut(fragment_num as usize) {
        Some(note_element) => match note_element {
            NoteElement::Text(text) => {
                let text_after = text.split_off(index);

                note.content.insert(
                    (fragment_num + 1) as usize,
                    NoteElement::Image(image_path.clone()),
                );
                note.content
                    .insert((fragment_num + 2) as usize, NoteElement::Text(text_after));

                Ok(())
            }
            NoteElement::Image(_) => Err(InternalError::from(
                "Cannot insert image into middle of image fragment",
            )),
        },
        None => Err(InternalError::from("Invalid note index")),
    }
}

pub fn remove_fragment_from_note(note: &mut Note, fragment_num: FragmentNum) -> InternalResult<()> {
    if note.content.len() < fragment_num as usize {
        return Err(InternalError::from("Invalid fragment number"));
    }

    note.content.remove(fragment_num as usize);

    let previous_fragment = note.content.get(fragment_num as usize);
    let next_fragment = note.content.get((fragment_num + 1) as usize);

    if previous_fragment.is_some() && next_fragment.is_some() {
        let previous_fragment = previous_fragment.unwrap();
        let next_fragment = next_fragment.unwrap();

        match previous_fragment {
            NoteElement::Text(prev_text) => match next_fragment {
                NoteElement::Text(next_text) => {
                    let new_text = format!("{}{}", prev_text, next_text);

                    note.content.remove(fragment_num as usize);
                    note.content.remove(fragment_num as usize);
                    note.content
                        .insert(fragment_num as usize, NoteElement::Text(new_text));
                }
                _ => {}
            },
            _ => {}
        };
    }

    Ok(())
}

pub fn get_note_from_db(note_id: NoteID, db_connection: &Connection) -> InternalResult<Note> {
    let mut note_statement =
        db_connection.prepare("SELECT rowid, date FROM notes WHERE rowid = (?1)")?;
    let mut date_map = HashMap::new();

    note_statement
        .query_map(params![note_id], |row| {
            add_note_date(&mut date_map, &row);
            Ok(())
        })?
        .next();

    let mut text_statement = db_connection
        .prepare("SELECT note_id, content, num FROM note_text_elements WHERE note_id = (?1)")?;
    let mut image_statement = db_connection
        .prepare("SELECT note_id, path, num FROM note_image_elements WHERE note_id = (?1)")?;

    let mut note_fragments = FragmentMap::new();
    let mut rows = text_statement.query(params![note_id])?;

    while let Some(row) = rows.next()? {
        add_fragment(&mut note_fragments, row, FragmentTag::Text);
    }

    let mut rows = image_statement.query(params![note_id])?;

    while let Some(row) = rows.next()? {
        add_fragment(&mut note_fragments, row, FragmentTag::Image);
    }

    let mut notes = construct_notes(&mut note_fragments, &date_map);

    if notes.len() != 1 {
        return Err(InternalError::from("Could not construct note"));
    }

    let (_, note) = notes.remove(0);

    Ok(note)
}

pub fn get_all_notes_from_db(db_connection: &Connection) -> InternalResult<Vec<(NoteID, Note)>> {
    let mut note_statement = db_connection.prepare("SELECT rowid, date FROM notes")?;
    let mut date_map = HashMap::new();

    note_statement
        .query_map(NO_PARAMS, |row| {
            add_note_date(&mut date_map, &row);
            Ok(())
        })?
        .next();

    let mut text_statement =
        db_connection.prepare("SELECT note_id, content, num FROM note_text_elements")?;
    let mut image_statement =
        db_connection.prepare("SELECT note_id, path, num FROM note_image_elements")?;

    let mut note_fragments = FragmentMap::new();
    let mut rows = text_statement.query(NO_PARAMS)?;

    while let Some(row) = rows.next()? {
        add_fragment(&mut note_fragments, row, FragmentTag::Text);
    }

    let mut rows = image_statement.query(NO_PARAMS)?;

    while let Some(row) = rows.next()? {
        add_fragment(&mut note_fragments, row, FragmentTag::Image);
    }

    Ok(construct_notes(&mut note_fragments, &date_map))
}
