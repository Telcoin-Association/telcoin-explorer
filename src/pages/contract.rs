// src/pages/contract.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    resolve_selectors, FunctionSignature,
    get_contract_info, get_block_number, get_token_transfers,
    parse_transfer_logs, get_token_symbol, ContractInfo,
    TokenTransfer, shorten_hash, shorten_addr, unix_to_age,
    CONSENSUS_REGISTRY, rpc_call,
};
use crate::components::loading::{Loading, ErrorBox, CopyButton};

// ── ABI function descriptor ───────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
struct AbiParam {
    name: &'static str,
    ty:   &'static str,
}

#[derive(Clone, Debug, PartialEq)]
struct AbiFunction {
    name:       &'static str,
    selector:   &'static str,   // 4-byte hex without 0x
    mutability: &'static str,   // "view" | "pure" | "nonpayable" | "payable"
    inputs:     &'static [AbiParam],
    output_desc: &'static str,  // human-readable output description
}

// ConsensusRegistry ABI — verified from artifacts/ConsensusRegistry.json
const CONSENSUS_ABI: &[AbiFunction] = &[
    // ── Read ─────────────────────────────────────────────────────────────
    AbiFunction { name: "getCurrentEpoch",      selector: "03dc7d1f", mutability: "view",  inputs: &[], output_desc: "uint32 epoch" },
    AbiFunction { name: "getCurrentEpochInfo",  selector: "e6f7e7bc", mutability: "view",  inputs: &[], output_desc: "tuple (committee[], epochIssuance, blockHeight, epochId, epochDuration, stakeVersion)" },
    AbiFunction { name: "getCurrentStakeVersion", selector: "536343d2", mutability: "view", inputs: &[], output_desc: "uint8 version" },
    AbiFunction { name: "getNextCommitteeSize", selector: "a06f8dcb", mutability: "view",  inputs: &[], output_desc: "uint16 size" },
    AbiFunction { name: "owner",               selector: "be0e67a3", mutability: "view",  inputs: &[], output_desc: "address" },
    AbiFunction { name: "paused",              selector: "8165747e", mutability: "view",  inputs: &[], output_desc: "bool" },
    AbiFunction { name: "totalSupply",         selector: "1f1881f8", mutability: "view",  inputs: &[], output_desc: "uint256" },
    AbiFunction { name: "undistributedIssuance", selector: "206d9d6b", mutability: "view", inputs: &[], output_desc: "uint256" },
    AbiFunction { name: "issuance",            selector: "a6374aad", mutability: "view",  inputs: &[], output_desc: "address" },
    AbiFunction { name: "SYSTEM_ADDRESS",      selector: "340303e7", mutability: "view",  inputs: &[], output_desc: "address" },
    AbiFunction { name: "getValidator",        selector: "2c2675b3", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "tuple (validatorAddress, activationEpoch, exitEpoch, currentStatus, isRetired, stakeVersion, region)" },
    AbiFunction { name: "getValidators",       selector: "5d0c5507", mutability: "view",
        inputs: &[AbiParam { name: "status (0=Staked,1=PendingActivation,2=Active,3=PendingExit,4=Exited,5=Retired)", ty: "uint8" }],
        output_desc: "tuple[] validators" },
    AbiFunction { name: "getCommitteeValidators", selector: "5817923f", mutability: "view",
        inputs: &[AbiParam { name: "epoch", ty: "uint32" }],
        output_desc: "tuple[] validators" },
    AbiFunction { name: "getEpochInfo",        selector: "32c8af6c", mutability: "view",
        inputs: &[AbiParam { name: "epoch", ty: "uint32" }],
        output_desc: "tuple (committee[], epochIssuance, blockHeight, epochId, epochDuration, stakeVersion)" },
    AbiFunction { name: "getRewards",          selector: "8fd2ef03", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "uint256 rewards (wei)" },
    AbiFunction { name: "getBalanceBreakdown", selector: "700fd567", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "uint256 staked, uint256 rewards, uint256 delegated" },
    AbiFunction { name: "isDelegated",         selector: "d546c737", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "bool" },
    AbiFunction { name: "isRetired",           selector: "da8c5b79", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "bool" },
    AbiFunction { name: "isValidator",         selector: "8f7f86f7", mutability: "view",
        inputs: &[AbiParam { name: "blsPubkey (hex)", ty: "bytes" }],
        output_desc: "bool" },
    AbiFunction { name: "balanceOf",           selector: "1d7976f3", mutability: "view",
        inputs: &[AbiParam { name: "owner", ty: "address" }],
        output_desc: "uint256" },
    AbiFunction { name: "getBlsPubkey",        selector: "eb20256c", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "bytes blsPubkey" },
    AbiFunction { name: "validators",          selector: "709b80e1", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "address, uint32 activationEpoch, uint32 exitEpoch, uint8 status, bool isRetired, uint8 stakeVersion, uint8 region" },
    // ── Write ────────────────────────────────────────────────────────────
    AbiFunction { name: "activate",            selector: "1e5ddb13", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "beginExit",           selector: "26153ba9", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "pause",               selector: "0e0ece9c", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "unpause",             selector: "b4708547", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "renounceOwnership",   selector: "5e0827e9", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "unstake",             selector: "a1e271cb", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "" },
    AbiFunction { name: "claimStakeRewards",   selector: "494b7be7", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "" },
    AbiFunction { name: "mint",                selector: "6765b390", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "" },
    AbiFunction { name: "burn",                selector: "05218768", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "" },
    AbiFunction { name: "setNextCommitteeSize",selector: "1fa3697a", mutability: "nonpayable",
        inputs: &[AbiParam { name: "newSize", ty: "uint16" }], output_desc: "" },
    AbiFunction { name: "setValidatorRegion",  selector: "06ec433c", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }, AbiParam { name: "region (0-255)", ty: "uint8" }], output_desc: "" },
    AbiFunction { name: "transferOwnership",   selector: "aae7857b", mutability: "nonpayable",
        inputs: &[AbiParam { name: "newOwner", ty: "address" }], output_desc: "" },
    AbiFunction { name: "allocateIssuance",    selector: "785b8291", mutability: "payable",    inputs: &[], output_desc: "" },
];

