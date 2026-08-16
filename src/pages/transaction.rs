// src/pages/transaction.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_tx_receipt_status, get_transaction, get_token_transfers_for_tx, get_block_by_number,
    Transaction, TokenTransfer, format_wei_exact, shorten_hash, shorten_addr,
    unix_to_age, unix_to_datetime, format_transfer_amount, is_native_tel_transfer};
use crate::components::loading::{Loading, ErrorBox, CopyButton};

#[component]
pub fn TransactionPage(hash: String) -> Element {
    let tx: Signal<Option<Transaction>>      = use_signal(|| None);
    let loading                              = use_signal(|| true);
    let error: Signal<Option<String>>        = use_signal(|| None);
    let mut tx_success: Signal<Option<bool>> = use_signal(|| None);
    let mut input_expanded = use_signal(|| false);
    let mut token_transfers: Signal<Vec<TokenTransfer>> = use_signal(|| vec![]);
    let mut block_timestamp: Signal<Option<u64>> = use_signal(|| None);
    let hash_clone = hash.clone();

    use_effect(move || {
        let hash       = hash_clone.clone();
        let mut tx      = tx.clone();
        let mut loading = loading.clone();
        let mut error   = error.clone();
        let mut tx_success = tx_success.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            match get_transaction(&hash).await {
                Ok(t)  => {
                    // Best-effort: find any ERC-20 transfers that happened inside this
                    // transaction by paging the sender's transfer history (the same
                    // data already shown on address/token pages) filtered to this tx.
                    if let Some(bn) = t.block_number {
                        let hash2 = hash.clone();
                        let from2 = t.from.clone();
                        let (transfers_res, block_res) = futures::join!(
                            get_token_transfers_for_tx(&hash2, &from2, bn),
                            get_block_by_number(bn),
                        );
                        token_transfers.set(transfers_res);
                        if let Ok(block) = block_res {
                            block_timestamp.set(Some(block.timestamp));
                        }
                    }
                    tx.set(Some(t));
                }
                Err(e) => error.set(Some(e)),
            }
            tx_success.set(get_tx_receipt_status(&hash).await);
            loading.set(false);
        });
    });

    rsx! {
        div { class: "page",

            if *loading.read() {
                Loading { msg: Some("Fetching transaction…".to_string()) }
            } else if let Some(err) = error.read().as_ref() {
                ErrorBox { msg: err.clone() }
            } else if let Some(t) = tx.read().as_ref() {
                div { class: "detail-grid",

                    // ── Overview panel ──────────────────────────────────
                    div { class: "detail-panel",
                        div { class: "detail-panel-title", "Transaction Details" }
                        div { class: "detail-table",

                            div { class: "detail-row",
                                div { class: "detail-key", "Status" }
                                div { class: "detail-val",
                                    if t.status == Some(true) {
                                        span { class: "chip success", "✓  Success" }
                                    } else if t.status == Some(false) {
                                        span { class: "chip fail", "✗  Failed" }
                                    } else {
                                        span { class: "chip pending", "⧗  Pending" }
                                    }
                                    span { class: "chip success", style: "margin-left:8px; font-size:10px;",
                                        "⚡ Instant Finality" }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Transaction Hash" }
                                div { class: "detail-val mono-wrap",
                                    "{t.hash}"
                                    CopyButton { text: t.hash.clone() }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Block" }
                                div { class: "detail-val",
                                    if let Some(bn) = t.block_number {
                                        Link { to: Route::BlockPage { block_number: bn },
                                            span { class: "hash-cell", "#{bn}" }
                                        }
                                    } else {
                                        span { "Pending" }
                                    }
                                }
                            }
                            if let Some(ts) = *block_timestamp.read() {
                                div { class: "detail-row",
                                    div { class: "detail-key", "Timestamp" }
                                    div { class: "detail-val",
                                        span { "{unix_to_age(ts)}" }
                                        span { style: "color:var(--text-muted); margin-left:8px; font-size:12px;", "({unix_to_datetime(ts)})" }
                                    }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "From" }
                                div { class: "detail-val",
                                    Link { to: Route::AddressPage { address: t.from.clone() },
                                        span { class: "hash-cell", "{t.from}" }
                                    }
                                    CopyButton { text: t.from.clone() }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "To" }
                                div { class: "detail-val",
                                    if let Some(to) = &t.to {
                                        Link { to: Route::AddressPage { address: to.clone() },
                                            span { class: "hash-cell", "{to}" }
                                        }
                                        CopyButton { text: to.clone() }
                                    } else {
                                        span { class: "chip pending", "Contract Creation" }
                                    }
                                }
                            }
                            if !token_transfers.read().is_empty() {
                                div { class: "detail-row",
                                    div { class: "detail-key",
                                        { format!("Tokens Transferred ({})", token_transfers.read().len()) }
                                    }
                                    div { class: "detail-val", style: "flex-direction:column; align-items:flex-start; gap:6px;",
                                        for transfer in token_transfers.read().iter() {
                                            div { style: "display:flex; align-items:center; gap:8px; flex-wrap:wrap;",
                                                if is_native_tel_transfer(&transfer.token_address) {
                                                    span { class: "chip success", style: "font-size:11px;", "TEL" }
                                                } else {
                                                    Link { to: Route::TokenPage { address: transfer.token_address.clone() },
                                                        span { class: "chip info", style: "font-size:11px;",
                                                            if !transfer.token_symbol.is_empty() { "{transfer.token_symbol}" } else { "{shorten_addr(&transfer.token_address)}" }
                                                        }
                                                    }
                                                }
                                                Link { to: Route::AddressPage { address: transfer.from.clone() },
                                                    span { class: "hash-cell", "{shorten_addr(&transfer.from)}" }
                                                }
                                                span { class: "transfer-arrow", "→" }
                                                Link { to: Route::AddressPage { address: transfer.to.clone() },
                                                    span { class: "hash-cell", "{shorten_addr(&transfer.to)}" }
                                                }
                                                span { style: "color:var(--accent-green); font-weight:600;",
                                                    { format_transfer_amount(transfer) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Value" }
                                div { class: "detail-val",
                                    span { class: "tx-value-big", { format!("{} TEL", format_wei_exact(t.value)) } }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Transaction Fee" }
                                div { class: "detail-val",
                                    {
                                        let fee_wei = t.gas_used as u128 * t.gas_price as u128;
                                        format!("{} TEL", format_wei_exact(fee_wei))
                                    }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Gas Price" }
                                div { class: "detail-val",
                                    {
                                        let gwei_whole = t.gas_price / 1_000_000_000;
                                        let gwei_frac  = t.gas_price % 1_000_000_000;
                                        let gwei = if gwei_frac == 0 {
                                            format!("{gwei_whole}")
                                        } else {
                                            let frac_str = format!("{:09}", gwei_frac);
                                            format!("{gwei_whole}.{}", frac_str.trim_end_matches('0'))
                                        };
                                        format!("{gwei} Gwei ({} wei)", t.gas_price)
                                    }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Gas Used / Limit" }
                                div { class: "detail-val",
                                    {
                                        let pct = if t.gas > 0 { t.gas_used as f64 / t.gas as f64 * 100.0 } else { 0.0 };
                                        format!("{} / {}  ({:.1}%)", t.gas_used, t.gas, pct)
                                    }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Nonce" }
                                div { class: "detail-val", "{t.nonce}" }
                            }
                            if let Some(idx) = t.transaction_index {
                                div { class: "detail-row",
                                    div { class: "detail-key", "Position in Block" }
                                    div { class: "detail-val", "{idx}" }
                                }
                            }
                        }
                    }

                    // ── Input Data panel ────────────────────────────────
                    div { class: "detail-panel",
                        div { class: "detail-panel-title", "Input Data" }
                        div { class: "detail-table",
                            if t.input == "0x" || t.input.is_empty() {
                                div { class: "empty-state", "No input data (simple TEL transfer)" }
                            } else {
                                if let Some(decoded) = &t.decoded_input {
                                    div { class: "detail-row",
                                        div { class: "detail-key", "Method" }
                                        div { class: "detail-val",
                                            span { class: "chip pending method-chip", "{decoded.method}" }
                                        }
                                    }
                                    div { class: "detail-row",
                                        div { class: "detail-key", "Signature" }
                                        div { class: "detail-val",
                                            code { class: "code-inline", "{decoded.signature}" }
                                        }
                                    }
                                    for (param_name, param_val) in decoded.params.iter() {
                                        div { class: "detail-row",
                                            div { class: "detail-key",
                                                span { style: "color: var(--tel-blue);", "{param_name}" }
                                            }
                                            div { class: "detail-val",
                                                if param_val.starts_with("0x") && param_val.len() == 42 {
                                                    Link { to: Route::AddressPage { address: param_val.clone() },
                                                        span { class: "hash-cell", "{param_val}" }
                                                    }
                                                } else {
                                                    span { "{param_val}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key",
                                        "Raw Hex"
                                        span { style: "display:block; font-size:11px; color:var(--text-muted); font-weight:400; margin-top:2px;",
                                            { format!("{} bytes", (t.input.len().saturating_sub(2)) / 2) }
                                        }
                                    }
                                    div { class: "detail-val", style: "flex-direction:column; align-items:flex-start; gap:8px;",
                                        div {
                                            class: "input-hex-box",
                                            style: if *input_expanded.read() {
                                                "max-height:none; overflow-wrap:anywhere; word-break:break-all;"
                                            } else {
                                                "max-height:120px; overflow:hidden; overflow-wrap:anywhere; word-break:break-all; position:relative;"
                                            },
                                            "{t.input}"
                                        }
                                        div { style: "display:flex; gap:10px; align-items:center;",
                                            if t.input.len() > 400 {
                                                button {
                                                    class: "action-link",
                                                    style: "background:none; border:none; cursor:pointer; padding:0; font-size:12px;",
                                                    onclick: move |_| { let cur = *input_expanded.read(); input_expanded.set(!cur); },
                                                    if *input_expanded.read() { "Show Less ▲" } else { "Show Full Input Data ▼" }
                                                }
                                            }
                                            CopyButton { text: t.input.clone() }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
