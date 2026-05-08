// src/pages/address.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    is_contract,
    get_balance, get_tx_count, get_block_number, get_token_transfers,
    parse_transfer_logs, get_token_symbol, TokenTransfer, shorten_hash, shorten_addr,
    unix_to_age, CONSENSUS_REGISTRY,
};
use crate::components::loading::{Loading, ErrorBox, CopyButton};

#[component]
pub fn AddressPage(address: String) -> Element {
    let balance: Signal<Option<f64>>         = use_signal(|| None);
    let tx_count: Signal<Option<u64>>        = use_signal(|| None);
    let transfers: Signal<Vec<TokenTransfer>> = use_signal(|| vec![]);
    let loading                              = use_signal(|| true);
    let error: Signal<Option<String>>        = use_signal(|| None);
    let active_tab                           = use_signal(|| "transfers");
    let mut contract_flag: Signal<bool>      = use_signal(|| false);
    let addr_clone = address.clone();

    use_effect(move || {
        let address        = addr_clone.clone();
        let mut balance    = balance.clone();
        let mut tx_count   = tx_count.clone();
        let mut transfers  = transfers.clone();
        let mut loading    = loading.clone();
        let mut error      = error.clone();
        let mut contract_flag = contract_flag.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            match get_balance(&address).await {
                Ok(b)  => balance.set(Some(b)),
                Err(e) => error.set(Some(e)),
            }
            if let Ok(n) = get_tx_count(&address).await {
                tx_count.set(Some(n));
            }
            if let Ok(latest) = get_block_number().await {
                let from = latest.saturating_sub(5000);
                if let Ok(logs) = get_token_transfers(&address, from, latest).await {
                    let mut parsed = parse_transfer_logs(logs);
                    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                    for t in parsed.iter_mut() {
                        let sym = if let Some(s) = seen.get(&t.token_address) {
                            s.clone()
                        } else {
                            let s = get_token_symbol(&t.token_address).await;
                            seen.insert(t.token_address.clone(), s.clone());
                            s
                        };
                        t.token_symbol = sym;
                    }
                    transfers.set(parsed);
                }
            }
            contract_flag.set(is_contract(&address).await);
            loading.set(false);
        });
    });

    let avatar_char = address.chars().nth(2).unwrap_or('?').to_uppercase().next().unwrap_or('?');
    let is_registry = address.to_lowercase() == CONSENSUS_REGISTRY.to_lowercase();

    rsx! {
        div { class: "page",

            if *loading.read() {
                Loading { msg: Some("Fetching address data…".to_string()) }
            } else {

                // ── Address header ──────────────────────────────────────
                div { class: "address-header",
                    div { class: "address-avatar", "{avatar_char}" }
                    div { class: "address-info",

                        // Type badges
                        div { class: "address-type-row",
                            if is_registry {
                                span { class: "addr-type-badge contract", "ConsensusRegistry" }
                            } else if *contract_flag.read() {
                                span { class: "addr-type-badge contract", "Contract" }
                            } else {
                                span { class: "addr-type-badge eoa", "EOA" }
                            }
                            if is_registry {
                                Link { to: Route::ValidatorsPage {},
                                    span { class: "chip success", style: "cursor:pointer; font-size:11px;", "View Validators →" }
                                }
                            }
                            if *contract_flag.read() {
                                Link { to: Route::ContractPage { address: address.clone() },
                                    span { class: "chip info", style: "cursor:pointer; font-size:11px;", "View Contract →" }
                                }
                            }
                        }

                        // Address + copy
                        div { class: "address-hash-row",
                            span { class: "address-hash-text", "{address}" }
                            CopyButton { text: address.clone() }
                        }

                        // Balance
                        if let Some(bal) = *balance.read() {
                            div { class: "address-balance-big",
                                { format!("{:.6}", bal) }
                                span { "TEL" }
                            }
                        }

                        // Tx count
                        if let Some(nonce) = *tx_count.read() {
                            div { class: "address-meta",
                                "Transactions sent: "
                                span { class: "address-meta-val", "{nonce}" }
                            }
                        }
                    }
                }

                if let Some(err) = error.read().as_ref() {
                    ErrorBox { msg: err.clone() }
                }

                // ── Tabs ────────────────────────────────────────────────
                div { class: "tabs-row",
                    button {
                        class: if *active_tab.read() == "transfers" { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| active_tab.clone().set("transfers"),
                        "ERC-20 Transfers"
                        span { class: "tab-count", " ({transfers.read().len()})" }
                    }
                }

                // ── Token Transfers ─────────────────────────────────────
                div { class: "panel",
                    div { class: "panel-header",
                        span { class: "panel-title", "ERC-20 Token Transfers" }
                        span { style: "color:var(--text-muted); font-size:11px;", "Last 5,000 blocks" }
                    }
                    div { class: "table-wrapper",
                        if transfers.read().is_empty() {
                            div { class: "empty-state",
                                div { style: "font-size:32px; margin-bottom:12px;", "📭" }
                                "No ERC-20 transfers found in the last 5,000 blocks"
                            }
                        } else {
                            table { class: "tx-table",
                                thead {
                                    tr {
                                        th { "TX HASH" }
                                        th { "BLOCK" }
                                        th { "FROM" }
                                        th { "" }
                                        th { "TO" }
                                        th { "TOKEN" }
                                        th { "AMOUNT" }
                                    }
                                }
                                tbody {
                                    for transfer in transfers.read().iter() {
                                        tr {
                                            td {
                                                Link { to: Route::TransactionPage { hash: transfer.tx_hash.clone() },
                                                    span { class: "hash-cell", "{shorten_hash(&transfer.tx_hash)}" }
                                                }
                                            }
                                            td {
                                                Link { to: Route::BlockPage { block_number: transfer.block_number },
                                                    span { class: "hash-cell", "#{transfer.block_number}" }
                                                }
                                            }
                                            td {
                                                Link { to: Route::AddressPage { address: transfer.from.clone() },
                                                    span { class: "hash-cell addr-short", "{shorten_addr(&transfer.from)}" }
                                                }
                                            }
                                            td { span { class: "transfer-arrow", "→" } }
                                            td {
                                                Link { to: Route::AddressPage { address: transfer.to.clone() },
                                                    span { class: "hash-cell addr-short", "{shorten_addr(&transfer.to)}" }
                                                }
                                            }
                                            td {
                                                if !transfer.token_symbol.is_empty() {
                                                    Link { to: Route::AddressPage { address: transfer.token_address.clone() },
                                                        span { class: "chip info", style: "font-size:11px;", "{transfer.token_symbol}" }
                                                    }
                                                } else {
                                                    Link { to: Route::AddressPage { address: transfer.token_address.clone() },
                                                        span { class: "hash-cell addr-short", "{shorten_addr(&transfer.token_address)}" }
                                                    }
                                                }
                                            }
                                            td { style: "color:var(--accent-green); font-weight:600; font-family:var(--font-mono); font-size:12px;",
                                                { format!("{:.4}", transfer.amount) }
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
}
