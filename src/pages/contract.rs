// src/pages/contract.rs — v0.2.0
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    resolve_selectors, FunctionSignature,
    get_contract_info, get_address_transfers, contract_call, ContractInfo,
    TokenTransfer, shorten_hash, shorten_addr, unix_to_age, format_wei_exact, format_amount,
    CONSENSUS_REGISTRY, INDEXER_URL,
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
    AbiFunction { name: "getCurrentEpoch",        selector: "b97dd9e2", mutability: "view",        inputs: &[], output_desc: "uint32 epoch" },
    AbiFunction { name: "getCurrentEpochInfo",    selector: "babc394f", mutability: "view",        inputs: &[], output_desc: "tuple (committee[], epochIssuance, blockHeight, epochId, epochDuration, stakeVersion)" },
    AbiFunction { name: "getCurrentStakeVersion", selector: "536343d2", mutability: "view",        inputs: &[], output_desc: "uint8 version" },
    AbiFunction { name: "getNextCommitteeSize",   selector: "a06f8dcb", mutability: "view",        inputs: &[], output_desc: "uint16 size" },
    AbiFunction { name: "owner",                  selector: "8da5cb5b", mutability: "view",        inputs: &[], output_desc: "address" },
    AbiFunction { name: "paused",                 selector: "5c975abb", mutability: "view",        inputs: &[], output_desc: "bool" },
    AbiFunction { name: "totalSupply",            selector: "18160ddd", mutability: "view",        inputs: &[], output_desc: "uint256" },
    AbiFunction { name: "undistributedIssuance",  selector: "008d6acd", mutability: "view",        inputs: &[], output_desc: "uint256" },
    AbiFunction { name: "issuance",               selector: "863623bb", mutability: "view",        inputs: &[], output_desc: "address" },
    AbiFunction { name: "SYSTEM_ADDRESS",         selector: "3434735f", mutability: "view",        inputs: &[], output_desc: "address" },
    AbiFunction { name: "getValidator",           selector: "1904bb2e", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "tuple (validatorAddress, activationEpoch, exitEpoch, currentStatus, isRetired, stakeVersion, region)" },
    AbiFunction { name: "getValidators",          selector: "8cc05eda", mutability: "view",
        inputs: &[AbiParam { name: "status (0=Staked,1=PendingActivation,2=Active,3=PendingExit,4=Exited,5=Retired)", ty: "uint8" }],
        output_desc: "tuple[] validators" },
    AbiFunction { name: "getCommitteeValidators", selector: "a0201be5", mutability: "view",
        inputs: &[AbiParam { name: "epoch", ty: "uint32" }], output_desc: "tuple[] validators" },
    AbiFunction { name: "getCommitteeBlsPubkeys", selector: "6ae9942c", mutability: "view",
        inputs: &[AbiParam { name: "epoch", ty: "uint32" }], output_desc: "bytes[] blsPubkeys" },
    AbiFunction { name: "getEpochInfo",           selector: "374ed18c", mutability: "view",
        inputs: &[AbiParam { name: "epoch", ty: "uint32" }],
        output_desc: "tuple (committee[], epochIssuance, blockHeight, epochId, epochDuration, stakeVersion)" },
    AbiFunction { name: "getRewards",             selector: "79ee54f7", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "uint256 rewards (wei)" },
    AbiFunction { name: "getBalanceBreakdown",    selector: "15b5709a", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "uint256 staked, uint256 rewards, uint256 delegated" },
    AbiFunction { name: "isDelegated",            selector: "3e28391d", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "bool" },
    AbiFunction { name: "isRetired",              selector: "6d3c6275", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "bool" },
    AbiFunction { name: "isValidator",            selector: "6a926591", mutability: "view",
        inputs: &[AbiParam { name: "blsPubkey (hex)", ty: "bytes" }], output_desc: "bool" },
    AbiFunction { name: "balanceOf",              selector: "70a08231", mutability: "view",
        inputs: &[AbiParam { name: "owner", ty: "address" }], output_desc: "uint256" },
    AbiFunction { name: "getBlsPubkey",           selector: "059b916c", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "bytes blsPubkey" },
    AbiFunction { name: "validators",             selector: "fa52c7d8", mutability: "view",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }],
        output_desc: "address, uint32 activationEpoch, uint32 exitEpoch, uint8 status, bool isRetired, uint8 stakeVersion, uint8 region" },
    AbiFunction { name: "activate",              selector: "0f15f4c0", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "beginExit",             selector: "2228dfa0", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "pause",                 selector: "8456cb59", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "unpause",               selector: "3f4ba83a", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "renounceOwnership",     selector: "715018a6", mutability: "nonpayable", inputs: &[], output_desc: "" },
    AbiFunction { name: "unstake",               selector: "f2888dbb", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "" },
    AbiFunction { name: "claimStakeRewards",     selector: "2b9f4985", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "" },
    AbiFunction { name: "mint",                  selector: "6a627842", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "" },
    AbiFunction { name: "burn",                  selector: "89afcb44", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }], output_desc: "" },
    AbiFunction { name: "setNextCommitteeSize",  selector: "62fcde4b", mutability: "nonpayable",
        inputs: &[AbiParam { name: "newSize", ty: "uint16" }], output_desc: "" },
    AbiFunction { name: "setValidatorRegion",    selector: "f0624768", mutability: "nonpayable",
        inputs: &[AbiParam { name: "validatorAddress", ty: "address" }, AbiParam { name: "region (0-255)", ty: "uint8" }], output_desc: "" },
    AbiFunction { name: "transferOwnership",     selector: "f2fde38b", mutability: "nonpayable",
        inputs: &[AbiParam { name: "newOwner", ty: "address" }], output_desc: "" },
    AbiFunction { name: "allocateIssuance",      selector: "355811a7", mutability: "payable", inputs: &[], output_desc: "" },
];

