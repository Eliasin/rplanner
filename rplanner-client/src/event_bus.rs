use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use yew::worker::*;

use crate::notes::api::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Request {
    FlushNoteChanges(NoteID),
}

pub struct EventBus {
    link: AgentLink<Self>,
    subscribers: HashSet<HandlerId>,
}

impl Agent for EventBus {
    type Reach = Context<Self>;
    type Message = Request;
    type Input = Request;
    type Output = Request;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            subscribers: HashSet::new(),
        }
    }

    fn update(&mut self, _msg: Self::Message) {}

    fn handle_input(&mut self, msg: Self::Input, _id: HandlerId) {
        match msg {
            Request::FlushNoteChanges(_) => {
                for sub in self.subscribers.iter() {
                    self.link.respond(*sub, msg.clone());
                }
            }
        }
    }

    fn connected(&mut self, id: HandlerId) {
        self.subscribers.insert(id);
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subscribers.remove(&id);
    }
}
