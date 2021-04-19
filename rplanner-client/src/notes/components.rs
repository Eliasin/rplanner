use std::collections::HashMap;
use std::time::Duration;

use chrono::offset::Utc;
use yew::services::fetch::{FetchService, FetchTask, Request};
use yew::services::interval::{IntervalService, IntervalTask};
use yew::{
    format::{Json, Nothing},
    prelude::*,
};

use anyhow::anyhow;

use wasm_bindgen::JsCast;

use super::api::*;

fn view_note_element(element: NoteElement, text_input_callback: Callback<InputData>) -> Html {
    match &element {
        NoteElement::Text(v) => {
            html! {
                <div class=vec!["note"] contentEditable=true oninput=text_input_callback>{v}</div>
            }
        }
        NoteElement::Image(v) => {
            html! {
                <img class=vec!["noteImage"] src=format!("images/{}", v) alt="Note" />
            }
        }
    }
}

fn view_note(note_id: NoteID, note: &Note, link: &ComponentLink<NotesComponent>) -> Html {
    html! {
        <div class="noteBlock">
            <button class="noteButton noteImage"><i class="las la-image"/></button>
            <button class="noteButton noteDelete" onclick={link.callback(move |_| NotesComponentMsg::DeleteNote(note_id))}><i class="las la-times"/></button>
            <div class="note" id=format!("note-{}", note_id)>
            { note.content.iter().map(|f: &NoteElement| {
                view_note_element(f.clone(), link.callback(move |_| NotesComponentMsg::EditNote(note_id)))
            }).collect::<Html>() }
            </div>
        </div>
    }
}

#[derive(Clone)]
struct NoteTimer {
    ticks_since_last_edit: u32,
}

#[derive(Debug)]
pub enum NotesComponentMsg {
    UpdateNotes,
    DeleteNote(NoteID),
    ReceivedNotes(Result<EnumeratedNotes, anyhow::Error>),
    EditNote(NoteID),
    TickNoteTimers,
    AddNote,
    Noop,
}

pub struct NotesComponent {
    _delete_fetch_task: Option<FetchTask>,
    _set_fetch_task: Option<FetchTask>,
    _get_fetch_task: Option<FetchTask>,
    _add_fetch_task: Option<FetchTask>,
    _interval_task: Option<IntervalTask>,
    notes: EnumeratedNotes,
    link: ComponentLink<Self>,
    note_timers: HashMap<NoteID, NoteTimer>,
}

impl NotesComponent {
    fn delete_note(&mut self, note_id: NoteID) -> Result<(), anyhow::Error> {
        let request_object = DeleteNoteRequest { note_id };

        let request = Request::post("/api/delete_note")
            .header("Content-Type", "application/json")
            .body(Json(&request_object))?;

        let callback = self
            .link
            .callback(|_: JsonFetchResponse<()>| NotesComponentMsg::UpdateNotes);

        let task = FetchService::fetch(request, callback)?;

        self._delete_fetch_task = Some(task);

        Ok(())
    }

    fn view_notes(&self) -> Html {
        html! {
            <div class="notes">
            { self.notes.iter().map(|(id, note)| {
                view_note(*id, note, &self.link)
            }).collect::<Html>() }
            </div>
        }
    }

    fn construct_note_from_fragment_divs(divs: web_sys::HtmlCollection) -> Vec<NoteElement> {
        let mut content = vec![];
        let mut index: u32 = 0;

        while divs.item(index).is_some() {
            if let Some(element) = divs.item(index) {
                if element.tag_name() == "IMG" {
                    let src_string = element
                        .dyn_into::<web_sys::HtmlImageElement>()
                        .unwrap()
                        .src();
                    let url = get_document().url().unwrap();

                    let prefile_string = url + "/image/";

                    content.push(NoteElement::Image(
                        src_string[prefile_string.len() - 1..].to_string(),
                    ));
                } else if element.tag_name() == "DIV" {
                    content.push(NoteElement::Text(
                        element.text_content().unwrap_or(String::new()),
                    ));
                }

                index += 1;
            } else {
                break;
            }
        }

        return content;
    }

