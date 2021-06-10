#![recursion_limit = "1024"]
use wasm_bindgen::prelude::*;
use yew::start_app;

mod event_bus;
mod notes;
mod root;

use root::components::RootComponent;

#[wasm_bindgen(start)]
pub fn run_app() {
    start_app::<RootComponent>();
}