// ── ABI encoding helpers ──────────────────────────────────────────────────

/// Encode a single param value into 32-byte ABI word (hex, no 0x)
fn abi_encode_param(ty: &str, value: &str) -> Result<String, String> {
    let v = value.trim();
    match ty {
        "address" => {
            let addr = v.trim_start_matches("0x").to_lowercase();
            if addr.len() != 40 { return Err(format!("Invalid address: {}", v)); }
            Ok(format!("{:0>64}", addr))
        }
        t if t.starts_with("uint") || t.starts_with("int") => {
            let n: u128 = v.parse().map_err(|_| format!("Invalid number: {}", v))?;
            Ok(format!("{:0>64x}", n))
        }
        "bool" => {
            let b = matches!(v.to_lowercase().as_str(), "true" | "1");
            Ok(format!("{:0>64x}", b as u8))
        }
        "bytes" | "bytes32" | "bytes4" | "bytes1" => {
            // accept hex input
            let hex = v.trim_start_matches("0x");
            Ok(format!("{:0<64}", hex))
        }
        _ => Err(format!("Unsupported type for encoding: {}", ty))
    }
}

/// Build calldata: 4-byte selector + encoded params
fn build_calldata(selector: &str, inputs: &[AbiParam], values: &[String]) -> Result<String, String> {
    let mut data = format!("0x{}", selector);
    for (param, val) in inputs.iter().zip(values.iter()) {
        let encoded = abi_encode_param(param.ty, val)?;
        data.push_str(&encoded);
    }
    Ok(data)
}

/// Decode raw hex result into human-readable string
fn decode_result(hex: &str, output_desc: &str) -> String {
    let raw = hex.trim_start_matches("0x");
    if raw.is_empty() || raw == "0" { return "(empty)".to_string(); }

    // Try to give a human-readable result based on output_desc
    if output_desc.contains("address") && raw.len() >= 64 {
        let addr = format!("0x{}", &raw[raw.len()-40..]);
        return addr;
    }
    if output_desc == "bool" && raw.len() >= 64 {
        let val = u64::from_str_radix(&raw[raw.len()-1..], 16).unwrap_or(0);
        return if val == 1 { "true".to_string() } else { "false".to_string() };
    }
    if (output_desc.contains("uint") || output_desc.contains("int")) && !output_desc.contains("tuple") {
        if raw.len() <= 64 {
            if let Ok(n) = u128::from_str_radix(raw.trim_start_matches('0').get(0..).unwrap_or("0"), 16) {
                return format!("{}", n);
            }
        }
    }
    // Return raw hex truncated
    if raw.len() > 80 { format!("0x{}…", &raw[..80]) } else { format!("0x{}", raw) }
}

