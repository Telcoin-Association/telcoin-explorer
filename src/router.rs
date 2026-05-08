use dioxus::prelude::*;
use crate::components::layout::Layout;
use crate::pages::epochs::EpochsPage;
use crate::pages::contract::ContractPage;
use crate::pages::{
    home::HomePage,
    block::BlockPage,
    blocks::BlocksPage,
    transaction::TransactionPage,
    transactions::TransactionsPage,
    address::AddressPage,
    token::TokenPage,
    validators::ValidatorsPage,
    not_found::NotFoundPage,
};

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[layout(Layout)]
        #[route("/")]
        HomePage {},
        #[route("/blocks/:page")]
        BlocksPage { page: u64 },
        #[route("/block/:block_number")]
        BlockPage { block_number: u64 },
        #[route("/txs/:page")]
        TransactionsPage { page: u64 },
        #[route("/tx/:hash")]
        TransactionPage { hash: String },
        #[route("/address/:address")]
        AddressPage { address: String },
        #[route("/token/:address")]
        TokenPage { address: String },
        #[route("/validators")]
        ValidatorsPage {},
        #[route("/epochs")]
        EpochsPage {},
        #[route("/contract/:address")]
        ContractPage { address: String },
    #[end_layout]
    #[route("/:..segments")]
    NotFoundPage { segments: Vec<String> },
}
