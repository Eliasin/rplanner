use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NoteElement {
    Text(String),
    Image(String),
}

pub enum FragmentTag {
    Text,
    Image,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Note {
    pub content: Vec<NoteElement>,
    pub date: String,
}

pub type NoteID = i64;
pub type FragmentNum = i64;
pub type NoteFragment = (NoteID, NoteElement, FragmentNum);

pub type FragmentMap = HashMap<NoteID, Vec<(NoteElement, FragmentNum)>>;

#[derive(Serialize, Debug)]
pub struct AddNoteResult {
    pub note_id: NoteID,
}

#[derive(Deserialize, Debug)]
pub struct SetNoteRequest {
    pub note_id: i64,
    pub note: Note,
}

#[derive(Deserialize, Debug)]
pub struct DeleteNoteRequest {
    pub note_id: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct ImageListResponse {
    pub images: Vec<String>,
}

#[derive(Deserialize, Debug)]
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
