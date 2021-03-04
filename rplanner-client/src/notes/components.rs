use std::collections::HashMap;
use std::time::Duration;

use yew::{format::{Json, Nothing}, prelude::*};
use yew::services::fetch::{FetchService, FetchTask, Request};
use yew::services::interval::{ IntervalService, IntervalTask };
use chrono::offset::Utc;

use anyhow::anyhow;

use wasm_bindgen::JsCast;

use super::api::*;

#[derive(Properties, Clone)]
pub struct NoteFragmentProps {
    note_id: NoteID,
    fragment_num: FragmentNum,
    content: NoteElement,
    send_message: Option<Callback<NotesComponentMsg>>,
}

pub struct NoteFragmentComponent {
    props: NoteFragmentProps,
    link: ComponentLink<Self>,
}

pub enum NoteFragmentComponentMessage {
    InputEvent,
}

impl Component for NoteFragmentComponent {
    type Message = NoteFragmentComponentMessage;
    type Properties = NoteFragmentProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        NoteFragmentComponent {
            props,
            link,
        }
    }

    fn view(&self) -> Html {
        match &self.props.content {
            NoteElement::Text(v) => {
                let input_handler = self.link.callback(|_: InputData| NoteFragmentComponentMessage::InputEvent);
                html! {
                    <div class=vec!["note"] contentEditable=true oninput=input_handler>{v}</div>
                }
            },
            NoteElement::Image(v) => {
                html! {
                    <img class=vec!["noteImage"] src=format!("images/{}", v) alt="Note" />
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            NoteFragmentComponentMessage::InputEvent => {
                if let Some(send_message) = &self.props.send_message {
                    send_message.emit(NotesComponentMsg::EditNote(self.props.note_id));
                }
                false
            }
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        true
    }

    fn rendered(&mut self, _first_render: bool) {}

    fn destroy(&mut self) {}
}

#[derive(Properties, Clone)]
pub struct NoteComponentProps {
    note_id: NoteID,
    note: Note,
    send_message: Option<Callback<NotesComponentMsg>>,
}

pub struct NoteComponent {
    props: NoteComponentProps,
    link: ComponentLink<Self>,
    _delete_task: Option<FetchTask>,
}

pub enum NoteComponentMessage {
    DeleteNote,
    UpdateNotes,
}

impl NoteComponent {

    fn delete_note(&mut self) -> Result<(), anyhow::Error> {
        let request_object = DeleteNoteRequest {
                                note_id: self.props.note_id,
                            };

        let request = Request::post("/api/delete_note")
            .header("Content-Type", "application/json")
            .body(Json(&request_object))?;

        let callback = self.link.callback(|_: JsonFetchResponse<()>| NoteComponentMessage::UpdateNotes);

        let task = FetchService::fetch(request, callback)?;

        self._delete_task = Some(task);

        Ok(())
    }

}

impl Component for NoteComponent {
    type Message = NoteComponentMessage;
    type Properties = NoteComponentProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        NoteComponent {
            props,
            link,
            _delete_task: None,
        }
    }

    fn view(&self) -> Html {
        html! {
            <div class="noteBlock">
                <button class="noteButton noteImage"><i class="las la-image"/></button>
                <button class="noteButton noteDelete" onclick=self.link.callback(|_| NoteComponentMessage::DeleteNote)><i class="las la-times"/></button>
                <div class="note" id=format!("note-{}", self.props.note_id)>
                { self.props.note.content.iter().enumerate().map(|(id, f): (usize, &NoteElement)| {
                    html! {
                        <NoteFragmentComponent note_id=self.props.note_id fragment_num={id as i64} content=f.clone() send_message=self.props.send_message.clone() />
                    }
                }).collect::<Html>() }
                </div>
            </div>
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            NoteComponentMessage::DeleteNote => {
                self.delete_note();
                false
            },
            NoteComponentMessage::UpdateNotes => {
                 match &self.props.send_message {
                    Some(send_message) => {
                        send_message.emit(NotesComponentMsg::UpdateNotes)
                    },
                    None => (),
                };

                false
            },
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        true
    }

    fn rendered(&mut self, _first_render: bool) {}

    fn destroy(&mut self) {}
}

#[derive(Clone)]
struct NoteTimer {
    ticks_since_last_edit: u32,
}

#[derive(Debug)]
pub enum NotesComponentMsg {
    UpdateNotes,
    ReceivedNotes(Result<EnumeratedNotes, anyhow::Error>),
    EditNote(NoteID),
    TickNoteTimers,
    AddNote,
    Noop,
}

pub struct NotesComponent {
    _set_fetch_task: Option<FetchTask>,
    _get_fetch_task: Option<FetchTask>,
    _add_fetch_task: Option<FetchTask>,
    _interval_task: Option<IntervalTask>,
    notes: EnumeratedNotes,
    link: ComponentLink<Self>,
    note_timers: HashMap<NoteID, NoteTimer>,
}

impl NotesComponent {

    fn view_notes(&self) -> Html {
        html! {
            <div class="notes">
            { self.notes.iter().map(|(id, note)| {
                html! {
                    <NoteComponent note_id=id note=note send_message=self.link.callback(|msg| msg) />
                }
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

                    let src_string = element.dyn_into::<web_sys::HtmlImageElement>().unwrap().src();
                    let url = get_document().url().unwrap();

                    let prefile_string = url + "/image/";

                    content.push(NoteElement::Image(src_string[prefile_string.len() - 1..].to_string()));

                } else if element.tag_name() == "DIV" {
                    content.push(NoteElement::Text(
                        element.text_content().unwrap_or(String::new())
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
        if let Some(note_element) = get_document().get_element_by_id(format!("note-{}", note_id).as_str()) {
            let note_content = NotesComponent::construct_note_from_fragment_divs(note_element.children());

            let note = Note {
                content: note_content,
                date: Utc::now().to_rfc2822(),
            };

            let request_object = SetNoteRequest {
                                    note,
                                    note_id,
                                };

            let request = Request::post("/api/set_note")
                .header("Content-Type", "application/json")
                .body(Json(&request_object))?;

            let callback = self.link.callback(|_: JsonFetchResponse<()>| NotesComponentMsg::Noop);

            let task = FetchService::fetch(request, callback)?;

            self._set_fetch_task = Some(task);

            Ok(())
        } else {
            Err(anyhow!("Cannot find note elements for note id: {}", note_id))
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

        let callback = self.link.callback(|_: JsonFetchResponse<()>| NotesComponentMsg::UpdateNotes);

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
        match msg {
            NotesComponentMsg::UpdateNotes => {
                let request = Request::get("/api/get_notes")
                    .body(Nothing)
                    .unwrap();

                let callback = self.link.callback(|response: JsonFetchResponse<EnumeratedNotes>| {
                    let Json(data) = response.into_body();
                    NotesComponentMsg::ReceivedNotes(data)
                });

                let task = FetchService::fetch(request, callback).unwrap();

                self._get_fetch_task = Some(task);

                false
            },
            NotesComponentMsg::ReceivedNotes(notes) => {
                match notes {
                    Ok(notes) => {
                        self.notes = notes;
                        true
                    },
                    Err(e) => {
                        log_error_to_js(e);
                        false
                    },
                }
            },
            NotesComponentMsg::EditNote(note_id) => {
                match self.note_timers.get_mut(&note_id) {
                    Some(timer) => {
                        timer.ticks_since_last_edit = 0;
                    },
                    None => {
                        self.note_timers.insert(note_id, NoteTimer { ticks_since_last_edit: 0 });
                    }
                };

                false
            },
            NotesComponentMsg::TickNoteTimers => {
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
            },
            NotesComponentMsg::AddNote => {
                if let Err(e) = self.add_note() {
                    log_error_to_js(e);
                }
                false
            },
            NotesComponentMsg::Noop => false,
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        true
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            self.link.send_message(NotesComponentMsg::UpdateNotes);
            self._interval_task = Some(IntervalService::spawn(Duration::new(1, 0), self.link.callback(|_| NotesComponentMsg::TickNoteTimers)));
        }
    }

    fn destroy(&mut self) {}
}
