use yew::prelude::*;
use yew::{
    format::{Json, Nothing},
    services::fetch::{FetchService, FetchTask, Request},
};

use web_sys::HtmlElement;

use super::agents::{EventBus, ModalEvent, NoteEvent, Request as BusRequest};
use crate::notes::api::{log_to_js, ImageListResponse, JsonFetchResponse};
use crate::notes::components::NotesComponent;

pub enum ModalImageSelectorMessage {
    GetImages,
    ReceivedImages(Vec<String>),
    InsertImageRequest(String),
}

#[derive(Properties, Clone)]
pub struct ModalImageSelectorProps {
    insert_image_callback: Callback<String>,
}

pub struct ModalImageSelector {
    _get_image_list_task: Option<FetchTask>,
    image_paths: Option<Vec<String>>,
    link: ComponentLink<Self>,
    insert_image_callback: Callback<String>,
}

fn view_modal_image_tooltip(path: &String, click_callback: Callback<MouseEvent>) -> Html {
    html! {
        <img class=classes!("image-thumbnail") src=format!("images/{}", path) onclick=click_callback />
    }
}

impl Component for ModalImageSelector {
    type Message = ModalImageSelectorMessage;
    type Properties = ModalImageSelectorProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            _get_image_list_task: None,
            image_paths: None,
            link,
            insert_image_callback: props.insert_image_callback,
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            ModalImageSelectorMessage::ReceivedImages(v) => {
                self.image_paths = Some(v);
                true
            }
            ModalImageSelectorMessage::GetImages => {
                let request = Request::get("/api/get_image_list").body(Nothing).unwrap();

                let callback =
                    self.link
                        .batch_callback(|response: JsonFetchResponse<ImageListResponse>| {
                            let Json(data) = response.into_body();
                            match data {
                                Ok(v) => Some(ModalImageSelectorMessage::ReceivedImages(v.images)),
                                Err(_) => None,
                            }
                        });

                let task = FetchService::fetch(request, callback).unwrap();
                self._get_image_list_task = Some(task);

                false
            }
            ModalImageSelectorMessage::InsertImageRequest(path) => {
                self.insert_image_callback.emit(path);
                false
            }
        }
    }

    fn view(&self) -> Html {
        match &self.image_paths {
            Some(v) => {
                html! {
                    <>
                    <div class=classes!("modal-title")>
                    {"Image Selection"}
                    </div>
                    <div class=classes!("image-viewer")>
                    {v.iter().map(|path| {
                        let path_clone = path.clone();
                        let callback = self.link.callback(move |_| ModalImageSelectorMessage::InsertImageRequest(path_clone.clone()));
                        view_modal_image_tooltip(path, callback)
                    }).collect::<Html>()}
                    </div>
                    </>
                }
            }
            None => {
                html! {
                    <div></div>
                }
            }
        }
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            self.link.send_message(ModalImageSelectorMessage::GetImages);
        }
    }
}

enum ModalState {
    ImageSelector,
    Closed,
}

pub enum ModalComponentMessage {
    ModalEvent(ModalEvent),
    InsertImageRequest(String),
}

pub struct ModalComponent {
    modal_ref: NodeRef,
    modal_background_ref: NodeRef,
    producer: Box<dyn Bridge<EventBus>>,
    link: ComponentLink<Self>,
    modal_state: ModalState,
}

impl ModalComponent {
    fn update_modal_event(&mut self, msg: ModalEvent) -> bool {
        match msg {
            ModalEvent::OpenImageSelector => {
                self.modal_ref
                    .cast::<HtmlElement>()
                    .unwrap()
                    .style()
                    .set_property("display", "block")
                    .unwrap();
                self.modal_background_ref
                    .cast::<HtmlElement>()
                    .unwrap()
                    .style()
                    .set_property("display", "block")
                    .unwrap();

                self.modal_state = ModalState::ImageSelector;
                true
            }
            ModalEvent::CloseModal => {
                self.modal_ref
                    .cast::<HtmlElement>()
                    .unwrap()
                    .style()
                    .set_property("display", "none")
                    .unwrap();
                self.modal_background_ref
                    .cast::<HtmlElement>()
                    .unwrap()
                    .style()
                    .set_property("display", "none")
                    .unwrap();

                self.modal_state = ModalState::Closed;
                true
            }
        }
    }
}

impl Component for ModalComponent {
    type Message = ModalComponentMessage;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            modal_ref: NodeRef::default(),
            modal_background_ref: NodeRef::default(),
            producer: EventBus::bridge(link.batch_callback(|msg| match msg {
                BusRequest::ModalEvent(msg) => Some(ModalComponentMessage::ModalEvent(msg)),
                _ => None,
            })),
            link,
            modal_state: ModalState::Closed,
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            ModalComponentMessage::ModalEvent(msg) => self.update_modal_event(msg),
            ModalComponentMessage::InsertImageRequest(path) => {
                self.producer
                    .send(BusRequest::NoteEvent(NoteEvent::InsertImageRequest(path)));
                false
            }
        }
    }

    fn view(&self) -> Html {
        html! {
            <>
            <div ref=self.modal_background_ref.clone() class="modal-background" onclick=self.link.callback(|_| ModalComponentMessage::ModalEvent(ModalEvent::CloseModal)) />
            <div ref=self.modal_ref.clone() class="modal">
                {
                match self.modal_state {
                    ModalState::ImageSelector => {
                        html! {
                            <ModalImageSelector insert_image_callback=self.link.callback(|s| ModalComponentMessage::InsertImageRequest(s)) />
                        }
                    },
                    ModalState::Closed => {
                        html! {}
                    },
                }
                }
            </div>
            </>
        }
    }
}

pub struct RootComponent {}

impl Component for RootComponent {
    type Message = ();
    type Properties = ();

    fn create(_props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Self {}
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <>
            <ModalComponent />
            <NotesComponent />
            </>
        }
    }
}
