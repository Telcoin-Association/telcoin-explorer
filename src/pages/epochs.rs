// src/pages/epochs.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_current_epoch_data, get_validator_leader_counts,
    get_block_number, EpochData,
    shorten_addr, CONSENSUS_REGISTRY,
};
use crate::components::loading::{Loading, ErrorBox};

// Pre-computed row structs — no borrowing issues in RSX
struct LeaderRow {
    rank:    usize,
    addr:    String,
    short:   String,
    count:   u64,
    pct:     String,
    bar_pct: String,
}

struct CommitteeRow {
    rank:  usize,
    addr:  String,
    led:   u64,
}

#[component]
pub fn EpochsPage() -> Element {
    let epoch_data: Signal<Option<EpochData>>     = use_signal(|| None);
    let leader_counts: Signal<Vec<(String, u64)>> = use_signal(|| vec![]);
    let loading  = use_signal(|| true);
    let error: Signal<Option<String>>             = use_signal(|| None);

    use_effect(move || {
        let mut epoch_data    = epoch_data.clone();
        let mut leader_counts = leader_counts.clone();
        let mut loading       = loading.clone();
        let mut error         = error.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            match get_current_epoch_data().await {
                Ok(data) => epoch_data.set(Some(data)),
                Err(e)   => { error.set(Some(e)); loading.set(false); return; }
            }
            let counts = get_validator_leader_counts(200).await;
            leader_counts.set(counts);
            loading.set(false);
        });
    });

    // Pre-compute leader rows outside RSX
    let leader_rows: Vec<LeaderRow> = {
        let counts = leader_counts.read().clone();
        let total: u64 = counts.iter().map(|(_, c)| c).sum();
        let max: u64   = counts.first().map(|(_, c)| *c).unwrap_or(1);
        counts.into_iter().enumerate().map(|(i, (addr, count))| {
            let pct     = if total > 0 { count as f64 / total as f64 * 100.0 } else { 0.0 };
            let bar_pct = if max   > 0 { count as f64 / max   as f64 * 100.0 } else { 0.0 };
            LeaderRow {
                rank:    i + 1,
                short:   shorten_addr(&addr),
                addr,
                count,
                pct:     format!("{pct:.1}%"),
                bar_pct: format!("{bar_pct:.1}%"),
            }
        }).collect()
    };

    // Pre-compute committee rows outside RSX
    let committee_rows: Vec<CommitteeRow> = {
        let counts = leader_counts.read().clone();
        let validators = epoch_data.read()
            .as_ref()
            .map(|d| d.validators.clone())
            .unwrap_or_default();
        validators.into_iter().enumerate().map(|(i, addr)| {
            let led = counts.iter()
                .find(|(a, _)| a.to_lowercase() == addr.to_lowercase())
                .map(|(_, c)| *c)
                .unwrap_or(0);
            CommitteeRow { rank: i + 1, addr, led }
        }).collect()
    };

    // Pre-compute quorum string
    let quorum_str = epoch_data.read().as_ref().map(|d| {
        let f      = (d.validator_count as f64 - 1.0) / 3.0;
        let quorum = (2.0 * f).floor() as usize + 1;
        format!("{} / {}", quorum, d.validator_count)
    }).unwrap_or_default();

    let epoch_num = epoch_data.read().as_ref().map(|d| d.epoch).unwrap_or(0);
    let val_count = epoch_data.read().as_ref().map(|d| d.validator_count).unwrap_or(0);
    let epoch_dur_h = epoch_data.read().as_ref().map(|d| d.epoch_duration / 3600).unwrap_or(6);

    rsx! {
        div { class: "page",

            div { class: "page-title-row",
                div {
                    h1 { class: "page-title", "Epochs" }
                    p { class: "page-subtitle",
                        "Consensus epoch state from "
                        Link { to: Route::AddressPage { address: CONSENSUS_REGISTRY.to_string() },
                            span { class: "highlight", "ConsensusRegistry" }
                        }
                    }
                }
                a {
                    href: "https://tnips.telcoin.network/tnips/tnips/tnip_2.html",
                    target: "_blank",
                    class: "docs-link",
                    "TNIP-2 Spec ↗"
                }
            }

            if *loading.read() {
                Loading { msg: Some("Reading ConsensusRegistry…".to_string()) }
            } else if let Some(err) = error.read().as_ref() {
                ErrorBox { msg: err.clone() }
            } else {

                // ── What is an epoch ─────────────────────────────────
                div { class: "info-note", style: "margin-bottom:20px;",
                    span { class: "info-note-icon", "ℹ" }
                    div {
                        strong { "What is an Epoch? " }
                        "An epoch is a fixed "
                        strong { "{epoch_dur_h}-hour period" }
                        " during which a specific validator committee runs DAG consensus. "
                        "At the end of each epoch, "
                        code { "concludeEpoch()" }
                        " distributes rewards based on leader selection counts, then shuffles the committee using Fisher-Yates seeded by the epoch's aggregate BLS12-381 signature. "
                        "Fresh consensus components are created each epoch while the execution engine and networks persist."
                    }
                }

                // ── Stat cards ───────────────────────────────────────
                div { class: "epoch-stat-grid",
                    div { class: "epoch-stat-card accent-blue",
                        div { class: "epoch-stat-icon",
                            svg { width:"22", height:"22", view_box:"0 0 24 24", fill:"none",
                                stroke:"currentColor", stroke_width:"1.5",
                                stroke_linecap:"round", stroke_linejoin:"round",
                                path { d:"M12 2L2 7l10 5 10-5-10-5z" }
                                path { d:"M2 17l10 5 10-5" }
                                path { d:"M2 12l10 5 10-5" }
                            }
                        }
                        div { class: "epoch-stat-label", "Current Epoch" }
                        div { class: "epoch-stat-value", "#{epoch_num}" }
                        div { class: "epoch-stat-sub", "Adiri Testnet" }
                    }
                    div { class: "epoch-stat-card accent-green",
                        div { class: "epoch-stat-icon",
                            svg { width:"22", height:"22", view_box:"0 0 24 24", fill:"none",
                                stroke:"currentColor", stroke_width:"1.5",
                                stroke_linecap:"round", stroke_linejoin:"round",
                                path { d:"M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" }
                                circle { cx:"9", cy:"7", r:"4" }
                                path { d:"M23 21v-2a4 4 0 0 0-3-3.87" }
                                path { d:"M16 3.13a4 4 0 0 1 0 7.75" }
                            }
                        }
                        div { class: "epoch-stat-label", "Active Validators" }
                        div { class: "epoch-stat-value", "{val_count}" }
                        div { class: "epoch-stat-sub", "Committee Voting Validators" }
                    }
                    div { class: "epoch-stat-card accent-purple",
                        div { class: "epoch-stat-icon",
                            svg { width:"22", height:"22", view_box:"0 0 24 24", fill:"none",
                                stroke:"currentColor", stroke_width:"1.5",
                                stroke_linecap:"round", stroke_linejoin:"round",
                                circle { cx:"12", cy:"12", r:"10" }
                                path { d:"M12 6v6l4 2" }
                            }
                        }
                        div { class: "epoch-stat-label", "Epoch Duration" }
                        div { class: "epoch-stat-value", "{epoch_dur_h}h" }
                        div { class: "epoch-stat-sub", "Fixed period per committee" }
                    }
                    div { class: "epoch-stat-card accent-cyan",
                        div { class: "epoch-stat-icon",
                            svg { width:"22", height:"22", view_box:"0 0 24 24", fill:"none",
                                stroke:"currentColor", stroke_width:"1.5",
                                stroke_linecap:"round", stroke_linejoin:"round",
                                path { d:"M22 11.08V12a10 10 0 1 1-5.93-9.14" }
                                path { d:"M22 4 12 14.01l-3-3" }
                            }
                        }
                        div { class: "epoch-stat-label", "BFT Quorum" }
                        div { class: "epoch-stat-value", "{quorum_str}" }
                        div { class: "epoch-stat-sub", "2f+1 required for finality" }
                    }
                }

                // ── Epoch lifecycle ──────────────────────────────────
                div { class: "panel", style: "margin-bottom:20px;",
                    div { class: "panel-header",
                        span { class: "panel-title", "Epoch Lifecycle" }
                    }
                    div { class: "epoch-lifecycle",
                        div { class: "elc-step",
                            div { class: "elc-num", "1" }
                            div { class: "elc-content",
                                div { class: "elc-title", "Epoch Starts" }
                                div { class: "elc-desc",
                                    "Fresh PrimaryNode, WorkerNode and Bullshark instances created. Committee read from ConsensusRegistry. Workers begin collecting transactions into batches."
                                }
                            }
                        }
                        div { class: "elc-arrow", "→" }
                        div { class: "elc-step",
                            div { class: "elc-num", "2" }
                            div { class: "elc-content",
                                div { class: "elc-title", "DAG Rounds" }
                                div { class: "elc-desc",
                                    "Validators propose headers referencing batch digests. Peers vote to form Certificates (2f+1 stake). Bullshark commits a leader's subdag when supported by 2f+1 in round R+2."
                                }
                            }
                        }
                        div { class: "elc-arrow", "→" }
                        div { class: "elc-step",
                            div { class: "elc-num", "3" }
                            div { class: "elc-content",
                                div { class: "elc-title", "Epoch Boundary" }
                                div { class: "elc-desc",
                                    "EpochManager monitors leader timestamps. When boundary reached, validators collect 2f+1 EpochVote signatures to form an EpochCertificate with aggregate BLS12-381 signature."
                                }
                            }
                        }
                        div { class: "elc-arrow", "→" }
                        div { class: "elc-step",
                            div { class: "elc-num", "4" }
                            div { class: "elc-content",
                                div { class: "elc-title", "concludeEpoch()" }
                                div { class: "elc-desc",
                                    "ConsensusRegistry distributes rewards by leader count. Fisher-Yates shuffle (seeded by BLS signature) selects next committee. Consensus restarts; execution engine and networks persist."
                                }
                            }
                        }
                    }
                }

                // ── Leader distribution ──────────────────────────────
                div { class: "panel", style: "margin-bottom:20px;",
                    div { class: "panel-header",
                        span { class: "panel-title", "Leader Distribution" }
                        span { class: "panel-count", "(last 200 blocks)" }
                    }
                    if leader_rows.is_empty() {
                        div { class: "panel-empty", "No leader data available." }
                    } else {
                        div { class: "leader-table-wrap",
                            div { class: "leader-header",
                                span { }
                                span { "VALIDATOR" }
                                span { "BLOCKS LED" }
                                span { "SHARE" }
                                span { "DISTRIBUTION" }
                            }
                            for row in leader_rows.iter() {
                                div { class: "leader-row",
                                    span { class: "leader-rank", "{row.rank}" }
                                    Link { to: Route::AddressPage { address: row.addr.clone() },
                                        span { class: "hash-cell", "{row.short}" }
                                    }
                                    span { class: "leader-count", "{row.count}" }
                                    span { class: "leader-pct", "{row.pct}" }
                                    div { class: "leader-bar-wrap",
                                        div { class: "leader-bar-fill",
                                            style: "width:{row.bar_pct}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ── Current committee ────────────────────────────────
                div { class: "panel",
                    div { class: "panel-header",
                        span { class: "panel-title", "Current Committee" }
                        span { class: "panel-count",
                            "({val_count} validators · epoch #{epoch_num})"
                        }
                    }
                    if committee_rows.is_empty() {
                        div { class: "panel-empty", "No validators found." }
                    } else {
                        div { class: "table-wrapper",
                            table { class: "tx-table",
                                thead {
                                    tr {
                                        th { "#" }
                                        th { "VALIDATOR ADDRESS" }
                                        th { "ROLE" }
                                        th { "BLOCKS LED (last 200)" }
                                        th { "ACTIONS" }
                                    }
                                }
                                tbody {
                                    for row in committee_rows.iter() {
                                        tr {
                                            td { style: "color:var(--text-muted);", "{row.rank}" }
                                            td {
                                                Link { to: Route::AddressPage { address: row.addr.clone() },
                                                    span { class: "hash-cell", "{row.addr}" }
                                                }
                                            }
                                            td { span { class: "vtype-badge cvv", "CVV" } }
                                            td {
                                                if row.led > 0 {
                                                    span { class: "tx-count-badge", "{row.led} blocks" }
                                                } else {
                                                    span { style: "color:var(--text-muted);font-size:12px;", "—" }
                                                }
                                            }
                                            td {
                                                Link { to: Route::AddressPage { address: row.addr.clone() },
                                                    span { class: "action-link", "View →" }
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
