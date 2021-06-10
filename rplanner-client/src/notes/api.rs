use serde::{Deserialize, Serialize};
use yew::format::Json;
use yew::services::fetch::Response;
use yew::services::ConsoleService;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NoteElement {
    Text(String),
    Image(String),
}

pub enum FragmentTag {
    Text,
    Image,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Note {
    pub content: Vec<NoteElement>,
    pub date: String,
}

pub type NoteID = i64;
pub type FragmentNum = i64;
pub type NoteFragment = (NoteID, NoteElement, FragmentNum);

#[derive(Serialize, Debug)]
pub struct AddNoteResult {
    pub note_id: NoteID,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetNoteRequest {
    pub note_id: i64,
    pub note: Note,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteNoteRequest {
    pub note_id: i64,
}

#[derive(Deserialize, Debug)]
pub struct ImageListResponse {
    pub images: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InsertImageRequest {
    pub note_id: NoteID,
    pub fragment_num: FragmentNum,
    pub index: usize,
    pub image_name: String,
}

#[derive(Deserialize, Debug)]
pub struct DeleteFragmentRequest {
    pub note_id: NoteID,
    pub fragment_num: FragmentNum,
}

pub type EnumeratedNotes = Vec<(NoteID, Note)>;

pub type JsonFetchResponse<T> = Response<Json<Result<T, anyhow::Error>>>;

pub fn log_error_to_js(e: anyhow::Error) {
    ConsoleService::log(format!("{}", e).as_str());
}

pub fn log_to_js(d: &impl std::fmt::Debug) {
    ConsoleService::log(format!("{:?}", d).as_str());
}

pub fn get_document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}
