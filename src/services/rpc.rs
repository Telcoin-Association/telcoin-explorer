// src/services/rpc.rs — v0.2.0
// Talks to the tn-block-explorer-indexer API instead of the public RPC.
// See: https://github.com/Telcoin-Association/tn-block-explorer-indexer
//
// CRITICAL: value fields are u128 JSON numbers. We MUST parse response text
// directly via serde_json::from_str::<T>() — NEVER through serde_json::Value —
// or integers above u64::MAX get silently corrupted. See indexer types.rs docs.
use serde::{Deserialize, Serialize};

// ── Constants ────────────────────────────────────────────────────────────────
pub const INDEXER_URL:              &str = "https://indexer.telscan.xyz";
pub const CHAIN_ID:                 u64  = 2017;
pub const NATIVE_TOKEN:             &str = "TEL";
pub const CONSENSUS_REGISTRY:       &str = "0x07e17e17e17e17e17e17e17e17e17e17e17e17e1";
pub const VALIDATOR_STAKE_REQUIRED: &str = "1,000,000";
/// Sentinel token_address the indexer uses to represent native TEL inside a
/// Transfer-shaped record. TEL is the native gas token with no real ERC-20
/// contract (same reason the TokenRegistry uses WTEL to represent it), so
/// events like faucet claims -- minted as a "Transfer" from the zero address
/// for consistency with the rest of the transfer-history API -- carry this
/// placeholder address instead of a real one. token_symbol is null and
/// amount is the raw un-adjusted wei value for these records; both need
/// special-casing wherever a TokenTransfer is displayed.
pub const NATIVE_TEL_TRANSFER_MARKER: &str = "0x00000000000000000000000000000000000007e1";
pub fn is_native_tel_transfer(token_address: &str) -> bool {
    token_address.to_lowercase() == NATIVE_TEL_TRANSFER_MARKER
}
/// The canonical TokenRegistry proxy — survives upgrades, the only address worth hardcoding.
pub const TOKEN_REGISTRY: &str = "0x96C48BA24D2b48b3bd4a703a3Fc7095E7770d92C";
/// Stateless, replaceable read helper for the registry — front ends talk to this.
/// Bulk/paged reads live here, not on the registry itself.
pub const TOKEN_REGISTRY_LENS: &str = "0xDF31403EBCB5eA4Bd32BB499a8a27967DAA882A3";
/// Fallback only — always prefer EpochData.epoch_duration read from chain via the indexer.
pub const EPOCH_DURATION_HOURS:     u64  = 6;

