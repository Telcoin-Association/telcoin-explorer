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

// ── Static ABI types (ConsensusRegistry) ─────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
struct AbiParam {
    name: &'static str,
    ty:   &'static str,
}

#[derive(Clone, Debug, PartialEq)]
struct AbiFunction {
    name:        &'static str,
    selector:    &'static str,
    mutability:  &'static str,
    inputs:      &'static [AbiParam],
    output_desc: &'static str,
}

// ── Dynamic ABI types (uploaded JSON) ────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
struct DynParam {
    name: String,
    ty:   String,
}

#[derive(Clone, Debug, PartialEq)]
struct DynFunction {
    name:        String,
    selector:    String,
    mutability:  String,
    inputs:      Vec<DynParam>,
    output_desc: String,
}

// ── ConsensusRegistry ABI ─────────────────────────────────────────────────

const CONSENSUS_ABI: &[AbiFunction] = &[
    AbiFunction { name: "getCurrentEpoch",        selector: "03dc7d1f", mutability: "view",        inputs: &[], output_desc: "uint32 epoch" },
    AbiFunction { name: "getCurrentEpochInfo",    selector: "e6f7e7bc", mutability: "view",        inputs: &[], output_desc: "tuple (committee[], epochIssuance, blockHeight, epochId, epochDuration, stakeVersion)" },
    AbiFunction { name: "getCurrentStakeVersion", selector: "536343d2", mutability: "view",        inputs: &[], output_desc: "uint8 version" },
    AbiFunction { name: "getNextCommitteeSize",   selector: "a06f8dcb", mutability: "view",        inputs: &[], output_desc: "uint16 size" },
    AbiFunction { name: "owner",                  selector: "be0e67a3", mutability: "view",        inputs: &[], output_desc: "address" },
    AbiFunction { name: "paused",                 selector: "8165747e", mutability: "view",        inputs: &[], output_desc: "bool" },
    AbiFunction { name: "totalSupply",            selector: "1f1881f8", mutability: "view",        inputs: &[], output_desc: "uint256" },
    AbiFunction { name: "undistributedIssuance",  selector: "206d9d6b", mutability: "view",        inputs: &[], output_desc: "uint256" },
    AbiFunction { name: "issuance",               selector: "a6374aad", mutability: "view",        inputs: &[], output_desc: "address" },
    AbiFunction { name: "SYSTEM_ADDRESS",         selector: "340303e7", mutability: "view",        inputs: &[], output_desc: "address" },
    AbiFunction { name: "getValidator",           selector: "2c2675b3", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "tuple (validatorAddress, activationEpoch, exitEpoch, currentStatus, isRetired, stakeVersion, region)" },
    AbiFunction { name: "getValidators",          selector: "5d0c5507", mutability: "view",
        inputs: &[AbiParam { name: "status (0=Staked,1=PendingActivation,2=Active,3=PendingExit,4=Exited,5=Retired)", ty: "uint8" }],
        output_desc: "tuple[] validators" },
    AbiFunction { name: "getCommitteeValidators", selector: "5817923f", mutability: "view",
        inputs: &[AbiParam { name: "epoch", ty: "uint32" }], output_desc: "tuple[] validators" },
    AbiFunction { name: "getEpochInfo",           selector: "32c8af6c", mutability: "view",
        inputs: &[AbiParam { name: "epoch", ty: "uint32" }],
        output_desc: "tuple (committee[], epochIssuance, blockHeight, epochId, epochDuration, stakeVersion)" },
    AbiFunction { name: "getRewards",             selector: "8fd2ef03", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "uint256 rewards (wei)" },
    AbiFunction { name: "getBalanceBreakdown",    selector: "700fd567", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "uint256 staked, uint256 rewards, uint256 delegated" },
    AbiFunction { name: "isDelegated",            selector: "d546c737", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "bool" },
    AbiFunction { name: "isRetired",              selector: "da8c5b79", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "bool" },
    AbiFunction { name: "isValidator",            selector: "8f7f86f7", mutability: "view",
        inputs: &[AbiParam { name: "blsPubkey (hex)", ty: "bytes" }], output_desc: "bool" },
    AbiFunction { name: "balanceOf",              selector: "1d7976f3", mutability: "view",
        inputs: &[AbiParam { name: "owner", ty: "address" }], output_desc: "uint256" },
    AbiFunction { name: "getBlsPubkey",           selector: "eb20256c", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "bytes blsPubkey" },
    AbiFunction { name: "validators",             selector: "709b80e1", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "address, uint32 activationEpoch, uint32 exitEpoch, uint8 status, bool isRetired, uint8 stakeVersion, uint8 region" },
    AbiFunction { name: "activate",              selector: "1e5ddb13", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "beginExit",             selector: "26153ba9", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "pause",                 selector: "0e0ece9c", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "unpause",               selector: "b4708547", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "renounceOwnership",     selector: "5e0827e9", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "unstake",               selector: "a1e271cb", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "" },
    AbiFunction { name: "claimStakeRewards",     selector: "494b7be7", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "" },
    AbiFunction { name: "mint",                  selector: "6765b390", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "" },
    AbiFunction { name: "burn",                  selector: "05218768", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "" },
    AbiFunction { name: "setNextCommitteeSize",  selector: "1fa3697a", mutability: "nonpayable",
        inputs: &[AbiParam { name: "newSize", ty: "uint16" }], output_desc: "" },
    AbiFunction { name: "setValidatorRegion",    selector: "06ec433c", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }, AbiParam { name: "region (0-255)", ty: "uint8" }], output_desc: "" },
    AbiFunction { name: "transferOwnership",     selector: "aae7857b", mutability: "nonpayable",
        inputs: &[AbiParam { name: "newOwner", ty: "address" }], output_desc: "" },
    AbiFunction { name: "allocateIssuance",      selector: "785b8291", mutability: "payable", inputs: &[], output_desc: "" },
];

