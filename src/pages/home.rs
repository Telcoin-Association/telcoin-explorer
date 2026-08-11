// src/pages/home.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_latest_blocks, get_network_stats, get_latest_txs,
    Transaction,
    Block, NetworkStats, shorten_hash, shorten_addr, unix_to_age, format_gas,
};
use crate::components::loading::{Loading, ErrorBox};

#[component]
pub fn HomePage() -> Element {
    let mut blocks: Signal<Vec<Block>>           = use_signal(|| vec![]);
    let mut stats:  Signal<Option<NetworkStats>> = use_signal(|| None);
    let mut loading                              = use_signal(|| true);
    let mut error: Signal<Option<String>>        = use_signal(|| None);
    let mut last_updated: Signal<String>         = use_signal(|| "".to_string());
    let mut home_txs: Signal<Vec<Transaction>>   = use_signal(|| vec![]);
    let mut txs_loading                          = use_signal(|| true);

    use_effect(move || {
        wasm_bindgen_futures::spawn_local(async move {
            txs_loading.set(true);
            // Fetch stats, blocks, and transactions all in parallel — no more
            // sequential per-hash lookups, so blocks and txs update together.
            let (stats_res, blocks_res, txs_res) = futures::join!(
                get_network_stats(),
                get_latest_blocks(10),
                get_latest_txs(0, 10),
            );
            // Initial load: show an error banner if any fetch fails — there's
            // no prior data to fall back to.
            match stats_res {
                Ok(s)  => stats.set(Some(s)),
                Err(e) => error.set(Some(e)),
            }
            match blocks_res {
                Ok(b)  => blocks.set(b),
                Err(e) => error.set(Some(e)),
            }
            match txs_res {
                Ok((t, _)) => home_txs.set(t),
                Err(e)     => error.set(Some(e)),
            }
            txs_loading.set(false);
            let now = js_sys::Date::new_0();
            last_updated.set(format!("{:02}:{:02}:{:02}",
                now.get_hours(), now.get_minutes(), now.get_seconds()));
            loading.set(false);
        });
    });

    use_future(move || async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(30_000).await;
            txs_loading.set(true);
            // All three fetch in parallel, same as the initial load — keeps
            // blocks and transactions updating in lockstep on every tick.
            let (stats_res, blocks_res, txs_res) = futures::join!(
                get_network_stats(),
                get_latest_blocks(10),
                get_latest_txs(0, 10),
            );
            // Background refresh: on success, update data and clear any stale
            // error banner. On failure, log and silently keep the last-known-good
            // data on screen rather than surfacing a persistent error banner.
            match stats_res {
                Ok(s)  => { stats.set(Some(s)); error.set(None); }
                Err(e) => { web_sys::console::warn_1(&format!("stats refresh failed: {e}").into()); }
            }
            match blocks_res {
                Ok(b)  => { blocks.set(b); error.set(None); }
                Err(e) => { web_sys::console::warn_1(&format!("blocks refresh failed: {e}").into()); }
            }
            match txs_res {
                Ok((t, _)) => { home_txs.set(t); error.set(None); }
                Err(e)     => { web_sys::console::warn_1(&format!("txs refresh failed: {e}").into()); }
            }
            txs_loading.set(false);
            let now = js_sys::Date::new_0();
            last_updated.set(format!("{:02}:{:02}:{:02}",
                now.get_hours(), now.get_minutes(), now.get_seconds()));
        }
    });

    rsx! {
        div {
            // ── Hero with search ──────────────────────────────────────
            div { class: "hero",
                div { class: "hero-inner",
                    h1 { class: "hero-title",
                        "The "
                        span { class: "hero-title-accent", "Telcoin Network" }
                        " Explorer"
                    }
                    div { class: "hero-search-box",
                        input {
                            class: "hero-search-input",
                            id: "home-search",
                            placeholder: "Search by address, tx hash, block number, token or contract",
                            onkeydown: move |e: Event<KeyboardData>| {
                                if e.key() == Key::Enter { run_search(); }
                            }
                        }
                        button {
                            class: "hero-search-btn",
                            onclick: move |_: Event<MouseData>| { run_search(); },
                            svg {
                                width: "18", height: "18",
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
                }
            }
            // ── Stats + Panels ────────────────────────────────────────
            div { class: "home-content",
                div { class: "stats-strip-card",
                    if let Some(s) = stats.read().as_ref() {
                        div { class: "stat-row live-row",
                            div { class: "stat-icon-wrap",
                                svg { width:"20", height:"20", view_box:"0 0 24 24", fill:"none",
                                    stroke:"#22c55e", stroke_width:"1.5",
                                    stroke_linecap:"round", stroke_linejoin:"round",
                                    path { d:"M22 12h-4l-3 9L9 3l-3 9H2" }
                                }
                            }
                            div { class: "stat-row-body",
                                span { class: "stat-row-label", "NETWORK" }
                                span { class: "stat-row-value live-value-inline",
                                    span { class: "live-dot" }
                                    "LIVE"
                                }
                                span { class: "stat-row-sub",
                                    if !last_updated.read().is_empty() {
                                        { format!("Updated {}", last_updated.read()) }
                                    } else {
                                        "rpc.telcoin.network"
                                    }
                                }
                            }
                        }
                        StatRow { label: "LATEST BLOCK",
                            value: format!("#{}", s.latest_block),
                            sub: Some("Telcoin Network".to_string()) }
                        StatRow { label: "CURRENT EPOCH",
                            value: format!("#{}", s.epoch_number.unwrap_or(0)),
                            sub: Some("Adiri Testnet".to_string()) }
                        StatRow { label: "VALIDATORS",
                            value: format!("{}", s.validator_count),
                            sub: Some("Active committee".to_string()) }
                    } else {
                        div { class: "stats-loading", "Loading network stats…" }
                    }
                }
                // ── Panels ────────────────────────────────────────────
                div { class: "dual-col",
                    // ── Latest Blocks ──────────────────────────────────
                    div { class: "panel",
                        div { class: "panel-header",
                            svg { width:"18", height:"18", view_box:"0 0 24 24", fill:"none",
                                stroke:"var(--tel-blue)", stroke_width:"1.5",
                                stroke_linecap:"round", stroke_linejoin:"round",
                                path { d:"M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" }
                                path { d:"M3.27 6.96 12 12.01l8.73-5.05" }
                                path { d:"M12 22.08V12" }
                            }
                            span { class: "panel-title", "Latest Blocks" }
                        }
                        if *loading.read() {
                            Loading { msg: Some("Loading blocks…".to_string()) }
                        } else if let Some(err) = error.read().as_ref() {
                            ErrorBox { msg: err.clone() }
                        } else {
                            div { class: "home-table-header",
                                span { "BLOCK" }
                                span { "AGE" }
                                span { "TXNS" }
                                span { "GAS USED" }
                                span { "LEADER" }
                            }
                            ul { class: "data-list",
                                for block in blocks.read().iter() {
                                    li { class: "home-block-row",
                                        div { class: "hbr-block",
                                            div { class: "hbr-icon",
                                                svg { width:"14", height:"14", view_box:"0 0 24 24", fill:"none",
                                                    stroke:"var(--tel-blue)", stroke_width:"2",
                                                    stroke_linecap:"round", stroke_linejoin:"round",
                                                    path { d:"M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" }
                                                }
                                            }
                                            Link { to: Route::BlockPage { block_number: block.number },
                                                span { class: "hash-cell", "{block.number}" }
                                            }
                                        }
                                        span { class: "hbr-age", "{unix_to_age(block.timestamp)}" }
                                        span { class: "hbr-txns",
                                            span { class: "tx-badge", "{block.transaction_count}" }
                                        }
                                        span { class: "hbr-gas",
                                            { format_gas(block.gas_used) }
                                            span { class: "hbr-gas-pct",
                                                {
                                                    if block.gas_limit > 0 {
                                                        format!(" ({:.1}%)", block.gas_used as f64 / block.gas_limit as f64 * 100.0)
                                                    } else { String::new() }
                                                }
                                            }
                                        }
                                        div { class: "hbr-leader",
                                            Link { to: Route::AddressPage { address: block.validator.clone() },
                                                span { class: "hash-cell", "{shorten_addr(&block.validator)}" }
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "panel-footer",
                                Link { to: Route::BlocksPage { page: 0 }, class: "panel-view-all-footer",
                                    "View All Blocks →"
                                }
                            }
                        }
                    }
                    // ── Latest Transactions ────────────────────────────
                    div { class: "panel",
                        div { class: "panel-header",
                            svg { width:"18", height:"18", view_box:"0 0 24 24", fill:"none",
                                stroke:"var(--tel-blue)", stroke_width:"2",
                                stroke_linecap:"round", stroke_linejoin:"round",
                                path { d:"M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" }
                                path { d:"M14 2v6h6" }
                                path { d:"M16 13H8" }
                                path { d:"M16 17H8" }
                                path { d:"M10 9H8" }
                            }
                            span { class: "panel-title", "Latest Transactions" }
                        }
                        if *loading.read() || *txs_loading.read() {
                            Loading { msg: Some("Loading transactions…".to_string()) }
                        } else if home_txs.read().is_empty() {
                            div { class: "panel-empty",
                                "No transactions in the latest 10 blocks"
                            }
                        } else {
                            div { class: "home-table-header home-tx-header",
                                span { "TX HASH" }
                                span { "METHOD" }
                                span { "BLOCK" }
                                span { "FROM" }
                                span { "TO" }
                                span { "VALUE" }
                            }
                            ul { class: "data-list",
                                for tx in home_txs.read().iter() {
                                    li { class: "home-tx-row",
                                        div { class: "htr-hash",
                                            div { class: "htr-icon",
                                                svg { width:"12", height:"12", view_box:"0 0 24 24", fill:"none",
                                                    stroke:"var(--tel-blue)", stroke_width:"2",
                                                    stroke_linecap:"round", stroke_linejoin:"round",
                                                    path { d:"M5 12h14" }
                                                    path { d:"m12 5 7 7-7 7" }
                                                }
                                            }
                                            Link { to: Route::TransactionPage { hash: tx.hash.clone() },
                                                span { class: "hash-cell", "{shorten_hash(&tx.hash)}" }
                                            }
                                        }
                                        span { class: "htr-method",
                                            {
                                                if let Some(ref di) = tx.decoded_input {
                                                    rsx! { span { class: "method-badge", "{di.method}" } }
                                                } else if tx.input == "0x" || tx.input.is_empty() {
                                                    rsx! { span { class: "method-badge method-transfer", "Transfer" } }
                                                } else {
                                                    rsx! { span { class: "method-badge method-unknown", "Contract Call" } }
                                                }
                                            }
                                        }
                                        span { class: "htr-block",
                                            if let Some(bn) = tx.block_number {
                                                Link { to: Route::BlockPage { block_number: bn },
                                                    span { class: "hash-cell", "#{bn}" }
                                                }
                                            }
                                        }
                                        div { class: "htr-from",
                                            Link { to: Route::AddressPage { address: tx.from.clone() },
                                                span { class: "hash-cell", "{shorten_addr(&tx.from)}" }
                                            }
                                        }
                                        div { class: "htr-to",
                                            {
                                                if let Some(ref to) = tx.to {
                                                    let to_clone = to.clone();
                                                    rsx! {
                                                        Link { to: Route::AddressPage { address: to_clone.clone() },
                                                            span { class: "hash-cell", "{shorten_addr(&to_clone)}" }
                                                        }
                                                    }
                                                } else {
                                                    rsx! { span { class: "method-badge", "Contract Create" } }
                                                }
                                            }
                                        }
                                        span { class: "htr-value",
                                            {
                                                if tx.value_tel > 0.0 {
                                                    format!("{:.4} TEL", tx.value_tel)
                                                } else {
                                                    "0 TEL".to_string()
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "panel-footer",
                                Link { to: Route::TransactionsPage { page: 0 }, class: "panel-view-all-footer",
                                    "View All Transactions →"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
#[component]
fn StatRow(label: String, value: String, sub: Option<String>) -> Element {
    let icon = match label.as_str() {
        "LATEST BLOCK" => rsx! {
            svg { width:"20", height:"20", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"1.5", stroke_linecap:"round", stroke_linejoin:"round",
                path { d:"M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" }
                path { d:"M3.27 6.96 12 12.01l8.73-5.05" }
                path { d:"M12 22.08V12" }
            }
        },
        "CURRENT EPOCH" => rsx! {
            svg { width:"20", height:"20", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"1.5", stroke_linecap:"round", stroke_linejoin:"round",
                path { d:"M12 2L2 7l10 5 10-5-10-5z" }
                path { d:"M2 17l10 5 10-5" }
                path { d:"M2 12l10 5 10-5" }
            }
        },
        "VALIDATORS" => rsx! {
            svg { width:"20", height:"20", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"1.5", stroke_linecap:"round", stroke_linejoin:"round",
                path { d:"M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" }
                circle { cx:"9", cy:"7", r:"4" }
                path { d:"M23 21v-2a4 4 0 0 0-3-3.87" }
                path { d:"M16 3.13a4 4 0 0 1 0 7.75" }
            }
        },
        _ => rsx! {
            svg { width:"20", height:"20", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"1.5", stroke_linecap:"round", stroke_linejoin:"round",
                circle { cx:"12", cy:"12", r:"10" }
                path { d:"M12 8v4" }
                path { d:"M12 16h.01" }
            }
        },
    };
    rsx! {
        div { class: "stat-row",
            div { class: "stat-icon-wrap", {icon} }
            div { class: "stat-row-body",
                span { class: "stat-row-label", "{label}" }
                span { class: "stat-row-value", "{value}" }
                if let Some(s) = sub {
                    span { class: "stat-row-sub", "{s}" }
                }
            }
        }
    }
}
fn run_search() {
    use wasm_bindgen::JsCast;
    let window = match web_sys::window() { Some(w) => w, None => return };
    let doc = match window.document() { Some(d) => d, None => return };
    if let Some(el) = doc.get_element_by_id("home-search") {
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
            let _ = js_sys::eval("var el=document.getElementById('home-search');if(el){el.style.borderColor='#ef4444';el.style.boxShadow='0 0 0 3px rgba(239,68,68,0.2)';setTimeout(function(){el.style.borderColor='';el.style.boxShadow='';},2000);}");
        }
    }
}
