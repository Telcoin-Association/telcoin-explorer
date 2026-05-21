// src/services/rpc.rs — v0.1.17
use serde::{Deserialize, Serialize};
use serde_json::Value;
// ── Constants ────────────────────────────────────────────────────────────────
pub const RPC_URL:                  &str = "https://rpc.telcoin.network";
pub const CHAIN_ID:                 u64  = 2017;
pub const NATIVE_TOKEN:             &str = "TEL";
pub const CONSENSUS_REGISTRY:       &str = "0x07e17e17e17e17e17e17e17e17e17e17e17e17e1";
pub const VALIDATOR_STAKE_REQUIRED: &str = "1,000,000";
/// Epoch duration — read from chain via getCurrentEpochInfo().epochDuration
/// This constant is kept for display fallback only; always prefer on-chain value
pub const EPOCH_DURATION_HOURS:     u64  = 6;
// ── Core types ───────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub number:            u64,
    pub hash:              String,
    pub parent_hash:       String,
    pub timestamp:         u64,
    pub transactions:      Vec<String>,
    pub transaction_count: usize,
    pub gas_used:          u64,
    pub gas_limit:         u64,
    pub miner:             String,
    pub validator:         String,
    pub extra_data:        String,
    pub base_fee:          Option<u64>,
    pub size:              u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedInput {
    pub method:    String,
    pub signature: String,
    pub params:    Vec<(String, String)>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash:              String,
    pub from:              String,
    pub to:                Option<String>,
    pub value:             u128,
    pub value_tel:         f64,
    pub gas:               u64,
    pub gas_price:         u64,
    pub gas_used:          u64,
    pub status:            Option<bool>,
    pub input:             String,
    pub decoded_input:     Option<DecodedInput>,
    pub block_number:      Option<u64>,
    pub transaction_index: Option<u64>,
    pub nonce:             u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub latest_block:   u64,
    pub gas_price_gwei: f64,
    pub chain_id:       u64,
    pub epoch_number:   Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTransfer {
    pub tx_hash:       String,
    pub from:          String,
    pub to:            String,
    pub value:         u128,
    pub amount:        f64,
    pub block_number:  u64,
    pub timestamp:     u64,
    pub token_address: String,
    pub token_symbol:  String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address:      String,
    pub name:         String,
    pub symbol:       String,
    pub decimals:     u8,
    pub total_supply: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInfo {
    pub address:        String,
    pub balance:        f64,
    pub tx_count:       u64,
    pub bytecode_hex:   String,
    pub bytecode_size:  usize,
    pub is_erc20:       bool,
    pub is_erc721:      bool,
    pub has_owner:      bool,
    pub has_pause:      bool,
    pub has_mint:       bool,
    pub token_name:     String,
    pub token_symbol:   String,
    pub token_decimals: u8,
    pub token_supply:   String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionSignature {
    pub selector:  String,
    pub signature: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub address:          String,
    pub activation_epoch: u32,
    pub exit_epoch:       u32,
    pub status:           u8,
    pub is_retired:       bool,
    pub stake_version:    u8,
    pub region:           u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochData {
    pub epoch:            u64,
    pub epoch_duration:   u64,   // seconds, read from chain
    pub validator_count:  usize,
    pub validators:       Vec<String>,
    pub start_block:      u64,
    pub latest_block:     u64,
}
// ── RPC plumbing ─────────────────────────────────────────────────────────────
pub async fn rpc_call<P: Serialize, R: for<'de> Deserialize<'de>>(
    method: &str,
    params: P,
) -> Result<R, String> {
    use gloo_net::http::Request;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method":  method,
        "params":  params,
        "id":      1
    });
    let resp = Request::post(RPC_URL)
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(err) = json.get("error") {
        return Err(err.to_string());
    }
    let result = json.get("result").ok_or("no result field")?;
    serde_json::from_value(result.clone()).map_err(|e| e.to_string())
}
// ── Hex helpers ──────────────────────────────────────────────────────────────
pub fn hex_to_u64(hex: &str) -> u64 {
    let s = hex.trim_start_matches("0x");
    if s.is_empty() { return 0; }
    u64::from_str_radix(s, 16).unwrap_or(0)
}
pub fn hex_to_u128(hex: &str) -> u128 {
    let s = hex.trim_start_matches("0x");
    if s.is_empty() { return 0; }
    u128::from_str_radix(s, 16).unwrap_or(0)
}
fn hex_to_u32(hex: &str) -> u32 {
    let s = hex.trim_start_matches("0x");
    if s.is_empty() { return 0; }
    u32::from_str_radix(s, 16).unwrap_or(0)
}
// ── Formatters ───────────────────────────────────────────────────────────────
pub fn format_tel(wei: f64) -> String {
    let whole = (wei / 1e18) as u64;
    let frac  = ((wei % 1e18) / 1e15) as u64;
    if frac == 0 { format!("{whole}") } else { format!("{whole}.{frac:03}") }
}
pub fn format_gas(gas: u64) -> String {
    if gas >= 1_000_000 { format!("{:.2}M", gas as f64 / 1_000_000.0) }
    else if gas >= 1_000 { format!("{:.1}K", gas as f64 / 1_000.0) }
    else                 { format!("{gas}") }
}
pub fn shorten_hash(h: &str) -> String {
    if h.len() > 12 { format!("{}…{}", &h[..6], &h[h.len()-4..]) }
    else { h.to_string() }
}
pub fn shorten_addr(a: &str) -> String {
    if a.len() > 12 { format!("{}…{}", &a[..6], &a[a.len()-4..]) }
    else { a.to_string() }
}
pub fn unix_to_age(ts: u64) -> String {
    let now = js_sys::Date::now() as u64 / 1000;
    let diff = now.saturating_sub(ts);
    if diff < 60         { format!("{diff}s ago") }
    else if diff < 3600  { format!("{}m ago",  diff / 60) }
    else if diff < 86400 { format!("{}h ago",  diff / 3600) }
    else                 { format!("{}d ago",  diff / 86400) }
}
pub fn unix_to_datetime(ts: u64) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ts as f64 * 1000.0));
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        d.get_utc_full_year(), d.get_utc_month() + 1, d.get_utc_date(),
        d.get_utc_hours(),     d.get_utc_minutes(),   d.get_utc_seconds(),
    )
}
// ── Chain queries ─────────────────────────────────────────────────────────────
pub async fn get_block_number() -> Result<u64, String> {
    let hex: String = rpc_call("eth_blockNumber", serde_json::json!([])).await?;
    Ok(hex_to_u64(&hex))
}
pub async fn get_gas_price() -> Option<f64> {
    let hex: String = rpc_call("eth_gasPrice", serde_json::json!([])).await.ok()?;
    Some(hex_to_u64(&hex) as f64 / 1e9)
}
pub async fn get_balance(addr: &str) -> Result<f64, String> {
    let hex: String = rpc_call("eth_getBalance", serde_json::json!([addr, "latest"])).await?;
    Ok(hex_to_u128(&hex) as f64 / 1e18)
}
pub async fn get_tx_count(addr: &str) -> Result<u64, String> {
    let hex: String = rpc_call("eth_getTransactionCount", serde_json::json!([addr, "latest"])).await?;
    Ok(hex_to_u64(&hex))
}
pub async fn is_contract(addr: &str) -> bool {
    let hex: String = match rpc_call("eth_getCode", serde_json::json!([addr, "latest"])).await {
        Ok(h) => h,
        Err(_) => return false,
    };
    hex.len() > 2
}
// ── Block parsing ─────────────────────────────────────────────────────────────
fn parse_block(v: &Value) -> Option<Block> {
    let txns: Vec<String> = v["transactions"].as_array()
        .map(|a| a.iter().filter_map(|t| {
            if let Some(s) = t.as_str() { Some(s.to_string()) }
            else { t["hash"].as_str().map(|s| s.to_string()) }
        }).collect())
        .unwrap_or_default();
    let tx_count = txns.len();
    let miner = v["miner"].as_str().unwrap_or("").to_string();
    let size  = hex_to_u64(v["size"].as_str().unwrap_or("0x0"));
    Some(Block {
        number:            hex_to_u64(v["number"].as_str()?),
        hash:              v["hash"].as_str().unwrap_or("").to_string(),
        parent_hash:       v["parentHash"].as_str().unwrap_or("").to_string(),
        timestamp:         hex_to_u64(v["timestamp"].as_str()?),
        transaction_count: tx_count,
        transactions:      txns,
        gas_used:          hex_to_u64(v["gasUsed"].as_str().unwrap_or("0x0")),
        gas_limit:         hex_to_u64(v["gasLimit"].as_str().unwrap_or("0x0")),
        validator:         miner.clone(),
        miner,
        extra_data:        v["extraData"].as_str().unwrap_or("").to_string(),
        base_fee:          v["baseFeePerGas"].as_str().map(hex_to_u64),
        size,
    })
}
pub async fn get_block_by_number(n: u64) -> Result<Block, String> {
    let hex = format!("0x{n:x}");
    let v: Value = rpc_call("eth_getBlockByNumber", serde_json::json!([hex, false])).await?;
    parse_block(&v).ok_or_else(|| "failed to parse block".to_string())
}
pub async fn get_latest_blocks(count: u64) -> Result<Vec<Block>, String> {
    let latest = get_block_number().await?;
    let mut blocks = Vec::new();
    for i in 0..count {
        if let Ok(b) = get_block_by_number(latest.saturating_sub(i)).await {
            blocks.push(b);
        }
    }
    Ok(blocks)
}
pub async fn get_blocks_page(page: u64, per_page: u64) -> Result<(Vec<Block>, u64), String> {
    let latest = get_block_number().await?;
    let start  = latest.saturating_sub(page * per_page);
    let mut blocks = Vec::new();
    for i in 0..per_page {
        let n = start.saturating_sub(i);
        if n == 0 { break; }
        if let Ok(b) = get_block_by_number(n).await {
            blocks.push(b);
        }
    }
    Ok((blocks, latest))
}
pub async fn get_avg_block_time(n: u64) -> f64 {
    let latest = match get_block_number().await { Ok(l) => l, Err(_) => return 1.0 };
    let newer  = match get_block_by_number(latest).await { Ok(b) => b, Err(_) => return 1.0 };
    let older  = match get_block_by_number(latest.saturating_sub(n)).await { Ok(b) => b, Err(_) => return 1.0 };
    if n == 0 { return 1.0; }
    newer.timestamp.saturating_sub(older.timestamp) as f64 / n as f64
}
pub async fn get_block_activity(n: u64) -> Vec<(u64, f64)> {
    let latest = match get_block_number().await { Ok(l) => l, Err(_) => return vec![] };
    let mut result = Vec::new();
    for i in (0..n).rev() {
        if let Ok(b) = get_block_by_number(latest.saturating_sub(i)).await {
            result.push((b.number, b.transaction_count as f64));
        }
    }
    result
}
pub async fn get_block_time_history(n: u64) -> Vec<(u64, f64)> {
    let latest = match get_block_number().await { Ok(l) => l, Err(_) => return vec![] };
    let mut blocks = Vec::new();
    for i in 0..=n {
        if let Ok(b) = get_block_by_number(latest.saturating_sub(i)).await {
            blocks.push(b);
        }
    }
    blocks.sort_by_key(|b| b.number);
    blocks.windows(2)
        .map(|w| (w[1].number, w[1].timestamp.saturating_sub(w[0].timestamp) as f64))
        .collect()
}
pub async fn subscribe_new_blocks() -> Result<u64, String> {
    get_block_number().await
}
// ── Transaction queries ───────────────────────────────────────────────────────
fn parse_tx(v: &Value) -> Option<Transaction> {
    let value_wei = hex_to_u128(v["value"].as_str().unwrap_or("0x0"));
    Some(Transaction {
        hash:              v["hash"].as_str()?.to_string(),
        from:              v["from"].as_str().unwrap_or("").to_string(),
        to:                v["to"].as_str().map(|s| s.to_string()),
        value:             value_wei,
        value_tel:         value_wei as f64 / 1e18,
        gas:               hex_to_u64(v["gas"].as_str().unwrap_or("0x0")),
        gas_price:         hex_to_u64(v["gasPrice"].as_str().unwrap_or("0x0")),
        gas_used:          0,
        status:            None,
        input:             v["input"].as_str().unwrap_or("0x").to_string(),
        decoded_input:     None,
        block_number:      v["blockNumber"].as_str().map(hex_to_u64),
        transaction_index: v["transactionIndex"].as_str().map(hex_to_u64),
        nonce:             hex_to_u64(v["nonce"].as_str().unwrap_or("0x0")),
    })
}
pub async fn get_transaction(hash: &str) -> Result<Transaction, String> {
    let v: Value = rpc_call("eth_getTransactionByHash", serde_json::json!([hash])).await?;
    let mut tx = parse_tx(&v).ok_or_else(|| "failed to parse tx".to_string())?;
    if let Ok(receipt) = rpc_call::<_, Value>(
        "eth_getTransactionReceipt", serde_json::json!([hash])
    ).await {
        tx.gas_used = receipt["gasUsed"].as_str().map(hex_to_u64).unwrap_or(0);
        tx.status   = receipt["status"].as_str().map(|s| hex_to_u64(s) == 1);
    }
    Ok(tx)
}
pub async fn get_tx_receipt_status(hash: &str) -> Option<bool> {
    let v: Value = rpc_call("eth_getTransactionReceipt", serde_json::json!([hash])).await.ok()?;
    Some(hex_to_u64(v["status"].as_str()?) == 1)
}
pub async fn get_transactions_for_block(hashes: &Vec<String>) -> Vec<Transaction> {
    let mut txs = Vec::new();
    for h in hashes {
        if let Ok(tx) = get_transaction(h).await {
            txs.push(tx);
        }
    }
    txs
}
// ── Token / ERC-20 ────────────────────────────────────────────────────────────
fn abi_decode_string(hex: &str) -> Option<String> {
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 128 { return None; }
    let len = usize::from_str_radix(&raw[64..128], 16).ok()? * 2;
    let s = raw.get(128..128 + len)?;
    let bytes: Vec<u8> = (0..s.len()).step_by(2)
        .filter_map(|i| u8::from_str_radix(&s[i..i+2], 16).ok())
        .collect();
    String::from_utf8(bytes).ok()
}
fn abi_call(contract: &str, selector: &str) -> serde_json::Value {
    serde_json::json!([{"to": contract, "data": selector}, "latest"])
}
pub async fn get_token_symbol(contract: &str) -> String {
    let hex: String = match rpc_call("eth_call", abi_call(contract, "0x95d89b41")).await {
        Ok(h) => h,
        Err(_) => return String::new(),
    };
    abi_decode_string(&hex).unwrap_or_default()
}
pub async fn get_token_info(addr: &str) -> Option<TokenInfo> {
    let name_hex:     String = rpc_call("eth_call", abi_call(addr, "0x06fdde03")).await.unwrap_or_default();
    let symbol_hex:   String = rpc_call("eth_call", abi_call(addr, "0x95d89b41")).await.unwrap_or_default();
    let decimals_hex: String = rpc_call("eth_call", abi_call(addr, "0x313ce567")).await.unwrap_or_default();
    let supply_hex:   String = rpc_call("eth_call", abi_call(addr, "0x18160ddd")).await.unwrap_or_default();
    let name     = abi_decode_string(&name_hex).unwrap_or_default();
    let symbol   = abi_decode_string(&symbol_hex).unwrap_or_default();
    if name.is_empty() && symbol.is_empty() {
        return None;
    }
    let decimals = hex_to_u64(&decimals_hex) as u8;
    let raw_sup  = hex_to_u128(&supply_hex);
    let div      = 10u128.pow(decimals as u32);
    let total_supply = if div > 0 { format!("{}", raw_sup / div) } else { raw_sup.to_string() };
    Some(TokenInfo { address: addr.to_string(), name, symbol, decimals, total_supply })
}
pub async fn get_token_transfers(
    addr: &str,
    from_block: u64,
    to_block: u64,
) -> Result<Vec<Value>, String> {
    let topic = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
    let addr_padded = format!("0x{:0>64}", addr.trim_start_matches("0x").to_lowercase());
    let mut logs: Vec<Value> = rpc_call("eth_getLogs", serde_json::json!([{
        "fromBlock": format!("0x{from_block:x}"),
        "toBlock":   format!("0x{to_block:x}"),
        "topics": [topic, null, addr_padded.clone()],
    }])).await.unwrap_or_default();
    let logs2: Vec<Value> = rpc_call("eth_getLogs", serde_json::json!([{
        "fromBlock": format!("0x{from_block:x}"),
        "toBlock":   format!("0x{to_block:x}"),
        "topics": [topic, addr_padded, null],
    }])).await.unwrap_or_default();
    logs.extend(logs2);
    Ok(logs)
}
pub fn parse_transfer_logs(logs: Vec<Value>) -> Vec<TokenTransfer> {
    logs.into_iter().filter_map(|log| {
        let topics = log["topics"].as_array()?;
        if topics.len() < 3 { return None; }
        let from  = format!("0x{}", &topics[1].as_str()?.trim_start_matches("0x")[24..]);
        let to    = format!("0x{}", &topics[2].as_str()?.trim_start_matches("0x")[24..]);
        let value = hex_to_u128(log["data"].as_str().unwrap_or("0x0"));
        Some(TokenTransfer {
            tx_hash:       log["transactionHash"].as_str().unwrap_or("").to_string(),
            from, to,
            value,
            amount:        value as f64 / 1e18,
            block_number:  hex_to_u64(log["blockNumber"].as_str().unwrap_or("0x0")),
            timestamp:     0,
            token_address: log["address"].as_str().unwrap_or("").to_string(),
            token_symbol:  String::new(),
        })
    }).collect()
}
// ── Contract inspection ───────────────────────────────────────────────────────
pub async fn get_contract_info(addr: &str) -> Result<ContractInfo, String> {
    let bytecode: String = rpc_call("eth_getCode", serde_json::json!([addr, "latest"]))
        .await.unwrap_or_default();
    let raw = bytecode.trim_start_matches("0x");
    let bytecode_size = raw.len() / 2;
    let balance  = get_balance(addr).await.unwrap_or(0.0);
    let tx_count = get_tx_count(addr).await.unwrap_or(0);
    let is_erc20  = raw.contains("a9059cbb") && raw.contains("70a08231") && raw.contains("18160ddd");
    let is_erc721 = raw.contains("6352211e") && raw.contains("70a08231");
    let has_owner = raw.contains("8da5cb5b");
    let has_pause = raw.contains("5c975abb");
    let has_mint  = raw.contains("40c10f19") || raw.contains("a0712d68");
    let (token_name, token_symbol, token_decimals, token_supply) = if is_erc20 {
        let n = abi_decode_string(
            &rpc_call::<_, String>("eth_call", abi_call(addr, "0x06fdde03")).await.unwrap_or_default()
        ).unwrap_or_default();
        let s = abi_decode_string(
            &rpc_call::<_, String>("eth_call", abi_call(addr, "0x95d89b41")).await.unwrap_or_default()
        ).unwrap_or_default();
        let d = hex_to_u64(
            &rpc_call::<_, String>("eth_call", abi_call(addr, "0x313ce567")).await.unwrap_or_default()
        ) as u8;
        let raw_sup = hex_to_u128(
            &rpc_call::<_, String>("eth_call", abi_call(addr, "0x18160ddd")).await.unwrap_or_default()
        );
        let div = 10u128.pow(d as u32);
        (n, s, d, if div > 0 { format!("{}", raw_sup / div) } else { raw_sup.to_string() })
    } else {
        (String::new(), String::new(), 0u8, String::new())
    };
    Ok(ContractInfo {
        address: addr.to_string(),
        balance, tx_count,
        bytecode_hex: raw.to_string(),
        bytecode_size,
        is_erc20, is_erc721, has_owner, has_pause, has_mint,
        token_name, token_symbol, token_decimals, token_supply,
    })
}
pub async fn resolve_selectors(bytecode_hex: &str) -> Vec<FunctionSignature> {
    use gloo_net::http::Request;
    use std::collections::HashSet;
    let raw = bytecode_hex.trim_start_matches("0x");
    let bytes: Vec<u8> = (0..raw.len().saturating_sub(1)).step_by(2)
        .filter_map(|i| u8::from_str_radix(&raw[i..i+2], 16).ok())
        .collect();
    let mut selectors: HashSet<String> = HashSet::new();
    let mut i = 0;
    while i + 4 < bytes.len() {
        if bytes[i] == 0x63 {
            selectors.insert(format!("{:02x}{:02x}{:02x}{:02x}",
                bytes[i+1], bytes[i+2], bytes[i+3], bytes[i+4]));
            i += 5;
        } else {
            i += 1;
        }
    }
    let mut results = Vec::new();
    for sel in selectors {
        let url = format!("https://www.4byte.directory/api/v1/signatures/?hex_signature=0x{sel}");
        if let Ok(resp) = Request::get(&url).send().await {
            if let Ok(json) = resp.json::<Value>().await {
                if let Some(sig) = json["results"].as_array()
                    .and_then(|a| a.first())
                    .and_then(|r| r["text_signature"].as_str())
                {
                    results.push(FunctionSignature { selector: sel, signature: sig.to_string() });
                }
            }
        }
    }
    results.sort_by(|a, b| a.selector.cmp(&b.selector));
    results
}
// ── Network stats ─────────────────────────────────────────────────────────────
/// getCurrentEpochInfo() → EpochInfo struct
/// Selector: 0xbabc394f
/// EpochInfo ABI layout (hex chars, 64 per slot):
///   slot 0  (0..64):    outer offset to struct = 0x20
///   slot 1  (64..128):  offset to committee[] from struct start = 0xC0
///   slot 2  (128..192): epochIssuance
///   slot 3  (192..256): blockHeight   ← epoch start block
///   slot 4  (256..320): epochId       ← epoch number
///   slot 5  (320..384): epochDuration ← seconds per epoch (read from chain!)
///   slot 6  (384..448): stakeVersion
///   committee array at char 448:
///     448..512: committee length
///     512+i*64: committee[i] address (last 40 chars)
pub async fn get_epoch_info_full() -> Option<(u64, u64, u64, Vec<String>)> {
    // returns (epoch_id, block_height, epoch_duration_secs, committee_addrs)
    let call = serde_json::json!([{"to": CONSENSUS_REGISTRY, "data": "0xbabc394f"}, "latest"]);
    let hex: String = match rpc_call("eth_call", call).await {
        Ok(h) => h,
        Err(_) => return None,
    };
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 512 { return None; }
    let block_height    = hex_to_u64(&raw[192..256]);
    let epoch_id        = hex_to_u64(&raw[256..320]);
    let epoch_duration  = hex_to_u64(&raw[320..384]);
    let count = match usize::from_str_radix(&raw[448..512], 16) {
        Ok(c) if c < 256 => c,
        _ => return None,
    };
    let mut addrs = Vec::with_capacity(count);
    for i in 0..count {
        let start = 512 + i * 64;
        if start + 64 > raw.len() { break; }
        let addr = format!("0x{}", &raw[start + 24..start + 64]);
        addrs.push(addr);
    }
    // Return even if epoch_id == 0 — epoch 0 is valid
    Some((epoch_id, block_height, epoch_duration, addrs))
}
pub async fn get_epoch_info() -> Option<u64> {
    get_epoch_info_full().await.map(|(id, _, _, _)| id)
}
pub async fn get_network_stats() -> Result<NetworkStats, String> {
    let latest_block   = get_block_number().await?;
    let gas_price_gwei = get_gas_price().await.unwrap_or(0.0);
    let epoch_number   = get_epoch_info().await;
    Ok(NetworkStats { latest_block, gas_price_gwei, chain_id: CHAIN_ID, epoch_number })
}
// ── Validators ────────────────────────────────────────────────────────────────
pub async fn get_validators_from_registry() -> Result<Vec<String>, String> {
    let latest = get_block_number().await?;
    let mut seen = std::collections::HashSet::new();
    for i in 0..50u64 {
        if let Ok(b) = get_block_by_number(latest.saturating_sub(i)).await {
            seen.insert(b.validator);
        }
    }
    let mut v: Vec<String> = seen.into_iter().collect();
    v.sort();
    Ok(v)
}
/// getValidators(uint8) — Active = 2
/// Selector: 0x8cc05eda
pub async fn get_validators() -> Result<Vec<ValidatorInfo>, String> {
    let data = "0x8cc05eda0000000000000000000000000000000000000000000000000000000000000002";
    let call = serde_json::json!([{"to": CONSENSUS_REGISTRY, "data": data}, "latest"]);
    let hex: String = rpc_call("eth_call", call).await?;
    Ok(decode_validators(&hex))
}
fn decode_validators(hex: &str) -> Vec<ValidatorInfo> {
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 128 { return vec![]; }
    let count = match usize::from_str_radix(&raw[64..128], 16) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let slot = 64usize;
    let struct_size = 7 * slot;
    let mut validators = Vec::with_capacity(count);
    for i in 0..count {
        let base = 128 + i * struct_size;
        if base + struct_size > raw.len() { break; }
        validators.push(ValidatorInfo {
            address:          format!("0x{}", &raw[base..base+slot][24..]),
            activation_epoch: hex_to_u32(&raw[base+slot..base+2*slot]),
            exit_epoch:       hex_to_u32(&raw[base+2*slot..base+3*slot]),
            status:           hex_to_u32(&raw[base+3*slot..base+4*slot]) as u8,
            is_retired:       hex_to_u32(&raw[base+4*slot..base+5*slot]) != 0,
            stake_version:    hex_to_u32(&raw[base+5*slot..base+6*slot]) as u8,
            region:           hex_to_u32(&raw[base+6*slot..base+7*slot]) as u8,
        });
    }
    validators
}
/// Get full current epoch data (epochs page)
/// Uses getCurrentEpochInfo() — reads epochDuration from chain, no hardcoding.
pub async fn get_current_epoch_data() -> Result<EpochData, String> {
    let (epoch_full_opt, latest_block) = futures::join!(
        get_epoch_info_full(),
        get_block_number(),
    );
    let latest = latest_block.unwrap_or(0);
    let (epoch, start_block, epoch_duration, committee) =
        if let Some((id, block_height, duration, addrs)) = epoch_full_opt {
            (id, block_height, duration, addrs)
        } else {
            // Fallback: scan recent blocks for miner addresses
            let mut seen = std::collections::HashSet::new();
            for i in 0..50u64 {
                if let Ok(b) = get_block_by_number(latest.saturating_sub(i)).await {
                    seen.insert(b.validator);
                }
            }
            let mut v: Vec<String> = seen.into_iter().collect();
            v.sort();
            (0u64, 0u64, EPOCH_DURATION_HOURS * 3600, v)
        };
    Ok(EpochData {
        epoch,
        epoch_duration,
        validator_count: committee.len(),
        validators: committee,
        start_block,
        latest_block: latest,
    })
}
/// Scan last `n` blocks, return (address, block_count) sorted desc
pub async fn get_validator_leader_counts(n: u64) -> Vec<(String, u64)> {
    let latest = match get_block_number().await { Ok(l) => l, Err(_) => return vec![] };
    let mut counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for i in 0..n {
        if let Ok(b) = get_block_by_number(latest.saturating_sub(i)).await {
            *counts.entry(b.validator).or_insert(0) += 1;
        }
    }
    let mut v: Vec<(String, u64)> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v
}
