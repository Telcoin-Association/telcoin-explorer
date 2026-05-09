// src/components/header.rs
use dioxus::prelude::*;
use crate::router::Route;
const LOGO: Asset = asset!("/assets/telcoin-logo.svg");

#[component]
pub fn Header() -> Element {
    let mut dark_mode = use_signal(|| true);

    use_effect(move || {
        let is_dark = *dark_mode.read();
        let window = web_sys::window().unwrap();
        let doc = window.document().unwrap();
        let html = doc.document_element().unwrap();
        if is_dark {
            html.remove_attribute("data-theme").ok();
        } else {
            html.set_attribute("data-theme", "light").ok();
        }
    });

    rsx! {
        header { class: "header",
            div { class: "header-inner",

                // Logo
                Link { to: Route::HomePage {},
                    div { class: "logo",
                        img { src: LOGO, class: "logo-img", alt: "Telcoin" }
                        div { class: "logo-text-wrap",
                            span { class: "logo-name", "TelScan" }
                            span { class: "logo-badge", "Adiri Testnet" }
                        }
                    }
                }

                // Search box
                div { class: "header-search-box",
                    input {
                        class: "header-search-input",
                        id: "header-search",
                        placeholder: "Search address / tx / block…",
                        onkeydown: move |e: Event<KeyboardData>| {
                            if e.key() == Key::Enter { run_header_search(); }
                        }
                    }
                    button {
                        class: "header-search-btn",
                        onclick: move |_: Event<MouseData>| { run_header_search(); },
                        svg {
                            width: "15", height: "15",
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2.5",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            circle { cx: "11", cy: "11", r: "8" }
                            path { d: "m21 21-4.35-4.35" }
                        }
                    }
                }

                // Nav
                nav { class: "header-nav",
                    Link { to: Route::HomePage {},
                        class: "header-nav-link",
                        "Home"
                    }
                    Link { to: Route::BlocksPage { page: 0 },
                        class: "header-nav-link",
                        "Blocks"
                    }
                    Link { to: Route::TransactionsPage { page: 0 },
                        class: "header-nav-link",
                        "Txns"
                    }
                    Link { to: Route::EpochsPage {},
                        class: "header-nav-link",
                        "Epochs"
                    }
                    Link { to: Route::ValidatorsPage {},
                        class: "header-nav-link",
                        "Validators"
                    }
                    a {
                        href: "https://telcoin.network/faucet",
                        target: "_blank",
                        class: "header-nav-faucet",
                        "Faucet ↗"
                    }
                    button {
                        class: "theme-toggle",
                        title: if *dark_mode.read() { "Switch to light mode" } else { "Switch to dark mode" },
                        onclick: move |_: Event<MouseData>| {
                            let current = *dark_mode.read();
                            dark_mode.set(!current);
                        },
                        if *dark_mode.read() { "☀" } else { "🌙" }
                    }
                }
            }
        }
    }
}

fn run_header_search() {
    use wasm_bindgen::JsCast;
    let window = match web_sys::window() { Some(w) => w, None => return };
    let doc = match window.document() { Some(d) => d, None => return };
    if let Some(el) = doc.get_element_by_id("header-search") {
        let input: web_sys::HtmlInputElement = match el.dyn_into() { Ok(i) => i, Err(_) => return };
        let q = input.value().trim().to_string();
        if q.is_empty() { return; }
        let window2 = window.clone();
        if q.len() == 66 && q.starts_with("0x") {
            window.location().set_href(&format!("/tx/{}", q)).ok();
        } else if q.len() == 42 && q.starts_with("0x") {
            wasm_bindgen_futures::spawn_local(async move {
                use crate::services::rpc::{is_contract, get_token_symbol};
                if is_contract(&q).await {
                    let sym = get_token_symbol(&q).await;
                    if !sym.is_empty() {
                        window2.location().set_href(&format!("/token/{}", q)).ok();
                    } else {
                        window2.location().set_href(&format!("/contract/{}", q)).ok();
                    }
                } else {
                    window2.location().set_href(&format!("/address/{}", q)).ok();
                }
            });
        } else if q.chars().all(|c| c.is_ascii_digit()) {
            window.location().set_href(&format!("/block/{}", q)).ok();
        } else {
            // flash red
            let _ = js_sys::eval("var el=document.getElementById('header-search');if(el){el.style.borderColor='#ef4444';setTimeout(function(){el.style.borderColor='';},2000);}");
        }
    }
}