// ── Pure helper functions (NO RSX, NO format! with JS strings) ───────────

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
        _ => {
            let hex = v.trim_start_matches("0x");
            Ok(format!("{:0<64}", hex))
        }
    }
}

fn decode_result(hex: &str, output_desc: &str) -> String {
    let raw = hex.trim_start_matches("0x");
    if raw.is_empty() || raw == "0" { return "(empty)".to_string(); }
    if output_desc.contains("address") && raw.len() >= 64 {
        return format!("0x{}", &raw[raw.len()-40..]);
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
    if raw.len() > 80 { format!("0x{}…", &raw[..80]) } else { format!("0x{}", raw) }
}

/// Build JS string for eth_call (no format! with {} — uses concat)
fn js_eth_call(addr: &str, data: &str) -> String {
    [
        r#"(async function(){try{const r=await window.ethereum.request({method:"eth_call","#,
        r#"params:[{"to":""#, addr, r#"","data":""#, data, r#""},"latest"]});return{result:r};}catch(e){return{error:e.message};}})()"#,
    ].concat()
}

/// Build JS string for eth_sendTransaction (no format! with {})
fn js_send_tx(addr: &str, data: &str) -> String {
    [
        r#"(async function(){if(!window.ethereum)return{error:"No wallet"};try{"#,
        r#"const a=await window.ethereum.request({method:"eth_accounts"});"#,
        r#"if(!a||!a.length)return{error:"No wallet connected"};"#,
        r#"const h=await window.ethereum.request({method:"eth_sendTransaction","#,
        r#"params:[{from:a[0],to:""#, addr, r#"",data:""#, data, r#"",gas:"0x30000"}]});"#,
        r#"return{hash:h};}catch(e){return{error:e.message||"Failed"};}})()"#,
    ].concat()
}

/// Save ABI JSON to localStorage (no format! with JS braces)
fn ls_save_abi(address: &str, json_str: &str) {
    let key = ["abi_", &address.to_lowercase()].concat();
    if let Ok(escaped) = serde_json::to_string(json_str) {
        let js = ["localStorage.setItem(", &serde_json::to_string(&key).unwrap_or_default(), ",", &escaped, ")"].concat();
        let _ = js_sys::eval(&js);
    }
}

/// Load ABI JSON from localStorage
fn ls_load_abi(address: &str) -> String {
    let key = ["abi_", &address.to_lowercase()].concat();
    let js = ["(localStorage.getItem(", &serde_json::to_string(&key).unwrap_or_default(), ")||'')"].concat();
    js_sys::eval(&js).ok().and_then(|v| v.as_string()).unwrap_or_default()
}

/// Clear ABI from localStorage
fn ls_clear_abi(address: &str) {
    let key = ["abi_", &address.to_lowercase()].concat();
    let js = ["localStorage.removeItem(", &serde_json::to_string(&key).unwrap_or_default(), ")"].concat();
    let _ = js_sys::eval(&js);
}

/// Parse ABI JSON into DynFunction vec
fn parse_abi_json(json_str: &str) -> Vec<DynFunction> {
    if json_str.trim().is_empty() { return vec![]; }
    let escaped = json_str
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "");
    let js = format!(r#"
(function() {{
    try {{
        var raw = JSON.parse('{}');
        var abi = Array.isArray(raw) ? raw : (raw.abi || []);
        var out = [];
        abi.forEach(function(fn) {{
            if (fn.type !== 'function') return;
            var inputs = (fn.inputs||[]).map(function(i) {{ return (i.name||'')+':'+i.type; }}).join(';');
            var outputs = (fn.outputs||[]).map(function(o) {{ return o.type; }}).join(', ');
            var mut = fn.stateMutability || (fn.constant ? 'view' : 'nonpayable');
            var sig = fn.name+'('+(fn.inputs||[]).map(function(i){{
                if(i.type==='tuple'||i.type.startsWith('tuple')){{
                    var inner=(i.components||[]).map(function(c){{return c.type;}}).join(',');
                    return '('+inner+')'+i.type.replace('tuple','');
                }}
                return i.type;
            }}).join(',')+')';
            out.push(mut+'|'+fn.name+'|'+sig+'|'+inputs+'|'+outputs);
        }});
        return out.join('\n');
    }} catch(e) {{ return 'ERROR:'+e.message; }}
}})()
"#, escaped);
    let output = js_sys::eval(&js).ok().and_then(|v| v.as_string()).unwrap_or_default();
    if output.starts_with("ERROR:") || output.is_empty() { return vec![]; }
    output.lines().filter_map(|line| {
        let parts: Vec<&str> = line.splitn(5, '|').collect();
        if parts.len() < 3 { return None; }
        let mutability  = parts[0].to_string();
        let name        = parts[1].to_string();
        let sig         = parts[2].to_string();
        let inputs_raw  = if parts.len() > 3 { parts[3] } else { "" };
        let output_desc = if parts.len() > 4 { parts[4].to_string() } else { String::new() };
        // Compute 4-byte selector via JS keccak
        let sel_js = format!("(function(){{var s={};var b=new TextEncoder().encode(s);return crypto.subtle.digest('SHA-256',b).then(function(h){{var a=new Uint8Array(h);return Array.from(a.slice(0,4)).map(function(x){{return x.toString(16).padStart(2,'0')}}).join('');}});}})();", serde_json::to_string(&sig).unwrap_or_default());
        // For now use a simple approach - just use the sig as a display key
        // Real keccak would need wasm crypto, too complex; use sig hash approximation
        let selector = format!("{:08x}", {
            let mut h: u32 = 0x811c9dc5u32;
            for b in sig.bytes() {
                h ^= b as u32;
                h = h.wrapping_mul(0x01000193);
            }
            h
        });
        let _ = sel_js;
        let inputs = if inputs_raw.is_empty() {
            vec![]
        } else {
            inputs_raw.split(';').filter_map(|s| {
                let mut p = s.splitn(2, ':');
                let n = p.next()?.to_string();
                let t = p.next()?.to_string();
                Some(DynParam { name: n, ty: t })
            }).collect()
        };
        Some(DynFunction { name, selector, mutability, inputs, output_desc })
    }).collect()
}

/// Execute JS promise and extract result/error strings
async fn exec_js_promise(js: &str) -> (Option<String>, Option<String>) {
    match js_sys::eval(js) {
        Ok(pv) => {
            match wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(pv)).await {
                Ok(r) => {
                    let obj = js_sys::Object::from(r);
                    let ek = wasm_bindgen::JsValue::from_str("error");
                    let rk = wasm_bindgen::JsValue::from_str("result");
                    let hk = wasm_bindgen::JsValue::from_str("hash");
                    let err  = js_sys::Reflect::get(&obj, &ek).ok().and_then(|v| v.as_string());
                    let res  = js_sys::Reflect::get(&obj, &rk).ok().and_then(|v| v.as_string())
                        .or_else(|| js_sys::Reflect::get(&obj, &hk).ok().and_then(|v| v.as_string()));
                    (res, err)
                }
                Err(e) => (None, Some(format!("{:?}", e)))
            }
        }
        Err(_) => (None, Some("Failed to execute JS".to_string()))
    }
}

