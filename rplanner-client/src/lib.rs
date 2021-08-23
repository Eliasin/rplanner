#![recursion_limit = "1024"]
#![feature(try_blocks)]
use wasm_bindgen::prelude::*;
use yew::start_app;

mod notes;
mod root;

use root::components::RootComponent;

#[wasm_bindgen(start)]
pub fn run_app() {
    start_app::<RootComponent>();
}
