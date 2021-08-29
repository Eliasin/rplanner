use rocket::data::{Data, ToByteUnit};
use rocket::serde::json::Json;
use rocket::{get, post, State};

use std::path::Path;
use tokio::fs::File;

use crate::internal_error::{InternalError, InternalResult};

use super::data::*;
use super::helpers::*;
use crate::data::DBConnection;

#[get("/get_notes")]
pub fn get_notes(db_connection: &State<DBConnection>) -> InternalResult<Json<Vec<(NoteID, Note)>>> {
    let db_connection = db_connection.lock()?;

    let notes = get_all_notes_from_db(&db_connection)?;

    Ok(Json(notes))
}

#[post("/add_note", format = "json", data = "<note>")]
pub fn add_note(
    note: Json<Note>,
    db_connection: &State<DBConnection>,
) -> InternalResult<Json<AddNoteResult>> {
    let db_connection = db_connection.lock()?;
    add_note_to_db(note.into_inner(), &db_connection).map(|r| Json(r))
}

#[post("/set_note", format = "json", data = "<set_note_request>")]
pub fn set_note(
    set_note_request: Json<SetNoteRequest>,
    db_connection: &State<DBConnection>,
) -> InternalResult<()> {
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
pub fn delete_note(
    delete_note_request: Json<DeleteNoteRequest>,
    db_connection: &State<DBConnection>,
) -> InternalResult<()> {
    let db_connection = db_connection.lock()?;

    let note_id = delete_note_request.note_id;
    delete_note_contents_from_db(note_id, &db_connection)?;
    delete_note_from_db(note_id, &db_connection)?;

    Ok(())
}

const MAX_IMAGE_SIZE_KIB: usize = 2048;
#[post("/upload_image/<name>", data = "<data>")]
pub async fn upload_image(data: Data<'_>, name: &str) -> InternalResult<()> {
    let image_folder_path = Path::new("images");

    let image_file_path = image_folder_path.join(Path::new(name));
    if validate_path_is_in_image_folder(&image_file_path) {
        data.open(MAX_IMAGE_SIZE_KIB.kibibytes())
            .stream_to(File::create(&image_file_path).await?)
            .await?;
    } else {
        return Err(InternalError::from("Invalid image name"));
    }

    Ok(())
}

#[get("/get_image_list")]
pub fn get_image_list() -> InternalResult<Json<ImageListResponse>> {
    let images = ImageListResponse {
        images: get_image_filenames()?,
    };

    Ok(Json(images))
}

#[post("/insert_image", format = "json", data = "<insert_image_request>")]
pub fn insert_image(
    insert_image_request: Json<InsertImageRequest>,
    db_connection: &State<DBConnection>,
) -> InternalResult<()> {
    let db_connection = db_connection.lock()?;

    let mut note = get_note_from_db(insert_image_request.note_id, &db_connection)?;

    insert_image_into_note(
        &mut note,
        insert_image_request.fragment_num,
        insert_image_request.index,
        &insert_image_request.image_name,
    )?;
    delete_note_contents_from_db(insert_image_request.note_id, &db_connection)?;

    add_note_contents_to_db(
        insert_image_request.note_id,
        note.content.clone().into_iter(),
        &*db_connection,
    )?;

    Ok(())
}

#[post(
    "/delete_fragment",
    format = "json",
    data = "<delete_fragment_request>"
)]
pub fn delete_fragment(
    delete_fragment_request: Json<DeleteFragmentRequest>,
    db_connection: &State<DBConnection>,
) -> InternalResult<()> {
    let db_connection = db_connection.lock()?;

    let mut note = get_note_from_db(delete_fragment_request.note_id, &db_connection)?;
    remove_fragment_from_note(&mut note, delete_fragment_request.fragment_num)?;

    delete_note_contents_from_db(delete_fragment_request.note_id, &db_connection)?;
    add_note_contents_to_db(
        delete_fragment_request.note_id,
        note.content.into_iter(),
        &db_connection,
    )?;

    Ok(())
}
