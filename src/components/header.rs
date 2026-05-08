// src/components/header.rs
use dioxus::prelude::*;
use crate::router::Route;

const LOGO: Asset = asset!("/assets/telcoin-logo.svg");

#[component]
pub fn Header() -> Element {
    let mut dark_mode = use_signal(|| true);

    use_effect(move || {
        let is_dark = *dark_mode.read();
        let window = web_sys::window().unwrap();
        let doc = window.document().unwrap();
        let html = doc.document_element().unwrap();
        if is_dark {
            html.remove_attribute("data-theme").ok();
        } else {
            html.set_attribute("data-theme", "light").ok();
        }
    });

    rsx! {
        header { class: "header",
            div { class: "header-inner",
                Link { to: Route::HomePage {},
                    div { class: "logo",
                        img { src: LOGO, class: "logo-img", alt: "Telcoin" }
                        div { class: "logo-text-wrap",
                            span { class: "logo-name", "TelScan" }
                            span { class: "logo-badge", "Adiri Testnet" }
                        }
                    }
                }
                div { style: "flex:1;" }
                nav { class: "header-nav",
                    Link { to: Route::BlocksPage { page: 0 },
                        class: "header-nav-link",
                        "Blocks"
                    }
                    Link { to: Route::TransactionsPage { page: 0 },
                        class: "header-nav-link",
                        "Transactions"
                    }
                    Link { to: Route::EpochsPage {},
                        class: "header-nav-link",
                        "Epochs"
                    }
                    Link { to: Route::ValidatorsPage {},
                        class: "header-nav-link",
                        "Validators"
                    }
                    a {
                        href: "https://telcoin.network/faucet",
                        target: "_blank",
                        class: "header-nav-faucet",
                        "Faucet ↗"
                    }
                    button {
                        class: "theme-toggle",
                        title: if *dark_mode.read() { "Switch to light mode" } else { "Switch to dark mode" },
                        onclick: move |_: Event<MouseData>| {
                            let current = *dark_mode.read();
                            dark_mode.set(!current);
                        },
                        if *dark_mode.read() { "☀" } else { "🌙" }
                    }
                }
            }
        }
    }
}
