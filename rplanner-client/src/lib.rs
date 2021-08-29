#![recursion_limit = "1024"]
#![feature(try_blocks)]
use wasm_bindgen::prelude::*;
use yew::start_app;

mod modal;
mod notes;
mod root;
mod todo;

use root::components::RootComponent;

#[wasm_bindgen(start)]
pub fn run_app() {
    start_app::<RootComponent>();
}
