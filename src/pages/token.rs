// src/pages/token.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_token_info, get_token_transfers_page, get_registered_token,
    TokenInfo, TokenTransfer, RegisteredToken,
    shorten_hash, shorten_addr, format_amount, format_token_amount,
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
    let mut registry_entry: Signal<Option<RegisteredToken>> = use_signal(|| None);
    let mut transfers_page: Signal<u64> = use_signal(|| 0);
    let mut more_loading = use_signal(|| false);

    // use_reactive: `address` is a plain String prop, not a Signal, so
    // without this the effect only runs once on first mount and never
    // restarts when navigating between two TokenPage instances via Link
    // (Dioxus reuses the same component/hook state across route param
    // changes for the same route type). See:
    // https://github.com/DioxusLabs/dioxus/issues/2784
    use_effect(use_reactive(&address, move |address| {
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            // Clear stale data from any previously-viewed token immediately.
            token.set(None);
            transfers.set(vec![]);
            transfers_total.set(0);
            not_token.set(false);
            registry_entry.set(None);
            transfers_page.set(0);
            let (info_res, registry_res) = futures::join!(
                get_token_info(&address),
                get_registered_token(&address),
            );
            registry_entry.set(registry_res);
            match info_res {
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
    }));

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
                // Overview -- compact logo/name/symbol sits as this card's own
                // header instead of a large standalone block above it, avoiding
                // the redundant "big header, then immediately another card" feel.
                div { class: "detail-panel",
                    div { class: "token-overview-header",
                        div { class: "token-icon-wrap token-icon-wrap-sm",
                            if let Some(logo) = registry_entry.read().as_ref().map(|r| r.logo_uri.clone()).filter(|l| !l.is_empty()) {
                                img { src: "{logo}", alt: "{t.symbol}", style: "width:100%; height:100%; border-radius:50%; object-fit:cover;" }
                            } else {
                                span { class: "token-icon-letter",
                                    { t.symbol.chars().next().unwrap_or('T').to_string() }
                                }
                            }
                        }
                        div { class: "token-overview-title-block",
                            div { class: "token-overview-name-row",
                                span { class: "token-overview-name", "{t.name}" }
                                span { class: "token-symbol-badge", "{t.symbol}" }
                            }
                            span { class: "token-overview-subtitle", "Token Overview" }
                        }
                    }
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
                                { format!("{} {}", format_token_amount(&t.total_supply, t.decimals), t.symbol) }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Token Standard" }
                            div { class: "detail-val",
                                span { class: "chip info", "ERC-20" }
                            }
                        }
                        if let Some(website) = registry_entry.read().as_ref().map(|r| r.website.clone()).filter(|w| !w.is_empty()) {
                            div { class: "detail-row",
                                div { class: "detail-key", "Website" }
                                div { class: "detail-val",
                                    a { href: "{website}", target: "_blank", rel: "noopener noreferrer nofollow", class: "action-link", "{website} ↗" }
                                    span { style: "font-size:11px; color:var(--text-muted); margin-left:8px;", "(self-reported, unverified)" }
                                }
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
                                            { format!("{} {}", format_amount(tx.amount), t.symbol) }
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
