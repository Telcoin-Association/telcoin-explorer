// src/components/search_box.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{is_contract, get_token_symbol, get_registered_tokens, RegisteredToken, shorten_addr};

#[derive(Clone, PartialEq)]
enum AddrKind {
    Token(String),
    Contract,
    Eoa,
}

/// Shared search box with instant feedback: as you type a token name/symbol,
/// a dropdown of matching registered tokens appears (client-side filter --
/// the whole registry is small and already fetchable in one call). As soon
/// as a full 42-char address is typed/pasted, a preview row shows whether
/// it's a token, contract, or EOA, before you even hit Enter. Full tx
/// hashes and block numbers still navigate instantly on Enter, same as
/// before -- this only adds feedback for the cases that previously felt
/// like a no-op until you clicked Search.
#[component]
pub fn SearchBox(id: String, placeholder: String, is_hero: bool) -> Element {
    let mut query: Signal<String> = use_signal(String::new);
    let mut focused = use_signal(|| false);
    let mut all_tokens: Signal<Vec<RegisteredToken>> = use_signal(Vec::new);
    let mut tokens_load_started = use_signal(|| false);
    let mut addr_preview: Signal<Option<(String, AddrKind)>> = use_signal(|| None);
    let mut addr_preview_loading = use_signal(|| false);

    let mut ensure_tokens_loaded = move || {
        if all_tokens.read().is_empty() && !*tokens_load_started.read() {
            tokens_load_started.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                all_tokens.set(get_registered_tokens().await);
            });
        }
    };

    let handle_input = move |e: Event<FormData>| {
        let val = e.value();
        query.set(val.clone());
        addr_preview.set(None);
        let trimmed = val.trim().to_string();
        if trimmed.len() == 42 && trimmed.starts_with("0x") {
            let addr = trimmed.clone();
            wasm_bindgen_futures::spawn_local(async move {
                addr_preview_loading.set(true);
                if is_contract(&addr).await {
                    let sym = get_token_symbol(&addr).await;
                    if !sym.is_empty() {
                        addr_preview.set(Some((addr.clone(), AddrKind::Token(sym))));
                    } else {
                        addr_preview.set(Some((addr.clone(), AddrKind::Contract)));
                    }
                } else {
                    addr_preview.set(Some((addr.clone(), AddrKind::Eoa)));
                }
                addr_preview_loading.set(false);
            });
        } else if !trimmed.is_empty()
            && !(trimmed.len() == 66 && trimmed.starts_with("0x"))
            && !trimmed.chars().all(|c| c.is_ascii_digit())
        {
            ensure_tokens_loaded();
        }
    };

    let id_for_flash = id.clone();
    let handle_keydown = move |e: Event<KeyboardData>| {
        if e.key() != Key::Enter { return; }
        let q = query.read().trim().to_string();
        if q.is_empty() { return; }
        let window = match web_sys::window() { Some(w) => w, None => return };
        if q.len() == 66 && q.starts_with("0x") {
            let _ = window.location().set_href(&format!("/tx/{}", q));
        } else if q.len() == 42 && q.starts_with("0x") {
            if let Some((addr, kind)) = addr_preview.read().as_ref() {
                let href = match kind {
                    AddrKind::Token(_) => format!("/token/{}", addr),
                    AddrKind::Contract => format!("/contract/{}", addr),
                    AddrKind::Eoa       => format!("/address/{}", addr),
                };
                let _ = window.location().set_href(&href);
            }
        } else if q.chars().all(|c| c.is_ascii_digit()) {
            let _ = window.location().set_href(&format!("/block/{}", q));
        } else {
            let q_lower = q.to_lowercase();
            if let Some(t) = all_tokens.read().iter().find(|t| {
                t.symbol.to_lowercase() == q_lower || t.name.to_lowercase() == q_lower
            }) {
                let _ = window.location().set_href(&format!("/token/{}", t.address));
            } else {
                let _ = js_sys::eval(&format!(
                    "var el=document.getElementById('{}');if(el){{el.style.borderColor='#ef4444';setTimeout(function(){{el.style.borderColor='';}},2000);}}", id_for_flash
                ));
            }
        }
    };

    let handle_click_search = move |_| {
        // Re-dispatch the same logic as Enter, for the magnifier button.
        let q = query.read().trim().to_string();
        if q.is_empty() { return; }
        let window = match web_sys::window() { Some(w) => w, None => return };
        if q.len() == 66 && q.starts_with("0x") {
            let _ = window.location().set_href(&format!("/tx/{}", q));
        } else if q.len() == 42 && q.starts_with("0x") {
            if let Some((addr, kind)) = addr_preview.read().as_ref() {
                let href = match kind {
                    AddrKind::Token(_) => format!("/token/{}", addr),
                    AddrKind::Contract => format!("/contract/{}", addr),
                    AddrKind::Eoa       => format!("/address/{}", addr),
                };
                let _ = window.location().set_href(&href);
            }
        } else if q.chars().all(|c| c.is_ascii_digit()) {
            let _ = window.location().set_href(&format!("/block/{}", q));
        } else {
            let q_lower = q.to_lowercase();
            let addr_for_href = all_tokens.read().iter().find(|t| {
                t.symbol.to_lowercase() == q_lower || t.name.to_lowercase() == q_lower
            }).map(|t| t.address.clone());
            if let Some(addr) = addr_for_href {
                let _ = window.location().set_href(&format!("/token/{}", addr));
            }
        }
    };

    let q_lower = query.read().trim().to_lowercase();
    let token_matches: Vec<RegisteredToken> = if !q_lower.is_empty() {
        all_tokens.read().iter()
            .filter(|t| t.symbol.to_lowercase().contains(&q_lower) || t.name.to_lowercase().contains(&q_lower))
            .cloned().collect()
    } else {
        vec![]
    };
    let has_dropdown_content = addr_preview.read().is_some() || *addr_preview_loading.read() || !token_matches.is_empty();
    let show_dropdown = *focused.read() && !query.read().trim().is_empty() && has_dropdown_content;

    let box_class = if is_hero { "hero-search-box" } else { "header-search-box" };
    let input_class = if is_hero { "hero-search-input" } else { "header-search-input" };
    let btn_class = if is_hero { "hero-search-btn" } else { "header-search-btn" };

    rsx! {
        div { class: "search-box-wrap",
            div { class: "{box_class}",
                input {
                    class: "{input_class}",
                    id: "{id}",
                    placeholder: "{placeholder}",
                    value: "{query}",
                    oninput: handle_input,
                    onkeydown: handle_keydown,
                    onfocus: move |_| focused.set(true),
                    onblur: move |_| {
                        wasm_bindgen_futures::spawn_local(async move {
                            gloo_timers::future::TimeoutFuture::new(150).await;
                            focused.set(false);
                        });
                    },
                }
                button {
                    class: "{btn_class}",
                    onclick: handle_click_search,
                    svg {
                        width: if is_hero { "18" } else { "15" },
                        height: if is_hero { "18" } else { "15" },
                        view_box: "0 0 24 24", fill: "none", stroke: "currentColor",
                        stroke_width: "2.5", stroke_linecap: "round", stroke_linejoin: "round",
                        circle { cx: "11", cy: "11", r: "8" }
                        path { d: "m21 21-4.35-4.35" }
                    }
                }
            }
            if show_dropdown {
                div { class: "search-dropdown",
                    if let Some((addr, kind)) = addr_preview.read().as_ref() {
                        {
                            let (href, label, chip_class) = match kind {
                                AddrKind::Token(sym) => (format!("/token/{addr}"), sym.clone(), "chip success"),
                                AddrKind::Contract    => (format!("/contract/{addr}"), "Contract".to_string(), "chip info"),
                                AddrKind::Eoa         => (format!("/address/{addr}"), "Wallet".to_string(), "chip pending"),
                            };
                            let addr_short = shorten_addr(addr);
                            // Plain onclick + window.location, NOT Dioxus's Link
                            // component -- a client-side route change here can
                            // unmount this component mid-flight (e.g. while an
                            // in-flight address-preview lookup is still writing
                            // to its signals), which panics with a
                            // Dropped/ValueDroppedError. A hard navigation
                            // sidesteps that unmount race entirely, matching
                            // the same pattern already used for Enter/Search-
                            // button navigation elsewhere in this component.
                            rsx! {
                                div { class: "search-dropdown-item", style: "cursor:pointer;",
                                    onclick: move |_| {
                                        if let Some(w) = web_sys::window() { let _ = w.location().set_href(&href); }
                                    },
                                    span { class: "{chip_class}", style: "font-size:10px; padding:2px 7px;", "{label}" }
                                    span { class: "search-dropdown-addr", "{addr_short}" }
                                }
                            }
                        }
                    } else if *addr_preview_loading.read() {
                        div { class: "search-dropdown-loading",
                            div { class: "spinner", style: "width:14px;height:14px;border-width:2px;" }
                            span { "Checking address…" }
                        }
                    }
                    for t in token_matches.iter() {
                        {
                            let href = format!("/token/{}", t.address);
                            let symbol = t.symbol.clone();
                            let name = t.name.clone();
                            rsx! {
                                div { class: "search-dropdown-item", style: "cursor:pointer;",
                                    onclick: move |_| {
                                        if let Some(w) = web_sys::window() { let _ = w.location().set_href(&href); }
                                    },
                                    span { class: "chip info", style: "font-size:10px; padding:2px 7px;", "{symbol}" }
                                    span { "{name}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
