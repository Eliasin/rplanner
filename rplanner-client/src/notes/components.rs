use std::collections::HashMap;
use std::convert::TryInto;
use std::time::Duration;
use web_sys::HtmlElement;

use chrono::offset::Utc;
use yew::services::fetch::{FetchService, FetchTask, Request};
use yew::services::interval::{IntervalService, IntervalTask};
use yew::services::reader::{FileData, ReaderService, ReaderTask};
use yew::{
    events::ChangeData,
    format::{Binary, Json, Nothing},
    prelude::*,
    utils,
};
use ModalEvent::OpenImageSelector;

use wasm_bindgen::JsCast;
use web_sys::File;

use anyhow::anyhow;
use anyhow::Error;

use super::api::InsertImageRequest as InsertImageRequestStruct;
use super::api::*;
use super::util::*;

use crate::root::agents::{EventBus, ModalEvent, NoteEvent, Request as BusRequest};

fn view_note_element(
    element: NoteElement,
    note_id: NoteID,
    order: u32,
    text_input_callback: Callback<InputData>,
    keypress_callback: Callback<KeyboardEvent>,
) -> Html {
    match &element {
        NoteElement::Text(v) => {
            html! {
                <div class=classes!("note") data-note-id=note_id.to_string() data-order=order.to_string() contentEditable="true" oninput=text_input_callback onkeypress=keypress_callback>{v}</div>
            }
        }
        NoteElement::Image(v) => {
            html! {
                <img class=classes!("note-content-image") src=format!("images/{}", v) alt="Note" />
            }
        }
    }
}

fn handle_keyboard_event(event: KeyboardEvent) -> Option<NotesComponentMsg> {
    match event.key().as_str() {
        "Backspace" => Some(NoteKeyEvent::Backspace.new_msg(event)),
        "ArrowUp" => Some(NoteKeyEvent::ArrowUp.new_msg(event)),
        "ArrowDown" => Some(NoteKeyEvent::ArrowDown.new_msg(event)),
        "Enter" => Some(NoteKeyEvent::Enter.new_msg(event)),
        _ => None,
    }
}

