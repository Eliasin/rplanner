use rusqlite::{ Connection, params, Row };
use chrono::offset::Utc;

use std::collections::HashMap;
use std::path::Path;
use std::fs::{ File, read_dir };
use std::io;
use std::io::Write;

use crate::internal_error::InternalResult;

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
                    },
                    None => {
                        fragments.insert(fragment.0, vec![(fragment.1, fragment.2)]);
                    },
                };
        },
        Err(e) => eprintln!("{}", e),
    }
}

pub fn construct_notes(note_fragments: &mut FragmentMap, date_map: &HashMap<i64, String>) -> Vec<(NoteID, Note)> {
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

pub fn add_note_date(date_map: &mut HashMap<i64, String>, row: &Row) {
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

pub fn add_note_contents_to_db(note_id: NoteID, note_content: impl Iterator<Item = NoteElement>, db_connection: &Connection) -> InternalResult<()> {
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

pub fn add_note_to_db(note: Note, db_connection: &Connection) -> InternalResult<AddNoteResult> {
    db_connection.execute("INSERT INTO notes VALUES (?1)", &[Utc::now().to_rfc2822()])?;
    let id = db_connection.last_insert_rowid();

    add_note_contents_to_db(id, &mut note.content.into_iter(), db_connection)?;

    Ok(AddNoteResult {
        note_id: id
    })
}

pub fn delete_note_contents_from_db(note_id: NoteID, db_connection: &Connection) -> InternalResult<()> {
    db_connection.execute("DELETE FROM note_text_elements WHERE note_id = (?1)", params![note_id])?;
    db_connection.execute("DELETE FROM note_image_elements WHERE note_id = (?1)", params![note_id])?;

    Ok(())
}

pub fn update_note_date(note_id: NoteID, date: String, db_connection: &Connection) -> InternalResult<()> {
    db_connection.execute("UPDATE notes SET date = (?1) WHERE rowid = (?2)", params![date, note_id])?;
    Ok(())
}

pub fn delete_note_from_db(note_id: NoteID, db_connection: &Connection) -> InternalResult<()> {
    db_connection.execute("DELETE FROM notes WHERE rowid = (?1)", params![note_id])?;

    Ok(())
}


pub fn write_data_to_disk(path: &Path, data: &Vec<u8>) -> io::Result<()> {
    let mut file = File::create(path)?;

    file.write(data)?;
    Ok(())
}

pub fn validate_path_is_in_image_folder(path: &Path) -> bool {
    match path.parent() {
        Some(parent) => parent == Path::new("images/"),
        None => false
    }
}

pub fn get_image_filenames() -> InternalResult<Vec<String>> {
    let image_files: Vec<String> = read_dir(Path::new("images/"))?.filter_map(|dir_entry| {
        match dir_entry {
            Ok(v) => Some(v.path().file_name()?.to_string_lossy().to_string()),
            Err(_) => None,
        }
    }).collect();
    Ok(image_files)
}
