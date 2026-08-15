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
    let _ = js_sys::eval("var s=document.createElement('script');s.src='https://cdnjs.cloudflare.com/ajax/libs/js-sha3/0.9.3/sha3.min.js';document.head.appendChild(s);");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Shared wallet-connection state, readable from any page (header owns
    // writing to it; contract.rs and others just read it reactively).
    use_context_provider(|| Signal::new(None::<String>));
    rsx! {
        Router::<Route> {}
    }
}