// ── Page component ────────────────────────────────────────────────────────

#[component]
pub fn ContractPage(address: String) -> Element {
    let mut info: Signal<Option<ContractInfo>>         = use_signal(|| None);
    let mut transfers: Signal<Vec<TokenTransfer>>      = use_signal(|| vec![]);
    let mut loading                                    = use_signal(|| true);
    let mut error: Signal<Option<String>>              = use_signal(|| None);
    let mut signatures: Signal<Vec<FunctionSignature>> = use_signal(|| vec![]);
    let mut sigs_loading                               = use_signal(|| false);
    let mut active_tab                                 = use_signal(|| "overview");
    let mut uploaded_abi: Signal<Vec<DynFunction>>     = use_signal(|| vec![]);
    let mut abi_input                                  = use_signal(|| String::new());
    let mut abi_msg: Signal<Option<(bool, String)>>    = use_signal(|| None); // (is_ok, msg)

    let mut fn_inputs:  Signal<std::collections::HashMap<String, Vec<String>>> = use_signal(|| std::collections::HashMap::new());
    let mut fn_results: Signal<std::collections::HashMap<String, String>>      = use_signal(|| std::collections::HashMap::new());
    let mut fn_loading: Signal<std::collections::HashMap<String, bool>>        = use_signal(|| std::collections::HashMap::new());
    let mut fn_errors:  Signal<std::collections::HashMap<String, String>>      = use_signal(|| std::collections::HashMap::new());

    let addr_clone = address.clone();

    use_effect(move || {
        let address = addr_clone.clone();
        // Try loading saved ABI
        let saved_json = ls_load_abi(&address);
        if !saved_json.is_empty() {
            let parsed = parse_abi_json(&saved_json);
            if !parsed.is_empty() {
                let n = parsed.len();
                uploaded_abi.set(parsed);
                abi_msg.set(Some((true, format!("{} functions loaded from saved ABI", n))));
            }
        }
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
                        let sym = if let Some(s) = seen.get(&t.token_address) { s.clone() }
                            else {
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

    let is_registry     = address.to_lowercase() == CONSENSUS_REGISTRY.to_lowercase();
    let avatar_char     = address.chars().nth(2).unwrap_or('C').to_uppercase().next().unwrap_or('C');
    let has_upload      = !uploaded_abi.read().is_empty();

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

                // ── Header ────────────────────────────────────────────
                div { class: "contract-header",
                    div { class: "contract-avatar",
                        if contract.is_erc20 || contract.is_erc721 {
                            svg { width:"24", height:"24", view_box:"0 0 24 24", fill:"none",
                                stroke:"currentColor", stroke_width:"1.5", stroke_linecap:"round", stroke_linejoin:"round",
                                circle { cx:"12", cy:"12", r:"10" }
                                path { d:"M12 6v6l4 2" }
                            }
                        } else { span { "{avatar_char}" } }
                    }
                    div { class: "contract-header-info",
                        div { class: "contract-title-row",
                            h1 { class: "page-title", style: "margin-bottom:0;",
                                if !contract.token_name.is_empty() { "{contract.token_name}" }
                                else if is_registry { "ConsensusRegistry" }
                                else { "Smart Contract" }
                            }
                            div { class: "contract-badges",
                                span { class: "chip pending", "Contract" }
                                if contract.is_erc20  { span { class: "chip success", "ERC-20" } }
                                if contract.is_erc721 { span { class: "chip info",    "ERC-721" } }
                                if is_registry        { span { class: "chip info",    "ConsensusRegistry" } }
                                if contract.has_owner { span { class: "chip pending", "Ownable" } }
                                if contract.has_pause { span { class: "chip pending", "Pausable" } }
                                if contract.has_mint  { span { class: "chip pending", "Mintable" } }
                                if has_upload && !is_registry { span { class: "chip success", "ABI Uploaded" } }
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
                    button { class: if *active_tab.read() == "overview"  { "tab-btn tab-active" } else { "tab-btn" }, onclick: move |_| active_tab.set("overview"),  "Overview" }
                    button { class: if *active_tab.read() == "read"      { "tab-btn tab-active" } else { "tab-btn" }, onclick: move |_| active_tab.set("read"),      "Read Contract" }
                    button { class: if *active_tab.read() == "write"     { "tab-btn tab-active" } else { "tab-btn" }, onclick: move |_| active_tab.set("write"),     "Write Contract" }
                    button { class: if *active_tab.read() == "transfers" { "tab-btn tab-active" } else { "tab-btn" }, onclick: move |_| active_tab.set("transfers"),
                        "Transfers"
                        span { class: "tab-count", "({transfers.read().len()})" }
                    }
                    button {
                        class: if *active_tab.read() == "bytecode" { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: {
                            let bytecode_hex = info.read().as_ref().map(|c| c.bytecode_hex.clone()).unwrap_or_default();
                            move |_| {
                                active_tab.set("bytecode");
                                if signatures.read().is_empty() && !bytecode_hex.is_empty() {
                                    let hex = bytecode_hex.clone();
                                    sigs_loading.set(true);
                                    wasm_bindgen_futures::spawn_local(async move {
                                        signatures.set(resolve_selectors(&hex).await);
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
                                div { class: "detail-val", span { "{contract.address}" } CopyButton { text: contract.address.clone() } }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "TEL Balance" }
                                div { class: "detail-val", span { style: "font-weight:600;", { format!("{:.6} TEL", contract.balance) } } }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Transactions" }
                                div { class: "detail-val", "{contract.tx_count}" }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Bytecode Size" }
                                div { class: "detail-val", { format!("{} bytes ({:.1} KB)", contract.bytecode_size, contract.bytecode_size as f64 / 1024.0) } }
                            }
                            div { class: "detail-row",
                                div { class: "detail-key", "Interfaces" }
                                div { class: "detail-val", style: "gap:6px; flex-wrap:wrap;",
                                    if contract.is_erc20  { span { class: "chip success", "ERC-20 Token" } }
                                    if contract.is_erc721 { span { class: "chip info",    "ERC-721 NFT" } }
                                    if contract.has_owner { span { class: "chip pending", "Ownable" } }
                                    if contract.has_pause { span { class: "chip pending", "Pausable" } }
                                    if contract.has_mint  { span { class: "chip pending", "Mintable" } }
                                    if !contract.is_erc20 && !contract.is_erc721 && !contract.has_owner && !contract.has_pause && !contract.has_mint {
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
                                    div { class: "detail-val", span { style: "font-weight:600;", "{contract.token_supply} {contract.token_symbol}" } }
                                }
                                div { class: "detail-row",
                                    div { class: "detail-key", "Token Page" }
                                    div { class: "detail-val",
                                        Link { to: Route::TokenPage { address: contract.address.clone() }, span { class: "action-link", "View Token Page →" } }
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

                    // ── ABI Upload (non-registry only) ────────────────
                    if !is_registry {
                        div { class: "detail-panel",
                            div { class: "detail-panel-title",
                                "ABI Upload"
                                span { style: "font-size:11px; color:var(--text-muted); font-weight:400; margin-left:8px;",
                                    "Paste ABI JSON to enable typed Read/Write functions"
                                }
                            }
                            div { style: "padding:16px 20px;",
                                if let Some((ok, msg)) = abi_msg.read().as_ref() {
                                    div {
                                        style: if *ok { "margin-bottom:12px; padding:8px 12px; background:rgba(34,197,94,0.08); border:1px solid rgba(34,197,94,0.2); border-radius:6px; font-size:12px; color:var(--accent-green);" }
                                               else   { "margin-bottom:12px; padding:8px 12px; background:rgba(239,68,68,0.08); border:1px solid rgba(239,68,68,0.2); border-radius:6px; font-size:12px; color:#ef4444;" },
                                        "{msg}"
                                    }
                                }
                                textarea {
                                    class: "abi-upload-textarea",
                                    placeholder: "Paste ABI JSON here — raw array or full artifact with abi key",
                                    value: "{abi_input}",
                                    oninput: move |e: Event<FormData>| {
                                        abi_input.set(e.value());
                                        abi_msg.set(None);
                                    }
                                }
                                div { style: "display:flex; gap:10px; margin-top:10px;",
                                    button {
                                        class: "contract-fn-btn contract-fn-btn-read",
                                        onclick: {
                                            let addr = address.clone();
                                            move |_| {
                                                let json_str = abi_input.read().trim().to_string();
                                                if json_str.is_empty() {
                                                    abi_msg.set(Some((false, "Please paste an ABI JSON first".to_string())));
                                                    return;
                                                }
                                                let parsed = parse_abi_json(&json_str);
                                                if parsed.is_empty() {
                                                    abi_msg.set(Some((false, "Could not parse ABI — check it is valid JSON".to_string())));
                                                    return;
                                                }
                                                let n = parsed.len();
                                                ls_save_abi(&addr, &json_str);
                                                uploaded_abi.set(parsed);
                                                abi_msg.set(Some((true, format!("{} functions loaded", n))));
                                                abi_input.set(String::new());
                                            }
                                        },
                                        "Load ABI"
                                    }
                                    if has_upload {
                                        button {
                                            class: "contract-fn-btn",
                                            style: "background:rgba(239,68,68,0.08); border:1px solid rgba(239,68,68,0.2); color:#ef4444;",
                                            onclick: {
                                                let addr = address.clone();
                                                move |_| {
                                                    ls_clear_abi(&addr);
                                                    uploaded_abi.set(vec![]);
                                                    abi_msg.set(Some((false, "ABI cleared".to_string())));
                                                }
                                            },
                                            "Clear ABI"
                                        }
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
                                    StaticFnCard { func: func.clone(), addr: address.clone(), is_write: false, fn_inputs, fn_results, fn_loading, fn_errors }
                                }
                            } else if has_upload {
                                for func in uploaded_abi.read().iter().filter(|f| f.mutability == "view" || f.mutability == "pure").cloned().collect::<Vec<_>>() {
                                    DynFnCard { func, addr: address.clone(), is_write: false, fn_results, fn_loading, fn_errors }
                                }
                            } else {
                                div { class: "info-note", style: "margin:16px 20px;",
                                    span { class: "info-note-icon", "ℹ" }
                                    div {
                                        p { style: "margin-bottom:6px;", "No ABI uploaded for this contract." }
                                        p { "Go to " strong { "Overview" } " and paste your ABI JSON, or visit " strong { "Bytecode" } " to load function signatures from 4byte.directory." }
                                    }
                                }
                                if !signatures.read().is_empty() {
                                    for sig in signatures.read().iter().filter(|s| {
                                        let n = s.signature.to_lowercase();
                                        !n.contains("transfer") && !n.contains("set") && !n.contains("mint") && !n.contains("burn")
                                    }).cloned().collect::<Vec<_>>() {
                                        GenericFnCard { sig, addr: address.clone(), is_write: false, fn_results, fn_loading, fn_errors }
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
                        div { class: "info-note", style: "margin:12px 20px;",
                            span { class: "info-note-icon", "ℹ" }
                            span { "Connect your wallet in the header before calling write functions." }
                        }
                        div { class: "contract-fn-list",
                            if is_registry {
                                for func in CONSENSUS_ABI.iter().filter(|f| f.mutability == "nonpayable" || f.mutability == "payable") {
                                    StaticFnCard { func: func.clone(), addr: address.clone(), is_write: true, fn_inputs, fn_results, fn_loading, fn_errors }
                                }
                            } else if has_upload {
                                for func in uploaded_abi.read().iter().filter(|f| f.mutability == "nonpayable" || f.mutability == "payable").cloned().collect::<Vec<_>>() {
                                    DynFnCard { func, addr: address.clone(), is_write: true, fn_results, fn_loading, fn_errors }
                                }
                            } else {
                                div { class: "info-note", style: "margin:16px 20px;",
                                    span { class: "info-note-icon", "ℹ" }
                                    div {
                                        p { style: "margin-bottom:6px;", "No ABI uploaded for this contract." }
                                        p { "Go to " strong { "Overview" } " and paste your ABI JSON to enable write functions." }
                                    }
                                }
                                if !signatures.read().is_empty() {
                                    for sig in signatures.read().iter().filter(|s| {
                                        let n = s.signature.to_lowercase();
                                        n.contains("transfer") || n.contains("set") || n.contains("mint") || n.contains("burn")
                                    }).cloned().collect::<Vec<_>>() {
                                        GenericFnCard { sig, addr: address.clone(), is_write: true, fn_results, fn_loading, fn_errors }
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
                                    thead { tr { th{"TX HASH"} th{"BLOCK"} th{"AGE"} th{"FROM"} th{""} th{"TO"} th{"AMOUNT"} } }
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
                            "Signatures from "
                            a { href: "https://www.4byte.directory", target: "_blank", class: "hash-cell", "4byte.directory" }
                            ". Decompile: "
                            a { href: format!("https://app.dedaub.com/decompile?md5={}", &contract.address), target: "_blank", class: "hash-cell", "Dedaub ↗" }
                            " / "
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
                            div { class: "bytecode-notice", span { "No signatures resolved yet." } }
                        } else {
                            div { class: "sig-table",
                                div { class: "sig-header", span{"SELECTOR"} span{"FUNCTION SIGNATURE"} span{"TYPE"} }
                                for sig in signatures.read().iter() {
                                    div { class: "sig-row",
                                        span { class: "sig-selector", "0x{sig.selector}" }
                                        span { class: "sig-name code-inline", "{sig.signature}" }
                                        span { class: "sig-type",
                                            if sig.signature.contains("transfer") || sig.signature.contains("approve") || sig.signature.contains("mint") || sig.signature.contains("set") {
                                                span { class: "chip pending", style: "font-size:10px; padding:1px 6px;", "Write" }
                                            } else {
                                                span { class: "chip info", style: "font-size:10px; padding:1px 6px;", "Read" }
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
                        div { style: "padding:16px 20px;",
                            div { class: "bytecode-actions", style: "margin-bottom:10px;",
                                CopyButton { text: format!("0x{}", contract.bytecode_hex.clone()) }
                                span { style: "font-size:11px; color:var(--text-muted);", "Copy full bytecode" }
                            }
                            div { class: "bytecode-full-box", "0x{contract.bytecode_hex}" }
                        }
                    }
                }
            }
        }
    }
}

// ── StaticFnCard — ConsensusRegistry typed ABI ────────────────────────────

#[component]
fn StaticFnCard(
    func: AbiFunction,
    addr: String,
    is_write: bool,
    mut fn_inputs:  Signal<std::collections::HashMap<String, Vec<String>>>,
    mut fn_results: Signal<std::collections::HashMap<String, String>>,
    mut fn_loading: Signal<std::collections::HashMap<String, bool>>,
    mut fn_errors:  Signal<std::collections::HashMap<String, String>>,
) -> Element {
    let key = func.name.to_string();
    let n   = func.inputs.len();
    { fn_inputs.write().entry(key.clone()).or_insert_with(|| vec![String::new(); n]); }
    let result  = fn_results.read().get(&key).cloned();
    let loading = fn_loading.read().get(&key).copied().unwrap_or(false);
    let err     = fn_errors.read().get(&key).cloned();

    rsx! {
        div { class: "contract-fn-card",
            div { class: "contract-fn-header",
                span { class: "contract-fn-name", "{func.name}" }
                if func.mutability == "payable" { span { class: "chip pending", style: "font-size:10px;", "payable" } }
                if !func.output_desc.is_empty() { span { class: "contract-fn-returns", "→ {func.output_desc}" } }
            }
            for (i, param) in func.inputs.iter().enumerate() {
                div { class: "contract-fn-input-row",
                    label { class: "contract-fn-label",
                        span { class: "contract-fn-param-name", "{param.name}" }
                        span { class: "contract-fn-param-type", " ({param.ty})" }
                    }
                    input {
                        class: "contract-fn-input",
                        placeholder: "{param.ty}",
                        value: { fn_inputs.read().get(&key).and_then(|v| v.get(i)).cloned().unwrap_or_default() },
                        oninput: {
                            let key = key.clone();
                            move |e: Event<FormData>| {
                                if let Some(vals) = fn_inputs.write().get_mut(&key) {
                                    if i < vals.len() { vals[i] = e.value(); }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "contract-fn-actions",
                button {
                    class: if is_write { "contract-fn-btn contract-fn-btn-write" } else { "contract-fn-btn contract-fn-btn-read" },
                    disabled: loading,
                    onclick: {
                        let key   = key.clone();
                        let sel   = func.selector.to_string();
                        let idefs: Vec<(String,String)> = func.inputs.iter().map(|p| (p.name.to_string(), p.ty.to_string())).collect();
                        let out   = func.output_desc.to_string();
                        let addr  = addr.clone();
                        let iw    = is_write;
                        move |_| {
                            let key   = key.clone();
                            let sel   = sel.clone();
                            let idefs = idefs.clone();
                            let out   = out.clone();
                            let addr  = addr.clone();
                            let vals  = fn_inputs.read().get(&key).cloned().unwrap_or_default();
                            fn_loading.write().insert(key.clone(), true);
                            fn_errors.write().remove(&key);
                            fn_results.write().remove(&key);
                            wasm_bindgen_futures::spawn_local(async move {
                                let mut data = sel.clone();
                                let mut enc_err: Option<String> = None;
                                for (i, (_, ty)) in idefs.iter().enumerate() {
                                    match abi_encode_param(ty, vals.get(i).map(|s| s.as_str()).unwrap_or("")) {
                                        Ok(e) => data.push_str(&e),
                                        Err(e) => { enc_err = Some(e); break; }
                                    }
                                }
                                if let Some(e) = enc_err {
                                    fn_errors.write().insert(key.clone(), e);
                                    fn_loading.write().insert(key.clone(), false);
                                    return;
                                }
                                let calldata = ["0x", &data].concat();
                                if !iw {
                                    let params = serde_json::json!([{"to": &addr, "data": &calldata}, "latest"]);
                                    match rpc_call::<_, String>("eth_call", params).await {
                                        Ok(hex) => { fn_results.write().insert(key.clone(), decode_result(&hex, &out)); }
                                        Err(e)  => { fn_errors.write().insert(key.clone(), e); }
                                    }
                                } else {
                                    let js = js_send_tx(&addr, &calldata);
                                    let (res, err) = exec_js_promise(&js).await;
                                    if let Some(r) = res { fn_results.write().insert(key.clone(), format!("Tx: {}", r)); }
                                    if let Some(e) = err { fn_errors.write().insert(key.clone(), e); }
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

// ── DynFnCard — uploaded ABI ──────────────────────────────────────────────

#[component]
fn DynFnCard(
    func: DynFunction,
    addr: String,
    is_write: bool,
    mut fn_results: Signal<std::collections::HashMap<String, String>>,
    mut fn_loading: Signal<std::collections::HashMap<String, bool>>,
    mut fn_errors:  Signal<std::collections::HashMap<String, String>>,
) -> Element {
    let key     = func.selector.clone();
    let n       = func.inputs.len();
    let mut vals: Signal<Vec<String>> = use_signal(|| vec![String::new(); n]);
    let result  = fn_results.read().get(&key).cloned();
    let loading = fn_loading.read().get(&key).copied().unwrap_or(false);
    let err     = fn_errors.read().get(&key).cloned();

    rsx! {
        div { class: "contract-fn-card",
            div { class: "contract-fn-header",
                span { class: "contract-fn-name", "{func.name}" }
                if func.mutability == "payable" { span { class: "chip pending", style: "font-size:10px;", "payable" } }
                if !func.output_desc.is_empty() { span { class: "contract-fn-returns", "→ {func.output_desc}" } }
            }
            for (i, param) in func.inputs.iter().enumerate() {
                div { class: "contract-fn-input-row",
                    label { class: "contract-fn-label",
                        span { class: "contract-fn-param-name", "{param.name}" }
                        span { class: "contract-fn-param-type", " ({param.ty})" }
                    }
                    input {
                        class: "contract-fn-input",
                        placeholder: "{param.ty}",
                        value: { vals.read().get(i).cloned().unwrap_or_default() },
                        oninput: move |e: Event<FormData>| {
                            let mut v = vals.write();
                            if i < v.len() { v[i] = e.value(); }
                        }
                    }
                }
            }
            div { class: "contract-fn-actions",
                button {
                    class: if is_write { "contract-fn-btn contract-fn-btn-write" } else { "contract-fn-btn contract-fn-btn-read" },
                    disabled: loading,
                    onclick: {
                        let key    = key.clone();
                        let sel    = func.selector.clone();
                        let inputs = func.inputs.clone();
                        let out    = func.output_desc.clone();
                        let addr   = addr.clone();
                        let iw     = is_write;
                        move |_| {
                            let key    = key.clone();
                            let sel    = sel.clone();
                            let inputs = inputs.clone();
                            let out    = out.clone();
                            let addr   = addr.clone();
                            let cur    = vals.read().clone();
                            fn_loading.write().insert(key.clone(), true);
                            fn_errors.write().remove(&key);
                            fn_results.write().remove(&key);
                            wasm_bindgen_futures::spawn_local(async move {
                                let mut data = sel.clone();
                                let mut enc_err: Option<String> = None;
                                for (i, p) in inputs.iter().enumerate() {
                                    match abi_encode_param(&p.ty, cur.get(i).map(|s| s.as_str()).unwrap_or("")) {
                                        Ok(e) => data.push_str(&e),
                                        Err(e) => { enc_err = Some(e); break; }
                                    }
                                }
                                if let Some(e) = enc_err {
                                    fn_errors.write().insert(key.clone(), e);
                                    fn_loading.write().insert(key.clone(), false);
                                    return;
                                }
                                let calldata = ["0x", &data].concat();
                                if !iw {
                                    let params = serde_json::json!([{"to": &addr, "data": &calldata}, "latest"]);
                                    match rpc_call::<_, String>("eth_call", params).await {
                                        Ok(hex) => { fn_results.write().insert(key.clone(), decode_result(&hex, &out)); }
                                        Err(e)  => { fn_errors.write().insert(key.clone(), e); }
                                    }
                                } else {
                                    let js = js_send_tx(&addr, &calldata);
                                    let (res, err) = exec_js_promise(&js).await;
                                    if let Some(r) = res { fn_results.write().insert(key.clone(), format!("Tx: {}", r)); }
                                    if let Some(e) = err { fn_errors.write().insert(key.clone(), e); }
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

// ── GenericFnCard — 4byte signatures, raw hex input ───────────────────────

#[component]
fn GenericFnCard(
    sig: FunctionSignature,
    addr: String,
    is_write: bool,
    mut fn_results: Signal<std::collections::HashMap<String, String>>,
    mut fn_loading: Signal<std::collections::HashMap<String, bool>>,
    mut fn_errors:  Signal<std::collections::HashMap<String, String>>,
) -> Element {
    let key     = sig.selector.clone();
    let loading = fn_loading.read().get(&key).copied().unwrap_or(false);
    let result  = fn_results.read().get(&key).cloned();
    let err     = fn_errors.read().get(&key).cloned();
    let mut raw = use_signal(|| String::new());

    rsx! {
        div { class: "contract-fn-card",
            div { class: "contract-fn-header",
                span { class: "contract-fn-name", "{sig.signature}" }
                span { style: "margin-left:8px; font-size:11px; color:var(--text-muted);", "0x{sig.selector}" }
            }
            div { class: "contract-fn-input-row",
                label { class: "contract-fn-label", "Calldata params (hex after selector, leave empty for no-arg)" }
                input {
                    class: "contract-fn-input",
                    placeholder: "000000000000000000000000abcd…",
                    value: "{raw}",
                    oninput: move |e: Event<FormData>| raw.set(e.value()),
                }
            }
            div { class: "contract-fn-actions",
                button {
                    class: if is_write { "contract-fn-btn contract-fn-btn-write" } else { "contract-fn-btn contract-fn-btn-read" },
                    disabled: loading,
                    onclick: {
                        let key  = key.clone();
                        let sel  = sig.selector.clone();
                        let addr = addr.clone();
                        let iw   = is_write;
                        move |_| {
                            let key  = key.clone();
                            let sel  = sel.clone();
                            let addr = addr.clone();
                            let p    = raw.read().trim().to_string();
                            let calldata = if p.is_empty() {
                                ["0x", &sel].concat()
                            } else {
                                ["0x", &sel, p.trim_start_matches("0x")].concat()
                            };
                            fn_loading.write().insert(key.clone(), true);
                            fn_errors.write().remove(&key);
                            fn_results.write().remove(&key);
                            wasm_bindgen_futures::spawn_local(async move {
                                if !iw {
                                    let params = serde_json::json!([{"to": &addr, "data": &calldata}, "latest"]);
                                    match rpc_call::<_, String>("eth_call", params).await {
                                        Ok(hex) => { fn_results.write().insert(key.clone(), hex); }
                                        Err(e)  => { fn_errors.write().insert(key.clone(), e); }
                                    }
                                } else {
                                    let js = js_send_tx(&addr, &calldata);
                                    let (res, err) = exec_js_promise(&js).await;
                                    if let Some(r) = res { fn_results.write().insert(key.clone(), format!("Tx: {}", r)); }
                                    if let Some(e) = err { fn_errors.write().insert(key.clone(), e); }
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
