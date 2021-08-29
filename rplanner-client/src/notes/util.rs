use anyhow::Error;
use web_sys::HtmlElement;
use yew::utils;

use wasm_bindgen::{closure::Closure, JsCast, JsValue};

use super::api::*;

pub fn map_jsvalue_error<
    M: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    V: AsRef<JsValue>,
>(
    error_context: M,
) -> Box<dyn FnOnce(V) -> Error> {
    Box::new(move |e| match e.as_ref().as_string() {
        Some(v) => Error::msg(format!("{} {}", error_context, v)),
        None => Error::msg(format!(
            "{} Could not convert JsValue error into string",
            error_context
        )),
    })
}

pub trait JsValueErrorResult<T, E, V> {
    fn map_error_to_anyhow(self, error_context: E) -> Result<T, Error>;
}

impl<T, E, V> JsValueErrorResult<T, E, V> for Result<T, V>
where
    E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    V: AsRef<JsValue>,
{
    fn map_error_to_anyhow(self, error_context: E) -> Result<T, Error> {
        self.map_err(map_jsvalue_error(error_context))
    }
}

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
        Ok(selection) => match selection {
            Some(selection) => {
                let anchor_node = selection
                    .anchor_node()
                    .ok_or(Error::msg("Could not get selection anchor node"))?;
                let anchor_offset = selection.anchor_offset();

                let note_element = if anchor_node.node_type() == web_sys::Node::TEXT_NODE {
                    anchor_node
                        .parent_element()
                        .ok_or(Error::msg("Text anchor node has no parent element"))?
                        .dyn_into::<HtmlElement>()
                        .map_error_to_anyhow(
                            "Could not cast text anchor node parent element into HtmlElement",
                        )?
                } else {
                    anchor_node.dyn_into::<HtmlElement>().map_error_to_anyhow(
                        "Could not cast anchor node element into HtmlElement",
                    )?
                };

                let note_id = get_note_element_id(&note_element)?;
                let fragment_num = get_note_element_fragment_num(&note_element)?;

                Ok(CaretPosition {
                    note_id,
                    fragment_num,
                    index: anchor_offset,
                })
            }
            None => Err(Error::msg("Could not get window selection")),
        },
        Err(e) => Err(Error::msg(
            e.as_string().unwrap_or("No error provided".to_string()),
        )),
    }
}

pub fn get_note_text_fragments(
    note_id: NoteID,
    notes: &EnumeratedNotes,
) -> Result<Vec<(FragmentNum, String)>, Error> {
    let note = get_note_by_id(note_id, notes).ok_or(Error::msg("Could not get note by id"))?;

    Ok(note
        .content
        .iter()
        .enumerate()
        .filter_map(
            |(index, element): (usize, &NoteElement)| -> Option<(FragmentNum, String)> {
                match element {
                    NoteElement::Text(v) => Some((index as FragmentNum, v.clone())),
                    NoteElement::Image(_) => None,
                }
            },
        )
        .collect())
}

pub fn get_text_fragment_offset_from_sibling(
    note_id: NoteID,
    sibling_fragment: FragmentNum,
    offset: i32,
    notes: &EnumeratedNotes,
) -> Result<FragmentNum, Error> {
    let text_fragments = get_note_text_fragments(note_id, notes)?;

    let mut sibling_position: Option<usize> = None;
    for (index, (fragment_num, _)) in text_fragments.iter().enumerate() {
        if *fragment_num == sibling_fragment {
            sibling_position = Some(index);
        }
    }

    if let Some(sibling_position) = sibling_position {
        let target_fragment_position = sibling_position as i32 + offset;

        if target_fragment_position < 0 {
            return Err(Error::msg("Offset fragment from sibling out of bounds"));
        }

        return match text_fragments.get(target_fragment_position as usize) {
            Some((fragment_num, _)) => Ok(*fragment_num),
            None => Err(Error::msg("Offset fragment from sibling out of bounds")),
        };
    }

    Err(Error::msg(format!(
        "Sibling fragment not found when deriving offset"
    )))
}

pub fn get_next_text_fragment_num(
    note_id: NoteID,
    fragment_num: FragmentNum,
    notes: &EnumeratedNotes,
) -> Option<FragmentNum> {
    get_text_fragment_offset_from_sibling(note_id, fragment_num, 1, notes).ok()
}

pub fn get_previous_text_fragment_num(
    note_id: NoteID,
    fragment_num: FragmentNum,
    notes: &EnumeratedNotes,
) -> Option<FragmentNum> {
    get_text_fragment_offset_from_sibling(note_id, fragment_num, -1, notes).ok()
}

