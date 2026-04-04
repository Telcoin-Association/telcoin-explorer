// src/components/footer.rs
use dioxus::prelude::*;

#[component]
pub fn Footer() -> Element {
    rsx! {
        footer { class: "site-footer",
            div { class: "footer-inner",
                div { class: "footer-left",
                    span { class: "footer-brand", "TelScan" }
                    span { class: "footer-sep", "·" }
                    span { class: "footer-sub", "Adiri Testnet Explorer" }
                }
                div { class: "footer-links",
                    a {
                        href: "https://www.telcoin.network",
                        target: "_blank",
                        class: "footer-link",
                        "🌐 Telcoin.network"
                    }
                    a {
                        href: "https://x.com/TelcoinTAO",
                        target: "_blank",
                        class: "footer-link",
                        svg {
                            width: "14", height: "14",
                            view_box: "0 0 24 24",
                            fill: "currentColor",
                            style: "display:inline;vertical-align:middle;margin-right:4px;",
                            path { d: "M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-4.714-6.231-5.401 6.231H2.748l7.73-8.835L1.254 2.25H8.08l4.253 5.622zm-1.161 17.52h1.833L7.084 4.126H5.117z" }
                        }
                        "@TelcoinTAO"
                    }
                }
                div { class: "footer-right",
                    span { class: "footer-version", "v0.1.8" }
                    span { class: "footer-copy", "2026 · © Telcoin Association" }
                }
            }
        }
    }
}
