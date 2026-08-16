// src/pages/address.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    is_contract,
    get_balance_wei, get_tx_count, get_address_txs, get_address_transfers,
    TokenTransfer, Transaction, shorten_hash, shorten_addr, format_wei_exact, format_wei_exact_commas,
    format_transfer_amount, transfer_amount_raw_str, is_native_tel_transfer,
    CONSENSUS_REGISTRY,
};
use crate::components::loading::{Loading, ErrorBox, CopyButton};

/// Escape a value for a CSV field: wrap in quotes and double any internal quotes.
fn csv_escape(v: &str) -> String {
    format!("\"{}\"", v.replace('"', "\"\""))
}

/// Trigger a browser download of `content` as a file named `filename`.
/// Uses a Blob + temporary <a download> click, all client-side — no server round-trip.
fn download_csv(filename: &str, content: &str) {
    let js = [
        "(function(){",
        "var blob=new Blob([", &serde_json::to_string(content).unwrap_or_default(), "],{type:'text/csv;charset=utf-8;'});",
        "var url=URL.createObjectURL(blob);",
        "var a=document.createElement('a');",
        "a.href=url;",
        "a.download=", &serde_json::to_string(filename).unwrap_or_default(), ";",
        "document.body.appendChild(a);",
        "a.click();",
        "document.body.removeChild(a);",
        "URL.revokeObjectURL(url);",
        "})()",
    ].concat();
    let _ = js_sys::eval(&js);
}

fn txs_to_csv(txs: &[Transaction]) -> String {
    let mut out = String::from("Hash,Block,From,To,Value (TEL),Gas Used,Gas Price (wei),Fee (TEL),Status,Nonce\n");
    for tx in txs {
        let fee_wei = tx.gas_used as u128 * tx.gas_price as u128;
        let status = match tx.status {
            Some(true)  => "Success",
            Some(false) => "Failed",
            None        => "",
        };
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape(&tx.hash),
            tx.block_number.map(|b| b.to_string()).unwrap_or_default(),
            csv_escape(&tx.from),
            csv_escape(&tx.to.clone().unwrap_or_else(|| "Contract Creation".to_string())),
            format_wei_exact(tx.value),
            tx.gas_used,
            tx.gas_price,
            format_wei_exact(fee_wei),
            status,
            tx.nonce,
        ));
    }
    out
}

fn transfers_to_csv(transfers: &[TokenTransfer]) -> String {
    let mut out = String::from("Tx Hash,Block,From,To,Token Address,Token Symbol,Amount\n");
    for t in transfers {
        out.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            csv_escape(&t.tx_hash),
            t.block_number,
            csv_escape(&t.from),
            csv_escape(&t.to),
            csv_escape(&t.token_address),
            csv_escape(if is_native_tel_transfer(&t.token_address) { "TEL" } else { &t.token_symbol }),
            transfer_amount_raw_str(t),
        ));
    }
    out
}

