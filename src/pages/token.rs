// src/pages/token.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_token_info, get_token_transfers_page,
    TokenInfo, TokenTransfer,
    shorten_hash, shorten_addr,
};
use crate::components::loading::{Loading, ErrorBox, CopyButton};

#[component]
pub fn TokenPage(address: String) -> Element {
    let mut token: Signal<Option<TokenInfo>>      = use_signal(|| None);
    let mut transfers: Signal<Vec<TokenTransfer>> = use_signal(|| vec![]);
    let mut transfers_total: Signal<u64>          = use_signal(|| 0);
    let mut loading  = use_signal(|| true);
    let error: Signal<Option<String>>             = use_signal(|| None);
    let mut not_token = use_signal(|| false);
    let mut transfers_page: Signal<u64> = use_signal(|| 0);
    let mut more_loading = use_signal(|| false);

    let addr_clone = address.clone();
    use_effect(move || {
        let address = addr_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            match get_token_info(&address).await {
                Some(info) => {
                    token.set(Some(info));
                    // Full transfer history — indexed by token contract, no block-range limit.
                    if let Ok((xfers, total)) = get_token_transfers_page(&address, 0, 25).await {
                        transfers.set(xfers);
                        transfers_total.set(total);
                    }
                }
                None => { not_token.set(true); }
            }
            loading.set(false);
        });
    });

    let load_more = {
        let address = address.clone();
        move |_| {
            let address = address.clone();
            wasm_bindgen_futures::spawn_local(async move {
                more_loading.set(true);
                let next = *transfers_page.read() + 1;
                if let Ok((mut more, _)) = get_token_transfers_page(&address, next, 25).await {
                    transfers.write().append(&mut more);
                    transfers_page.set(next);
                }
                more_loading.set(false);
            });
        }
    };

    rsx! {
        div { class: "page",
            if *loading.read() {
                Loading { msg: Some("Loading token info…".to_string()) }
            } else if *not_token.read() {
                div { class: "detail-panel",
                    div { class: "empty-state",
                        "This address is not an ERC-20 token contract."
                        br {}
                        Link { to: Route::AddressPage { address: address.clone() },
                            span { class: "hash-cell", "View as Address →" }
                        }
                    }
                }
            } else if let Some(err) = error.read().as_ref() {
                ErrorBox { msg: err.clone() }
            } else if let Some(t) = token.read().as_ref() {
                // Header
                div { class: "token-page-header",
                    div { class: "token-icon-wrap",
                        span { class: "token-icon-letter",
                            { t.symbol.chars().next().unwrap_or('T').to_string() }
                        }
                    }
                    div {
                        h1 { class: "page-title", "{t.name}" }
                        span { class: "token-symbol-badge", "{t.symbol}" }
                    }
                }
                // Overview
                div { class: "detail-panel",
                    div { class: "detail-panel-title", "Token Overview" }
                    div { class: "detail-table",
                        div { class: "detail-row",
                            div { class: "detail-key", "Contract Address" }
                            div { class: "detail-val",
                                Link { to: Route::AddressPage { address: t.address.clone() },
                                    span { class: "hash-cell", "{t.address}" }
                                }
                                CopyButton { text: t.address.clone() }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Name" }
                            div { class: "detail-val", "{t.name}" }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Symbol" }
                            div { class: "detail-val",
                                span { class: "token-symbol-badge", "{t.symbol}" }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Decimals" }
                            div { class: "detail-val", "{t.decimals}" }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Total Supply" }
                            div { class: "detail-val",
                                { format!("{} {}", t.total_supply, t.symbol) }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Token Standard" }
                            div { class: "detail-val",
                                span { class: "chip info", "ERC-20" }
                            }
                        }
                    }
                }
                // Recent Transfers
                div { class: "detail-panel",
                    div { class: "detail-panel-title",
                        { format!("Recent Transfers ({} total)", transfers_total.read()) }
                    }
                    if transfers.read().is_empty() {
                        div { class: "empty-state", "No transfers found for this token." }
                    } else {
                        div { class: "block-tx-table",
                            div { class: "btx-header",
                                span { class: "btx-col-hash", "TX HASH" }
                                span { class: "btx-col-from", "FROM" }
                                span { class: "btx-col-to", "TO" }
                                span { class: "btx-col-value", "AMOUNT" }
                                span { class: "btx-col-fee", "BLOCK" }
                            }
                            for tx in transfers.read().iter() {
                                div { class: "btx-row",
                                    div { class: "btx-col-hash",
                                        Link { to: Route::TransactionPage { hash: tx.tx_hash.clone() },
                                            span { class: "hash-cell", "{shorten_hash(&tx.tx_hash)}" }
                                        }
                                    }
                                    div { class: "btx-col-from",
                                        Link { to: Route::AddressPage { address: tx.from.clone() },
                                            span { class: "hash-cell small", "{shorten_addr(&tx.from)}" }
                                        }
                                    }
                                    div { class: "btx-col-to",
                                        Link { to: Route::AddressPage { address: tx.to.clone() },
                                            span { class: "hash-cell small", "{shorten_addr(&tx.to)}" }
                                        }
                                    }
                                    div { class: "btx-col-value",
                                        span { class: "btx-value",
                                            { format!("{} {}", tx.amount, t.symbol) }
                                        }
                                    }
                                    div { class: "btx-col-fee",
                                        Link { to: Route::BlockPage { block_number: tx.block_number },
                                            span { class: "hash-cell small",
                                                { format!("#{}", tx.block_number) }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if transfers.read().len() < *transfers_total.read() as usize {
                            div { style: "padding:16px; text-align:center;",
                                button {
                                    class: "contract-fn-btn contract-fn-btn-read",
                                    disabled: *more_loading.read(),
                                    onclick: load_more,
                                    if *more_loading.read() { "Loading…" } else { "Load More" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