// ── ABI encoding ──────────────────────────────────────────────────────────
fn is_dynamic_type(ty: &str) -> bool {
    ty == "string" || ty == "bytes" || ty.ends_with("[]")
}
fn abi_encode_static(ty: &str, value: &str) -> Result<String, String> {
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
fn abi_encode_dynamic(ty: &str, value: &str) -> Result<String, String> {
    let v = value.trim();
    match ty {
        "string" => {
            let bytes = v.as_bytes();
            let len = bytes.len();
            let len_hex = format!("{:0>64x}", len);
            let data_hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
            let pad = if len % 32 == 0 { 0 } else { 32 - (len % 32) };
            Ok(format!("{}{}{}", len_hex, data_hex, "00".repeat(pad)))
        }
        "bytes" => {
            let hex = v.trim_start_matches("0x");
            let len = hex.len() / 2;
            let len_hex = format!("{:0>64x}", len);
            let pad = if len % 32 == 0 { 0 } else { 32 - (len % 32) };
            Ok(format!("{}{}{}", len_hex, hex, "00".repeat(pad)))
        }
        _ => Err(format!("Unsupported dynamic type: {}", ty))
    }
}
fn build_calldata(selector: &str, types: &[String], values: &[String]) -> Result<String, String> {
    let n = types.len();
    let mut heads: Vec<String> = Vec::new();
    let mut tails: Vec<String> = Vec::new();
    let base_offset = 32 * n;
    let mut dyn_offset = base_offset;
    for (i, ty) in types.iter().enumerate() {
        let val = values.get(i).map(|s| s.as_str()).unwrap_or("");
        if is_dynamic_type(ty) {
            heads.push(format!("{:0>64x}", dyn_offset));
            let tail = abi_encode_dynamic(ty, val)?;
            dyn_offset += tail.len() / 2;
            tails.push(tail);
        } else {
            heads.push(abi_encode_static(ty, val)?);
        }
    }
    let mut result = format!("0x{}", selector);
    for h in &heads { result.push_str(h); }
    for t in &tails { result.push_str(t); }
    Ok(result)
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
    if raw.len() >= 128 {
        if let Ok(len) = u64::from_str_radix(raw.get(64..128).unwrap_or("0").trim_start_matches('0').get(0..).unwrap_or("0"), 16) {
            let byte_len = (len as usize) * 2;
            if let Some(data_hex) = raw.get(128..128+byte_len) {
                let bytes: Vec<u8> = (0..data_hex.len()).step_by(2)
                    .filter_map(|i| u8::from_str_radix(&data_hex[i..i+2], 16).ok())
                    .collect();
                if let Ok(s) = std::str::from_utf8(&bytes) {
                    if s.chars().all(|c| !c.is_control() || c == '\n') {
                        return s.to_string();
                    }
                }
            }
        }
    }
    if raw.len() > 80 { format!("0x{}…", &raw[..80]) } else { format!("0x{}", raw) }
}
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
fn ls_save_abi(address: &str, json_str: &str) {
    let key = ["abi_", &address.to_lowercase()].concat();
    if let Ok(escaped) = serde_json::to_string(json_str) {
        let js = ["localStorage.setItem(", &serde_json::to_string(&key).unwrap_or_default(), ",", &escaped, ")"].concat();
        let _ = js_sys::eval(&js);
    }
}
fn ls_load_abi(address: &str) -> String {
    let key = ["abi_", &address.to_lowercase()].concat();
    let js = ["(localStorage.getItem(", &serde_json::to_string(&key).unwrap_or_default(), ")||'')"].concat();
    js_sys::eval(&js).ok().and_then(|v| v.as_string()).unwrap_or_default()
}
fn ls_clear_abi(address: &str) {
    let key = ["abi_", &address.to_lowercase()].concat();
    let js = ["localStorage.removeItem(", &serde_json::to_string(&key).unwrap_or_default(), ")"].concat();
    let _ = js_sys::eval(&js);
}
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
        let sel_key = serde_json::to_string(&sig).unwrap_or_default();
        let keccak_js = ["window.keccak256?window.keccak256(", &sel_key, ").slice(0,8):'00000000'"].concat();
        let selector = js_sys::eval(&keccak_js)
            .ok().and_then(|v| v.as_string())
            .unwrap_or_else(|| "00000000".to_string());
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
async fn exec_js_promise(js: &str) -> (Option<String>, Option<String>) {
    match js_sys::eval(js) {
        Ok(pv) => {
            match wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(pv)).await {
                Ok(r) => {
                    let obj = js_sys::Object::from(r);
                    let ek = wasm_bindgen::JsValue::from_str("error");
                    let rk = wasm_bindgen::JsValue::from_str("result");
                    let hk = wasm_bindgen::JsValue::from_str("hash");
                    let err = js_sys::Reflect::get(&obj, &ek).ok().and_then(|v| v.as_string());
                    let res = js_sys::Reflect::get(&obj, &rk).ok().and_then(|v| v.as_string())
                        .or_else(|| js_sys::Reflect::get(&obj, &hk).ok().and_then(|v| v.as_string()));
                    (res, err)
                }
                Err(e) => (None, Some(format!("{:?}", e)))
            }
        }
        Err(_) => (None, Some("Failed to execute JS".to_string()))
    }
}
/// Build the JS that renders a DynFnCard entirely in vanilla JS.
/// Reads now go through the indexer's POST /call (`call_url`), replacing the
/// old direct eth_call against the public RPC WebSocket.
fn build_dyncard_js(
    container_id: &str,
    func: &DynFunction,
    addr: &str,
    is_write: bool,
    all_funcs: &[DynFunction],
    call_url: &str,
) -> String {
    let inputs_json = serde_json::to_string(
        &func.inputs.iter().map(|p| serde_json::json!({"name": p.name, "ty": p.ty})).collect::<Vec<_>>()
    ).unwrap_or_else(|_| "[]".to_string());
    let reader_map: std::collections::HashMap<String, serde_json::Value> = all_funcs.iter()
        .filter(|f| (f.mutability == "view" || f.mutability == "pure") && f.inputs.is_empty())
        .map(|f| (f.name.to_lowercase(), serde_json::json!({"sel": f.selector, "out": f.output_desc})))
        .collect();
    let readers_json = serde_json::to_string(&reader_map).unwrap_or_else(|_| "{}".to_string());
    let is_write_js = if is_write { "true" } else { "false" };
    let sel = &func.selector;
    let fn_name = &func.name;
    let output_desc = &func.output_desc;
    [
        r#"(function(){
var cid="#, &serde_json::to_string(container_id).unwrap_or_default(), r#";
var el=document.getElementById(cid);
if(!el)return;
var fnName="#, &serde_json::to_string(fn_name).unwrap_or_default(), r#";
var sel="#, &serde_json::to_string(sel).unwrap_or_default(), r#";
var inputs="#, &inputs_json, r#";
var readers="#, &readers_json, r#";
var addr="#, &serde_json::to_string(addr).unwrap_or_default(), r#";
var callUrl="#, &serde_json::to_string(call_url).unwrap_or_default(), r#";
var isWrite="#, is_write_js, r#";
var outDesc="#, &serde_json::to_string(output_desc).unwrap_or_default(), r#";
// Build input fields HTML
var fieldsHtml='';
inputs.forEach(function(p,i){
  fieldsHtml+='<div class="contract-fn-input-row">';
  fieldsHtml+='<label class="contract-fn-label"><span class="contract-fn-param-name">'+p.name+'</span><span class="contract-fn-param-type"> ('+p.ty+')</span></label>';
  fieldsHtml+='<input class="contract-fn-input" id="'+cid+'-'+i+'" placeholder="'+p.ty+'">';
  fieldsHtml+='</div>';
});
var btnClass=isWrite?'contract-fn-btn contract-fn-btn-write':'contract-fn-btn contract-fn-btn-read';
var btnLabel=isWrite?'Send Transaction':'Query';
var prefillBtn=isWrite&&inputs.length>0?'<button class="contract-fn-btn contract-fn-btn-read" id="'+cid+'-prefill">↺ Pre-fill current values</button>':'';
el.innerHTML=
  '<div class="contract-fn-header">'+
  '<span class="contract-fn-name">'+fnName+'</span>'+
  (outDesc?'<span class="contract-fn-returns">→ '+outDesc+'</span>':'')+
  '</div>'+
  fieldsHtml+
  '<div style="margin-top:8px;display:flex;gap:8px;flex-wrap:wrap;">'+
  prefillBtn+
  '<button class="'+btnClass+'" id="'+cid+'-btn">'+btnLabel+'</button>'+
  '</div>'+
  '<div id="'+cid+'-result" style="display:none"></div>'+
  '<div id="'+cid+'-error" style="display:none"></div>';
// ABI encode helpers
function encodeStatic(ty,v){
  v=(v||'').trim();
  if(ty==='address'){var a=v.replace('0x','').toLowerCase();if(a.length!==40)throw'Invalid address: '+v;return a.padStart(64,'0');}
  if(/^u?int/.test(ty)){var n=BigInt(v);return n.toString(16).padStart(64,'0');}
  if(ty==='bool'){return(v==='true'||v==='1'?1:0).toString(16).padStart(64,'0');}
  return v.replace('0x','').padEnd(64,'0');
}
function encodeDynamic(ty,v){
  v=(v||'').trim();
  if(ty==='string'||ty==='bytes'){
    var bytes=new TextEncoder().encode(v);
    var len=bytes.length.toString(16).padStart(64,'0');
    var hex=Array.from(bytes).map(function(b){return b.toString(16).padStart(2,'0')}).join('');
    var pad=bytes.length%32===0?0:32-(bytes.length%32);
    return len+hex+'00'.repeat(pad);
  }
  throw'Unsupported dynamic type: '+ty;
}
function isDynamic(ty){return ty==='string'||ty==='bytes'||ty.endsWith('[]');}
function buildCalldata(sel,inputs,values){
  var n=inputs.length;
  var heads=[];var tails=[];var dynOffset=32*n;
  inputs.forEach(function(p,i){
    var ty=p.ty;var v=values[i]||'';
    if(isDynamic(ty)){heads.push(dynOffset.toString(16).padStart(64,'0'));var t=encodeDynamic(ty,v);dynOffset+=t.length/2;tails.push(t);}
    else{heads.push(encodeStatic(ty,v));}
  });
  return'0x'+sel+heads.join('')+tails.join('');
}
// Decode result
function decodeResult(hex,outDesc){
  var raw=(hex||'').replace('0x','');
  if(!raw||raw==='0')return'(empty)';
  if(outDesc&&outDesc.includes('address')&&raw.length>=64)return'0x'+raw.slice(-40);
  if(outDesc==='bool'&&raw.length>=64)return parseInt(raw.slice(-1),16)===1?'true':'false';
  if(outDesc&&(outDesc.includes('uint')||outDesc.includes('int'))&&!outDesc.includes('tuple')&&raw.length<=64){
    try{return BigInt('0x'+raw.replace(/^0+/,'')||'0').toString();}catch(e){}
  }
  if(raw.length>=128){
    try{
      var len=parseInt(raw.slice(64,128),16);
      var data=raw.slice(128,128+len*2);
      var bytes=[];
      for(var i=0;i<data.length;i+=2)bytes.push(parseInt(data.slice(i,i+2),16));
      var s=new TextDecoder().decode(new Uint8Array(bytes));
      if(s&&s.split('').every(function(c){var code=c.charCodeAt(0);return code>=32||code===10;}))return s;
    }catch(e){}
  }
  return raw.length>80?'0x'+raw.slice(0,80)+'…':'0x'+raw;
}
// Show result/error
function showResult(text){
  var r=document.getElementById(cid+'-result');
  if(r){r.className='contract-fn-result';r.innerHTML='<span class="contract-fn-result-label">Result:</span> <span class="contract-fn-result-value">'+text+'</span>';r.style.display='';}
}
function showError(text){
  var e=document.getElementById(cid+'-error');
  if(e){e.className='contract-fn-error';e.textContent='⚠ '+text;e.style.display='';}
}
function clearOutput(){
  var r=document.getElementById(cid+'-result');var e=document.getElementById(cid+'-error');
  if(r)r.style.display='none';if(e)e.style.display='none';
}
// Prefill button — reads via indexer POST /call: {to,data} -> {ok,result,error}
var prefillEl=document.getElementById(cid+'-prefill');
if(prefillEl){
  prefillEl.addEventListener('click',function(){
    prefillEl.textContent='Loading…';prefillEl.disabled=true;
    var pending=inputs.length;
    if(pending===0){prefillEl.textContent='Pre-fill current values';prefillEl.disabled=false;return;}
    var done=0;
    inputs.forEach(function(p,i){
      var bare=p.name.replace(/^_/,'').toLowerCase();
      var reader=readers[bare];
      if(!reader){done++;if(done>=pending){prefillEl.textContent='Pre-fill current values';prefillEl.disabled=false;}return;}
      fetch(callUrl,{method:'POST',headers:{'Content-Type':'application/json'},
        body:JSON.stringify({to:addr,data:'0x'+reader.sel})
      }).then(function(r){return r.json();}).then(function(d){
        var val=decodeResult(d.result,reader.out);
        var inp=document.getElementById(cid+'-'+i);
        if(inp)inp.value=val;
        done++;if(done>=pending){prefillEl.textContent='Pre-fill current values';prefillEl.disabled=false;}
      }).catch(function(){done++;if(done>=pending){prefillEl.textContent='Pre-fill current values';prefillEl.disabled=false;}});
    });
  });
}
// Submit button
var btn=document.getElementById(cid+'-btn');
if(btn){
  btn.addEventListener('click',function(){
    clearOutput();
    var values=inputs.map(function(_,i){var inp=document.getElementById(cid+'-'+i);return inp?inp.value:'';});
    var calldata;
    try{calldata=buildCalldata(sel,inputs,values);}
    catch(e){showError(e.toString());return;}
    btn.disabled=true;btn.textContent=isWrite?'Sending…':'Querying…';
    if(!isWrite){
      fetch(callUrl,{method:'POST',headers:{'Content-Type':'application/json'},
        body:JSON.stringify({to:addr,data:calldata})
      }).then(function(r){return r.json();}).then(function(d){
        if(!d.ok)showError(d.error||'call reverted');
        else showResult(decodeResult(d.result,outDesc));
      }).catch(function(e){showError(e.toString());})
      .finally(function(){btn.disabled=false;btn.textContent=btnLabel;});
    } else {
      if(!window.ethereum){showError('No wallet connected');btn.disabled=false;btn.textContent=btnLabel;return;}
      window.ethereum.request({method:'eth_accounts'}).then(function(accounts){
        if(!accounts||!accounts.length){showError('No wallet connected');btn.disabled=false;btn.textContent=btnLabel;return;}
        return window.ethereum.request({method:'eth_sendTransaction',params:[{from:accounts[0],to:addr,data:calldata,gas:'0x30000'}]});
      }).then(function(hash){
        if(hash)showResult('Tx: '+hash);
      }).catch(function(e){showError(e.message||'Transaction failed');})
      .finally(function(){btn.disabled=false;btn.textContent=btnLabel;});
    }
  });
}
})();"#
    ].concat()
}

