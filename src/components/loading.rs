// src/components/loading.rs
use dioxus::prelude::*;
/// Signals can be dropped out from under an in-flight async task if the
/// component unmounts (e.g. the user navigates away) before the task
/// resolves -- `.set()` panics in that case (ValueDroppedError). Both
/// CopyButton and AddrDisplay have a ~1.5s delayed reset after copying,
/// which is plenty of time for someone to click a link and navigate away
/// before it fires. Writes from inside spawn_local go through this instead,
/// which just silently no-ops if the signal's gone.
fn safe_set<T: 'static>(mut sig: Signal<T>, val: T) {
    if let Ok(mut w) = sig.try_write() {
        *w = val;
    }
}

#[component]
pub fn Loading(msg: Option<String>) -> Element {
    let message = msg.unwrap_or_else(|| "Loading…".to_string());
    rsx! {
        div { class: "loading-wrapper",
            div { class: "spinner" }
            p { class: "loading-text", "{message}" }
        }
    }
}

#[component]
pub fn ErrorBox(msg: String) -> Element {
    rsx! {
        div { class: "error-box",
            "⚠ Error: {msg}"
        }
    }
}

// ── Copy to clipboard component ────────────────────────────────────────────

#[component]
pub fn CopyButton(text: String) -> Element {
    let mut copied = use_signal(|| false);
    let text_clone = text.clone();

    rsx! {
        span { class: "copy-wrap",
            button {
                class: if *copied.read() { "copy-btn copied" } else { "copy-btn" },
                onclick: move |_: Event<MouseData>| {
                    let t = text_clone.clone();
                    let mut copied = copied.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        // Use JSON.stringify to safely encode the string for clipboard
                        let script = format!("navigator.clipboard.writeText({})", 
                            serde_json::to_string(&t).unwrap_or_default());
                        let _ = js_sys::eval(&script);
                        safe_set(copied, true);
                        gloo_timers::future::TimeoutFuture::new(1500).await;
                        safe_set(copied, false);
                    });
                },
                if *copied.read() { "✓ Copied" } else { "Copy" }
            }
        }
    }
}


// ── Address display with tooltip + copy ───────────────────────────────────

#[component]
pub fn AddrDisplay(address: String, short: String) -> Element {
    let mut copied = use_signal(|| false);
    let addr_clone = address.clone();
    let addr_copy  = address.clone();

    rsx! {
        span { class: "addr-wrap",
            span { class: "hash-cell", "{short}" }
            // Tooltip with full address
            span { class: "addr-tooltip", "{addr_clone}" }
            // Copy button
            button {
                class: if *copied.read() { "addr-copy-btn copied" } else { "addr-copy-btn" },
                title: "Copy address",
                onclick: move |_: Event<MouseData>| {
                    let t = addr_copy.clone();
                    let mut copied = copied.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        let mut script = String::from("navigator.clipboard.writeText(\"");
                        for ch in t.chars() {
                            match ch {
                                '"'  => script.push_str("\\\""),
                                '\\' => script.push_str("\\\\"),
                                c    => script.push(c),
                            }
                        }
                        script.push_str("\")");
                        let _ = js_sys::eval(&script);
                        safe_set(copied, true);
                        gloo_timers::future::TimeoutFuture::new(1500).await;
                        safe_set(copied, false);
                    });
                },
                if *copied.read() {
                    // Checkmark icon
                    svg { width:"13", height:"13", view_box:"0 0 24 24", fill:"none",
                        stroke:"currentColor", stroke_width:"2.5",
                        stroke_linecap:"round", stroke_linejoin:"round",
                        path { d:"M20 6 9 17l-5-5" }
                    }
                } else {
                    // Clipboard icon
                    svg { width:"13", height:"13", view_box:"0 0 24 24", fill:"none",
                        stroke:"currentColor", stroke_width:"2",
                        stroke_linecap:"round", stroke_linejoin:"round",
                        rect { x:"9", y:"9", width:"13", height:"13", rx:"2", ry:"2" }
                        path { d:"M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
                    }
                }
            }
        }
    }
}
