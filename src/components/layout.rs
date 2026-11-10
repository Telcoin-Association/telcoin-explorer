// src/components/layout.rs
use dioxus::prelude::*;
use crate::components::header::Header;
use crate::components::footer::Footer;

const CSS:  Asset = asset!("/assets/main.css");
const LOGO: Asset = asset!("/assets/telcoin-logo.svg");

#[component]
pub fn Layout() -> Element {
    rsx! {
        document::Stylesheet { href: CSS }
        document::Link { rel: "icon", href: LOGO, r#type: "image/svg+xml" }
        document::Meta { property: "og:title", content: "TelScan — Telcoin Network Explorer" }
        document::Meta { property: "og:description", content: "Explore blocks, transactions, and addresses on the Telcoin Network (Adiri Testnet)" }
        document::Meta { property: "og:image", content: "https://telscan.xyz/assets/telcoin-logo.svg" }
        document::Meta { property: "og:type", content: "website" }
        document::Meta { name: "twitter:card", content: "summary" }
        document::Meta { name: "twitter:title", content: "TelScan — Telcoin Network Explorer" }
        document::Meta { name: "twitter:description", content: "Explore blocks, transactions, and addresses on the Telcoin Network (Adiri Testnet)" }
        div { class: "app-wrap",
            Header {}
            main { class: "app-main",
                Outlet::<crate::router::Route> {}
            }
            Footer {}
        }
    }
}
