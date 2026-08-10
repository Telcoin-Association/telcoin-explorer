// src/pages/address.rs
use dioxus::prelude::*;
use crate::router::Route;
use crate::services::rpc::{
    is_contract,
    get_balance, get_tx_count, get_address_txs, get_address_transfers,
    TokenTransfer, Transaction, shorten_hash, shorten_addr,
    CONSENSUS_REGISTRY,
};
use crate::components::loading::{Loading, ErrorBox, CopyButton};

#[component]
pub fn AddressPage(address: String) -> Element {
    let mut balance: Signal<Option<f64>>          = use_signal(|| None);
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

    let addr_clone = address.clone();
    use_effect(move || {
        let address = addr_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loading.set(true);
            let (bal_res, count_res) = futures::join!(
                get_balance(&address),
                get_tx_count(&address),
            );
            match bal_res   { Ok(b) => balance.set(Some(b)), Err(e) => error.set(Some(e)) }
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
    });

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
                        if let Some(bal) = *balance.read() {
                            div { class: "address-balance-big",
                                { format!("{:.6}", bal) }
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
                            span { style: "color:var(--text-muted); font-size:11px;",
                                { format!("{} total", txs_total.read()) }
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
                                                td {
                                                    Link { to: Route::TransactionPage { hash: tx.hash.clone() },
                                                        span { class: "hash-cell", "{shorten_hash(&tx.hash)}" }
                                                    }
                                                }
                                                td {
                                                    if let Some(bn) = tx.block_number {
                                                        Link { to: Route::BlockPage { block_number: bn },
                                                            span { class: "hash-cell", "#{bn}" }
                                                        }
                                                    }
                                                }
                                                td {
                                                    if tx.from.to_lowercase() == address.to_lowercase() {
                                                        span { class: "chip info", style: "font-size:10px;", "Self" }
                                                    } else {
                                                        Link { to: Route::AddressPage { address: tx.from.clone() },
                                                            span { class: "hash-cell addr-short", "{shorten_addr(&tx.from)}" }
                                                        }
                                                    }
                                                }
                                                td { span { class: "transfer-arrow", "→" } }
                                                td {
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
                                                td { style: "font-family:var(--font-mono); font-size:12px;",
                                                    if tx.value_tel > 0.0 {
                                                        span { style: "color:var(--accent-green);",
                                                            { format!("{:.4} TEL", tx.value_tel) }
                                                        }
                                                    } else {
                                                        span { class: "td-faint", "—" }
                                                    }
                                                }
                                                td { style: "font-family:var(--font-mono); font-size:11px; color:var(--text-muted);",
                                                    {
                                                        let fee = tx.gas_used as f64 * tx.gas_price as f64 / 1e18;
                                                        if fee > 0.0 { format!("{:.6}", fee) } else { "—".to_string() }
                                                    }
                                                }
                                                td {
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
                            span { style: "color:var(--text-muted); font-size:11px;",
                                { format!("{} total", transfers_total.read()) }
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
                                                td {
                                                    Link { to: Route::TransactionPage { hash: transfer.tx_hash.clone() },
                                                        span { class: "hash-cell", "{shorten_hash(&transfer.tx_hash)}" }
                                                    }
                                                }
                                                td {
                                                    Link { to: Route::BlockPage { block_number: transfer.block_number },
                                                        span { class: "hash-cell", "#{transfer.block_number}" }
                                                    }
                                                }
                                                td {
                                                    Link { to: Route::AddressPage { address: transfer.from.clone() },
                                                        span { class: "hash-cell addr-short", "{shorten_addr(&transfer.from)}" }
                                                    }
                                                }
                                                td { span { class: "transfer-arrow", "→" } }
                                                td {
                                                    Link { to: Route::AddressPage { address: transfer.to.clone() },
                                                        span { class: "hash-cell addr-short", "{shorten_addr(&transfer.to)}" }
                                                    }
                                                }
                                                td {
                                                    if !transfer.token_symbol.is_empty() {
                                                        Link { to: Route::AddressPage { address: transfer.token_address.clone() },
                                                            span { class: "chip info", style: "font-size:11px;", "{transfer.token_symbol}" }
                                                        }
                                                    } else {
                                                        Link { to: Route::AddressPage { address: transfer.token_address.clone() },
                                                            span { class: "hash-cell addr-short", "{shorten_addr(&transfer.token_address)}" }
                                                        }
                                                    }
                                                }
                                                td { style: "color:var(--accent-green); font-weight:600; font-family:var(--font-mono); font-size:12px;",
                                                    { format!("{:.4}", transfer.amount) }
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
