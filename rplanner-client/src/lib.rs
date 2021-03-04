#![recursion_limit="1024"]
use wasm_bindgen::prelude::*;
use yew::start_app;

mod event_bus;
mod notes;
use notes::components::NotesComponent;

#[wasm_bindgen(start)]
pub fn run_app() {
    start_app::<NotesComponent>();
}