    fn flush_note_change(&mut self, note_id: NoteID) -> Result<(), anyhow::Error> {
        if let Some(note_element) =
            get_document().get_element_by_id(format!("note-{}", note_id).as_str())
        {
            let note_content =
                NotesComponent::construct_note_from_fragment_divs(note_element.children());

            let note = Note {
                content: note_content,
                date: Utc::now().to_rfc2822(),
            };

            let request_object = SetNoteRequest { note, note_id };

            let request = Request::post("/api/set_note")
                .header("Content-Type", "application/json")
                .body(Json(&request_object))?;

            let callback = self
                .link
                .callback(|_: JsonFetchResponse<()>| NotesComponentMsg::Noop);

            let task = FetchService::fetch(request, callback)?;

            self._set_fetch_task = Some(task);

            Ok(())
        } else {
            Err(anyhow!(
                "Cannot find note elements for note id: {}",
                note_id
            ))
        }
    }

    fn add_note(&mut self) -> Result<(), anyhow::Error> {
        let note = Note {
            content: vec![NoteElement::Text("New note...".to_string())],
            date: Utc::now().to_rfc2822(),
        };

        let request = Request::post("/api/add_note")
            .header("Content-Type", "application/json")
            .body(Json(&note))?;

        let callback = self
            .link
            .callback(|_: JsonFetchResponse<()>| NotesComponentMsg::UpdateNotes);

        let task = FetchService::fetch(request, callback)?;

        self._add_fetch_task = Some(task);

        Ok(())
    }
}

impl Component for NotesComponent {
    type Message = NotesComponentMsg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        NotesComponent {
            _delete_fetch_task: None,
            _interval_task: None,
            _get_fetch_task: None,
            _set_fetch_task: None,
            _add_fetch_task: None,
            link,
            notes: vec![],
            note_timers: HashMap::new(),
        }
    }

    fn view(&self) -> Html {
        html! {
            <>
            { self.view_notes() }
            <div class="functionBar">
                <button class="addNote" onclick=self.link.callback(|_| NotesComponentMsg::AddNote)><i class="las la-plus" /></button>
            </div>
            </>
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        use NotesComponentMsg::*;
        match msg {
            UpdateNotes => {
                let request = Request::get("/api/get_notes").body(Nothing).unwrap();

                let callback =
                    self.link
                        .callback(|response: JsonFetchResponse<EnumeratedNotes>| {
                            let Json(data) = response.into_body();
                            NotesComponentMsg::ReceivedNotes(data)
                        });

                let task = FetchService::fetch(request, callback).unwrap();

                self._get_fetch_task = Some(task);

                false
            }
            ReceivedNotes(notes) => match notes {
                Ok(notes) => {
                    self.notes = notes;
                    true
                }
                Err(e) => {
                    log_error_to_js(e);
                    false
                }
            },
            EditNote(note_id) => {
                match self.note_timers.get_mut(&note_id) {
                    Some(timer) => {
                        timer.ticks_since_last_edit = 0;
                    }
                    None => {
                        self.note_timers.insert(
                            note_id,
                            NoteTimer {
                                ticks_since_last_edit: 0,
                            },
                        );
                    }
                };

                false
            }
            TickNoteTimers => {
                const TICKS_BEFORE_FLUSH: u32 = 4;

                let mut need_flush = vec![];

                for (note_id, timer) in self.note_timers.iter_mut() {
                    if timer.ticks_since_last_edit == TICKS_BEFORE_FLUSH {
                        continue;
                    }

                    timer.ticks_since_last_edit += 1;
                    if timer.ticks_since_last_edit == TICKS_BEFORE_FLUSH {
                        need_flush.push(*note_id);
                    }
                }

                for note_id in need_flush {
                    if let Err(e) = self.flush_note_change(note_id) {
                        log_error_to_js(e);
                    }
                }

                false
            }
            AddNote => {
                if let Err(e) = self.add_note() {
                    log_error_to_js(e);
                }
                false
            }
            DeleteNote(note_id) => {
                match self.delete_note(note_id) {
                    Ok(_) => {}
                    Err(e) => {
                        log_error_to_js(e);
                    }
                };
                true
            }
            Noop => false,
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        true
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            self.link.send_message(NotesComponentMsg::UpdateNotes);
            self._interval_task = Some(IntervalService::spawn(
                Duration::new(1, 0),
                self.link.callback(|_| NotesComponentMsg::TickNoteTimers),
            ));
        }
    }

    fn destroy(&mut self) {}
}
