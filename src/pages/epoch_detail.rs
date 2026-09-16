// src/pages/epoch_detail.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_epoch_by_number, get_consensus_epoch_extra,
    ApiEpoch, ApiConsensusEpochExtra, unix_to_datetime, unix_to_age,
};
use crate::components::loading::{Loading, ErrorBox, CopyButton};

#[component]
pub fn EpochDetailPage(epoch_number: u64) -> Element {
    let mut epoch: Signal<Option<ApiEpoch>> = use_signal(|| None);
    let mut extra: Signal<Option<ApiConsensusEpochExtra>> = use_signal(|| None);
    let mut loading = use_signal(|| true);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    use_effect(use_reactive(&epoch_number, move |epoch_number| {
        epoch.set(None);
        extra.set(None);
        error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            let (epoch_res, extra_res) = futures::join!(
                get_epoch_by_number(epoch_number),
                get_consensus_epoch_extra(epoch_number),
            );
            match epoch_res {
                Ok(e) => epoch.set(Some(e)),
                Err(e) => error.set(Some(e)),
            }
            // Best-effort supplementary data -- pack completeness / last
            // committed rounds / final reputation scores are extras the
            // page can render without; a failure here never blocks the
            // core epoch record/certificate the page already shows.
            if let Ok(x) = extra_res { extra.set(Some(x)); }
            loading.set(false);
        });
    }));

    rsx! {
        div { class: "page",
            if *loading.read() {
                Loading { msg: Some(format!("Fetching epoch #{epoch_number}...")) }
            } else if let Some(err) = error.read().as_ref() {
                ErrorBox { msg: err.clone() }
            } else if let Some(e) = epoch.read().as_ref() {
                div { class: "page-title-row",
                    h1 { class: "page-title", "Epoch #{e.epoch}" }
                }

                div { class: "detail-panel", style: "margin-bottom:20px;",
                    div { class: "detail-panel-title", "Overview" }
                    div { class: "detail-table",
                        div { class: "detail-row",
                            div { class: "detail-key", "Status" }
                            div { class: "detail-val",
                                if e.is_current {
                                    span { class: "chip pending", "In Progress" }
                                } else if e.certified {
                                    span { class: "chip success", "Certified" }
                                } else {
                                    span { class: "chip failed", "Uncertified" }
                                }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Block Range" }
                            div { class: "detail-val",
                                Link { to: Route::BlockPage { block_number: e.start_block },
                                    span { class: "hash-cell", "#{e.start_block}" }
                                }
                                if let Some(end) = e.end_block {
                                    span { style: "margin:0 6px; color:var(--text-muted);", "to" }
                                    Link { to: Route::BlockPage { block_number: end },
                                        span { class: "hash-cell", "#{end}" }
                                    }
                                } else {
                                    span { style: "margin-left:8px; color:var(--text-muted);", "(in progress)" }
                                }
                            }
                        }
                        if let Some(end_time) = e.end_time {
                            div { class: "detail-row",
                                div { class: "detail-key", "Ended" }
                                div { class: "detail-val", { format!("{} ({})", unix_to_datetime(end_time), unix_to_age(end_time)) } }
                            }
                        }
                        div { class: "detail-row",
                            div { class: "detail-key", "Committee Size" }
                            div { class: "detail-val", "{e.committee_size} validators" }
                        }
                    }
                }

                if let Some(record) = &e.record {
                    div { class: "detail-panel", style: "margin-bottom:20px;",
                        div { class: "detail-panel-title", "Epoch Record" }
                        div { class: "detail-table",
                            div { class: "detail-row",
                                div { class: "detail-key", "Record Digest" }
                                div { class: "detail-val mono-wrap",
                                    span { class: "hash-cell", "{record.digest}" }
                                    CopyButton { text: record.digest.clone() }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Parent Record" }
                                div { class: "detail-val mono-wrap",
                                    if e.epoch > 0 {
                                        Link { to: Route::EpochDetailPage { epoch_number: e.epoch - 1 },
                                            span { class: "hash-cell", "{record.parent_hash}" }
                                        }
                                    } else {
                                        span { class: "hash-cell", "{record.parent_hash}" }
                                    }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Final Execution Block" }
                                div { class: "detail-val",
                                    Link { to: Route::BlockPage { block_number: record.final_state.number },
                                        span { class: "hash-cell", "#{record.final_state.number}" }
                                    }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Final Consensus Block" }
                                div { class: "detail-val", "#{record.final_consensus.number}" }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "BFT Quorum" }
                                div { class: "detail-val", "{record.super_quorum} / {record.committee.len()}" }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Next Committee Size" }
                                div { class: "detail-val", "{record.next_committee.len()} validators" }
                            }
                        }
                    }
                }

                if let Some(cert) = &e.certificate {
                    div { class: "detail-panel", style: "margin-bottom:20px;",
                        div { class: "detail-panel-title", "Epoch Certificate" }
                        div { class: "detail-table",
                            div { class: "detail-row",
                                div { class: "detail-key", "Verification" }
                                div { class: "detail-val",
                                    if cert.verified == Some(true) {
                                        span { class: "chip success", "Verified" }
                                    } else if cert.verified == Some(false) {
                                        span { class: "chip failed", "Verification Failed" }
                                    } else {
                                        span { class: "chip pending", "Not Verified" }
                                    }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Signers" }
                                div { class: "detail-val",
                                    { format!("{} signed (quorum requires {})", cert.signer_count, cert.super_quorum) }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Aggregate Signature" }
                                div { class: "detail-val mono-wrap",
                                    span { class: "hash-cell", "{cert.signature}" }
                                    CopyButton { text: cert.signature.clone() }
                                }
                            }
                        }
                    }
                }

                if let Some(committee_addresses) = &e.committee_addresses {
                    div { class: "detail-panel",
                        div { class: "detail-panel-title", { format!("Committee ({} validators)", committee_addresses.len()) } }
                        div { class: "table-wrapper",
                            table { class: "tx-table",
                                thead { tr { th { "#" } th { "VALIDATOR ADDRESS" } } }
                                tbody {
                                    for (i, addr) in committee_addresses.iter().enumerate() {
                                        tr {
                                            td { style: "color:var(--text-muted);", "{i + 1}" }
                                            td {
                                                Link { to: Route::AddressPage { address: addr.clone() },
                                                    span { class: "hash-cell", "{addr}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(extra) = extra.read().as_ref() {
                    if let Some(complete) = extra.pack_complete {
                        div { class: "detail-panel", style: "margin-top:20px;",
                            div { class: "detail-panel-title", "Consensus Pack Status" }
                            div { class: "detail-table",
                                div { class: "detail-row",
                                    div { class: "detail-key", "Pack Complete" }
                                    div { class: "detail-val",
                                        if complete {
                                            span { class: "chip success", "Complete" }
                                        } else {
                                            span { class: "chip pending", "Incomplete / Not Held Locally" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(rounds) = &extra.last_committed_rounds {
                        if !rounds.is_empty() {
                            div { class: "detail-panel", style: "margin-top:20px;",
                                div { class: "detail-panel-title", { format!("Last Committed Rounds ({})", rounds.len()) } }
                                div { class: "table-wrapper",
                                    table { class: "tx-table",
                                        thead { tr { th { "AUTHORITY" } th { "ROUND" } } }
                                        tbody {
                                            for r in rounds.iter() {
                                                tr {
                                                    td { style: "font-family:var(--font-mono); font-size:12px;", "{crate::services::rpc::shorten_addr(&r.authority)}" }
                                                    td { class: "td-mono", "{r.round}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(scores) = &extra.final_reputation_scores {
                        if !scores.scores.is_empty() {
                            div { class: "detail-panel", style: "margin-top:20px;",
                                div { class: "detail-panel-title", "Final Reputation Scores" }
                                div { class: "table-wrapper",
                                    table { class: "tx-table",
                                        thead { tr { th { "AUTHORITY" } th { "SCORE" } } }
                                        tbody {
                                            for s in scores.scores.iter() {
                                                tr {
                                                    td { style: "font-family:var(--font-mono); font-size:12px;", "{crate::services::rpc::shorten_addr(&s.authority)}" }
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
    }
}
