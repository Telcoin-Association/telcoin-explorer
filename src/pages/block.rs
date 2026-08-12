// src/pages/block.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_block_by_number, get_transactions_for_block,
    Block, Transaction, shorten_hash, shorten_addr,
    unix_to_age, unix_to_datetime, format_wei_exact,
};
use crate::components::loading::{Loading, ErrorBox, CopyButton, AddrDisplay};

#[component]
pub fn BlockPage(block_number: u64) -> Element {
    let block: Signal<Option<Block>>           = use_signal(|| None);
    let txs:   Signal<Vec<Transaction>>        = use_signal(|| vec![]);
    let loading     = use_signal(|| true);
    let loading_txs = use_signal(|| false);
    let error: Signal<Option<String>>          = use_signal(|| None);

    use_effect(move || {
        let mut block       = block.clone();
        let mut txs         = txs.clone();
        let mut loading     = loading.clone();
        let mut loading_txs = loading_txs.clone();
        let mut error       = error.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            match get_block_by_number(block_number).await {
                Ok(b) => {
                    let hashes = b.transactions.clone();
                    block.set(Some(b));
                    loading.set(false);
                    // Fetch tx details separately so block info shows immediately
                    if !hashes.is_empty() {
                        loading_txs.set(true);
                        let fetched = get_transactions_for_block(&hashes).await;
                        txs.set(fetched);
                        loading_txs.set(false);
                    }
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });
    });

    rsx! {
        div { class: "page",

            if *loading.read() {
                Loading { msg: Some(format!("Fetching block #{}…", block_number)) }
            } else if let Some(err) = error.read().as_ref() {
                ErrorBox { msg: err.clone() }
            } else if let Some(b) = block.read().as_ref() {

                // ── Page title + nav ──────────────────────────────────
                div { class: "block-page-header",
                    div { class: "block-page-title",
                        svg { width:"20", height:"20", view_box:"0 0 24 24", fill:"none",
                            stroke:"var(--tel-blue)", stroke_width:"2", stroke_linecap:"round", stroke_linejoin:"round",
                            path { d:"M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" }
                        }
                        h1 { class: "page-title", "Block " span { class: "page-title-num", "#{b.number}" } }
                    }
                    div { class: "block-nav-btns",
                        if b.number > 0 {
                            Link { to: Route::BlockPage { block_number: b.number - 1 },
                                button { class: "page-btn", "← Prev" }
                            }
                        }
                        Link { to: Route::BlockPage { block_number: b.number + 1 },
                            button { class: "page-btn", "Next →" }
                        }
                    }
                }

                // ── Overview panel ────────────────────────────────────
                div { class: "detail-panel",
                    div { class: "detail-panel-title", "Overview" }
                    div { class: "detail-table",

                        div { class: "detail-row",
                            div { class: "detail-key", "Block Height" }
                            div { class: "detail-val",
                                span { "#{b.number}" }
                                CopyButton { text: b.number.to_string() }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Timestamp" }
                            div { class: "detail-val",
                                svg { width:"14", height:"14", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", style:"margin-right:6px;opacity:0.5;",
                                    circle { cx:"12", cy:"12", r:"10" }
                                    path { d:"M12 6v6l4 2" }
                                }
                                { format!("{}  ({})", unix_to_datetime(b.timestamp), unix_to_age(b.timestamp)) }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Transactions" }
                            div { class: "detail-val",
                                span { class: "tx-count-badge",
                                    { format!("{} transaction{}", b.transaction_count,
                                        if b.transaction_count == 1 { "" } else { "s" }) }
                                }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Leader" }
                            div { class: "detail-val",
                                Link { to: Route::AddressPage { address: b.validator.clone() },
                                    span { class: "hash-cell", "{b.validator}" }
                                }
                                CopyButton { text: b.validator.clone() }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Block Hash" }
                            div { class: "detail-val mono-wrap",
                                span { class: "hash-cell", "{b.hash}" }
                                CopyButton { text: b.hash.clone() }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Parent Hash" }
                            div { class: "detail-val mono-wrap",
                                if b.number > 0 {
                                    Link { to: Route::BlockPage { block_number: b.number - 1 },
                                        span { class: "hash-cell", "{b.parent_hash}" }
                                    }
                                } else {
                                    span { class: "hash-cell", "{b.parent_hash}" }
                                }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Gas Used / Limit" }
                            div { class: "detail-val",
                                {
                                    let pct = if b.gas_limit > 0 {
                                        b.gas_used as f64 / b.gas_limit as f64 * 100.0
                                    } else { 0.0 };
                                    let color = if pct > 80.0 { "var(--accent-green)" }
                                        else if pct > 40.0 { "var(--tel-blue)" }
                                        else { "var(--text-muted)" };
                                    rsx! {
                                        span { { format!("{} / {} ", b.gas_used, b.gas_limit) } }
                                        span { style: "color:{color}; font-weight:600;",
                                            { format!("({:.1}%)", pct) }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Consensus" }
                            div { class: "detail-val",
                                span { class: "chip info", "Narwhal / Bullshark DAG-BFT" }
                                span { class: "chip success", style: "margin-left:8px;",
                                    title: "Blocks are final once 2f+1 validators commit the subdag. No reorgs possible.",
                                    "✓ Instant Finality"
                                }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Block Leader" }
                            div { class: "detail-val",
                                span { class: "block-leader-note",
                                    "The validator address shown is the "
                                    strong { "leader" }
                                    " selected by Bullshark for this consensus round. All validators contributed to committing this block."
                                }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Size" }
                            div { class: "detail-val", "{b.size} bytes" }
                        }
                    }
                }

                // ── Transactions panel ────────────────────────────────
                div { class: "detail-panel",
                    div { class: "detail-panel-title-row",
                        div { class: "detail-panel-title",
                            { format!("{} Transaction{}", b.transaction_count,
                                if b.transaction_count == 1 { "" } else { "s" }) }
                        }
                    }

                    if b.transactions.is_empty() {
                        div { class: "empty-state", "No transactions in this block." }
                    } else if *loading_txs.read() {
                        Loading { msg: Some("Loading transactions…".to_string()) }
                    } else {
                        div { class: "block-tx-table",
                            // Header
                            div { class: "btx-header",
                                span { class: "btx-col-hash", "TX HASH" }
                                span { class: "btx-col-from", "FROM" }
                                span { class: "btx-col-to", "TO" }
                                span { class: "btx-col-value", "VALUE" }
                                span { class: "btx-col-fee", "GAS USED" }
                            }
                            // Rows
                            for tx in txs.read().iter() {
                                div { class: "btx-row",
                                    // Hash + status
                                    div { class: "btx-col-hash",
                                        Link { to: Route::TransactionPage { hash: tx.hash.clone() },
                                            span { class: "hash-cell", "{shorten_hash(&tx.hash)}" }
                                        }
                                        if let Some(true) = tx.status {
                                            span { class: "btx-status success", "✓" }
                                        } else if let Some(false) = tx.status {
                                            span { class: "btx-status failed", "✗" }
                                        }
                                    }
                                    // From
                                    div { class: "btx-col-from",
                                        Link { to: Route::AddressPage { address: tx.from.clone() },
                                            span { class: "hash-cell small", "{shorten_addr(&tx.from)}" }
                                        }
                                    }
                                    // To
                                    div { class: "btx-col-to",
                                        if let Some(to) = &tx.to {
                                            Link { to: Route::AddressPage { address: to.clone() },
                                                span { class: "hash-cell small", "{shorten_addr(to)}" }
                                            }
                                        } else {
                                            span { class: "btx-contract", "Contract Deploy" }
                                        }
                                    }
                                    // Value
                                    div { class: "btx-col-value",
                                        span { class: "btx-value",
                                            { format!("{} TEL", format_wei_exact(tx.value)) }
                                        }
                                    }
                                    // Gas
                                    div { class: "btx-col-fee",
                                        span { class: "btx-fee", "{tx.gas_used}" }
                                    }
                                }
                            }
                            // If block has more txs than we fetched
                            if b.transaction_count > 50 {
                                div { class: "btx-more",
                                    { format!("Showing 50 of {} transactions", b.transaction_count) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