#[component]
pub fn AddressPage(address: String) -> Element {
    let mut balance_wei: Signal<Option<String>>   = use_signal(|| None);
    let mut tx_count: Signal<Option<u64>>         = use_signal(|| None);
    let mut transfers: Signal<Vec<TokenTransfer>> = use_signal(|| vec![]);
    let mut transfers_total: Signal<u64>          = use_signal(|| 0);
    let mut native_txs: Signal<Vec<Transaction>>  = use_signal(|| vec![]);
    let mut txs_total: Signal<u64>                = use_signal(|| 0);
    let mut txs_page: Signal<u64>                 = use_signal(|| 0);
    let mut txs_more_loading                      = use_signal(|| false);
    let mut loading                               = use_signal(|| true);
    let mut txs_loading                           = use_signal(|| true);
    let mut error: Signal<Option<String>>         = use_signal(|| None);
    let mut active_tab                            = use_signal(|| "txs");
    let mut contract_flag: Signal<bool>           = use_signal(|| false);
    let mut transfers_page: Signal<u64>           = use_signal(|| 0);
    let mut transfers_more_loading                = use_signal(|| false);
    let mut txs_export_loading                    = use_signal(|| false);
    let mut transfers_export_loading              = use_signal(|| false);

    // use_reactive is required here: `address` is a plain String prop, not a
    // Signal, so without it this effect only runs once on first mount and
    // never restarts when navigating between two AddressPage instances via
    // Link (Dioxus reuses the same component/hook state across route param
    // changes for the same route type -- a known limitation, see
    // https://github.com/DioxusLabs/dioxus/issues/2784). Previously this
    // left stale data on screen after clicking a From/To link: the header
    // re-rendered with the new address immediately, but the transaction/
    // transfer lists kept showing the PREVIOUS address's data.
    use_effect(use_reactive(&address, move |address| {
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            txs_loading.set(true);
            // Clear stale data from any previously-viewed address immediately,
            // rather than leaving it on screen until the new fetch resolves.
            native_txs.set(vec![]);
            transfers.set(vec![]);
            balance_wei.set(None);
            tx_count.set(None);
            error.set(None);
            txs_page.set(0);
            transfers_page.set(0);
            let (bal_res, count_res) = futures::join!(
                get_balance_wei(&address),
                get_tx_count(&address),
            );
            match bal_res   { Ok(w) => balance_wei.set(Some(w)), Err(e) => error.set(Some(e)) }
            match count_res { Ok(n) => tx_count.set(Some(n)), Err(_) => {} }
            loading.set(false);
            // Full transaction history — indexed pointer lookup, no block scanning.
            if let Ok((txs, total)) = get_address_txs(&address, 0, 25).await {
                native_txs.set(txs);
                txs_total.set(total);
            }
            // Full ERC-20 transfer history — token_symbol already cached by the indexer.
            if let Ok((xfers, total)) = get_address_transfers(&address, 0, 25).await {
                transfers.set(xfers);
                transfers_total.set(total);
            }
            contract_flag.set(is_contract(&address).await);
            txs_loading.set(false);
        });
    }));

    let load_more_txs = {
        let address = address.clone();
        move |_| {
            let address = address.clone();
            wasm_bindgen_futures::spawn_local(async move {
                txs_more_loading.set(true);
                let next = *txs_page.read() + 1;
                if let Ok((mut more, _)) = get_address_txs(&address, next, 25).await {
                    native_txs.write().append(&mut more);
                    txs_page.set(next);
                }
                txs_more_loading.set(false);
            });
        }
    };
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

    // Export fetches the FULL history (looping every page), independent of
    // however much is currently loaded on screen via "Load More".
    let export_txs = {
        let address = address.clone();
        move |_| {
            let address = address.clone();
            wasm_bindgen_futures::spawn_local(async move {
                txs_export_loading.set(true);
                let mut all: Vec<Transaction> = Vec::new();
                let mut page = 0u64;
                loop {
                    match get_address_txs(&address, page, 100).await {
                        Ok((mut batch, total)) => {
                            if batch.is_empty() { break; }
                            all.append(&mut batch);
                            if all.len() as u64 >= total { break; }
                            page += 1;
                        }
                        Err(_) => break,
                    }
                }
                let csv = txs_to_csv(&all);
                download_csv(&format!("export-{}-transactions.csv", address), &csv);
                txs_export_loading.set(false);
            });
        }
    };
    let export_transfers = {
        let address = address.clone();
        move |_| {
            let address = address.clone();
            wasm_bindgen_futures::spawn_local(async move {
                transfers_export_loading.set(true);
                let mut all: Vec<TokenTransfer> = Vec::new();
                let mut page = 0u64;
                loop {
                    match get_address_transfers(&address, page, 100).await {
                        Ok((mut batch, total)) => {
                            if batch.is_empty() { break; }
                            all.append(&mut batch);
                            if all.len() as u64 >= total { break; }
                            page += 1;
                        }
                        Err(_) => break,
                    }
                }
                let csv = transfers_to_csv(&all);
                download_csv(&format!("export-{}-token-transfers.csv", address), &csv);
                transfers_export_loading.set(false);
            });
        }
    };

    let avatar_char = address.chars().nth(2).unwrap_or('?').to_uppercase().next().unwrap_or('?');
    let is_registry = address.to_lowercase() == CONSENSUS_REGISTRY.to_lowercase();

    rsx! {
        div { class: "page",
            if *loading.read() {
                Loading { msg: Some("Fetching address data…".to_string()) }
            } else {
                // ── Address header ──────────────────────────────────────
                div { class: "address-header",
                    div { class: "address-avatar", "{avatar_char}" }
                    div { class: "address-info",
                        div { class: "address-type-row",
                            if is_registry {
                                span { class: "addr-type-badge contract", "ConsensusRegistry" }
                            } else if *contract_flag.read() {
                                span { class: "addr-type-badge contract", "Contract" }
                            } else {
                                span { class: "addr-type-badge eoa", "EOA" }
                            }
                            if is_registry {
                                Link { to: Route::ValidatorsPage {},
                                    span { class: "chip success", style: "cursor:pointer; font-size:11px;", "View Validators →" }
                                }
                            }
                            if *contract_flag.read() {
                                Link { to: Route::ContractPage { address: address.clone() },
                                    span { class: "chip info", style: "cursor:pointer; font-size:11px;", "View Contract →" }
                                }
                            }
                        }
                        div { class: "address-hash-row",
                            span { class: "address-hash-text", "{address}" }
                            CopyButton { text: address.clone() }
                        }
                        if let Some(wei_str) = balance_wei.read().as_ref() {
                            div { class: "address-balance-big",
                                { format_wei_exact_commas(wei_str.parse().unwrap_or(0)) }
                                span { "TEL" }
                            }
                        }
                        if let Some(nonce) = *tx_count.read() {
                            div { class: "address-meta",
                                "Transactions sent: "
                                span { class: "address-meta-val", "{nonce}" }
                            }
                        }
                    }
                }
                if let Some(err) = error.read().as_ref() {
                    ErrorBox { msg: err.clone() }
                }
                // ── Tabs ────────────────────────────────────────────────
                div { class: "tabs-row",
                    button {
                        class: if *active_tab.read() == "txs" { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| active_tab.set("txs"),
                        "Transactions"
                        span { class: "tab-count", " ({txs_total.read()})" }
                    }
                    button {
                        class: if *active_tab.read() == "transfers" { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| active_tab.set("transfers"),
                        "Token Transfers"
                        span { class: "tab-count", " ({transfers_total.read()})" }
                    }
                }
                // ── Transactions tab ────────────────────────────────────
                if *active_tab.read() == "txs" {
                    div { class: "panel",
                        div { class: "panel-header",
                            span { class: "panel-title", "Transactions" }
                            div { style: "display:flex; align-items:center; gap:12px;",
                                span { style: "color:var(--text-muted); font-size:11px;",
                                    { format!("{} total", txs_total.read()) }
                                }
                                if *txs_total.read() > 0 {
                                    button {
                                        class: "contract-fn-btn contract-fn-btn-read",
                                        style: "padding:4px 12px; font-size:11px;",
                                        disabled: *txs_export_loading.read(),
                                        onclick: export_txs,
                                        if *txs_export_loading.read() { "Exporting…" } else { "⬇ Export CSV" }
                                    }
                                }
                            }
                        }
                        if *txs_loading.read() {
                            Loading { msg: Some("Loading transaction history…".to_string()) }
                        } else if native_txs.read().is_empty() {
                            div { class: "empty-state",
                                div { style: "font-size:32px; margin-bottom:12px;", "📭" }
                                "No transactions found for this address"
                            }
                        } else {
                            div { class: "table-wrapper",
                                table { class: "tx-table",
                                    thead {
                                        tr {
                                            th { "TX HASH" }
                                            th { "BLOCK" }
                                            th { "FROM" }
                                            th { "" }
                                            th { "TO" }
                                            th { "VALUE" }
                                            th { "FEE" }
                                            th { "STATUS" }
                                        }
                                    }
                                    tbody {
                                        for tx in native_txs.read().iter() {
                                            tr {
                                                td { "data-label": "Tx Hash",
                                                    Link { to: Route::TransactionPage { hash: tx.hash.clone() },
                                                        span { class: "hash-cell", "{shorten_hash(&tx.hash)}" }
                                                    }
                                                }
                                                td { "data-label": "Block",
                                                    if let Some(bn) = tx.block_number {
                                                        Link { to: Route::BlockPage { block_number: bn },
                                                            span { class: "hash-cell", "#{bn}" }
                                                        }
                                                    }
                                                }
                                                td { "data-label": "From",
                                                    if tx.from.to_lowercase() == address.to_lowercase() {
                                                        span { class: "chip info", style: "font-size:10px;", "Self" }
                                                    } else {
                                                        Link { to: Route::AddressPage { address: tx.from.clone() },
                                                            span { class: "hash-cell addr-short", "{shorten_addr(&tx.from)}" }
                                                        }
                                                    }
                                                }
                                                td { class: "td-arrow", span { class: "transfer-arrow", "→" } }
                                                td { "data-label": "To",
                                                    if let Some(ref to) = tx.to {
                                                        if to.to_lowercase() == address.to_lowercase() {
                                                            span { class: "chip success", style: "font-size:10px;", "Self" }
                                                        } else {
                                                            Link { to: Route::AddressPage { address: to.clone() },
                                                                span { class: "hash-cell addr-short", "{shorten_addr(to)}" }
                                                            }
                                                        }
                                                    } else {
                                                        span { class: "method-badge method-unknown", "Create" }
                                                    }
                                                }
                                                td { "data-label": "Value", style: "font-family:var(--font-mono); font-size:12px;",
                                                    if tx.value > 0 {
                                                        span { style: "color:var(--accent-green);",
                                                            { format!("{} TEL", format_wei_exact(tx.value)) }
                                                        }
                                                    } else {
                                                        span { class: "td-faint", "—" }
                                                    }
                                                }
                                                td { "data-label": "Fee", style: "font-family:var(--font-mono); font-size:11px; color:var(--text-muted);",
                                                    {
                                                        let fee_wei = tx.gas_used as u128 * tx.gas_price as u128;
                                                        if fee_wei > 0 { format_wei_exact(fee_wei) } else { "—".to_string() }
                                                    }
                                                }
                                                td { "data-label": "Status",
                                                    if tx.status == Some(true) {
                                                        span { class: "chip success", style: "font-size:10px;", "✓" }
                                                    } else if tx.status == Some(false) {
                                                        span { class: "chip failed", style: "font-size:10px;", "✗" }
                                                    } else {
                                                        span { class: "td-faint", "—" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if native_txs.read().len() < *txs_total.read() as usize {
                            div { style: "padding:16px; text-align:center;",
                                button {
                                    class: "contract-fn-btn contract-fn-btn-read",
                                    disabled: *txs_more_loading.read(),
                                    onclick: load_more_txs,
                                    if *txs_more_loading.read() { "Loading…" } else { "Load More" }
                                }
                            }
                        }
                    }
                }
                // ── Token Transfers tab ────────────────────────────────
                if *active_tab.read() == "transfers" {
                    div { class: "panel",
                        div { class: "panel-header",
                            span { class: "panel-title", "ERC-20 Token Transfers" }
                            div { style: "display:flex; align-items:center; gap:12px;",
                                span { style: "color:var(--text-muted); font-size:11px;",
                                    { format!("{} total", transfers_total.read()) }
                                }
                                if *transfers_total.read() > 0 {
                                    button {
                                        class: "contract-fn-btn contract-fn-btn-read",
                                        style: "padding:4px 12px; font-size:11px;",
                                        disabled: *transfers_export_loading.read(),
                                        onclick: export_transfers,
                                        if *transfers_export_loading.read() { "Exporting…" } else { "⬇ Export CSV" }
                                    }
                                }
                            }
                        }
                        div { class: "table-wrapper",
                            if transfers.read().is_empty() {
                                div { class: "empty-state",
                                    div { style: "font-size:32px; margin-bottom:12px;", "📭" }
                                    "No ERC-20 token transfers found for this address"
                                }
                            } else {
                                table { class: "tx-table",
                                    thead {
                                        tr {
                                            th { "TX HASH" }
                                            th { "BLOCK" }
                                            th { "FROM" }
                                            th { "" }
                                            th { "TO" }
                                            th { "TOKEN" }
                                            th { "AMOUNT" }
                                        }
                                    }
                                    tbody {
                                        for transfer in transfers.read().iter() {
                                            tr {
                                                td { "data-label": "Tx Hash",
                                                    Link { to: Route::TransactionPage { hash: transfer.tx_hash.clone() },
                                                        span { class: "hash-cell", "{shorten_hash(&transfer.tx_hash)}" }
                                                    }
                                                }
                                                td { "data-label": "Block",
                                                    Link { to: Route::BlockPage { block_number: transfer.block_number },
                                                        span { class: "hash-cell", "#{transfer.block_number}" }
                                                    }
                                                }
                                                td { "data-label": "From",
                                                    Link { to: Route::AddressPage { address: transfer.from.clone() },
                                                        span { class: "hash-cell addr-short", "{shorten_addr(&transfer.from)}" }
                                                    }
                                                }
                                                td { class: "td-arrow", span { class: "transfer-arrow", "→" } }
                                                td { "data-label": "To",
                                                    Link { to: Route::AddressPage { address: transfer.to.clone() },
                                                        span { class: "hash-cell addr-short", "{shorten_addr(&transfer.to)}" }
                                                    }
                                                }
                                                td { "data-label": "Token",
                                                    if is_native_tel_transfer(&transfer.token_address) {
                                                        span { class: "chip success", style: "font-size:11px;", "TEL" }
                                                    } else if !transfer.token_symbol.is_empty() {
                                                        Link { to: Route::TokenPage { address: transfer.token_address.clone() },
                                                            span { class: "chip info", style: "font-size:11px;", "{transfer.token_symbol}" }
                                                        }
                                                    } else {
                                                        Link { to: Route::TokenPage { address: transfer.token_address.clone() },
                                                            span { class: "hash-cell addr-short", "{shorten_addr(&transfer.token_address)}" }
                                                        }
                                                    }
                                                }
                                                td { "data-label": "Amount", style: "color:var(--accent-green); font-weight:600; font-family:var(--font-mono); font-size:12px;",
                                                    { format_transfer_amount(transfer) }
                                                }
                                            }
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
        }
    }
}
