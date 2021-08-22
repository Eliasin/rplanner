use anyhow::Error;
use web_sys::HtmlElement;
use yew::utils;

use wasm_bindgen::JsCast;

use super::api::*;

pub fn get_note_by_id<'a>(note_id: NoteID, notes: &'a EnumeratedNotes) -> Option<&'a Note> {
    for (id, note) in notes.iter() {
        if *id == note_id {
            return Some(&note);
        }
    }

    None
}

pub fn get_last_fragment_num(
    note_id: NoteID,
    fragment_num: FragmentNum,
    notes: &EnumeratedNotes,
) -> Option<FragmentNum> {
    let note = get_note_by_id(note_id, notes)?;

    if fragment_num - 1 > 0 && ((fragment_num - 1) as usize) < note.content.len() {
        return Some(fragment_num - 1);
    }

    None
}

pub fn get_note_element_id(note_element: &HtmlElement) -> Result<NoteID, Error> {
    let dataset = note_element.dataset();

    let dataset_string: String = dataset
        .get("noteId")
        .ok_or(Error::msg("Could not get noteId from element dataset"))?;

    Ok(dataset_string.parse::<FragmentNum>()?)
}

pub fn get_note_element_fragment_num(note_element: &HtmlElement) -> Result<FragmentNum, Error> {
    let dataset = note_element.dataset();

    let dataset_string: String = dataset.get("order").ok_or(Error::msg(
        "Could not get note fragment order from element dataset",
    ))?;

    Ok(dataset_string.parse::<FragmentNum>()?)
}

pub fn get_caret_position() -> Result<CaretPosition, Error> {
    match utils::window().get_selection() {
        Ok(selection) => {
            match selection {
                Some(selection) => {
                    let anchor_node = selection
                        .anchor_node()
                        .ok_or(Error::msg("Could not get selection anchor node"))?;
                    let anchor_offset = selection.anchor_offset();

                    let note_element = if anchor_node.node_type() == web_sys::Node::TEXT_NODE {
                        anchor_node
                        .parent_element()
                        .ok_or(Error::msg("Text anchor node has no parent element"))?
                        .dyn_into::<HtmlElement>().map_err(|_| Error::msg("Could not cast text anchor node parent element into HtmlElement"))?
                    } else {
                        anchor_node.dyn_into::<HtmlElement>().map_err(|_| {
                            Error::msg("Could not cast anchor node element into HtmlElement")
                        })?
                    };

                    let note_id = get_note_element_id(&note_element)?;
                    let fragment_num = get_note_element_fragment_num(&note_element)?;

                    Ok(CaretPosition {
                        noteID: note_id,
                        fragmentNum: fragment_num,
                        index: anchor_offset,
                    })
                }
                None => Err(Error::msg("Could not get window selection")),
            }
        }
        Err(e) => Err(Error::msg(
            e.as_string().unwrap_or("No error provided".to_string()),
        )),
    }
}
