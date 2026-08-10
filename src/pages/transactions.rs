// src/pages/transactions.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_latest_txs,
    Transaction, shorten_hash, shorten_addr,
};
use crate::components::loading::{Loading, ErrorBox};

const PER_PAGE: u64 = 25;

#[component]
pub fn TransactionsPage(page: u64) -> Element {
    let mut txs:   Signal<Vec<Transaction>> = use_signal(|| vec![]);
    let mut total: Signal<u64>              = use_signal(|| 0);
    let mut loading                         = use_signal(|| true);
    let mut error: Signal<Option<String>>   = use_signal(|| None);

    let mut current_page = use_signal(|| page);
    if *current_page.read() != page {
        current_page.set(page);
    }

    use_effect(move || {
        let p = *current_page.read();
        txs.set(vec![]);
        loading.set(true);
        error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match get_latest_txs(p, PER_PAGE).await {
                Ok((t, tot)) => { txs.set(t); total.set(tot); }
                Err(e)       => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    let total_txs   = *total.read();
    let total_pages = total_txs.saturating_div(PER_PAGE);
    let prev_page   = page.saturating_sub(1);
    let next_page   = page + 1;

    rsx! {
        div { class: "blocks-full-wrap",
            div { class: "blocks-inner",
                div { class: "blocks-page-header",
                    div {
                        h1 { class: "page-title", style: "margin-bottom:4px;", "Transactions" }
                        div { class: "page-subtitle",
                            span { class: "highlight", { format!("{}", total_txs) } }
                            " total transactions · Page "
                            span { class: "highlight", { format!("{}", page + 1) } }
                            " of "
                            span { class: "highlight", { format!("{}", total_pages + 1) } }
                        }
                    }
                    div { class: "blocks-page-nav",
                        if page > 0 {
                            Link { to: Route::TransactionsPage { page: 0 },
                                span { class: "page-btn-link", "« Latest" }
                            }
                            Link { to: Route::TransactionsPage { page: prev_page },
                                span { class: "page-btn-link", "← Newer" }
                            }
                        }
                        if page < total_pages {
                            Link { to: Route::TransactionsPage { page: next_page },
                                span { class: "page-btn-link", "Older →" }
                            }
                            Link { to: Route::TransactionsPage { page: total_pages },
                                span { class: "page-btn-link", "Oldest »" }
                            }
                        } else {
                            span { class: "page-btn-link disabled", "Older →" }
                            span { class: "page-btn-link disabled", "Oldest »" }
                        }
                    }
                }
                if *loading.read() {
                    Loading { msg: Some(format!("Fetching page {}...", page + 1)) }
                } else if let Some(err) = error.read().as_ref() {
                    ErrorBox { msg: err.clone() }
                } else if txs.read().is_empty() {
                    div { class: "empty-state",
                        div { style: "font-size:32px; margin-bottom:12px;", "📭" }
                        "No transactions found"
                    }
                } else {
                    div { class: "blocks-table-wrap",
                        table { class: "blocks-table",
                            thead {
                                tr {
                                    th { "TX HASH" }
                                    th { "METHOD" }
                                    th { "BLOCK" }
                                    th { "FROM" }
                                    th { "" }
                                    th { "TO" }
                                    th { "VALUE" }
                                    th { "FEE" }
                                }
                            }
                            tbody {
                                for tx in txs.read().iter() {
                                    tr {
                                        td {
                                            Link { to: Route::TransactionPage { hash: tx.hash.clone() },
                                                span { class: "hash-cell", "{shorten_hash(&tx.hash)}" }
                                            }
                                        }
                                        td {
                                            {
                                                if let Some(ref di) = tx.decoded_input {
                                                    rsx! { span { class: "method-badge", "{di.method}" } }
                                                } else if tx.input == "0x" || tx.input.is_empty() {
                                                    rsx! { span { class: "method-badge method-transfer", "Transfer" } }
                                                } else {
                                                    rsx! { span { class: "method-badge method-unknown", "Call" } }
                                                }
                                            }
                                        }
                                        td {
                                            if let Some(bn) = tx.block_number {
                                                Link { to: Route::BlockPage { block_number: bn },
                                                    span { class: "hash-cell", "#{bn}" }
                                                }
                                            }
                                        }
                                        td {
                                            Link { to: Route::AddressPage { address: tx.from.clone() },
                                                span { class: "hash-cell addr-short", "{shorten_addr(&tx.from)}" }
                                            }
                                        }
                                        td { span { class: "transfer-arrow", "→" } }
                                        td {
                                            if let Some(ref to) = tx.to {
                                                Link { to: Route::AddressPage { address: to.clone() },
                                                    span { class: "hash-cell addr-short", "{shorten_addr(to)}" }
                                                }
                                            } else {
                                                span { class: "method-badge method-unknown", "Create" }
                                            }
                                        }
                                        td { class: "td-mono",
                                            if tx.value_tel > 0.0 {
                                                span { style: "color:var(--accent-green);",
                                                    { format!("{:.4}", tx.value_tel) }
                                                }
                                            } else {
                                                span { class: "td-faint", "0" }
                                            }
                                        }
                                        td { class: "td-mono td-faint",
                                            {
                                                let fee = tx.gas_used as f64 * tx.gas_price as f64 / 1e18;
                                                if fee > 0.0 { format!("{:.6}", fee) } else { "—".to_string() }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "blocks-pagination",
                        if page > 0 {
                            Link { to: Route::TransactionsPage { page: 0 },
                                span { class: "page-btn-link", "Latest" }
                            }
                            Link { to: Route::TransactionsPage { page: prev_page },
                                span { class: "page-btn-link", "Newer" }
                            }
                        } else {
                            span { class: "page-btn-link disabled", "Latest" }
                            span { class: "page-btn-link disabled", "Newer" }
                        }
                        span { class: "page-info",
                            { format!("Page {} of {}", page + 1, total_pages + 1) }
                        }
                        if page < total_pages {
                            Link { to: Route::TransactionsPage { page: next_page },
                                span { class: "page-btn-link", "Older" }
                            }
                            Link { to: Route::TransactionsPage { page: total_pages },
                                span { class: "page-btn-link", "Oldest »" }
                            }
                        } else {
                            span { class: "page-btn-link disabled", "Older" }
                            span { class: "page-btn-link disabled", "Oldest »" }
                        }
                    }
                }
            }
        }
    }
}
