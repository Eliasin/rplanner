use web_sys::HtmlElement;
use yew::utils;

use wasm_bindgen::JsCast;

use super::api::*;

pub fn get_note_element_id(note_element: &HtmlElement) -> Option<NoteID> {
    let dataset = note_element.dataset();

    Some(dataset.get("noteId")?.parse().ok()?)
}

pub fn get_note_element_fragment_num(note_element: &HtmlElement) -> Option<FragmentNum> {
    let dataset = note_element.dataset();

    Some(dataset.get("order")?.parse().ok()?)
}

pub fn get_caret_position() -> Option<CaretPosition> {
    match utils::window().get_selection() {
        Ok(selection) => match selection {
            Some(selection) => {
                let anchor_node = selection.anchor_node()?;
                let anchor_offset = selection.anchor_offset();

                let note_element = if anchor_node.node_type() == web_sys::Node::TEXT_NODE {
                    anchor_node
                        .parent_element()?
                        .dyn_into::<HtmlElement>()
                        .ok()?
                } else {
                    anchor_node.dyn_into::<HtmlElement>().ok()?
                };

                let note_id = get_note_element_id(&note_element)?;
                let fragment_num = get_note_element_fragment_num(&note_element)?;

                Some(CaretPosition {
                    noteID: note_id,
                    fragmentNum: fragment_num,
                    index: anchor_offset,
                })
            }
            None => None,
        },
        Err(_) => None,
    }
}
