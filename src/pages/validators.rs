// src/pages/validators.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_validators, get_current_epoch_data, ValidatorInfo,
    shorten_addr, CONSENSUS_REGISTRY, VALIDATOR_STAKE_REQUIRED,
};
use crate::components::loading::{Loading, ErrorBox};

#[component]
pub fn ValidatorsPage() -> Element {
    let validators: Signal<Vec<ValidatorInfo>> = use_signal(|| vec![]);
    let epoch_num:  Signal<Option<u64>>         = use_signal(|| None);
    // Real on-chain epoch duration (hours), not a hardcoded constant --
    // epochs.rs already fetches this live; this page previously showed a
    // static "6h" that happened to currently be correct but would silently
    // go stale if the real value ever changed.
    let epoch_duration_h: Signal<u64>           = use_signal(|| 6);
    let loading     = use_signal(|| true);
    let error: Signal<Option<String>>           = use_signal(|| None);

    use_effect(move || {
        let mut validators = validators.clone();
        let mut epoch_num  = epoch_num.clone();
        let mut epoch_duration_h = epoch_duration_h.clone();
        let mut loading    = loading.clone();
        let mut error      = error.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            if let Ok(d) = get_current_epoch_data().await {
                epoch_num.set(Some(d.epoch));
                epoch_duration_h.set(d.epoch_duration / 3600);
            }
            match get_validators().await {
                Ok(v)  => validators.set(v),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    let epoch_sub = match *epoch_num.read() {
        Some(e) => format!("Current epoch: {}", e),
        None    => "Rotating validator committees".to_string(),
    };

    rsx! {
        div { class: "page",

            div { class: "page-title-row",
                h1 { class: "page-title", "Validators" }
                a {
                    href: "https://tnips.telcoin.network/tnips/tnips/tnip_2.html",
                    target: "_blank",
                    class: "docs-link",
                    "TNIP-2 Spec ↗"
                }
            }

            // ── Network Architecture explanation ─────────────────────
            div { class: "consensus-explainer",
                div { class: "ce-header",
                    svg { width:"20", height:"20", view_box:"0 0 24 24", fill:"none",
                        stroke:"var(--tel-blue)", stroke_width:"1.5",
                        stroke_linecap:"round", stroke_linejoin:"round",
                        path { d:"M12 2L2 7l10 5 10-5-10-5z" }
                        path { d:"M2 17l10 5 10-5" }
                        path { d:"M2 12l10 5 10-5" }
                    }
                    span { class: "ce-title", "DAG-BFT Consensus" }
                    span { class: "chip info", style: "margin-left: auto;", "Narwhal + Bullshark" }
                }
                div { class: "ce-body",
                    p {
                        "Telcoin Network uses a "
                        strong { "Directed Acyclic Graph (DAG)" }
                        "-based Byzantine Fault Tolerant consensus. Unlike traditional Proof-of-Stake chains with a single block proposer, all validators simultaneously collect transactions into "
                        em { "batches" }
                        " via "
                        strong { "Narwhal" }
                        " (the DAG/mempool layer), while "
                        strong { "Bullshark" }
                        " causally orders them for the EVM to execute."
                    }
                    div { class: "ce-steps",
                        div { class: "ce-step",
                            div { class: "ce-step-num", "1" }
                            div { class: "ce-step-text",
                                strong { "Workers" }
                                " collect transactions into batches and request attestation from 2f+1 validators"
                            }
                        }
                        div { class: "ce-step",
                            div { class: "ce-step-num", "2" }
                            div { class: "ce-step-text",
                                strong { "Primaries" }
                                " propose headers referencing batch digests; peers vote to form certificates (2f+1 stake required)"
                            }
                        }
                        div { class: "ce-step",
                            div { class: "ce-step-num", "3" }
                            div { class: "ce-step-text",
                                strong { "Bullshark" }
                                " commits a leader's subdag when 2f+1 stake in round R+2 references it — providing instant finality"
                            }
                        }
                        div { class: "ce-step",
                            div { class: "ce-step-num", "4" }
                            div { class: "ce-step-text",
                                strong { "EVM execution" }
                                " via Reth processes the ordered transactions — fully Ethereum-compatible"
                            }
                        }
                    }
                }
            }

            // ── Info cards ───────────────────────────────────────────
            div { class: "validator-info-grid",
                div { class: "info-card",
                    div { class: "info-card-icon", "⬡" }
                    div {
                        div { class: "info-card-label", "Consensus" }
                        div { class: "info-card-value", "Narwhal / Bullshark" }
                        div { class: "info-card-sub", "DAG-BFT · Instant finality" }
                    }
                }
                div { class: "info-card",
                    div { class: "info-card-icon", "⏱" }
                    div {
                        div { class: "info-card-label", "Epoch Duration" }
                        div { class: "info-card-value", "{epoch_duration_h}h" }
                        div { class: "info-card-sub", "{epoch_sub}" }
                    }
                }
                div { class: "info-card",
                    div { class: "info-card-icon", "⅔" }
                    div {
                        div { class: "info-card-label", "BFT Quorum" }
                        div { class: "info-card-value", "2f + 1" }
                        div { class: "info-card-sub", "Tolerates f Byzantine validators" }
                    }
                }
                div { class: "info-card",
                    div { class: "info-card-icon", "📋" }
                    div {
                        div { class: "info-card-label", "Registry Contract" }
                        div { class: "info-card-value mono-sm",
                            Link { to: Route::AddressPage { address: CONSENSUS_REGISTRY.to_string() },
                                span { class: "hash-cell", "{shorten_addr(CONSENSUS_REGISTRY)}" }
                            }
                        }
                        div { class: "info-card-sub", "ConsensusRegistry on-chain" }
                    }
                }
            }

            // ── Validator types ──────────────────────────────────────
            div { class: "panel", style: "margin-bottom: 20px;",
                div { class: "panel-header",
                    span { class: "panel-title", "Validator Roles" }
                }
                div { class: "validator-types-grid",
                    div { class: "vtype-card",
                        div { class: "vtype-badge cvv", "CVV" }
                        div { class: "vtype-name", "Committee Voting Validators" }
                        div { class: "vtype-desc",
                            "Actively participate in DAG consensus each round. Propose headers, collect votes, form certificates, and cast votes every round. Earn block rewards proportional to leader selections."
                        }
                    }
                    div { class: "vtype-card",
                        div { class: "vtype-badge nvv", "NVV" }
                        div { class: "vtype-name", "Non-Voting Validators" }
                        div { class: "vtype-desc",
                            "Track and execute consensus output but do not vote in DAG rounds. Participate in epoch transition votes only. Maintain full execution state."
                        }
                    }
                    div { class: "vtype-card",
                        div { class: "vtype-badge ov", "OV" }
                        div { class: "vtype-name", "Observing Validators" }
                        div { class: "vtype-desc",
                            "Follow consensus output and execute blocks without participating in voting. Used as independent verification clients and light followers."
                        }
                    }
                }
            }

            // ── Active validators list ───────────────────────────────
            div { class: "panel",
                div { class: "panel-header",
                    span { class: "panel-title",
                        "Active Validators"
                    }
                    span { class: "panel-count",
                        "({validators.read().len()} in current committee)"
                    }
                }

                if *loading.read() {
                    Loading { msg: Some("Fetching validators from ConsensusRegistry…".to_string()) }
                } else if let Some(err) = error.read().as_ref() {
                    ErrorBox { msg: err.clone() }
                } else if validators.read().is_empty() {
                    div { class: "empty-state",
                        "No validators found in the current committee."
                    }
                } else {
                    div { class: "table-wrapper",
                        table { class: "tx-table",
                            thead {
                                tr {
                                    th { "#" }
                                    th { "VALIDATOR ADDRESS" }
                                    th { "ROLE" }
                                    th { "STATUS" }
                                    th { "ACTIONS" }
                                }
                            }
                            tbody {
                                for (i, v) in validators.read().iter().enumerate() {
                                    tr {
                                        td { style: "color: var(--text-muted);", "{i + 1}" }
                                        td {
                                            Link { to: Route::AddressPage { address: v.address.clone() },
                                                span { class: "hash-cell", "{v.address}" }
                                            }
                                        }
                                        td { span { class: "vtype-badge cvv", "CVV" } }
                                        td {
                                            if v.is_retired {
                                                span { class: "chip failed", "Retired" }
                                            } else {
                                                span { class: "chip success", "Active" }
                                            }
                                        }
                                        td {
                                            Link { to: Route::AddressPage { address: v.address.clone() },
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

            // ── Epoch info note ──────────────────────────────────────
            div { class: "info-note",
                span { class: "info-note-icon", "ℹ" }
                div {
                    strong { "Epoch Transitions: " }
                    "Every "
                    strong { "{epoch_duration_h} hours" }
                    ", the ConsensusRegistry runs "
                    code { "concludeEpoch()" }
                    " to finalise validator rewards based on leader selection counts, then shuffles the committee for the next epoch using Fisher-Yates seeded by the epoch's aggregate BLS12-381 signature. "
                    "Validators must stake "
                    strong { "{VALIDATOR_STAKE_REQUIRED} TEL" }
                    " and hold a GSMA ConsensusNFT. "
                    a { href: "https://tnips.telcoin.network/tnips/tnips/tnip_2.html", target: "_blank", "Read TNIP-2 ↗" }
                }
            }
        }
    }
}