// ── Core types — field-compatible with the indexer's Api* wire types ────────
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
    pub latest_block:    u64,
    pub gas_price_gwei:  f64,
    pub chain_id:        u64,
    pub epoch_number:    Option<u64>,
    pub validator_count: usize,
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
/// One token from the on-chain TokenRegistry (via the Lens contract) — not
/// hardcoded; the list is resolved live from a single `getAllRecords()` call
/// so new registrations (and logo/website updates) show up automatically
/// without a TelScan deploy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredToken {
    pub address:   String,
    pub name:      String,
    pub symbol:    String,
    pub decimals:  u8,
    /// Self-attested by the listing holder, never verified on-chain — treat
    /// as a display hint, sanitize before rendering, never auto-follow.
    pub website:   String,
    /// Self-attested logo URL; empty string if unset.
    pub logo_uri:  String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInfo {
    pub address:        String,
    pub balance:        f64,
    pub balance_wei:    String,
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

// ── Wire types returned directly by the indexer API ──────────────────────────
#[derive(Debug, Clone, Deserialize)]
pub struct Envelope<T> {
    pub items:    Vec<T>,
    pub total:    u64,
    pub page:     u64,
    pub per_page: u64,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ApiTokenTransfer {
    pub tx_hash:       String,
    pub from:          String,
    pub to:            String,
    pub value:         u128,
    pub value_exact:   String,
    pub amount:        f64,
    pub block_number:  u64,
    pub timestamp:     u64,
    pub token_address: String,
    pub token_symbol:  Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ApiAddress {
    pub address:          String,
    pub balance_wei:       String,
    pub balance_tel:        f64,
    pub nonce:             u64,
    pub is_contract:       bool,
    pub code:              Option<String>,
    pub indexed_tx_count:  u64,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ApiTokenInfo {
    pub address:         String,
    pub name:            Option<String>,
    pub symbol:          Option<String>,
    pub decimals:        Option<u8>,
    pub total_supply:    Option<String>,
    pub metadata_status: u8,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ApiHealth {
    pub status:         String,
    pub indexing_live:  bool,
    pub last_indexed:   u64,
    pub node_tip:       u64,
    pub lag:            u64,
}
#[derive(Debug, Clone, Deserialize)]
pub struct LeaderEntry {
    pub address: String,
    pub blocks:  u64,
}
#[derive(Debug, Clone, Deserialize)]
pub struct LeadersResponse {
    pub window:     u64,
    pub from_block: u64,
    pub to_block:   u64,
    pub leaders:    Vec<LeaderEntry>,
}
#[derive(Debug, Clone, Serialize)]
pub struct CallRequest {
    pub to:   String,
    pub data: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct CallResponse {
    pub ok:           bool,
    pub result:        Option<String>,
    pub error:          Option<String>,
    pub revert_data:    Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ApiEpoch {
    pub epoch:               u64,
    pub start_block:         u64,
    pub end_block:           Option<u64>,
    pub end_time:            Option<u64>,
    pub committee_size:      usize,
    pub committee_bls:       Vec<String>,
    pub certified:           bool,
    pub is_current:          bool,
    pub committee_addresses: Option<Vec<String>>,
}

// ── HTTP plumbing — text-first parsing to preserve u128 precision ───────────
async fn indexer_get<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, String> {
    use gloo_net::http::Request;
    let url = format!("{INDEXER_URL}{path}");
    let resp = Request::get(&url).send().await.map_err(|e| e.to_string())?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(extract_error(&text));
    }
    serde_json::from_str(&text).map_err(|e| format!("parse error: {e} — body: {text}"))
}
async fn indexer_post<B: Serialize, T: for<'de> Deserialize<'de>>(path: &str, body: &B) -> Result<T, String> {
    use gloo_net::http::Request;
    let url = format!("{INDEXER_URL}{path}");
    let resp = Request::post(&url)
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(extract_error(&text));
    }
    serde_json::from_str(&text).map_err(|e| format!("parse error: {e} — body: {text}"))
}
fn extract_error(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| body.to_string())
}

// ── Hex helpers (still used for contract read/write ABI encoding) ────────────
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

// ── Formatters ───────────────────────────────────────────────────────────────
pub fn format_tel(wei: f64) -> String {
    let whole = (wei / 1e18) as u64;
    let frac  = ((wei % 1e18) / 1e15) as u64;
    if frac == 0 { format!("{whole}") } else { format!("{whole}.{frac:03}") }
}
/// Exact wei -> TEL decimal string using pure integer math (no f64 rounding),
/// trailing zeros trimmed. Use this everywhere a wei-denominated u128 is
/// available (transaction value, fee, balance) instead of a pre-divided f64.
pub fn format_wei_exact(wei: u128) -> String {
    const BASE: u128 = 1_000_000_000_000_000_000;
    let whole = wei / BASE;
    let frac  = wei % BASE;
    if frac == 0 {
        format!("{whole}")
    } else {
        let frac_str = format!("{:018}", frac);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{whole}.{trimmed}")
    }
}
/// Trim a float display to its meaningful digits instead of an arbitrary
/// fixed decimal count — avoids both truncating real precision and padding
/// with meaningless zeros. Caps at 8 decimals (beyond that f64 has no more
/// meaningful precision anyway). Used for token transfer amounts, where the
/// indexer already computed `amount` as f64 server-side (decimals-adjusted)
/// and per-transfer decimals aren't available client-side for exact math.
pub fn format_amount(amount: f64) -> String {
    if amount == 0.0 { return "0".to_string(); }
    let mut s = format!("{amount:.8}");
    while s.ends_with('0') { s.pop(); }
    if s.ends_with('.') { s.pop(); }
    s
}
/// Format a raw on-chain token amount (an exact integer string in the
/// token's base units, e.g. totalSupply()) as a human-readable value:
/// decimal point inserted at `decimals`, trailing zeros trimmed, thousands
/// separators on the whole part. Pure integer/string math -- no f64 -- so
/// this stays exact even for supplies beyond f64's safe integer range.
pub fn format_token_amount(raw: &str, decimals: u8) -> String {
    let digits: String = raw.trim().chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() { return "0".to_string(); }
    let decimals = decimals as usize;
    let padded = if digits.len() <= decimals {
        format!("{digits:0>width$}", width = decimals + 1)
    } else {
        digits
    };
    let split_at = padded.len() - decimals;
    let (whole, frac) = padded.split_at(split_at);
    let whole_with_commas = add_thousands_separators(whole);
    let frac_trimmed = frac.trim_end_matches('0');
    if frac_trimmed.is_empty() {
        whole_with_commas
    } else {
        format!("{whole_with_commas}.{frac_trimmed}")
    }
}
fn add_thousands_separators(s: &str) -> String {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}
/// Human-readable "amount SYMBOL" for a token transfer, correctly handling
/// the native-TEL sentinel address (see NATIVE_TEL_TRANSFER_MARKER) -- shows
/// "N TEL" using exact wei math instead of the raw un-adjusted integer with
/// no symbol that `amount`/`token_symbol` would otherwise produce.
pub fn format_transfer_amount(t: &TokenTransfer) -> String {
    if is_native_tel_transfer(&t.token_address) {
        format!("{} TEL", format_wei_exact(t.value))
    } else {
        format!("{} {}", format_amount(t.amount), t.token_symbol)
    }
}
/// Numeric-only amount (no symbol) for a token transfer -- for CSV export,
/// where the symbol has its own column and shouldn't be embedded in the
/// amount field. Same native-TEL special-casing as format_transfer_amount.
pub fn transfer_amount_raw_str(t: &TokenTransfer) -> String {
    if is_native_tel_transfer(&t.token_address) {
        format_wei_exact(t.value)
    } else {
        format_amount(t.amount)
    }
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

// ── Health ────────────────────────────────────────────────────────────────────
pub async fn get_indexer_health() -> Result<ApiHealth, String> {
    indexer_get("/health").await
}

// ── Network stats ─────────────────────────────────────────────────────────────
/// Single call to /stats — replaces the old multi-RPC get_network_stats.
pub async fn get_network_stats() -> Result<NetworkStats, String> {
    #[derive(Deserialize)]
    struct ApiStats {
        latest_block:    u64,
        gas_price_gwei:  f64,
        chain_id:        u64,
        epoch_number:    Option<u64>,
        validator_count: usize,
    }
    let s: ApiStats = indexer_get("/stats").await?;
    Ok(NetworkStats {
        latest_block:    s.latest_block,
        gas_price_gwei:  s.gas_price_gwei,
        chain_id:        s.chain_id,
        epoch_number:    s.epoch_number,
        validator_count: s.validator_count,
    })
}
pub async fn get_block_number() -> Result<u64, String> {
    Ok(get_network_stats().await?.latest_block)
}
pub async fn get_gas_price() -> Option<f64> {
    get_network_stats().await.ok().map(|s| s.gas_price_gwei)
}

// ── Address ────────────────────────────────────────────────────────────────────
pub async fn get_balance(addr: &str) -> Result<f64, String> {
    let a: ApiAddress = indexer_get(&format!("/address/{addr}")).await?;
    Ok(a.balance_tel)
}
pub async fn get_tx_count(addr: &str) -> Result<u64, String> {
    let a: ApiAddress = indexer_get(&format!("/address/{addr}")).await?;
    Ok(a.nonce)
}
/// Exact balance in wei as a decimal string — no f64 rounding. Use with
/// `format_wei_exact` (parsed to u128) for display.
pub async fn get_balance_wei(addr: &str) -> Result<String, String> {
    let a: ApiAddress = indexer_get(&format!("/address/{addr}")).await?;
    Ok(a.balance_wei)
}
pub async fn is_contract(addr: &str) -> bool {
    indexer_get::<ApiAddress>(&format!("/address/{addr}")).await
        .map(|a| a.is_contract)
        .unwrap_or(false)
}
pub async fn get_address_summary(addr: &str) -> Result<ApiAddress, String> {
    indexer_get(&format!("/address/{addr}")).await
}

// ── Blocks ─────────────────────────────────────────────────────────────────────
pub async fn get_block_by_number(n: u64) -> Result<Block, String> {
    indexer_get(&format!("/blocks/{n}")).await
}
pub async fn get_latest_blocks(count: u64) -> Result<Vec<Block>, String> {
    let env: Envelope<Block> = indexer_get(&format!("/blocks?page=0&per_page={count}")).await?;
    Ok(env.items)
}
pub async fn get_blocks_page(page: u64, per_page: u64) -> Result<(Vec<Block>, u64), String> {
    let env: Envelope<Block> = indexer_get(&format!("/blocks?page={page}&per_page={per_page}")).await?;
    Ok((env.items, env.total))
}
pub async fn get_avg_block_time(n: u64) -> f64 {
    let latest = match get_block_number().await { Ok(l) => l, Err(_) => return 1.0 };
    let newer  = match get_block_by_number(latest).await { Ok(b) => b, Err(_) => return 1.0 };
    let older  = match get_block_by_number(latest.saturating_sub(n)).await { Ok(b) => b, Err(_) => return 1.0 };
    if n == 0 { return 1.0; }
    newer.timestamp.saturating_sub(older.timestamp) as f64 / n as f64
}
pub async fn get_block_activity(n: u64) -> Vec<(u64, f64)> {
    let (blocks, _) = match get_blocks_page(0, n).await { Ok(b) => b, Err(_) => return vec![] };
    blocks.into_iter().rev().map(|b| (b.number, b.transaction_count as f64)).collect()
}
pub async fn get_block_time_history(n: u64) -> Vec<(u64, f64)> {
    let (mut blocks, _) = match get_blocks_page(0, n + 1).await { Ok(b) => b, Err(_) => return vec![] };
    blocks.sort_by_key(|b| b.number);
    blocks.windows(2)
        .map(|w| (w[1].number, w[1].timestamp.saturating_sub(w[0].timestamp) as f64))
        .collect()
}
pub async fn subscribe_new_blocks() -> Result<u64, String> {
    get_block_number().await
}

// ── Transactions ───────────────────────────────────────────────────────────────
pub async fn get_transaction(hash: &str) -> Result<Transaction, String> {
    indexer_get(&format!("/txs/{hash}")).await
}
pub async fn get_tx_receipt_status(hash: &str) -> Option<bool> {
    get_transaction(hash).await.ok().and_then(|t| t.status)
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
pub async fn get_latest_txs(page: u64, per_page: u64) -> Result<(Vec<Transaction>, u64), String> {
    let env: Envelope<Transaction> = indexer_get(&format!("/txs?page={page}&per_page={per_page}")).await?;
    Ok((env.items, env.total))
}
/// Full transaction history for an address — the big unlock over public RPC (no 5000-block scan limit).
pub async fn get_address_txs(addr: &str, page: u64, per_page: u64) -> Result<(Vec<Transaction>, u64), String> {
    let env: Envelope<Transaction> = indexer_get(&format!("/address/{addr}/txs?page={page}&per_page={per_page}")).await?;
    Ok((env.items, env.total))
}

// ── Token / ERC-20 ────────────────────────────────────────────────────────────
fn api_transfer_to_token_transfer(t: ApiTokenTransfer) -> TokenTransfer {
    TokenTransfer {
        tx_hash:       t.tx_hash,
        from:          t.from,
        to:            t.to,
        value:         t.value,
        amount:        t.amount,
        block_number:  t.block_number,
        timestamp:     t.timestamp,
        token_address: t.token_address,
        token_symbol:  t.token_symbol.unwrap_or_default(),
    }
}
pub async fn get_token_symbol(contract: &str) -> String {
    indexer_get::<ApiTokenInfo>(&format!("/tokens/{contract}")).await
        .ok()
        .and_then(|t| t.symbol)
        .unwrap_or_default()
}
pub async fn get_token_info(addr: &str) -> Option<TokenInfo> {
    let t: ApiTokenInfo = indexer_get(&format!("/tokens/{addr}")).await.ok()?;
    let name = t.name.unwrap_or_default();
    let symbol = t.symbol.clone().unwrap_or_default();
    if name.is_empty() && symbol.is_empty() {
        return None;
    }
    Some(TokenInfo {
        address: addr.to_string(),
        name,
        symbol,
        decimals: t.decimals.unwrap_or(0),
        total_supply: t.total_supply.unwrap_or_default(),
    })
}
/// Transfers where `addr` is sender or recipient — full history, paginated.
pub async fn get_address_transfers(addr: &str, page: u64, per_page: u64) -> Result<(Vec<TokenTransfer>, u64), String> {
    let env: Envelope<ApiTokenTransfer> = indexer_get(&format!("/address/{addr}/transfers?page={page}&per_page={per_page}")).await?;
    Ok((env.items.into_iter().map(api_transfer_to_token_transfer).collect(), env.total))
}
/// A token's own transfer feed — full history, paginated.
pub async fn get_token_transfers_page(token: &str, page: u64, per_page: u64) -> Result<(Vec<TokenTransfer>, u64), String> {
    let env: Envelope<ApiTokenTransfer> = indexer_get(&format!("/tokens/{token}/transfers?page={page}&per_page={per_page}")).await?;
    Ok((env.items.into_iter().map(api_transfer_to_token_transfer).collect(), env.total))
}
/// Back-compat shim for pages still calling the old "scan blocks" signature.
/// Ignores from_block/to_block — the indexer serves full history already.
pub async fn get_token_transfers_for_address(addr: &str) -> Vec<TokenTransfer> {
    get_address_transfers(addr, 0, 25).await.map(|(items, _)| items).unwrap_or_default()
}

// ── Contract inspection ───────────────────────────────────────────────────────
pub async fn get_contract_info(addr: &str) -> Result<ContractInfo, String> {
    let account: ApiAddress = indexer_get(&format!("/address/{addr}")).await?;
    let bytecode_hex = account.code.clone().unwrap_or_default();
    let raw = bytecode_hex.trim_start_matches("0x");
    let bytecode_size = raw.len() / 2;
    let is_erc20  = raw.contains("a9059cbb") && raw.contains("70a08231") && raw.contains("18160ddd");
    let is_erc721 = raw.contains("6352211e") && raw.contains("70a08231");
    let has_owner = raw.contains("8da5cb5b");
    let has_pause = raw.contains("5c975abb");
    let has_mint  = raw.contains("40c10f19") || raw.contains("a0712d68");
    let (token_name, token_symbol, token_decimals, token_supply) = if is_erc20 {
        if let Ok(t) = indexer_get::<ApiTokenInfo>(&format!("/tokens/{addr}")).await {
            (
                t.name.unwrap_or_default(),
                t.symbol.unwrap_or_default(),
                t.decimals.unwrap_or(0),
                t.total_supply.unwrap_or_default(),
            )
        } else {
            (String::new(), String::new(), 0u8, String::new())
        }
    } else {
        (String::new(), String::new(), 0u8, String::new())
    };
    Ok(ContractInfo {
        address: addr.to_string(),
        balance: account.balance_tel,
        balance_wei: account.balance_wei.clone(),
        tx_count: account.nonce,
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
            if let Ok(json) = resp.json::<serde_json::Value>().await {
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

// ── Contract read/write — replaces eth_call via public RPC ────────────────────
/// POST /call — {to,data} in, {ok,result|error,revert_data} out.
/// Used by contract.rs's StaticFnCard/DynFnCard/GenericFnCard for view-function
/// reads, replacing the previous direct eth_call via the public RPC WebSocket.
pub async fn contract_call(to: &str, data: &str) -> Result<CallResponse, String> {
    indexer_post("/call", &CallRequest { to: to.to_string(), data: data.to_string() }).await
}

// ── Epochs ─────────────────────────────────────────────────────────────────────
/// Single call to /epochs/current — replaces the old ABI-decoded getCurrentEpochInfo().
pub async fn get_current_epoch_data() -> Result<EpochData, String> {
    #[derive(Deserialize)]
    struct ApiEpochData {
        epoch:           u64,
        epoch_duration:  u64,
        validator_count: usize,
        validators:      Vec<String>,
        start_block:     u64,
        latest_block:    u64,
    }
    let d: ApiEpochData = indexer_get("/epochs/current").await?;
    Ok(EpochData {
        epoch:           d.epoch,
        epoch_duration:  d.epoch_duration,
        validator_count: d.validator_count,
        validators:      d.validators,
        start_block:     d.start_block,
        latest_block:    d.latest_block,
    })
}
pub async fn get_epoch_info() -> Option<u64> {
    get_current_epoch_data().await.ok().map(|d| d.epoch)
}
pub async fn get_epochs_page(page: u64, per_page: u64) -> Result<(Vec<ApiEpoch>, u64), String> {
    let env: Envelope<ApiEpoch> = indexer_get(&format!("/epochs?page={page}&per_page={per_page}")).await?;
    Ok((env.items, env.total))
}
pub async fn get_epoch_by_number(n: u64) -> Result<ApiEpoch, String> {
    indexer_get(&format!("/epochs/{n}")).await
}

// ── Validators ────────────────────────────────────────────────────────────────
/// GET /validators — current committee, pre-decoded (no more manual ABI decode).
pub async fn get_validators() -> Result<Vec<ValidatorInfo>, String> {
    #[derive(Deserialize)]
    struct ApiValidator {
        address:          String,
        activation_epoch: u32,
        exit_epoch:       u32,
        status:           u8,
        is_retired:       bool,
        stake_version:    u8,
        region:           u8,
    }
    let vs: Vec<ApiValidator> = indexer_get("/validators").await?;
    Ok(vs.into_iter().map(|v| ValidatorInfo {
        address:          v.address,
        activation_epoch: v.activation_epoch,
        exit_epoch:       v.exit_epoch,
        status:           v.status,
        is_retired:       v.is_retired,
        stake_version:    v.stake_version,
        region:           v.region,
    }).collect())
}
pub async fn get_validators_from_registry() -> Result<Vec<String>, String> {
    Ok(get_validators().await?.into_iter().map(|v| v.address).collect())
}
/// GET /validators/leaders?window=n — replaces the old N-block scan loop.
pub async fn get_validator_leader_counts(n: u64) -> Vec<(String, u64)> {
    let window = n.clamp(1, 1000);
    match indexer_get::<LeadersResponse>(&format!("/validators/leaders?window={window}")).await {
        Ok(resp) => resp.leaders.into_iter().map(|l| (l.address, l.blocks)).collect(),
        Err(_) => vec![],
    }
}

// ── Transaction-scoped transfers (best-effort, reuses existing endpoints) ────
/// ERC-20 transfers associated with a specific transaction, found by paging
/// `participant`'s transfer history (newest-first, via the existing
/// /address/:addr/transfers endpoint) until either a tx_hash match is found
/// or we've paged past the transaction's block number. No indexer changes
/// needed -- this is exactly the data already shown on address/token pages,
/// just filtered down to one transaction.
pub async fn get_token_transfers_for_tx(tx_hash: &str, participant: &str, block_number: u64) -> Vec<TokenTransfer> {
    let mut found = Vec::new();
    let mut page = 0u64;
    loop {
        let (items, total) = match get_address_transfers(participant, page, 100).await {
            Ok(r) => r,
            Err(_) => break,
        };
        if items.is_empty() { break; }
        let mut past_target = false;
        for t in &items {
            if t.tx_hash.eq_ignore_ascii_case(tx_hash) {
                found.push(t.clone());
            }
            if t.block_number < block_number {
                past_target = true;
            }
        }
        if past_target || page.saturating_mul(100) >= total || page >= 5 { break; }
        page += 1;
    }
    found
}

// ── Token Registry ─────────────────────────────────────────────────────────────
/// Decode a plain ABI-encoded `address[]` return (standard dynamic array:
/// offset word, length word, then N right-aligned 32-byte address words).
fn word_at(raw: &str, hex_offset: usize) -> &str {
    raw.get(hex_offset..hex_offset + 64).unwrap_or("")
}
fn word_to_usize(word: &str) -> usize {
    let trimmed = word.trim_start_matches('0');
    if trimmed.is_empty() { 0 } else { usize::from_str_radix(trimmed, 16).unwrap_or(0) }
}
fn word_to_address(word: &str) -> String {
    if word.len() < 40 { return String::new(); }
    format!("0x{}", &word[word.len() - 40..])
}
fn word_to_u8(word: &str) -> u8 {
    if word.len() < 2 { return 0; }
    u8::from_str_radix(&word[word.len() - 2..], 16).unwrap_or(0)
}
/// Decode an ABI `string` whose length word starts at the given HEX-CHARACTER
/// offset (length word, then the UTF-8 bytes right after it).
fn decode_string_at(raw: &str, abs_hex_offset: usize) -> String {
    let len = word_to_usize(word_at(raw, abs_hex_offset));
    let data_start = abs_hex_offset + 64;
    let data_hex = raw.get(data_start..data_start + len * 2).unwrap_or("");
    let bytes: Vec<u8> = (0..data_hex.len()).step_by(2)
        .filter_map(|i| u8::from_str_radix(&data_hex[i..i + 2], 16).ok())
        .collect();
    String::from_utf8(bytes).unwrap_or_default()
}
/// Decode `getAllRecords()`'s return: `TokenRecord[]` where each record is
/// (address token, uint8 decimals, uint8 status, uint40 listedAt,
/// uint40 updatedAt, uint8 flags, string name, string symbol, string website,
/// string logoURI). Six static head words then four dynamic-string offset
/// words (10 words / 640 hex chars of head per tuple), tails holding the
/// four strings in order.
fn decode_token_records(hex: &str) -> Vec<RegisteredToken> {
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 128 { return vec![]; }
    let array_data_start = word_to_usize(word_at(raw, 0)) * 2;
    if raw.len() < array_data_start + 64 { return vec![]; }
    let count = word_to_usize(word_at(raw, array_data_start));
    let elements_start = array_data_start + 64;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let offset_word_pos = elements_start + i * 64;
        if raw.len() < offset_word_pos + 64 { break; }
        let rel_offset = word_to_usize(word_at(raw, offset_word_pos));
        let elem_start = elements_start + rel_offset * 2;
        if raw.len() < elem_start + 10 * 64 { continue; }
        let token = word_to_address(word_at(raw, elem_start));
        let decimals = word_to_u8(word_at(raw, elem_start + 64));
        let name_off    = word_to_usize(word_at(raw, elem_start + 384));
        let symbol_off  = word_to_usize(word_at(raw, elem_start + 448));
        let website_off = word_to_usize(word_at(raw, elem_start + 512));
        let logo_off    = word_to_usize(word_at(raw, elem_start + 576));
        out.push(RegisteredToken {
            address:  token,
            name:     decode_string_at(raw, elem_start + name_off * 2),
            symbol:   decode_string_at(raw, elem_start + symbol_off * 2),
            decimals,
            website:  decode_string_at(raw, elem_start + website_off * 2),
            logo_uri: decode_string_at(raw, elem_start + logo_off * 2),
        });
    }
    out
}
/// Full list of tokens listed in the on-chain TokenRegistry: resolved live in
/// ONE call to the Lens's getAllRecords() (selector 0xa7f9fe72), including
/// name/symbol/decimals/website/logo. No hardcoded token list, no per-token
/// round trips -- new registrations (and logo/website edits) show up
/// automatically, nothing to redeploy.
pub async fn get_registered_tokens() -> Vec<RegisteredToken> {
    let resp = match contract_call(TOKEN_REGISTRY_LENS, "0xa7f9fe72").await {
        Ok(r) if r.ok => r,
        _ => return vec![],
    };
    decode_token_records(&resp.result.unwrap_or_default())
}
/// One registry entry by address, if listed -- for the individual token page
/// (logo/website), rather than the search-only bulk list.
pub async fn get_registered_token(address: &str) -> Option<RegisteredToken> {
    let addr_lower = address.to_lowercase();
    get_registered_tokens().await.into_iter()
        .find(|t| t.address.to_lowercase() == addr_lower)
}
