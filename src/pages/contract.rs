// src/pages/contract.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    get_contract_info, get_block_number, get_token_transfers,
    parse_transfer_logs, get_token_symbol, ContractInfo,
    TokenTransfer, shorten_hash, shorten_addr, unix_to_age,
    CONSENSUS_REGISTRY,
};
use crate::components::loading::{Loading, ErrorBox, CopyButton};

#[component]
pub fn ContractPage(address: String) -> Element {
    let info: Signal<Option<ContractInfo>>        = use_signal(|| None);
    let transfers: Signal<Vec<TokenTransfer>>     = use_signal(|| vec![]);
    let loading  = use_signal(|| true);
    let error: Signal<Option<String>>             = use_signal(|| None);
    let active_tab = use_signal(|| "overview");
    let addr_clone = address.clone();

    use_effect(move || {
        let address       = addr_clone.clone();
        let mut info      = info.clone();
        let mut transfers = transfers.clone();
        let mut loading   = loading.clone();
        let mut error     = error.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            match get_contract_info(&address).await {
                Ok(i)  => info.set(Some(i)),
                Err(e) => { error.set(Some(e)); loading.set(false); return; }
            }
            // Fetch recent token transfers involving this contract
            if let Ok(latest) = get_block_number().await {
                let from = latest.saturating_sub(5000);
                if let Ok(logs) = get_token_transfers(&address, from, latest).await {
                    let mut parsed = parse_transfer_logs(logs);
                    let mut seen: std::collections::HashMap<String, String> =
                        std::collections::HashMap::new();
                    for t in parsed.iter_mut() {
                        let sym = if let Some(s) = seen.get(&t.token_address) {
                            s.clone()
                        } else {
                            let s = get_token_symbol(&t.token_address).await;
                            seen.insert(t.token_address.clone(), s.clone());
                            s
                        };
                        t.token_symbol = sym;
                    }
                    transfers.set(parsed);
                }
            }
            loading.set(false);
        });
    });

    let is_registry = address.to_lowercase() == CONSENSUS_REGISTRY.to_lowercase();
    let avatar_char = address.chars().nth(2).unwrap_or('C').to_uppercase().next().unwrap_or('C');

    rsx! {
        div { class: "page",

            if *loading.read() {
                Loading { msg: Some("Reading contract…".to_string()) }
            } else if let Some(err) = error.read().as_ref() {
                div {
                    ErrorBox { msg: err.clone() }
                    div { style: "margin-top:16px;",
                        Link { to: Route::AddressPage { address: address.clone() },
                            span { class: "action-link", "← View as Address" }
                        }
                    }
                }
            } else if let Some(contract) = info.read().as_ref() {

                // ── Page header ──────────────────────────────────────
                div { class: "contract-header",
                    div { class: "contract-avatar",
                        if contract.is_erc20 || contract.is_erc721 {
                            svg { width:"24", height:"24", view_box:"0 0 24 24", fill:"none",
                                stroke:"currentColor", stroke_width:"1.5",
                                stroke_linecap:"round", stroke_linejoin:"round",
                                circle { cx:"12", cy:"12", r:"10" }
                                path { d:"M12 6v6l4 2" }
                            }
                        } else {
                            span { "{avatar_char}" }
                        }
                    }
                    div { class: "contract-header-info",
                        div { class: "contract-title-row",
                            h1 { class: "page-title", style: "margin-bottom:0;",
                                if !contract.token_name.is_empty() {
                                    "{contract.token_name}"
                                } else if is_registry {
                                    "ConsensusRegistry"
                                } else {
                                    "Smart Contract"
                                }
                            }
                            // Type badges
                            div { class: "contract-badges",
                                span { class: "chip pending", "Contract" }
                                if contract.is_erc20 {
                                    span { class: "chip success", "ERC-20" }
                                }
                                if contract.is_erc721 {
                                    span { class: "chip info", "ERC-721" }
                                }
                                if is_registry {
                                    span { class: "chip info", "ConsensusRegistry" }
                                }
                                if contract.has_owner {
                                    span { class: "chip pending", "Ownable" }
                                }
                                if contract.has_pause {
                                    span { class: "chip pending", "Pausable" }
                                }
                                if contract.has_mint {
                                    span { class: "chip pending", "Mintable" }
                                }
                            }
                        }
                        // Address + copy
                        div { class: "contract-addr-row",
                            span { class: "hash-cell", style: "font-size:13px;",
                                "{contract.address}"
                            }
                            CopyButton { text: contract.address.clone() }
                        }
                    }
                }

                // ── Tabs ─────────────────────────────────────────────
                div { class: "tabs-row",
                    button {
                        class: if *active_tab.read() == "overview" { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| active_tab.clone().set("overview"),
                        "Overview"
                    }
                    button {
                        class: if *active_tab.read() == "transfers" { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| active_tab.clone().set("transfers"),
                        "Transfers"
                        span { class: "tab-count", "({transfers.read().len()})" }
                    }
                    button {
                        class: if *active_tab.read() == "bytecode" { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| active_tab.clone().set("bytecode"),
                        "Bytecode"
                    }
                    if is_registry {
                        Link { to: Route::ValidatorsPage {},
                            button { class: "tab-btn",
                                "Validators →"
                            }
                        }
                        Link { to: Route::EpochsPage {},
                            button { class: "tab-btn",
                                "Epochs →"
                            }
                        }
                    }
                }

                // ── Overview tab ─────────────────────────────────────
                if *active_tab.read() == "overview" {
                    div { class: "detail-panel",
                        div { class: "detail-panel-title", "Contract Information" }
                        div { class: "detail-table",

                            div { class: "detail-row",
                                div { class: "detail-key", "Address" }
                                div { class: "detail-val",
                                    span { "{contract.address}" }
                                    CopyButton { text: contract.address.clone() }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "TEL Balance" }
                                div { class: "detail-val",
                                    span { style: "font-weight:600;",
                                        { format!("{:.6} TEL", contract.balance) }
                                    }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Transactions" }
                                div { class: "detail-val", "{contract.tx_count}" }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Bytecode Size" }
                                div { class: "detail-val",
                                    { format!("{} bytes", contract.bytecode_size) }
                                    span { style: "margin-left:8px; color:var(--text-muted); font-size:11px;",
                                        { format!("({:.1} KB)", contract.bytecode_size as f64 / 1024.0) }
                                    }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Interfaces" }
                                div { class: "detail-val", style: "gap:6px; flex-wrap:wrap;",
                                    if contract.is_erc20  { span { class: "chip success", "ERC-20 Token" } }
                                    if contract.is_erc721 { span { class: "chip info",    "ERC-721 NFT" } }
                                    if contract.has_owner { span { class: "chip pending", "Ownable" } }
                                    if contract.has_pause { span { class: "chip pending", "Pausable" } }
                                    if contract.has_mint  { span { class: "chip pending", "Mintable" } }
                                    if !contract.is_erc20 && !contract.is_erc721 &&
                                       !contract.has_owner && !contract.has_pause && !contract.has_mint {
                                        span { style: "color:var(--text-muted); font-size:12px;",
                                            "No standard interfaces detected"
                                        }
                                    }
                                }
                            }

                            // Token-specific rows
                            if contract.is_erc20 {
                                div { class: "detail-row",
                                    div { class: "detail-key", "Token Name" }
                                    div { class: "detail-val", "{contract.token_name}" }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key", "Symbol" }
                                    div { class: "detail-val",
                                        span { class: "token-symbol-badge", "{contract.token_symbol}" }
                                    }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key", "Decimals" }
                                    div { class: "detail-val", "{contract.token_decimals}" }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key", "Total Supply" }
                                    div { class: "detail-val",
                                        span { style: "font-weight:600;",
                                            "{contract.token_supply} {contract.token_symbol}"
                                        }
                                    }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key", "Token Page" }
                                    div { class: "detail-val",
                                        Link { to: Route::TokenPage { address: contract.address.clone() },
                                            span { class: "action-link", "View Token Page →" }
                                        }
                                    }
                                }
                            }

                            // ConsensusRegistry special rows
                            if is_registry {
                                div { class: "detail-row",
                                    div { class: "detail-key", "Purpose" }
                                    div { class: "detail-val",
                                        "Manages validator registration, staking, committee selection and epoch rewards for Telcoin Network consensus."
                                    }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key", "Consensus" }
                                    div { class: "detail-val",
                                        span { class: "chip info", "Narwhal / Bullshark DAG-BFT" }
                                    }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key", "Explorer" }
                                    div { class: "detail-val", style: "gap:8px;",
                                        Link { to: Route::ValidatorsPage {},
                                            span { class: "action-link", "Validators →" }
                                        }
                                        Link { to: Route::EpochsPage {},
                                            span { class: "action-link", style: "margin-left:12px;", "Epochs →" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ── Transfers tab ─────────────────────────────────────
                if *active_tab.read() == "transfers" {
                    div { class: "panel",
                        div { class: "panel-header",
                            span { class: "panel-title", "Token Transfers" }
                            span { style: "color:var(--text-muted); font-size:11px;",
                                "Last 5,000 blocks"
                            }
                        }
                        if transfers.read().is_empty() {
                            div { class: "panel-empty",
                                "No token transfers found in the last 5,000 blocks."
                            }
                        } else {
                            div { class: "table-wrapper",
                                table { class: "tx-table",
                                    thead {
                                        tr {
                                            th { "TX HASH" }
                                            th { "BLOCK" }
                                            th { "AGE" }
                                            th { "FROM" }
                                            th { "" }
                                            th { "TO" }
                                            th { "AMOUNT" }
                                        }
                                    }
                                    tbody {
                                        for t in transfers.read().iter() {
                                            tr {
                                                td {
                                                    Link { to: Route::TransactionPage { hash: t.tx_hash.clone() },
                                                        span { class: "hash-cell", "{shorten_hash(&t.tx_hash)}" }
                                                    }
                                                }
                                                td {
                                                    Link { to: Route::BlockPage { block_number: t.block_number },
                                                        span { class: "hash-cell", "#{t.block_number}" }
                                                    }
                                                }
                                                td { style: "color:var(--text-muted);",
                                                    "{unix_to_age(t.timestamp)}"
                                                }
                                                td {
                                                    Link { to: Route::AddressPage { address: t.from.clone() },
                                                        span { class: "hash-cell addr-short",
                                                            "{shorten_addr(&t.from)}"
                                                        }
                                                    }
                                                }
                                                td { span { class: "transfer-arrow", "→" } }
                                                td {
                                                    Link { to: Route::AddressPage { address: t.to.clone() },
                                                        span { class: "hash-cell addr-short",
                                                            "{shorten_addr(&t.to)}"
                                                        }
                                                    }
                                                }
                                                td { style: "color:var(--accent-green); font-weight:600;",
                                                    { format!("{:.4} {}", t.amount, t.token_symbol) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ── Bytecode tab ──────────────────────────────────────
                if *active_tab.read() == "bytecode" {
                    div { class: "detail-panel",
                        div { class: "detail-panel-title",
                            { format!("Deployed Bytecode ({} bytes)", contract.bytecode_size) }
                        }
                        div { class: "bytecode-notice",
                            svg { width:"14", height:"14", view_box:"0 0 24 24", fill:"none",
                                stroke:"currentColor", stroke_width:"2",
                                circle { cx:"12", cy:"12", r:"10" }
                                path { d:"M12 8v4m0 4h.01" }
                            }
                            span {
                                "Source code verification is not yet supported. "
                                "Showing bytecode fingerprint only."
                            }
                        }
                        div { class: "detail-table",
                            div { class: "detail-row",
                                div { class: "detail-key", "Size" }
                                div { class: "detail-val",
                                    { format!("{} bytes ({:.2} KB)",
                                        contract.bytecode_size,
                                        contract.bytecode_size as f64 / 1024.0) }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Detected Selectors" }
                                div { class: "detail-val", style: "flex-wrap:wrap; gap:5px;",
                                    if contract.is_erc20 {
                                        span { class: "code-inline", "transfer(address,uint256)" }
                                        span { class: "code-inline", "balanceOf(address)" }
                                        span { class: "code-inline", "totalSupply()" }
                                        span { class: "code-inline", "approve(address,uint256)" }
                                    }
                                    if contract.is_erc721 {
                                        span { class: "code-inline", "ownerOf(uint256)" }
                                        span { class: "code-inline", "tokenURI(uint256)" }
                                    }
                                    if contract.has_owner {
                                        span { class: "code-inline", "owner()" }
                                    }
                                    if contract.has_pause {
                                        span { class: "code-inline", "pause()" }
                                        span { class: "code-inline", "paused()" }
                                    }
                                    if contract.has_mint {
                                        span { class: "code-inline", "mint(address,uint256)" }
                                    }
                                    if !contract.is_erc20 && !contract.is_erc721 &&
                                       !contract.has_owner && !contract.has_pause && !contract.has_mint {
                                        span { style: "color:var(--text-muted); font-size:12px;",
                                            "No common selectors detected"
                                        }
                                    }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Bytecode" }
                                div { class: "detail-val", style: "flex-direction:column; align-items:flex-start; gap:8px;",
                                    div { class: "bytecode-actions",
                                        CopyButton { text: format!("0x{}", contract.bytecode_hex.clone()) }
                                        span { style: "font-size:11px; color:var(--text-muted);",
                                            "Click to copy full bytecode"
                                        }
                                    }
                                    div { class: "bytecode-full-box",
                                        "0x{contract.bytecode_hex}"
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