pub fn move_caret_into_position(position: CaretPosition) -> Result<(), Error> {
    let text_fragments = utils::document().get_elements_by_class_name("note-text");

    for index in 0..text_fragments.length() {
        let text_fragment = text_fragments
            .item(index)
            .unwrap()
            .dyn_into::<HtmlElement>()
            .map_error_to_anyhow("Could not cast text fragment node into HtmlElement")?;

        let note_id = get_note_element_id(&text_fragment)?;

        let fragment_num = get_note_element_fragment_num(&text_fragment)?;

        if note_id != position.note_id || fragment_num != position.fragment_num {
            continue;
        }

        let selection = utils::window()
            .get_selection()
            .map_error_to_anyhow("Cannot get window selection")?
            .ok_or(Error::msg("No window selection exists"))?;

        let range = utils::document()
            .create_range()
            .map_error_to_anyhow("Could not create document range")?;

        /* When we try to move the caret into a note fragment with no text,
         * we need to set the start of the range to be the note element as opposed
         * to what we do normally which is using the text node.
         */
        let (range_start_node, target_index) = match text_fragment.child_nodes().item(0) {
            Some(v) => (v, position.index),
            None => (web_sys::Node::from(text_fragment), 0),
        };

        range
            .set_start(&range_start_node, target_index)
            .map_error_to_anyhow("Could not set range start when moving caret")?;

        range.collapse();

        selection
            .remove_all_ranges()
            .map_error_to_anyhow("Could not remove all ranges from window selection")?;

        selection
            .add_range(&range)
            .map_error_to_anyhow("Could not add range to selection when moving caret")?;

        /* For some ridiculous reason, when we change the selection something
         * changes it to the start of the fragment, so we listen for the next
         * selection change and override it once
         * */
        utils::document().set_onselectionchange(Some(
            Closure::once_into_js(move || -> () {
                match try {
                    let range = utils::document()
                        .create_range()
                        .map_error_to_anyhow("Could not create document range")?;

                    range
                        .set_start(&range_start_node, target_index)
                        .map_error_to_anyhow("Could not set range start")?;
                    range.collapse();
                    selection.remove_all_ranges().map_error_to_anyhow(
                        "Could not remove all ranges from document selection",
                    )?;
                    selection
                        .add_range(&range)
                        .map_error_to_anyhow("Could not add new range to document selection")?;

                    utils::document().set_onselectionchange(None);
                    ()
                } {
                    Ok(_) => {}
                    Err(e) => log_error_to_js(e),
                };
            })
            .as_ref()
            .unchecked_ref(),
        ));

        return Ok(());
    }

    Err(Error::msg("Could not find position in existing notes"))
}

pub fn get_fragment_from_id(
    note_id: NoteID,
    fragment_num: FragmentNum,
    notes: &EnumeratedNotes,
) -> Result<&NoteElement, Error> {
    let note = get_note_by_id(note_id, notes).ok_or(Error::msg(format!(
        "Note with id {} does not exist",
        note_id
    )))?;

    Ok(note
        .content
        .get(fragment_num as usize)
        .ok_or(Error::msg(format!(
            "Fragment num {} does not exist in note with id {}",
            fragment_num, note_id
        )))?)
}

pub fn get_text_fragment_content(
    note_id: NoteID,
    fragment_num: FragmentNum,
    notes: &EnumeratedNotes,
) -> Result<&str, Error> {
    let fragment = get_fragment_from_id(note_id, fragment_num, notes)?;

    match fragment {
        NoteElement::Text(v) => Ok(v),
        NoteElement::Image(_) => Err(Error::msg(format!(
            "Expected text fragment at (note_id:fragment_num) {}:{} bu got image fragment",
            note_id, fragment_num
        ))),
    }
}

/* Returns a tuple of the offset of the caret within the line and the line number. */
pub fn get_line_based_position(
    position: &CaretPosition,
    notes: &EnumeratedNotes,
) -> Result<(usize, usize), Error> {
    let text_content = get_text_fragment_content(position.note_id, position.fragment_num, notes)?;

    let text_lines = text_content.split('\n').collect::<Vec<&str>>();

    let mut chars_so_far = 0;
    for (i, line) in text_lines.iter().enumerate() {
        // We add one to account for the newline we removed
        let chars_at_end_of_line = (line.chars().count() + chars_so_far) as usize;
        if chars_at_end_of_line >= position.index as usize {
            return Ok((position.index as usize - chars_so_far, i));
        }

        chars_so_far = chars_at_end_of_line + 1;
    }

    Err(Error::msg(format!(
        "No line at position given: {:?}",
        position
    )))
}
