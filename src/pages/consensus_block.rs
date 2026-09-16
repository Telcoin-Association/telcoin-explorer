// src/pages/consensus_block.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_consensus_block, get_consensus_block_batches,
    ApiConsensusBlock, ApiConsensusBatch, shorten_addr, shorten_hash, unix_to_age, unix_to_datetime,
};
use crate::components::loading::{Loading, ErrorBox, CopyButton};

#[component]
pub fn ConsensusBlockPage(number: u64) -> Element {
    let mut block: Signal<Option<ApiConsensusBlock>> = use_signal(|| None);
    let mut batches_full: Signal<Vec<ApiConsensusBatch>> = use_signal(|| vec![]);
    let mut expanded_batch: Signal<Option<u64>> = use_signal(|| None);
    let mut loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    use_effect(use_reactive(&number, move |number| {
        block.set(None);
        batches_full.set(vec![]);
        expanded_batch.set(None);
        error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            let (block_res, batches_res) = futures::join!(
                get_consensus_block(number),
                get_consensus_block_batches(number),
            );
            match block_res {
                Ok(b) => block.set(Some(b)),
                Err(e) => error.set(Some(e)),
            }
            // Best-effort: the plain detail fetch already has batch summaries,
            // so a failure here just means "no tx hashes to expand" rather
            // than blocking the page.
            if let Ok(full) = batches_res {
                batches_full.set(full.batches);
            }
            loading.set(false);
        });
    }));

    rsx! {
        div { class: "page",
            if *loading.read() {
                Loading { msg: Some(format!("Fetching consensus round #{number}...")) }
            } else if let Some(err) = error.read().as_ref() {
                ErrorBox { msg: err.clone() }
            } else if let Some(b) = block.read().as_ref() {
                div { class: "page-title-row",
                    h1 { class: "page-title", "Consensus Round #{b.header.round}" }
                }

                // Overview panel
                div { class: "detail-panel", style: "margin-bottom:20px;",
                    div { class: "detail-panel-title", "Overview" }
                    div { class: "detail-table",
                        div { class: "detail-row",
                            div { class: "detail-key", "Consensus Number" }
                            div { class: "detail-val", "#{b.header.number}" }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Epoch" }
                            div { class: "detail-val",
                                Link { to: Route::EpochDetailPage { epoch_number: b.header.epoch as u64 },
                                    span { class: "hash-cell", "#{b.header.epoch}" }
                                }
                                if b.closes_epoch == Some(true) {
                                    span { class: "chip success", style: "margin-left:8px; font-size:10px;", "Closes Epoch" }
                                }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Committed At" }
                            div { class: "detail-val",
                                { format!("{} ({})", unix_to_datetime(b.header.committed_at), unix_to_age(b.header.committed_at)) }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Leader" }
                            div { class: "detail-val mono-wrap",
                                span { class: "hash-cell", "{b.header.leader}" }
                                CopyButton { text: b.header.leader.clone() }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Sub-Dag" }
                            div { class: "detail-val", "{b.header.sub_dag_header_count} headers" }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Batches" }
                            div { class: "detail-val", "{b.header.batch_count}" }
                        }
                        if let Some(exec) = &b.header.exec_blocks {
                            div { class: "detail-row",
                                div { class: "detail-key", "Execution Blocks" }
                                div { class: "detail-val",
                                    Link { to: Route::BlockPage { block_number: exec.first },
                                        span { class: "hash-cell", "#{exec.first}" }
                                    }
                                    if exec.last != exec.first {
                                        span { style: "margin:0 6px; color:var(--text-muted);", "to" }
                                        Link { to: Route::BlockPage { block_number: exec.last },
                                            span { class: "hash-cell", "#{exec.last}" }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Digest" }
                            div { class: "detail-val mono-wrap",
                                span { class: "hash-cell", "{b.header.digest}" }
                                CopyButton { text: b.header.digest.clone() }
                            }
                        }
                    }
                }

                // Sub-DAG headers panel
                div { class: "detail-panel", style: "margin-bottom:20px;",
                    div { class: "detail-panel-title", { format!("Sub-DAG ({} headers)", b.sub_dag.len()) } }
                    div { class: "table-wrapper",
                        table { class: "tx-table",
                            thead {
                                tr {
                                    th { "AUTHOR" }
                                    th { "ROUND" }
                                    th { "CREATED" }
                                    th { "EXEC TIP" }
                                    th { "" }
                                }
                            }
                            tbody {
                                for h in b.sub_dag.iter() {
                                    tr {
                                        td { style: "font-family:var(--font-mono); font-size:12px;", "{shorten_addr(&h.author)}" }
                                        td { "{h.round}" }
                                        td { class: "td-faint", "{unix_to_age(h.created_at)}" }
                                        td {
                                            Link { to: Route::BlockPage { block_number: h.latest_execution_block.number },
                                                span { class: "hash-cell", "#{h.latest_execution_block.number}" }
                                            }
                                        }
                                        td {
                                            if h.is_leader {
                                                span { class: "chip success", style: "font-size:10px;", "Leader" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Batches panel
                if !batches_full.read().is_empty() {
                    div { class: "detail-panel", style: "margin-bottom:20px;",
                        div { class: "detail-panel-title", { format!("Batches ({})", batches_full.read().len()) } }
                        div { class: "table-wrapper",
                            table { class: "tx-table",
                                thead {
                                    tr {
                                        th { "#" }
                                        th { "AUTHORITY" }
                                        th { "TXNS" }
                                        th { "SIZE" }
                                        th { "EXEC BLOCK" }
                                        th { "" }
                                    }
                                }
                                tbody {
                                    for batch in batches_full.read().iter() {
                                        {
                                            let idx = batch.index;
                                            let is_open = *expanded_batch.read() == Some(idx);
                                            let hashes = batch.tx_hashes.clone().unwrap_or_default();
                                            rsx! {
                                                tr {
                                                    td { class: "td-faint", "{batch.index}" }
                                                    td {
                                                        Link { to: Route::AddressPage { address: batch.authority_address.clone() },
                                                            span { class: "hash-cell addr-short", "{shorten_addr(&batch.authority_address)}" }
                                                        }
                                                    }
                                                    td { class: "td-mono", "{batch.tx_count}" }
                                                    td { class: "td-mono td-faint", "{batch.size_bytes} bytes" }
                                                    td {
                                                        if let Some(n) = batch.exec_block_number {
                                                            Link { to: Route::BlockPage { block_number: n },
                                                                span { class: "hash-cell", "#{n}" }
                                                            }
                                                        } else {
                                                            span { class: "td-faint", "pending" }
                                                        }
                                                    }
                                                    td {
                                                        if batch.tx_count > 0 {
                                                            button {
                                                                class: "action-link",
                                                                style: "background:none; border:none; cursor:pointer; padding:0; font-size:12px;",
                                                                onclick: move |_| {
                                                                    expanded_batch.set(if is_open { None } else { Some(idx) });
                                                                },
                                                                if is_open { "Hide" } else { "Txns" }
                                                            }
                                                        }
                                                    }
                                                }
                                                if is_open {
                                                    tr {
                                                        td { colspan: "6", style: "background:var(--bg-base);",
                                                            div { style: "display:flex; flex-direction:column; gap:4px; padding:8px 4px;",
                                                                for hash in hashes.iter() {
                                                                    Link { to: Route::TransactionPage { hash: hash.clone() },
                                                                        span { class: "hash-cell", style: "font-size:12px;", "{shorten_hash(hash)}" }
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
                    }
                }

                // Reputation scores panel
                if !b.reputation_scores.scores.is_empty() {
                    div { class: "detail-panel",
                        div { class: "detail-panel-title",
                            { format!("Reputation Scores{}", if b.reputation_scores.final_of_schedule { " (final of schedule)" } else { "" }) }
                        }
                        div { class: "table-wrapper",
                            table { class: "tx-table",
                                thead { tr { th { "AUTHORITY" } th { "SCORE" } } }
                                tbody {
                                    for s in b.reputation_scores.scores.iter() {
                                        tr {
                                            td { style: "font-family:var(--font-mono); font-size:12px;", "{shorten_addr(&s.authority)}" }
                                            td { class: "td-mono", "{s.score}" }
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