// ── Page component ────────────────────────────────────────────────────────

#[component]
pub fn ContractPage(address: String) -> Element {
    let mut info: Signal<Option<ContractInfo>>        = use_signal(|| None);
    let mut transfers: Signal<Vec<TokenTransfer>>     = use_signal(|| vec![]);
    let mut loading                                   = use_signal(|| true);
    let mut error: Signal<Option<String>>             = use_signal(|| None);
    let mut signatures: Signal<Vec<FunctionSignature>> = use_signal(|| vec![]);
    let mut sigs_loading                              = use_signal(|| false);
    let mut active_tab                                = use_signal(|| "overview");
    // Read/Write state: map of fn_name -> (input_values, result, loading, error)
    let mut fn_inputs: Signal<std::collections::HashMap<String, Vec<String>>>  = use_signal(|| std::collections::HashMap::new());
    let mut fn_results: Signal<std::collections::HashMap<String, String>>      = use_signal(|| std::collections::HashMap::new());
    let mut fn_loading: Signal<std::collections::HashMap<String, bool>>        = use_signal(|| std::collections::HashMap::new());
    let mut fn_errors: Signal<std::collections::HashMap<String, String>>       = use_signal(|| std::collections::HashMap::new());
    let addr_clone = address.clone();

    use_effect(move || {
        let address = addr_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            match get_contract_info(&address).await {
                Ok(i)  => info.set(Some(i)),
                Err(e) => { error.set(Some(e)); loading.set(false); return; }
            }
            if let Ok(latest) = get_block_number().await {
                let from = latest.saturating_sub(5000);
                if let Ok(logs) = get_token_transfers(&address, from, latest).await {
                    let mut parsed = parse_transfer_logs(logs);
                    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
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

                // ── Page header ───────────────────────────────────────
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
                            div { class: "contract-badges",
                                span { class: "chip pending", "Contract" }
                                if contract.is_erc20  { span { class: "chip success", "ERC-20" } }
                                if contract.is_erc721 { span { class: "chip info",    "ERC-721" } }
                                if is_registry        { span { class: "chip info",    "ConsensusRegistry" } }
                                if contract.has_owner { span { class: "chip pending", "Ownable" } }
                                if contract.has_pause { span { class: "chip pending", "Pausable" } }
                                if contract.has_mint  { span { class: "chip pending", "Mintable" } }
                            }
                        }
                        div { class: "contract-addr-row",
                            span { class: "hash-cell", style: "font-size:13px;", "{contract.address}" }
                            CopyButton { text: contract.address.clone() }
                        }
                    }
                }

                // ── Tabs ──────────────────────────────────────────────
                div { class: "tabs-row",
                    button {
                        class: if *active_tab.read() == "overview"  { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| active_tab.set("overview"), "Overview"
                    }
                    button {
                        class: if *active_tab.read() == "read" { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| active_tab.set("read"), "Read Contract"
                    }
                    button {
                        class: if *active_tab.read() == "write" { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| active_tab.set("write"), "Write Contract"
                    }
                    button {
                        class: if *active_tab.read() == "transfers" { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| active_tab.set("transfers"),
                        "Transfers"
                        span { class: "tab-count", "({transfers.read().len()})" }
                    }
                    button {
                        class: if *active_tab.read() == "bytecode"  { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: {
                            let bytecode_hex = info.read().as_ref().map(|c| c.bytecode_hex.clone()).unwrap_or_default();
                            move |_| {
                                active_tab.set("bytecode");
                                if signatures.read().is_empty() && !bytecode_hex.is_empty() {
                                    let hex = bytecode_hex.clone();
                                    sigs_loading.set(true);
                                    wasm_bindgen_futures::spawn_local(async move {
                                        let sigs = resolve_selectors(&hex).await;
                                        signatures.set(sigs);
                                        sigs_loading.set(false);
                                    });
                                }
                            }
                        },
                        "Bytecode"
                    }
                    if is_registry {
                        Link { to: Route::ValidatorsPage {}, button { class: "tab-btn", "Validators →" } }
                        Link { to: Route::EpochsPage {},     button { class: "tab-btn", "Epochs →" } }
                    }
                }

                // ── Overview tab ──────────────────────────────────────
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
                                    span { style: "font-weight:600;", { format!("{:.6} TEL", contract.balance) } }
                                }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Transactions" }
                                div { class: "detail-val", "{contract.tx_count}" }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Bytecode Size" }
                                div { class: "detail-val",
                                    { format!("{} bytes ({:.1} KB)", contract.bytecode_size, contract.bytecode_size as f64 / 1024.0) }
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
                                        span { style: "color:var(--text-muted); font-size:12px;", "No standard interfaces detected" }
                                    }
                                }
                            }
                            if contract.is_erc20 {
                                div { class: "detail-row",
                                    div { class: "detail-key", "Token Name" }
                                    div { class: "detail-val", "{contract.token_name}" }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key", "Symbol" }
                                    div { class: "detail-val", span { class: "token-symbol-badge", "{contract.token_symbol}" } }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key", "Decimals" }
                                    div { class: "detail-val", "{contract.token_decimals}" }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key", "Total Supply" }
                                    div { class: "detail-val",
                                        span { style: "font-weight:600;", "{contract.token_supply} {contract.token_symbol}" }
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
                            if is_registry {
                                div { class: "detail-row",
                                    div { class: "detail-key", "Purpose" }
                                    div { class: "detail-val", "Manages validator registration, staking, committee selection and epoch rewards for Telcoin Network consensus." }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key", "Consensus" }
                                    div { class: "detail-val", span { class: "chip info", "Narwhal / Bullshark DAG-BFT" } }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key", "Explorer" }
                                    div { class: "detail-val", style: "gap:8px;",
                                        Link { to: Route::ValidatorsPage {}, span { class: "action-link", "Validators →" } }
                                        Link { to: Route::EpochsPage {},     span { class: "action-link", style: "margin-left:12px;", "Epochs →" } }
                                    }
                                }
                            }
                        }
                    }
                }

                // ── Read Contract tab ─────────────────────────────────
                if *active_tab.read() == "read" {
                    div { class: "detail-panel",
                        div { class: "detail-panel-title",
                            "Read Contract"
                            span { style: "font-size:11px; color:var(--text-muted); font-weight:400; margin-left:8px;",
                                "Call view functions — no wallet required"
                            }
                        }
                        div { class: "contract-fn-list",
                            if is_registry {
                                for func in CONSENSUS_ABI.iter().filter(|f| f.mutability == "view" || f.mutability == "pure") {
                                    ContractFnCard {
                                        func: func.clone(),
                                        contract_addr: address.clone(),
                                        is_write: false,
                                        fn_inputs: fn_inputs,
                                        fn_results: fn_results,
                                        fn_loading: fn_loading,
                                        fn_errors: fn_errors,
                                    }
                                }
                            } else if signatures.read().is_empty() {
                                div { class: "info-note", style: "margin: 12px 20px;",
                                    span { class: "info-note-icon", "ℹ" }
                                    span { "Click the Bytecode tab first to load function signatures, then return here." }
                                }
                            } else {
                                for sig in signatures.read().iter().filter(|s| {
                                    let n = s.signature.to_lowercase();
                                    !n.contains("transfer") && !n.contains("approve") && !n.contains("mint") && !n.contains("burn") && !n.contains("set") && !n.contains("stake") && !n.contains("send")
                                }).cloned().collect::<Vec<_>>().into_iter() {
                                    GenericFnCard {
                                        sig: sig.clone(),
                                        contract_addr: address.clone(),
                                        is_write: false,
                                        fn_results: fn_results,
                                        fn_loading: fn_loading,
                                        fn_errors: fn_errors,
                                    }
                                }
                            }
                        }
                    }
                }

                // ── Write Contract tab ────────────────────────────────
                if *active_tab.read() == "write" {
                    div { class: "detail-panel",
                        div { class: "detail-panel-title",
                            "Write Contract"
                            span { style: "font-size:11px; color:var(--text-muted); font-weight:400; margin-left:8px;",
                                "Send transactions — wallet must be connected"
                            }
                        }
                        div { class: "info-note", style: "margin: 12px 20px;",
                            span { class: "info-note-icon", "ℹ" }
                            span { "Connect your wallet in the header before calling write functions. Transactions will be sent from your connected account." }
                        }
                        div { class: "contract-fn-list",
                            if is_registry {
                                for func in CONSENSUS_ABI.iter().filter(|f| f.mutability == "nonpayable" || f.mutability == "payable") {
                                    ContractFnCard {
                                        func: func.clone(),
                                        contract_addr: address.clone(),
                                        is_write: true,
                                        fn_inputs: fn_inputs,
                                        fn_results: fn_results,
                                        fn_loading: fn_loading,
                                        fn_errors: fn_errors,
                                    }
                                }
                            } else if signatures.read().is_empty() {
                                div { class: "info-note", style: "margin: 12px 20px;",
                                    span { class: "info-note-icon", "ℹ" }
                                    span { "Click the Bytecode tab first to load function signatures, then return here." }
                                }
                            } else {
                                for sig in signatures.read().iter().filter(|s| {
                                    let n = s.signature.to_lowercase();
                                    n.contains("transfer") || n.contains("approve") || n.contains("mint") || n.contains("burn") || n.contains("set") || n.contains("stake") || n.contains("send")
                                }).cloned().collect::<Vec<_>>().into_iter() {
                                    GenericFnCard {
                                        sig: sig.clone(),
                                        contract_addr: address.clone(),
                                        is_write: true,
                                        fn_results: fn_results,
                                        fn_loading: fn_loading,
                                        fn_errors: fn_errors,
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
                            span { style: "color:var(--text-muted); font-size:11px;", "Last 5,000 blocks" }
                        }
                        if transfers.read().is_empty() {
                            div { class: "panel-empty", "No token transfers found in the last 5,000 blocks." }
                        } else {
                            div { class: "table-wrapper",
                                table { class: "tx-table",
                                    thead {
                                        tr {
                                            th { "TX HASH" } th { "BLOCK" } th { "AGE" }
                                            th { "FROM" } th { "" } th { "TO" } th { "AMOUNT" }
                                        }
                                    }
                                    tbody {
                                        for t in transfers.read().iter() {
                                            tr {
                                                td { Link { to: Route::TransactionPage { hash: t.tx_hash.clone() }, span { class: "hash-cell", "{shorten_hash(&t.tx_hash)}" } } }
                                                td { Link { to: Route::BlockPage { block_number: t.block_number }, span { class: "hash-cell", "#{t.block_number}" } } }
                                                td { style: "color:var(--text-muted);", "{unix_to_age(t.timestamp)}" }
                                                td { Link { to: Route::AddressPage { address: t.from.clone() }, span { class: "hash-cell addr-short", "{shorten_addr(&t.from)}" } } }
                                                td { span { class: "transfer-arrow", "→" } }
                                                td { Link { to: Route::AddressPage { address: t.to.clone() }, span { class: "hash-cell addr-short", "{shorten_addr(&t.to)}" } } }
                                                td { style: "color:var(--accent-green); font-weight:600;", { format!("{:.4} {}", t.amount, t.token_symbol) } }
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
                    div { class: "bytecode-unverified-banner",
                        div { class: "bub-icon",
                            svg { width:"18", height:"18", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", stroke_linecap:"round", stroke_linejoin:"round",
                                path { d:"M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" }
                                path { d:"M12 9v4m0 4h.01" }
                            }
                        }
                        div { class: "bub-text",
                            strong { "Source code not verified. " }
                            "Function signatures below are resolved from the "
                            a { href: "https://www.4byte.directory", target: "_blank", class: "hash-cell", "4byte.directory" }
                            " public database. For decompilation use "
                            a { href: format!("https://app.dedaub.com/decompile?md5={}", &contract.address), target: "_blank", class: "hash-cell", "Dedaub ↗" }
                            " or "
                            a { href: format!("https://ethervm.io/decompile?address={}&network=custom&rpc=https://rpc.telcoin.network", &contract.address), target: "_blank", class: "hash-cell", "EtherVM ↗" }
                        }
                    }
                    div { class: "detail-panel", style: "margin-bottom:16px;",
                        div { class: "detail-panel-title", "Contract Interface (4byte.directory)" }
                        if *sigs_loading.read() {
                            div { class: "loading-wrapper", style: "padding:24px;",
                                div { class: "spinner" }
                                span { class: "loading-text", "Resolving function signatures…" }
                            }
                        } else if signatures.read().is_empty() {
                            div { class: "bytecode-notice", span { "No function signatures resolved yet. Click the Bytecode tab to load." } }
                        } else {
                            div { class: "sig-table",
                                div { class: "sig-header",
                                    span { "SELECTOR" } span { "FUNCTION SIGNATURE" } span { "TYPE" }
                                }
                                for sig in signatures.read().iter() {
                                    div { class: "sig-row",
                                        span { class: "sig-selector", "0x{sig.selector}" }
                                        span { class: "sig-name code-inline", "{sig.signature}" }
                                        span { class: "sig-type",
                                            {
                                                if sig.signature.contains("transfer") || sig.signature.contains("approve") || sig.signature.contains("mint") || sig.signature.contains("burn") || sig.signature.contains("set") || sig.signature.contains("stake") {
                                                    rsx! { span { class: "chip pending", style: "font-size:10px; padding:1px 6px;", "Write" } }
                                                } else {
                                                    rsx! { span { class: "chip info", style: "font-size:10px; padding:1px 6px;", "Read" } }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "detail-panel",
                        div { class: "detail-panel-title",
                            { format!("Raw Bytecode — {} bytes ({:.2} KB)", contract.bytecode_size, contract.bytecode_size as f64 / 1024.0) }
                        }
                        div { style: "padding: 16px 20px;",
                            div { class: "bytecode-actions", style: "margin-bottom:10px;",
                                CopyButton { text: format!("0x{}", contract.bytecode_hex.clone()) }
                                span { style: "font-size:11px; color:var(--text-muted);", "Copy full bytecode to clipboard" }
                            }
                            div { class: "bytecode-full-box", "0x{contract.bytecode_hex}" }
                        }
                    }
                }
            }
        }
    }
}

// ── Contract function card component ─────────────────────────────────────

#[component]
fn ContractFnCard(
    func: AbiFunction,
    contract_addr: String,
    is_write: bool,
    mut fn_inputs:  Signal<std::collections::HashMap<String, Vec<String>>>,
    mut fn_results: Signal<std::collections::HashMap<String, String>>,
    mut fn_loading: Signal<std::collections::HashMap<String, bool>>,
    mut fn_errors:  Signal<std::collections::HashMap<String, String>>,
) -> Element {
    let key = func.name.to_string();
    let num_inputs = func.inputs.len();

    // Ensure input slots exist
    {
        let mut inputs_map = fn_inputs.write();
        inputs_map.entry(key.clone()).or_insert_with(|| vec!["".to_string(); num_inputs]);
    }

    let result  = fn_results.read().get(&key).cloned();
    let loading = fn_loading.read().get(&key).copied().unwrap_or(false);
    let err     = fn_errors.read().get(&key).cloned();
    let is_payable = func.mutability == "payable";

    rsx! {
        div { class: "contract-fn-card",
            div { class: "contract-fn-header",
                span { class: "contract-fn-name", "{func.name}" }
                if is_payable {
                    span { class: "chip pending", style: "font-size:10px;", "payable" }
                }
                if !func.output_desc.is_empty() {
                    span { class: "contract-fn-returns", "→ {func.output_desc}" }
                }
            }

            // Input fields
            for (i, param) in func.inputs.iter().enumerate() {
                div { class: "contract-fn-input-row",
                    label { class: "contract-fn-label",
                        span { class: "contract-fn-param-name", "{param.name}" }
                        span { class: "contract-fn-param-type", " ({param.ty})" }
                    }
                    input {
                        class: "contract-fn-input",
                        placeholder: "{param.ty}",
                        value: {
                            fn_inputs.read().get(&key)
                                .and_then(|v| v.get(i))
                                .cloned()
                                .unwrap_or_default()
                        },
                        oninput: {
                            let key = key.clone();
                            move |e: Event<FormData>| {
                                let mut map = fn_inputs.write();
                                if let Some(vals) = map.get_mut(&key) {
                                    if i < vals.len() {
                                        vals[i] = e.value();
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ETH value input for payable
            if is_payable {
                div { class: "contract-fn-input-row",
                    label { class: "contract-fn-label",
                        span { class: "contract-fn-param-name", "value" }
                        span { class: "contract-fn-param-type", " (TEL in wei)" }
                    }
                    input {
                        class: "contract-fn-input",
                        placeholder: "0",
                        id: "payable-{key}",
                    }
                }
            }

            // Call button
            div { class: "contract-fn-actions",
                button {
                    class: if is_write { "contract-fn-btn contract-fn-btn-write" } else { "contract-fn-btn contract-fn-btn-read" },
                    disabled: loading,
                    onclick: {
                        let key        = key.clone();
                        let func_sel   = func.selector.to_string();
                        let func_inputs_def: Vec<(String, String)> = func.inputs.iter().map(|p| (p.name.to_string(), p.ty.to_string())).collect();
                        let func_out   = func.output_desc.to_string();
                        let addr       = contract_addr.clone();
                        let is_write2  = is_write;

                        move |_| {
                            let key        = key.clone();
                            let func_sel   = func_sel.clone();
                            let func_inputs_def = func_inputs_def.clone();
                            let func_out   = func_out.clone();
                            let addr       = addr.clone();

                            // Collect current input values
                            let values: Vec<String> = fn_inputs.read()
                                .get(&key).cloned().unwrap_or_default();

                            fn_loading.write().insert(key.clone(), true);
                            fn_errors.write().remove(&key);
                            fn_results.write().remove(&key);

                            wasm_bindgen_futures::spawn_local(async move {
                                // Build calldata
                                let mut data = format!("0x{}", func_sel);
                                let mut encode_err: Option<String> = None;
                                for (i, (_, ty)) in func_inputs_def.iter().enumerate() {
                                    let val = values.get(i).cloned().unwrap_or_default();
                                    match abi_encode_param(ty, &val) {
                                        Ok(enc) => data.push_str(&enc),
                                        Err(e)  => { encode_err = Some(e); break; }
                                    }
                                }

                                if let Some(e) = encode_err {
                                    fn_errors.write().insert(key.clone(), e);
                                    fn_loading.write().insert(key.clone(), false);
                                    return;
                                }

                                if !is_write2 {
                                    // eth_call
                                    let call_params = serde_json::json!([{"to": addr, "data": data}, "latest"]);
                                    match rpc_call::<_, String>("eth_call", call_params).await {
                                        Ok(hex) => {
                                            let decoded = decode_result(&hex, &func_out);
                                            fn_results.write().insert(key.clone(), decoded);
                                        }
                                        Err(e) => { fn_errors.write().insert(key.clone(), e); }
                                    }
                                } else {
                                    // eth_sendTransaction via wallet
                                    let js = format!(r#"
(async function() {{
    if (!window.ethereum) return {{error: 'No wallet connected. Please connect your wallet first.'}};
    try {{
        const accounts = await window.ethereum.request({{ method: 'eth_accounts' }});
        if (!accounts || accounts.length === 0) return {{error: 'No wallet connected. Please connect your wallet first.'}};
        const txHash = await window.ethereum.request({{
            method: 'eth_sendTransaction',
            params: [{{
                from: accounts[0],
                to: '{}',
                data: '{}',
                gas: '0x30000'
            }}]
        }});
        return {{hash: txHash}};
    }} catch(err) {{
        if (err.code === 4001) return {{error: 'Transaction rejected by user.'}};
        return {{error: err.message || 'Transaction failed'}};
    }}
}})()
"#, addr, data);
                                    match js_sys::eval(&js) {
                                        Ok(promise_val) => {
                                            match wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise_val)).await {
                                                Ok(result) => {
                                                    let obj = js_sys::Object::from(result);
                                                    let err_key  = wasm_bindgen::JsValue::from_str("error");
                                                    let hash_key = wasm_bindgen::JsValue::from_str("hash");
                                                    if let Some(e) = js_sys::Reflect::get(&obj, &err_key).ok().and_then(|v| v.as_string()) {
                                                        fn_errors.write().insert(key.clone(), e);
                                                    } else if let Some(h) = js_sys::Reflect::get(&obj, &hash_key).ok().and_then(|v| v.as_string()) {
                                                        fn_results.write().insert(key.clone(), format!("Tx sent: {}", h));
                                                    }
                                                }
                                                Err(e) => { fn_errors.write().insert(key.clone(), format!("{:?}", e)); }
                                            }
                                        }
                                        Err(_) => { fn_errors.write().insert(key.clone(), "Failed to execute wallet call".to_string()); }
                                    }
                                }

                                fn_loading.write().insert(key.clone(), false);
                            });
                        }
                    },
                    if loading {
                        span { class: "spinner", style: "width:12px;height:12px;border-width:2px;" }
                    } else if is_write {
                        "Send Transaction"
                    } else {
                        "Query"
                    }
                }
            }

            // Result display
            if let Some(ref res) = result {
                div { class: "contract-fn-result",
                    span { class: "contract-fn-result-label", "Result:" }
                    span { class: "contract-fn-result-value", "{res}" }
                }
            }
            if let Some(ref e) = err {
                div { class: "contract-fn-error", "⚠ {e}" }
            }
        }
    }
}

// ── Generic function card (for non-registry contracts) ───────────────────

#[component]
fn GenericFnCard(
    sig: FunctionSignature,
    contract_addr: String,
    is_write: bool,
    mut fn_results: Signal<std::collections::HashMap<String, String>>,
    mut fn_loading: Signal<std::collections::HashMap<String, bool>>,
    mut fn_errors:  Signal<std::collections::HashMap<String, String>>,
) -> Element {
    let key     = sig.selector.clone();
    let loading = fn_loading.read().get(&key).copied().unwrap_or(false);
    let result  = fn_results.read().get(&key).cloned();
    let err     = fn_errors.read().get(&key).cloned();
    let mut raw_input = use_signal(|| "".to_string());

    rsx! {
        div { class: "contract-fn-card",
            div { class: "contract-fn-header",
                span { class: "contract-fn-name", "{sig.signature}" }
                span { class: "sig-selector", style: "margin-left:8px; font-size:11px; color:var(--text-muted);", "0x{sig.selector}" }
            }
            div { class: "contract-fn-input-row",
                label { class: "contract-fn-label", "Calldata params (hex, after selector — leave empty for no-arg)" }
                input {
                    class: "contract-fn-input",
                    placeholder: "e.g. 000000000000000000000000abcd…",
                    value: "{raw_input}",
                    oninput: move |e: Event<FormData>| raw_input.set(e.value()),
                }
            }
            div { class: "contract-fn-actions",
                button {
                    class: if is_write { "contract-fn-btn contract-fn-btn-write" } else { "contract-fn-btn contract-fn-btn-read" },
                    disabled: loading,
                    onclick: {
                        let key       = key.clone();
                        let selector  = sig.selector.clone();
                        let addr      = contract_addr.clone();
                        let is_write2 = is_write;
                        move |_| {
                            let key      = key.clone();
                            let selector = selector.clone();
                            let addr     = addr.clone();
                            let params   = raw_input.read().trim().to_string();
                            let data = if params.is_empty() {
                                format!("0x{}", selector)
                            } else {
                                format!("0x{}{}", selector, params.trim_start_matches("0x"))
                            };
                            fn_loading.write().insert(key.clone(), true);
                            fn_errors.write().remove(&key);
                            fn_results.write().remove(&key);
                            wasm_bindgen_futures::spawn_local(async move {
                                if !is_write2 {
                                    let call_params = serde_json::json!([{"to": addr, "data": data}, "latest"]);
                                    match rpc_call::<_, String>("eth_call", call_params).await {
                                        Ok(hex) => { fn_results.write().insert(key.clone(), hex); }
                                        Err(e)  => { fn_errors.write().insert(key.clone(), e); }
                                    }
                                } else {
                                    let js = format!(r#"(async function(){{if(!window.ethereum)return{{error:"No wallet"}};try{{const a=await window.ethereum.request({{method:"eth_accounts"}});if(!a||!a.length)return{{error:"No wallet"}};const h=await window.ethereum.request({{method:"eth_sendTransaction",params:[{{from:a[0],to:"{}",data:"{}",gas:"0x30000"}}]}});return{{hash:h}};}}catch(e){{return{{error:e.message}};}}}})()"#, addr, data);
                                    if let Ok(pv) = js_sys::eval(&js) {
                                        if let Ok(r) = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(pv)).await {
                                            let obj = js_sys::Object::from(r);
                                            let ek = wasm_bindgen::JsValue::from_str("error");
                                            let hk = wasm_bindgen::JsValue::from_str("hash");
                                            if let Some(e) = js_sys::Reflect::get(&obj,&ek).ok().and_then(|v|v.as_string()) {
                                                fn_errors.write().insert(key.clone(), e);
                                            } else if let Some(h) = js_sys::Reflect::get(&obj,&hk).ok().and_then(|v|v.as_string()) {
                                                fn_results.write().insert(key.clone(), format!("Tx: {}", h));
                                            }
                                        }
                                    }
                                }
                                fn_loading.write().insert(key.clone(), false);
                            });
                        }
                    },
                    if loading { span { class: "spinner", style: "width:12px;height:12px;border-width:2px;" } }
                    else if is_write { "Send Transaction" }
                    else { "Query" }
                }
            }
            if let Some(ref res) = result {
                div { class: "contract-fn-result",
                    span { class: "contract-fn-result-label", "Result:" }
                    span { class: "contract-fn-result-value", "{res}" }
                }
            }
            if let Some(ref e) = err {
                div { class: "contract-fn-error", "⚠ {e}" }
            }
        }
    }
}