// ── Page component ────────────────────────────────────────────────────────
#[component]
pub fn ContractPage(address: String) -> Element {
    let mut info: Signal<Option<ContractInfo>>         = use_signal(|| None);
    let mut transfers: Signal<Vec<TokenTransfer>>      = use_signal(|| vec![]);
    let mut transfers_total: Signal<u64>               = use_signal(|| 0);
    let mut loading                                    = use_signal(|| true);
    let mut error: Signal<Option<String>>              = use_signal(|| None);
    let mut signatures: Signal<Vec<FunctionSignature>> = use_signal(|| vec![]);
    let mut sigs_loading                               = use_signal(|| false);
    let mut active_tab                                 = use_signal(|| "overview");
    let mut uploaded_abi: Signal<Vec<DynFunction>>     = use_signal(|| vec![]);
    let mut abi_input                                  = use_signal(|| String::new());
    let mut abi_msg: Signal<Option<(bool, String)>>    = use_signal(|| None);
    let mut fn_inputs:  Signal<std::collections::HashMap<String, Vec<String>>> = use_signal(|| std::collections::HashMap::new());
    let mut fn_results: Signal<std::collections::HashMap<String, String>>      = use_signal(|| std::collections::HashMap::new());
    let mut fn_loading: Signal<std::collections::HashMap<String, bool>>        = use_signal(|| std::collections::HashMap::new());
    let mut fn_errors:  Signal<std::collections::HashMap<String, String>>      = use_signal(|| std::collections::HashMap::new());
    let mut transfers_page: Signal<u64> = use_signal(|| 0);
    let mut transfers_more_loading      = use_signal(|| false);

    let addr_clone = address.clone();
    use_effect(move || {
        let address = addr_clone.clone();
        let saved_json = ls_load_abi(&address);
        if !saved_json.is_empty() {
            wasm_bindgen_futures::spawn_local(async move {
                for _ in 0..20 {
                    let ready = js_sys::eval("typeof keccak256 === 'function'")
                        .ok().and_then(|v| v.as_bool()).unwrap_or(false);
                    if ready { break; }
                    gloo_timers::future::TimeoutFuture::new(100).await;
                }
                let parsed = parse_abi_json(&saved_json);
                if !parsed.is_empty() {
                    let n = parsed.len();
                    uploaded_abi.set(parsed);
                    abi_msg.set(Some((true, format!("{} functions loaded from saved ABI", n))));
                }
            });
        }
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            match get_contract_info(&address).await {
                Ok(i)  => info.set(Some(i)),
                Err(e) => { error.set(Some(e)); loading.set(false); return; }
            }
            // Full transfer history where this contract is sender/recipient of an ERC-20 transfer.
            if let Ok((xfers, total)) = get_address_transfers(&address, 0, 25).await {
                transfers.set(xfers);
                transfers_total.set(total);
            }
            loading.set(false);
        });
    });

    let load_more_transfers = {
        let address = address.clone();
        move |_| {
            let address = address.clone();
            wasm_bindgen_futures::spawn_local(async move {
                transfers_more_loading.set(true);
                let next = *transfers_page.read() + 1;
                if let Ok((mut more, _)) = get_address_transfers(&address, next, 25).await {
                    transfers.write().append(&mut more);
                    transfers_page.set(next);
                }
                transfers_more_loading.set(false);
            });
        }
    };

    let is_registry = address.to_lowercase() == CONSENSUS_REGISTRY.to_lowercase();
    let avatar_char = address.chars().nth(2).unwrap_or('C').to_uppercase().next().unwrap_or('C');
    let has_upload  = !uploaded_abi.read().is_empty();

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
                div { class: "tabs-row",
                    button { class: if *active_tab.read() == "overview"  { "tab-btn tab-active" } else { "tab-btn" }, onclick: move |_| active_tab.set("overview"),  "Overview" }
                    button { class: if *active_tab.read() == "read"      { "tab-btn tab-active" } else { "tab-btn" }, onclick: move |_| active_tab.set("read"),      "Read Contract" }
                    button { class: if *active_tab.read() == "write"     { "tab-btn tab-active" } else { "tab-btn" }, onclick: move |_| active_tab.set("write"),     "Write Contract" }
                    button { class: if *active_tab.read() == "transfers" { "tab-btn tab-active" } else { "tab-btn" }, onclick: move |_| active_tab.set("transfers"),
                        "Transfers"
                        span { class: "tab-count", "({transfers_total.read()})" }
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
                                div { class: "detail-val", span { style: "font-weight:600;", { format!("{} TEL", format_wei_exact(contract.balance_wei.parse().unwrap_or(0))) } } }
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
                                    DynFnCard { func: func.clone(), addr: address.clone(), is_write: false, all_funcs: uploaded_abi.read().clone() }
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
                                    DynFnCard { func: func.clone(), addr: address.clone(), is_write: true, all_funcs: uploaded_abi.read().clone() }
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
                if *active_tab.read() == "transfers" {
                    div { class: "panel",
                        div { class: "panel-header",
                            span { class: "panel-title", "Token Transfers" }
                            span { style: "color:var(--text-muted); font-size:11px;", { format!("{} total", transfers_total.read()) } }
                        }
                        if transfers.read().is_empty() {
                            div { class: "panel-empty", "No token transfers found for this contract." }
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
                                                td { style: "color:var(--accent-green); font-weight:600;", { format!("{} {}", format_amount(t.amount), t.token_symbol) } }
                                            }
                                        }
                                    }
                                }
                            }
                            if transfers.read().len() < *transfers_total.read() as usize {
                                div { style: "padding:16px; text-align:center;",
                                    button {
                                        class: "contract-fn-btn contract-fn-btn-read",
                                        disabled: *transfers_more_loading.read(),
                                        onclick: load_more_transfers,
                                        if *transfers_more_loading.read() { "Loading…" } else { "Load More" }
                                    }
                                }
                            }
                        }
                    }
                }
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
                                let types: Vec<String> = idefs.iter().map(|(_, t)| t.clone()).collect();
                                match build_calldata(&sel, &types, &vals) {
                                    Ok(calldata) => {
                                        if !iw {
                                            match contract_call(&addr, &calldata).await {
                                                Ok(resp) if resp.ok => {
                                                    let hex = resp.result.unwrap_or_default();
                                                    fn_results.write().insert(key.clone(), decode_result(&hex, &out));
                                                }
                                                Ok(resp) => {
                                                    fn_errors.write().insert(key.clone(), resp.error.unwrap_or_else(|| "call reverted".to_string()));
                                                }
                                                Err(e) => { fn_errors.write().insert(key.clone(), e); }
                                            }
                                        } else {
                                            let js = js_send_tx(&addr, &calldata);
                                            let (res, err) = exec_js_promise(&js).await;
                                            if let Some(r) = res { fn_results.write().insert(key.clone(), format!("Tx: {}", r)); }
                                            if let Some(e) = err { fn_errors.write().insert(key.clone(), e); }
                                        }
                                    }
                                    Err(e) => { fn_errors.write().insert(key.clone(), e); }
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

// ── DynFnCard — pure JS rendering, bypasses Dioxus input issues ───────────
#[component]
fn DynFnCard(
    func: DynFunction,
    addr: String,
    is_write: bool,
    all_funcs: Vec<DynFunction>,
) -> Element {
    let container_id = format!("dyncard-{}-{}", func.selector, if is_write { "w" } else { "r" });
    let call_url = format!("{INDEXER_URL}/call");
    let js = build_dyncard_js(
        &container_id,
        &func,
        &addr,
        is_write,
        &all_funcs,
        &call_url,
    );
    use_effect(move || {
        let js2 = js.clone();
        wasm_bindgen_futures::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(50).await;
            let _ = js_sys::eval(&js2);
        });
    });
    rsx! {
        div { class: "contract-fn-card",
            div { id: "{container_id}" }
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
                                    match contract_call(&addr, &calldata).await {
                                        Ok(resp) if resp.ok => {
                                            fn_results.write().insert(key.clone(), resp.result.unwrap_or_default());
                                        }
                                        Ok(resp) => {
                                            fn_errors.write().insert(key.clone(), resp.error.unwrap_or_else(|| "call reverted".to_string()));
                                        }
                                        Err(e) => { fn_errors.write().insert(key.clone(), e); }
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
