// src/components/header.rs
use dioxus::prelude::*;
use crate::router::Route;
const LOGO: Asset = asset!("/assets/telcoin-logo.svg");

const CHAIN_ID_HEX: &str = "0x7E1";
const CHAIN_NAME:   &str = "Telcoin Network Adiri Testnet";
const RPC_URL:      &str = "https://rpc.telcoin.network";
const EXPLORER_URL: &str = "https://www.telscan.xyz";

fn ls_get(key: &str) -> String {
    js_sys::eval(&format!("localStorage.getItem('{}') || ''", key))
        .ok().and_then(|v| v.as_string()).unwrap_or_default()
}
fn ls_set(key: &str, val: &str) {
    let _ = js_sys::eval(&format!("localStorage.setItem('{}','{}')", key, val));
}
fn ls_remove(key: &str) {
    let _ = js_sys::eval(&format!("localStorage.removeItem('{}')", key));
}

#[component]
pub fn Header() -> Element {
    let mut dark_mode                              = use_signal(|| true);
    let mut wallet_address: Signal<Option<String>> = use_signal(|| None);
    let mut wallet_error: Signal<Option<String>>   = use_signal(|| None);
    let mut menu_open                              = use_signal(|| false);

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

    // Restore wallet from localStorage on mount
    use_effect(move || {
        if wallet_address.read().is_none() {
            let saved = ls_get("wallet_address");
            if !saved.is_empty() {
                wasm_bindgen_futures::spawn_local(async move {
                    let js = r#"(async function(){
                        if(!window.ethereum) return null;
                        try {
                            const a = await window.ethereum.request({method:'eth_accounts'});
                            return (a && a.length) ? a[0] : null;
                        } catch(e) { return null; }
                    })()"#;
                    if let Ok(pv) = js_sys::eval(js) {
                        if let Ok(r) = wasm_bindgen_futures::JsFuture::from(
                            js_sys::Promise::from(pv)
                        ).await {
                            if let Some(current) = r.as_string() {
                                if current.to_lowercase() == saved.to_lowercase() {
                                    wallet_address.set(Some(saved));
                                } else {
                                    ls_remove("wallet_address");
                                }
                            } else {
                                ls_remove("wallet_address");
                            }
                        }
                    }
                });
            }
        }
    });

    rsx! {
        header { class: "header",
            div { class: "header-inner",

                // ── Logo ─────────────────────────────────────────────
                Link { to: Route::HomePage {},
                    div { class: "logo",
                        img { src: LOGO, class: "logo-img", alt: "Telcoin" }
                        div { class: "logo-text-wrap",
                            span { class: "logo-name", "TelScan" }
                            span { class: "logo-badge", "Adiri Testnet" }
                        }
                    }
                }

                // ── Search (desktop) ──────────────────────────────────
                div { class: "header-search-box",
                    input {
                        class: "header-search-input",
                        id: "header-search",
                        placeholder: "Search address / tx hash / block…",
                        onkeydown: move |e: Event<KeyboardData>| {
                            if e.key() == Key::Enter { run_header_search(); }
                        }
                    }
                    button {
                        class: "header-search-btn",
                        onclick: move |_: Event<MouseData>| { run_header_search(); },
                        svg {
                            width: "15", height: "15", view_box: "0 0 24 24",
                            fill: "none", stroke: "currentColor",
                            stroke_width: "2.5", stroke_linecap: "round", stroke_linejoin: "round",
                            circle { cx: "11", cy: "11", r: "8" }
                            path { d: "m21 21-4.35-4.35" }
                        }
                    }
                }

                // ── Desktop nav ───────────────────────────────────────
                nav { class: "header-nav desktop-nav",
                    Link { to: Route::HomePage {},               class: "header-nav-link", "Home" }
                    Link { to: Route::BlocksPage { page: 0 },   class: "header-nav-link", "Blocks" }
                    Link { to: Route::TransactionsPage { page: 0 }, class: "header-nav-link", "Transactions" }
                    Link { to: Route::EpochsPage {},             class: "header-nav-link", "Epochs" }
                    Link { to: Route::ValidatorsPage {},         class: "header-nav-link", "Validators" }

                    // Wallet
                    if let Some(ref addr) = *wallet_address.read() {
                        Link {
                            to: Route::AddressPage { address: addr.clone() },
                            class: "wallet-btn wallet-connected",
                            title: "{addr}",
                            span { class: "wallet-dot" }
                            { format!("{}…{}", &addr[..6], &addr[addr.len()-4..]) }
                        }
                        button {
                            class: "wallet-disconnect",
                            title: "Disconnect",
                            onclick: move |_| {
                                wallet_address.set(None);
                                wallet_error.set(None);
                                ls_remove("wallet_address");
                            },
                            "×"
                        }
                    } else {
                        button {
                            class: "wallet-btn",
                            onclick: move |_| {
                                wasm_bindgen_futures::spawn_local(async move {
                                    match connect_wallet().await {
                                        Ok(addr) => {
                                            wallet_error.set(None);
                                            ls_set("wallet_address", &addr);
                                            wallet_address.set(Some(addr));
                                        }
                                        Err(e) => wallet_error.set(Some(e)),
                                    }
                                });
                            },
                            svg {
                                width: "13", height: "13", view_box: "0 0 24 24",
                                fill: "none", stroke: "currentColor",
                                stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                                rect { x: "2", y: "5", width: "20", height: "14", rx: "2" }
                                path { d: "M16 12h.01" }
                                path { d: "M2 10h20" }
                            }
                            "Connect"
                        }
                    }
                    if let Some(ref err) = *wallet_error.read() {
                        span { class: "wallet-error", title: "{err}", "⚠" }
                    }

                    button {
                        class: "theme-toggle",
                        title: if *dark_mode.read() { "Switch to light" } else { "Switch to dark" },
                        onclick: move |_: Event<MouseData>| {
                            let cur = *dark_mode.read();
                            dark_mode.set(!cur);
                        },
                        if *dark_mode.read() { "☀" } else { "🌙" }
                    }
                }

                // ── Mobile right side: wallet + hamburger ─────────────
                div { class: "mobile-nav-right",
                    if let Some(ref addr) = *wallet_address.read() {
                        Link {
                            to: Route::AddressPage { address: addr.clone() },
                            class: "wallet-btn wallet-connected mobile-wallet",
                            span { class: "wallet-dot" }
                            { format!("{}…{}", &addr[..6], &addr[addr.len()-4..]) }
                        }
                    }
                    button {
                        class: "hamburger",
                        onclick: move |_| { let cur = *menu_open.read(); menu_open.set(!cur); },
                        if *menu_open.read() {
                            svg { width: "22", height: "22", view_box: "0 0 24 24", fill: "none",
                                stroke: "currentColor", stroke_width: "2",
                                stroke_linecap: "round", stroke_linejoin: "round",
                                path { d: "M18 6 6 18" }
                                path { d: "m6 6 12 12" }
                            }
                        } else {
                            svg { width: "22", height: "22", view_box: "0 0 24 24", fill: "none",
                                stroke: "currentColor", stroke_width: "2",
                                stroke_linecap: "round", stroke_linejoin: "round",
                                path { d: "M4 6h16" }
                                path { d: "M4 12h16" }
                                path { d: "M4 18h16" }
                            }
                        }
                    }
                }
            }

            // ── Mobile dropdown menu ──────────────────────────────────
            if *menu_open.read() {
                div { class: "mobile-menu",
                    div { class: "mobile-menu-search",
                        input {
                            class: "header-search-input",
                            id: "mobile-search",
                            placeholder: "Search address / tx / block…",
                            onkeydown: move |e: Event<KeyboardData>| {
                                if e.key() == Key::Enter {
                                    menu_open.set(false);
                                    run_mobile_search();
                                }
                            }
                        }
                        button {
                            class: "header-search-btn",
                            onclick: move |_| { menu_open.set(false); run_mobile_search(); },
                            svg {
                                width: "15", height: "15", view_box: "0 0 24 24",
                                fill: "none", stroke: "currentColor",
                                stroke_width: "2.5", stroke_linecap: "round", stroke_linejoin: "round",
                                circle { cx: "11", cy: "11", r: "8" }
                                path { d: "m21 21-4.35-4.35" }
                            }
                        }
                    }
                    Link { to: Route::HomePage {}, class: "mobile-nav-link", onclick: move |_| menu_open.set(false), "Home" }
                    Link { to: Route::BlocksPage { page: 0 }, class: "mobile-nav-link", onclick: move |_| menu_open.set(false), "Blocks" }
                    Link { to: Route::TransactionsPage { page: 0 }, class: "mobile-nav-link", onclick: move |_| menu_open.set(false), "Transactions" }
                    Link { to: Route::EpochsPage {}, class: "mobile-nav-link", onclick: move |_| menu_open.set(false), "Epochs" }
                    Link { to: Route::ValidatorsPage {}, class: "mobile-nav-link", onclick: move |_| menu_open.set(false), "Validators" }
                    if let Some(ref addr) = *wallet_address.read() {
                        div { class: "mobile-menu-wallet",
                            span { class: "wallet-dot" }
                            span { style: "font-family:var(--font-mono); font-size:12px;", "{addr}" }
                            button {
                                class: "wallet-disconnect",
                                onclick: move |_| {
                                    wallet_address.set(None);
                                    wallet_error.set(None);
                                    menu_open.set(false);
                                    ls_remove("wallet_address");
                                },
                                "Disconnect"
                            }
                        }
                    } else {
                        button {
                            class: "wallet-btn mobile-connect-btn",
                            onclick: move |_| {
                                menu_open.set(false);
                                wasm_bindgen_futures::spawn_local(async move {
                                    match connect_wallet().await {
                                        Ok(addr) => {
                                            wallet_error.set(None);
                                            ls_set("wallet_address", &addr);
                                            wallet_address.set(Some(addr));
                                        }
                                        Err(e) => wallet_error.set(Some(e)),
                                    }
                                });
                            },
                            "Connect Wallet"
                        }
                    }
                    button {
                        class: "mobile-nav-link mobile-theme-btn",
                        onclick: move |_| {
                            let cur = *dark_mode.read();
                            dark_mode.set(!cur);
                            menu_open.set(false);
                        },
                        if *dark_mode.read() { "☀ Light Mode" } else { "🌙 Dark Mode" }
                    }
                }
            }
        }
    }
}