fn view_note(note_id: NoteID, note: &Note, link: &ComponentLink<NotesComponent>) -> Html {
    html! {
        <div class="noteBlock">
            <button class="noteButton noteImage" onclick={link.callback(|_| InternalNotesComponentMessage::open_image_modal_msg())}><i class="las la-image"/></button>
            <button class="noteButton noteDelete" onclick={link.callback(move |_| InternalNotesComponentMessage::delete_note_msg(note_id))}><i class="las la-times"/></button>
            <div class="note" id=format!("note-{}", note_id)>
            { note.content.iter().enumerate().map(|(i, f): (usize, &NoteElement)| {
                view_note_element(f.clone(), note_id, i.try_into().unwrap(), link.callback(move |_| {
                    InternalNotesComponentMessage::note_edited_msg(note_id)
                }),
                link.batch_callback(move |event: KeyboardEvent| handle_keyboard_event(event)))
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
pub enum NoteKeyEvent {
    Backspace,
    ArrowUp,
    ArrowDown,
    Enter,
}

impl NoteKeyEvent {
    pub fn new_msg(self, event: KeyboardEvent) -> NotesComponentMsg {
        NotesComponentMsg::Internal(InternalNotesComponentMessage::NoteKeyEvent(self, event))
    }
}

#[derive(Debug)]
pub enum InternalNotesComponentMessage {
    UpdateNotes,
    DeleteNote(NoteID),
    ReceivedNotes(Result<EnumeratedNotes, anyhow::Error>),
    NoteKeyEvent(NoteKeyEvent, KeyboardEvent),
    NoteEdited(NoteID),
    TickNoteTimers,
    AddNote,
    OpenImageModel,
    UploadImage(String, Vec<u8>),
    StartReadingImage(File),
}

impl InternalNotesComponentMessage {
    pub fn open_image_modal_msg() -> NotesComponentMsg {
        NotesComponentMsg::Internal(InternalNotesComponentMessage::OpenImageModel)
    }

    pub fn delete_note_msg(note_id: NoteID) -> NotesComponentMsg {
        NotesComponentMsg::Internal(InternalNotesComponentMessage::DeleteNote(note_id))
    }

    pub fn note_edited_msg(note_id: NoteID) -> NotesComponentMsg {
        NotesComponentMsg::Internal(InternalNotesComponentMessage::NoteEdited(note_id))
    }

    pub fn update_notes_msg() -> NotesComponentMsg {
        NotesComponentMsg::Internal(InternalNotesComponentMessage::UpdateNotes)
    }
}

#[derive(Debug)]
pub enum NotesComponentMsg {
    Internal(InternalNotesComponentMessage),
    NoteEvent(NoteEvent),
}

pub struct NotesComponent {
    _delete_fetch_task: Option<FetchTask>,
    _set_fetch_task: Option<FetchTask>,
    _get_fetch_task: Option<FetchTask>,
    _add_fetch_task: Option<FetchTask>,
    _interval_task: Option<IntervalTask>,
    _upload_image_fetch_task: Option<FetchTask>,
    _read_image_task: Option<ReaderTask>,
    _insert_image_task: Option<FetchTask>,
    _delete_note_fragment_fetch_task: Option<FetchTask>,
    event_bus: Box<dyn Bridge<EventBus>>,
    notes: EnumeratedNotes,
    link: ComponentLink<Self>,
    note_timers: HashMap<NoteID, NoteTimer>,
}

impl NotesComponent {
    fn handle_backspace_in_note(&mut self) -> Result<(), Error> {
        let position = get_caret_position()?;

        let note_id = position.noteID;

        let at_beginning_of_fragment = position.index == 0;

        if at_beginning_of_fragment {
            let last_fragment_num =
                get_last_fragment_num(note_id, position.fragmentNum, &self.notes)
                    .ok_or(Error::msg("No fragments to delete"))?;

            self.delete_note_fragment(note_id, last_fragment_num)?;
        }

        Ok(())
    }

    fn handle_enter_in_note(&mut self, keyboard_event: KeyboardEvent) -> Result<(), Error> {
        /* The default enter key behaviour in content editable divs is to insert new div elements with
         * br elements inside so we must change it to instead insert newlines
         */
        let raw_event = AsRef::<web_sys::Event>::as_ref(&keyboard_event);
        raw_event.prevent_default();
        match utils::window().get_selection() {
            Ok(selection) => match selection {
                Some(selection) => {
                    let anchor_offset = selection.anchor_offset();

                    let note_element = raw_event
                        .target()
                        .ok_or(Error::msg("Could not get enter key event target"))?
                        .dyn_into::<HtmlElement>()
                        .map_err(|_| {
                            Error::msg("Could not cast enter key event target into HtmlElement")
                        })?;

                    let note_id = get_note_element_id(&note_element)?;
                    let note_text = note_element.text_content().ok_or(Error::msg(
                        "Cannot proceed with enter key event as note node does not have text",
                    ))?;

                    let (start_text, end_text) = note_text.split_at(anchor_offset as usize);

                    note_element
                        .set_text_content(Some(format!("{}\n{}", start_text, end_text).as_str()));

                    let range = utils::document()
                        .create_range()
                        .map_err(|_| Error::msg("Could not create js range"))?;

                    let first_child_node = note_element
                        .child_nodes()
                        .item(0)
                        .ok_or(Error::msg("Element does not have child nodes"))?;

                    range
                        .set_start(&first_child_node, anchor_offset + 1)
                        .map_err(|_| Error::msg("Could not set range start"))?;

                    selection
                        .remove_all_ranges()
                        .map_err(|_| Error::msg("Could not remove ranges from selection"))?;

                    selection
                        .add_range(&range)
                        .map_err(|_| Error::msg("Could not add range to selection"))?;

                    self.link
                        .send_message(InternalNotesComponentMessage::note_edited_msg(note_id));

                    Ok(())
                }
                None => Err(Error::msg("Could not get window selection")),
            },
            Err(e) => Err(Error::msg(
                e.as_string().unwrap_or("No error provided".to_string()),
            )),
        }
    }

    fn delete_note_fragment(
        &mut self,
        note_id: NoteID,
        fragment_num: FragmentNum,
    ) -> Result<(), anyhow::Error> {
        let delete_fragment_request = DeleteFragmentRequest {
            note_id,
            fragment_num,
        };

        let request = Request::post("/api/delete_fragment")
            .header("Content-Type", "application/json")
            .body(Json(&delete_fragment_request))?;

        let callback = self
            .link
            .callback(|_: JsonFetchResponse<()>| InternalNotesComponentMessage::update_notes_msg());

        let task = FetchService::fetch(request, callback)?;

        self._delete_note_fragment_fetch_task = Some(task);

        Ok(())
    }

    fn delete_note(&mut self, note_id: NoteID) -> Result<(), anyhow::Error> {
        let request_object = DeleteNoteRequest { note_id };

        let request = Request::post("/api/delete_note")
            .header("Content-Type", "application/json")
            .body(Json(&request_object))?;

        let callback = self
            .link
            .callback(|_: JsonFetchResponse<()>| InternalNotesComponentMessage::update_notes_msg());

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

            let callback = self.link.batch_callback(|_: JsonFetchResponse<()>| None);

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
            .callback(|_: JsonFetchResponse<()>| InternalNotesComponentMessage::update_notes_msg());

        let task = FetchService::fetch(request, callback)?;

        self._add_fetch_task = Some(task);

        Ok(())
    }

    fn upload_image(&mut self, name: &str, image_bytes: Vec<u8>) -> Result<(), anyhow::Error> {
        let request =
            Request::post(format!("/api/upload_image/{}", name)).body(Binary::Ok(image_bytes))?;

        let callback = self.link.batch_callback(|_| None);

        let task = FetchService::fetch_binary::<Binary, Binary>(request, callback)?;

        self._upload_image_fetch_task = Some(task);

        Ok(())
    }
}

impl NotesComponent {
    fn reset_note_flush_timer(&mut self, note_id: NoteID) {
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
    }

    fn update_internal(&mut self, msg: InternalNotesComponentMessage) -> bool {
        use InternalNotesComponentMessage::*;
        match msg {
            NoteKeyEvent(event_type, event) => {
                use self::NoteKeyEvent::*;
                let result = match event_type {
                    Backspace => self.handle_backspace_in_note(),
                    Enter => self.handle_enter_in_note(event),
                    _ => Err(Error::msg("Unimplemented key event")),
                };

                match result {
                    Ok(_) => {}
                    Err(e) => {
                        log_error_to_js(e);
                    }
                }

                true
            }
            UpdateNotes => {
                let request = Request::get("/api/get_notes").body(Nothing).unwrap();

                let callback =
                    self.link
                        .callback(|response: JsonFetchResponse<EnumeratedNotes>| {
                            let Json(data) = response.into_body();
                            NotesComponentMsg::Internal(ReceivedNotes(data))
                        });

                let task = FetchService::fetch(request, callback).unwrap();

                self._get_fetch_task = Some(task);

                false
            }
            ReceivedNotes(notes) => match notes {
                Ok(mut notes) => {
                    notes.sort_by(|a, b| a.0.cmp(&b.0));
                    self.notes = notes;
                    true
                }
                Err(e) => {
                    log_error_to_js(e);
                    false
                }
            },
            NoteEdited(note_id) => {
                self.reset_note_flush_timer(note_id);
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
            OpenImageModel => {
                self.event_bus
                    .send(BusRequest::ModalEvent(OpenImageSelector));
                true
            }
            UploadImage(name, image_bytes) => {
                match self.upload_image(name.as_str(), image_bytes) {
                    Ok(_) => {}
                    Err(e) => log_error_to_js(e),
                };

                false
            }
            StartReadingImage(file) => {
                match ReaderService::read_file(
                    file,
                    self.link.batch_callback(|file_data: FileData| {
                        Some(NotesComponentMsg::Internal(UploadImage(
                            file_data.name,
                            file_data.content,
                        )))
                    }),
                ) {
                    Ok(reader_task) => {
                        self._read_image_task = Some(reader_task);
                    }
                    Err(e) => {
                        log_error_to_js(e);
                    }
                };
                false
            }
        }
    }

    fn update_note_events(&mut self, msg: NoteEvent) -> bool {
        use NoteEvent::*;
        match msg {
            InsertImageRequest(path) => {
                let caret_position = match get_caret_position() {
                    Ok(v) => v,
                    Err(e) => {
                        log_error_to_js(e);
                        return false;
                    }
                };

                let insert_image_request = InsertImageRequestStruct {
                    note_id: caret_position.noteID,
                    fragment_num: caret_position.fragmentNum,
                    index: caret_position.index as usize,
                    image_name: path,
                };

                let request = match Request::post("/api/insert_image")
                    .header("Content-Type", "application/json")
                    .body(Json(&insert_image_request))
                {
                    Ok(v) => v,
                    Err(e) => {
                        log_error_to_js(anyhow::Error::new(e));
                        return false;
                    }
                };

                let callback = self.link.callback(|_: JsonFetchResponse<()>| {
                    NotesComponentMsg::Internal(InternalNotesComponentMessage::UpdateNotes)
                });

                let task = match FetchService::fetch(request, callback) {
                    Ok(v) => v,
                    Err(e) => {
                        log_error_to_js(e);
                        return false;
                    }
                };

                log_to_js(&"HERE");

                self._insert_image_task = Some(task);

                false
            }
        }
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
            _upload_image_fetch_task: None,
            _read_image_task: None,
            _insert_image_task: None,
            _delete_note_fragment_fetch_task: None,
            event_bus: EventBus::bridge(link.batch_callback(|msg| match msg {
                BusRequest::NoteEvent(msg) => Some(NotesComponentMsg::NoteEvent(msg)),
                _ => None,
            })),
            link,
            notes: vec![],
            note_timers: HashMap::new(),
        }
    }

    fn view(&self) -> Html {
        use InternalNotesComponentMessage::*;
        html! {
            <>
            { self.view_notes() }
            <div class="functionBar">
                <button class="addNote" onclick=self.link.callback(|_| NotesComponentMsg::Internal(AddNote))><i class=classes!("las", "la-plus") /></button>
                <label class="image-file-upload">
                <input type="file" accept=".png,.jpg" onchange=self.link.batch_callback(|event| {
                    match event {
                        ChangeData::Files(file_list) => {
                            for file_num in 0..file_list.length() {
                                let file = file_list.get(file_num);
                                match file {
                                    Some(file) => {
                                        return Some(NotesComponentMsg::Internal(StartReadingImage(file)));
                                    },
                                    None => {
                                        return None;
                                    },
                                }
                            };
                            None
                        },
                        _ => {
                            None
                        }
                    }
                })/>
                    <i class=classes!("las", "la-file-image") />
                </label>
            </div>
            </>
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            NotesComponentMsg::Internal(msg) => self.update_internal(msg),
            NotesComponentMsg::NoteEvent(msg) => self.update_note_events(msg),
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        true
    }

    fn rendered(&mut self, first_render: bool) {
        use InternalNotesComponentMessage::*;
        if first_render {
            self.link
                .send_message(NotesComponentMsg::Internal(UpdateNotes));
            self._interval_task = Some(IntervalService::spawn(
                Duration::new(1, 0),
                self.link
                    .callback(|_| NotesComponentMsg::Internal(TickNoteTimers)),
            ));
        }
    }

    fn destroy(&mut self) {}
}
