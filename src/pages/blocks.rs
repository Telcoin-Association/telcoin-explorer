use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{get_blocks_page, Block, shorten_addr, unix_to_age, format_gas};
use crate::components::loading::{Loading, ErrorBox};

const PER_PAGE: u64 = 25;

#[component]
pub fn BlocksPage(page: u64) -> Element {
    let mut blocks: Signal<Vec<Block>>    = use_signal(|| vec![]);
    let mut latest: Signal<u64>           = use_signal(|| 0);
    let mut loading                       = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    // Mirror the page prop into a signal so use_effect can react to it
    let mut current_page = use_signal(|| page);
    if *current_page.read() != page {
        current_page.set(page);
    }

    use_effect(move || {
        let p = *current_page.read(); // reactive dependency — re-runs when page changes
        blocks.set(vec![]);
        loading.set(true);
        error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match get_blocks_page(p, PER_PAGE).await {
                Ok((b, l)) => { blocks.set(b); latest.set(l); }
                Err(e)     => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    let total_pages = (*latest.read()).saturating_div(PER_PAGE);
    let prev_page   = page.saturating_sub(1);
    let next_page   = page + 1;

    rsx! {
        div { class: "blocks-full-wrap",
            div { class: "blocks-inner",

                div { class: "blocks-page-header",
                    div {
                        h1 { class: "page-title", style: "margin-bottom:4px;", "All Blocks" }
                        div { class: "page-subtitle",
                            "Latest: "
                            span { class: "highlight", { format!("#{}", *latest.read()) } }
                            " · Page "
                            span { class: "highlight", { format!("{}", page + 1) } }
                            " of "
                            span { class: "highlight", { format!("{}", total_pages + 1) } }
                        }
                    }
                    div { class: "blocks-page-nav",
                        if page > 0 {
                            Link { to: Route::BlocksPage { page: 0 },
                                span { class: "page-btn-link", "« Latest" }
                            }
                            Link { to: Route::BlocksPage { page: prev_page },
                                span { class: "page-btn-link", "← Newer" }
                            }
                        }
                        if page < total_pages {
                            Link { to: Route::BlocksPage { page: next_page },
                                span { class: "page-btn-link", "Older →" }
                            }
                            Link { to: Route::BlocksPage { page: total_pages },
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
                } else {
                    div { class: "blocks-table-wrap",
                        table { class: "blocks-table",
                            thead {
                                tr {
                                    th { "BLOCK" }
                                    th { "AGE" }
                                    th { "VALIDATOR" }
                                    th { "TXS" }
                                    th { "GAS USED" }
                                    th { "GAS LIMIT" }
                                    th { "UTILISATION" }
                                    th { "SIZE" }
                                }
                            }
                            tbody {
                                for block in blocks.read().iter() {
                                    tr {
                                        td {
                                            Link { to: Route::BlockPage { block_number: block.number },
                                                span { class: "hash-cell block-num",
                                                    { format!("#{}", block.number) }
                                                }
                                            }
                                        }
                                        td { class: "td-muted", "{unix_to_age(block.timestamp)}" }
                                        td {
                                            Link { to: Route::AddressPage { address: block.validator.clone() },
                                                span { class: "hash-cell addr-short",
                                                    "{shorten_addr(&block.validator)}"
                                                }
                                            }
                                        }
                                        td { class: "td-center",
                                            if block.transaction_count > 0 {
                                                span { class: "tx-count-badge", "{block.transaction_count}" }
                                            } else {
                                                span { class: "td-muted", "0" }
                                            }
                                        }
                                        td { class: "td-muted td-mono", { format_gas(block.gas_used) } }
                                        td { class: "td-faint td-mono", { format_gas(block.gas_limit) } }
                                        td {
                                            {
                                                let pct = if block.gas_limit > 0 {
                                                    block.gas_used as f64 / block.gas_limit as f64 * 100.0
                                                } else { 0.0 };
                                                // Red = busy (>=80%), amber = moderate (50-80%), green = quiet (<50%).
                                                let color = if pct >= 80.0 { "#ef4444" }
                                                    else if pct >= 50.0 { "var(--accent-yellow)" }
                                                    else { "var(--accent-green)" };
                                                rsx! {
                                                    div { class: "util-cell",
                                                        div { class: "util-bar",
                                                            div { class: "util-fill",
                                                                style: "width:{pct:.1}%; background:{color};"
                                                            }
                                                        }
                                                        span { style: "color:{color}; font-size:11px;",
                                                            { format!("{:.1}%", pct) }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        td { class: "td-faint td-mono", { format!("{}B", block.size) } }
                                    }
                                    // Mobile card (hidden on desktop via CSS)
                                    tr { class: "block-mobile-card",
                                        td { colspan: "8",
                                            div { class: "bmc-inner",
                                                div { class: "bmc-top",
                                                    Link { to: Route::BlockPage { block_number: block.number },
                                                        span { class: "hash-cell bmc-num",
                                                            { format!("#{}", block.number) }
                                                        }
                                                    }
                                                    span { class: "bmc-age", "{unix_to_age(block.timestamp)}" }
                                                    if block.transaction_count > 0 {
                                                        span { class: "tx-count-badge", "{block.transaction_count} txns" }
                                                    } else {
                                                        span { class: "bmc-zero", "0 txns" }
                                                    }
                                                }
                                                div { class: "bmc-bot",
                                                    span { class: "bmc-label", "Validator" }
                                                    Link { to: Route::AddressPage { address: block.validator.clone() },
                                                        span { class: "hash-cell bmc-addr",
                                                            "{shorten_addr(&block.validator)}"
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

                    div { class: "blocks-pagination",
                        if page > 0 {
                            Link { to: Route::BlocksPage { page: 0 },
                                span { class: "page-btn-link", "Latest" }
                            }
                            Link { to: Route::BlocksPage { page: prev_page },
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
                            Link { to: Route::BlocksPage { page: next_page },
                                span { class: "page-btn-link", "Older" }
                            }
                            Link { to: Route::BlocksPage { page: total_pages },
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
