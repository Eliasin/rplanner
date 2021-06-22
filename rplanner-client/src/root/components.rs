use yew::prelude::*;
use yew::{
    format::{Json, Nothing},
    services::fetch::{FetchService, FetchTask, Request},
};

use web_sys::HtmlElement;

use super::agents::{EventBus, ModalEvent, Request as BusRequest};
use crate::notes::api::{ImageListResponse, JsonFetchResponse};
use crate::notes::components::NotesComponent;

pub enum ModalImageSelectorMessage {
    GetImages,
    ReceivedImages(Vec<String>),
}

pub struct ModalImageSelector {
    _get_image_list_task: Option<FetchTask>,
    image_paths: Option<Vec<String>>,
    link: ComponentLink<Self>,
}

fn view_modal_image_tooltip(path: &String) -> Html {
    html! {
        <img class=classes!("image-thumbnail") src=format!("images/{}", path) />
    }
}

impl Component for ModalImageSelector {
    type Message = ModalImageSelectorMessage;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            _get_image_list_task: None,
            image_paths: None,
            link,
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
                                Ok(v) => vec![ModalImageSelectorMessage::ReceivedImages(v.images)],
                                Err(_) => vec![],
                            }
                        });

                let task = FetchService::fetch(request, callback).unwrap();
                self._get_image_list_task = Some(task);

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
                        view_modal_image_tooltip(path)
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

pub struct ModalComponent {
    modal_ref: NodeRef,
    modal_background_ref: NodeRef,
    _producer: Box<dyn Bridge<EventBus>>,
    link: ComponentLink<Self>,
    modal_state: ModalState,
}

impl Component for ModalComponent {
    type Message = BusRequest;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            modal_ref: NodeRef::default(),
            modal_background_ref: NodeRef::default(),
            _producer: EventBus::bridge(link.callback(|msg| msg)),
            link,
            modal_state: ModalState::Closed,
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            BusRequest::ModalEvent(msg) => match msg {
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
            },
        }
    }

    fn view(&self) -> Html {
        html! {
            <>
            <div ref=self.modal_background_ref.clone() class="modal-background" onclick=self.link.callback(|_| BusRequest::ModalEvent(ModalEvent::CloseModal)) />
            <div ref=self.modal_ref.clone() class="modal">
                {
                match self.modal_state {
                    ModalState::ImageSelector => {
                        html! {
                            <ModalImageSelector />
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
