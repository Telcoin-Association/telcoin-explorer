// src/pages/consensus.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_consensus_blocks,
    ConsensusHeader, shorten_addr, unix_to_age,
};
use crate::components::loading::{Loading, ErrorBox};

const PER_PAGE: u64 = 25;

#[component]
pub fn ConsensusPage(page: u64) -> Element {
    let mut items: Signal<Vec<ConsensusHeader>> = use_signal(|| vec![]);
    let mut total: Signal<u64>                  = use_signal(|| 0);
    let mut loading                             = use_signal(|| true);
    let mut error: Signal<Option<String>>       = use_signal(|| None);

    let mut current_page = use_signal(|| page);
    if *current_page.read() != page {
        current_page.set(page);
    }

    use_effect(move || {
        let p = *current_page.read();
        items.set(vec![]);
        loading.set(true);
        error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match get_consensus_blocks(p, PER_PAGE).await {
                Ok((i, tot)) => { items.set(i); total.set(tot); }
                Err(e)       => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    let total_items  = *total.read();
    let total_pages  = total_items.saturating_div(PER_PAGE);
    let prev_page    = page.saturating_sub(1);
    let next_page    = page + 1;

    rsx! {
        div { class: "blocks-full-wrap",
            div { class: "blocks-inner",
                div { class: "blocks-page-header",
                    div {
                        h1 { class: "page-title", style: "margin-bottom:4px;", "Consensus" }
                        div { class: "page-subtitle",
                            span { class: "highlight", { format!("{}", total_items) } }
                            " consensus rounds · Page "
                            span { class: "highlight", { format!("{}", page + 1) } }
                            " of "
                            span { class: "highlight", { format!("{}", total_pages + 1) } }
                        }
                    }
                    div { class: "blocks-page-nav",
                        if page > 0 {
                            Link { to: Route::ConsensusPage { page: 0 },
                                span { class: "page-btn-link", "« Latest" }
                            }
                            Link { to: Route::ConsensusPage { page: prev_page },
                                span { class: "page-btn-link", "← Newer" }
                            }
                        }
                        if page < total_pages {
                            Link { to: Route::ConsensusPage { page: next_page },
                                span { class: "page-btn-link", "Older →" }
                            }
                            Link { to: Route::ConsensusPage { page: total_pages },
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
                } else if items.read().is_empty() {
                    div { class: "empty-state",
                        div { style: "font-size:32px; margin-bottom:12px;", "📭" }
                        "No consensus rounds found"
                    }
                } else {
                    div { class: "blocks-table-wrap",
                        table { class: "tx-table",
                            thead {
                                tr {
                                    th { "ROUND" }
                                    th { "EPOCH" }
                                    th { "AGE" }
                                    th { "LEADER" }
                                    th { "SUB-DAG" }
                                    th { "BATCHES" }
                                }
                            }
                            tbody {
                                for c in items.read().iter() {
                                    tr {
                                        td { "data-label": "Round",
                                            span { class: "hash-cell", "#{c.round}" }
                                        }
                                        td { "data-label": "Epoch",
                                            Link { to: Route::EpochDetailPage { epoch_number: c.epoch as u64 },
                                                span { class: "hash-cell", "#{c.epoch}" }
                                            }
                                        }
                                        td { "data-label": "Age", class: "td-faint",
                                            "{unix_to_age(c.committed_at)}"
                                        }
                                        td { "data-label": "Leader",
                                            span { class: "hash-cell addr-short", "{shorten_addr(&c.leader)}" }
                                        }
                                        td { "data-label": "Sub-Dag", class: "td-mono",
                                            "{c.sub_dag_header_count} headers"
                                        }
                                        td { "data-label": "Batches", class: "td-mono",
                                            if c.batch_count > 0 {
                                                span { class: "tx-badge", "{c.batch_count}" }
                                            } else {
                                                span { class: "td-faint", "0" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "blocks-pagination",
                        if page > 0 {
                            Link { to: Route::ConsensusPage { page: 0 },
                                span { class: "page-btn-link", "Latest" }
                            }
                            Link { to: Route::ConsensusPage { page: prev_page },
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
                            Link { to: Route::ConsensusPage { page: next_page },
                                span { class: "page-btn-link", "Older" }
                            }
                            Link { to: Route::ConsensusPage { page: total_pages },
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