async fn connect_wallet() -> Result<String, String> {
    let js = format!(r#"
(async function() {{
    if (!window.ethereum) return {{error: 'No wallet detected. Please install MetaMask.'}};
    try {{
        const accounts = await window.ethereum.request({{ method: 'eth_requestAccounts' }});
        if (!accounts || accounts.length === 0) return {{error: 'No accounts found.'}};
        try {{
            await window.ethereum.request({{
                method: 'wallet_switchEthereumChain',
                params: [{{ chainId: '{chain_id}' }}]
            }});
        }} catch(switchErr) {{
            if (switchErr.code === 4902) {{
                try {{
                    await window.ethereum.request({{
                        method: 'wallet_addEthereumChain',
                        params: [{{
                            chainId: '{chain_id}',
                            chainName: '{chain_name}',
                            nativeCurrency: {{ name: 'TEL', symbol: 'TEL', decimals: 18 }},
                            rpcUrls: ['{rpc}'],
                            blockExplorerUrls: ['{explorer}']
                        }}]
                    }});
                }} catch(addErr) {{
                    return {{error: 'Failed to add network: ' + addErr.message}};
                }}
            }} else if (switchErr.code !== 4001) {{
                return {{error: 'Failed to switch network: ' + switchErr.message}};
            }}
        }}
        return {{address: accounts[0]}};
    }} catch(err) {{
        if (err.code === 4001) return {{error: 'Connection rejected by user.'}};
        return {{error: err.message || 'Unknown error'}};
    }}
}})()
"#,
        chain_id   = CHAIN_ID_HEX,
        chain_name = CHAIN_NAME,
        rpc        = RPC_URL,
        explorer   = EXPLORER_URL,
    );
    let promise = match js_sys::eval(&js) {
        Ok(v) => v,
        Err(_) => return Err("Failed to run wallet JS".to_string()),
    };
    let result = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise))
        .await.map_err(|e| format!("{:?}", e))?;
    let obj       = js_sys::Object::from(result);
    let error_key = wasm_bindgen::JsValue::from_str("error");
    let addr_key  = wasm_bindgen::JsValue::from_str("address");
    if let Some(err) = js_sys::Reflect::get(&obj, &error_key).ok().and_then(|v| v.as_string()) {
        return Err(err);
    }
    js_sys::Reflect::get(&obj, &addr_key).ok()
        .and_then(|v| v.as_string())
        .ok_or_else(|| "No address returned".to_string())
}

fn run_header_search() { run_search_for("header-search"); }
fn run_mobile_search()  { run_search_for("mobile-search"); }

fn run_search_for(id: &str) {
    use wasm_bindgen::JsCast;
    let window = match web_sys::window() { Some(w) => w, None => return };
    let doc = match window.document() { Some(d) => d, None => return };
    if let Some(el) = doc.get_element_by_id(id) {
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
            let _ = js_sys::eval(&format!(
                "var el=document.getElementById('{}');if(el){{el.style.borderColor='#ef4444';setTimeout(function(){{el.style.borderColor='';}},2000);}}", id
            ));
        }
    }
}
