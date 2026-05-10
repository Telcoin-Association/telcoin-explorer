#![allow(non_snake_case)]

mod components;
mod pages;
mod router;
mod services;

use dioxus::prelude::*;
use router::Route;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    // Inject keccak256 library for ABI selector computation
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Some(head) = doc.head() {
                if let Ok(script) = doc.create_element("script") {
                    script.set_attribute("src", "https://cdnjs.cloudflare.com/ajax/libs/js-sha3/0.9.3/sha3.min.js").ok();
                    head.append_child(&script).ok();
                }
            }
        }
    }
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}
