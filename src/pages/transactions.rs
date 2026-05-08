// src/pages/transactions.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_block_number, get_block_by_number, get_transaction,
    Transaction, shorten_hash, shorten_addr, unix_to_age,
};
use crate::components::loading::{Loading, ErrorBox};

const BLOCKS_TO_SCAN: u64 = 15;  // scan last N blocks for tx hashes
const TXS_PER_PAGE:   usize = 25;

#[component]
pub fn TransactionsPage(page: u64) -> Element {
    let mut txs:     Signal<Vec<Transaction>> = use_signal(|| vec![]);
    let mut total:   Signal<usize>            = use_signal(|| 0);
    let mut loading                           = use_signal(|| true);
    let mut error:   Signal<Option<String>>   = use_signal(|| None);
    let mut scanned: Signal<u64>              = use_signal(|| 0);

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
            // 1. Get latest block
            let latest = match get_block_number().await {
                Ok(n)  => n,
                Err(e) => { error.set(Some(e)); loading.set(false); return; }
            };
            scanned.set(latest);

            // 2. Collect tx hashes from recent blocks (parallel block fetches)
            let block_nums: Vec<u64> = (0..BLOCKS_TO_SCAN)
                .map(|i| latest.saturating_sub(i))
                .filter(|&n| n > 0)
                .collect();

            let block_futures: Vec<_> = block_nums.iter()
                .map(|&n| get_block_by_number(n))
                .collect();

            let block_results = futures::future::join_all(block_futures).await;

            let mut all_hashes: Vec<String> = block_results
                .into_iter()
                .filter_map(|r| r.ok())
                .flat_map(|b| b.transactions)
                .collect();

            total.set(all_hashes.len());

            // 3. Paginate hash list
            let start = p as usize * TXS_PER_PAGE;
            let end   = (start + TXS_PER_PAGE).min(all_hashes.len());
            if start >= all_hashes.len() {
                loading.set(false);
                return;
            }
            let page_hashes = all_hashes.drain(start..end).collect::<Vec<_>>();

            // 4. Fetch tx details in parallel
            let tx_futures: Vec<_> = page_hashes.iter()
                .map(|h| get_transaction(h))
                .collect();

            let tx_results = futures::future::join_all(tx_futures).await;
            let page_txs: Vec<Transaction> = tx_results
                .into_iter()
                .filter_map(|r| r.ok())
                .collect();

            txs.set(page_txs);
            loading.set(false);
        });
    });

    let total_txs   = *total.read();
    let total_pages = if total_txs == 0 { 0 } else { (total_txs - 1) / TXS_PER_PAGE };
    let prev_page   = page.saturating_sub(1);
    let next_page   = page + 1;

    rsx! {
        div { class: "blocks-full-wrap",
            div { class: "blocks-inner",

                div { class: "blocks-page-header",
                    div {
                        h1 { class: "page-title", style: "margin-bottom:4px;", "Transactions" }
                        div { class: "page-subtitle",
                            "Latest "
                            span { class: "highlight", { format!("{}", total_txs) } }
                            " transactions from the last "
                            span { class: "highlight", { format!("{}", BLOCKS_TO_SCAN) } }
                            " blocks · "
                            span { style: "color:var(--text-muted);",
                                "Full history available after indexer launch"
                            }
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
                        if page < total_pages as u64 {
                            Link { to: Route::TransactionsPage { page: next_page },
                                span { class: "page-btn-link", "Older →" }
                            }
                        } else {
                            span { class: "page-btn-link disabled", "Older →" }
                        }
                    }
                }

                if *loading.read() {
                    Loading { msg: Some(format!("Scanning last {} blocks for transactions…", BLOCKS_TO_SCAN)) }
                } else if let Some(err) = error.read().as_ref() {
                    ErrorBox { msg: err.clone() }
                } else if txs.read().is_empty() {
                    div { class: "empty-state",
                        div { style: "font-size:32px; margin-bottom:12px;", "📭" }
                        "No transactions found in the last {BLOCKS_TO_SCAN} blocks"
                    }
                } else {
                    div { class: "blocks-table-wrap",
                        table { class: "blocks-table",
                            thead {
                                tr {
                                    th { "TX HASH" }
                                    th { "METHOD" }
                                    th { "BLOCK" }
                                    th { "AGE" }
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
                                        td { class: "td-muted",
                                            if let Some(bn) = tx.block_number {
                                                // We don't have the timestamp here, show block number age approximation
                                                span { "#{bn}" }
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
                        if page < total_pages as u64 {
                            Link { to: Route::TransactionsPage { page: next_page },
                                span { class: "page-btn-link", "Older" }
                            }
                        } else {
                            span { class: "page-btn-link disabled", "Older" }
                        }
                    }
                }
            }
        }
    }
}
