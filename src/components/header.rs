// src/components/header.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::components::SearchBox;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
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
/// Clears our local wallet state AND asks MetaMask to revoke the site's
/// permission grant (best-effort -- older wallets may not support this
/// method, hence the try/catch). Without the revoke, MetaMask keeps
/// treating the site as already-authorized for the original account, so a
/// later "Connect" click just silently re-returns it instead of showing an
/// account picker, even if the user switched accounts in MetaMask meanwhile.
fn disconnect_wallet() {
    let _ = js_sys::eval(
        "if(window.ethereum && window.ethereum.request){            window.ethereum.request({method:'wallet_revokePermissions',params:[{eth_accounts:{}}]}).catch(function(){});        }"
    );
}

#[component]
pub fn Header() -> Element {
    let mut wallet_address: Signal<Option<String>> = use_context();
    let mut wallet_error: Signal<Option<String>>   = use_signal(|| None);
    let mut menu_open                              = use_signal(|| false);

    // Check current route to hide search on home page
    let route: Route = use_route();
    let is_home = matches!(route, Route::HomePage {});

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
    // Listen for MetaMask's own accountsChanged event, so switching accounts
    // directly in the wallet (without using our Disconnect button first)
    // updates the UI immediately instead of silently continuing to show the
    // old address until the user manually disconnects/reconnects.
    use_effect(move || {
        let window = match web_sys::window() { Some(w) => w, None => return };
        let ethereum = match js_sys::Reflect::get(&window, &JsValue::from_str("ethereum")) {
            Ok(e) if !e.is_undefined() && !e.is_null() => e,
            _ => return,
        };
        let closure = Closure::wrap(Box::new(move |accounts: JsValue| {
            let arr = js_sys::Array::from(&accounts);
            if arr.length() == 0 {
                wallet_address.set(None);
                ls_remove("wallet_address");
            } else if let Some(addr) = arr.get(0).as_string() {
                ls_set("wallet_address", &addr);
                wallet_address.set(Some(addr));
            }
        }) as Box<dyn FnMut(JsValue)>);
        if let Ok(on_fn) = js_sys::Reflect::get(&ethereum, &JsValue::from_str("on"))
            .and_then(|f| f.dyn_into::<js_sys::Function>())
        {
            let _ = on_fn.call2(&ethereum, &JsValue::from_str("accountsChanged"), closure.as_ref().unchecked_ref());
        }
        // Intentionally leaked: this listener needs to live for as long as
        // the header (effectively the whole app) is mounted, same lifetime
        // as window.ethereum itself.
        closure.forget();
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

                // ── Search (hidden on home page) ───────────────────────
                if !is_home {
                    SearchBox {
                        id: "header-search".to_string(),
                        placeholder: "Search address / tx hash / block…".to_string(),
                        is_hero: false,
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
                                disconnect_wallet();
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
                    // Search in mobile menu (always show)
                    div { class: "mobile-menu-search",
                        SearchBox {
                            id: "mobile-search".to_string(),
                            placeholder: "Search address / tx / block…".to_string(),
                            is_hero: false,
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
                                    disconnect_wallet();
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
