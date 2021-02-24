use rusqlite::{ NO_PARAMS };
use rocket::{ State, Data, post, get };
use rocket::http::ContentType;
use rocket_multipart_form_data::{ MultipartFormDataOptions, MultipartFormData, mime, MultipartFormDataField };
use rocket_contrib::json::Json;

use std::collections::HashMap;
use std::path::Path;

use crate::internal_error::{ InternalResult, InternalError };

use super::helpers::*;
use super::data::*;

#[get("/get_notes")]
pub fn get_notes(db_connection: State<DBConnection>) -> InternalResult<Json<Vec<(NoteID, Note)>>> {
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

#[post("/add_note", format = "json", data = "<note>")]
pub fn add_note(note: Json<Note>, db_connection: State<DBConnection>) -> InternalResult<Json<AddNoteResult>> {
    let db_connection = db_connection.lock()?;
    add_note_to_db(note.into_inner(), &db_connection).map(|r| Json(r))
}

#[post("/set_note", format = "json", data = "<set_note_request>")]
pub fn set_note(set_note_request: Json<SetNoteRequest>, db_connection: State<DBConnection>) -> InternalResult<()> {
    let db_connection = db_connection.lock()?;

    let date = set_note_request.note.date.clone();
    let note_id = set_note_request.note_id;
    update_note_date(note_id, date, &db_connection)?;
    delete_note_contents_from_db(note_id, &db_connection)?;

    let note_contents = set_note_request.into_inner().note.content;
    add_note_contents_to_db(note_id, note_contents.into_iter(), &*db_connection)?;

    Ok(())
}

#[post("/delete_note", format = "json", data = "<delete_note_request>")]
pub fn delete_note(delete_note_request: Json<DeleteNoteRequest>, db_connection: State<DBConnection>) -> InternalResult<()> {
    let db_connection = db_connection.lock()?;

    let note_id = delete_note_request.note_id;
    delete_note_contents_from_db(note_id, &db_connection)?;
    delete_note_from_db(note_id, &db_connection)?;

    Ok(())
}

#[post("/upload_image?<name>", format="multipart/form-data", data="<data>")]
pub fn upload_image(name: String, content_type: &ContentType, data: Data) -> InternalResult<()> {
    let options = MultipartFormDataOptions::with_multipart_form_data_fields(vec![
        MultipartFormDataField::raw("image").size_limit(32 * 1024 * 1024).content_type_by_string(Some(mime::IMAGE_STAR)).unwrap()
    ]);

    let mut multipart_form_data = MultipartFormData::parse(content_type, data, options)?;

    let image = multipart_form_data.raw.remove("image");

    match image {
        Some(mut image) => {
            let raw = image.remove(0);

            let data = raw.raw;

            write_data_to_disk(&Path::new("images").join(Path::new(&name)), &data)?;
            Ok(())
        },
        None => {
            Err(InternalError::from("Not a file"))
        }
    }
}
