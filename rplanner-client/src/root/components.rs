use yew::prelude::*;

use crate::modal::components::ModalComponent;
use crate::notes::components::NotesComponent;

pub enum Application {
    Notes,
    Calendar,
    Todo,
}

#[derive(Properties, Clone)]
pub struct SidebarComponentProps {
    switch_application_callback: Callback<SwitchApplicationMsg>,
}

pub struct SidebarComponent {
    switch_application_callback: Callback<SwitchApplicationMsg>,
    link: ComponentLink<Self>,
}

impl Component for SidebarComponent {
    type Message = Application;
    type Properties = SidebarComponentProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            switch_application_callback: props.switch_application_callback,
            link,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        self.switch_application_callback.emit(msg);
        false
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div class=classes!("sidebar")>
                <div onclick=self.link.callback(|_| Application::Notes)>{"Notes"}</div>
                <div onclick=self.link.callback(|_| Application::Calendar)>{"Calendar"}</div>
                <div onclick=self.link.callback(|_| Application::Todo)>{"Todo"}</div>
            </div>
        }
    }
}

pub type SwitchApplicationMsg = Application;

pub struct RootComponent {
    current_application: Application,
    link: ComponentLink<Self>,
}

impl Component for RootComponent {
    type Message = SwitchApplicationMsg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            current_application: Application::Notes,
            link,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        self.current_application = msg;

        true
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <>
            <SidebarComponent switch_application_callback=self.link.callback(|application| application) />
            <ModalComponent />
            {
                match self.current_application {
                    Application::Notes => html! {
                        <NotesComponent />
                    },
                    Application::Calendar => html! {

                    },
                    Application::Todo => html! {

                    },
                }
            }
            </>
        }
    }
}
