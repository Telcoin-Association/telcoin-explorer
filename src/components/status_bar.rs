// src/components/status_bar.rs
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use crate::services::rpc::{get_network_stats, NetworkStats, CHAIN_ID, NATIVE_TOKEN};

/// See src/components/loading.rs for why this exists: a background loop's
/// .set() call can panic if it fires at the exact moment of a hard page
/// navigation (window.location.set_href, used by the search box), which
/// tears down the whole WASM app mid-flight rather than giving components a
/// chance to unmount gracefully first.
fn safe_set<T: 'static>(mut sig: Signal<T>, val: T) {
    if let Ok(mut w) = sig.try_write() {
        *w = val;
    }
}

#[component]
pub fn StatusBar() -> Element {
    let stats: Signal<Option<NetworkStats>> = use_signal(|| None);
    let online = use_signal(|| true);

    use_effect(move || {
        let mut stats = stats.clone();
        let mut online = online.clone();
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                match get_network_stats().await {
                    Ok(s) => { safe_set(stats, Some(s)); safe_set(online, true); }
                    Err(_) => { safe_set(online, false); }
                }
                TimeoutFuture::new(12_000).await;
            }
        });
    });

    let dot_class = if *online.read() { "status-dot" } else { "status-dot offline" };
    let status_text = if *online.read() { "LIVE" } else { "OFFLINE" };

    rsx! {
        div { class: "status-bar",
            div { class: "status-bar-inner",
                div { class: "status-item",
                    div { class: "{dot_class}" }
                    span { class: "status-label", "Network" }
                    span { class: "status-value", "{status_text}" }
                }
                div { class: "status-item",
                    span { class: "status-label", "Chain" }
                    span { class: "status-value", "{CHAIN_ID}" }
                }
                div { class: "status-item",
                    span { class: "status-label", "Token" }
                    span { class: "status-value", "{NATIVE_TOKEN}" }
                }
                if let Some(s) = stats.read().as_ref() {
                    div { class: "status-item",
                        span { class: "status-label", "Block" }
                        span { class: "status-value", "#{s.latest_block}" }
                    }
                    div { class: "status-item",
                        span { class: "status-label", "Gas" }
                        span { class: "status-value", "{s.gas_price_gwei:.2} Gwei" }
                    }
                } else {
                    div { class: "status-item",
                        span { class: "status-label", "Block" }
                        span { class: "status-value", "Loading…" }
                    }
                }
                div { class: "status-item",
                    span { class: "status-label", "RPC" }
                    span { class: "status-value", "rpc.telcoin.network" }
                }
            }
        }
    }
}
