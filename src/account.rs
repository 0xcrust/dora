#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use fantoccini::{Client, ClientBuilder};
use select::{
    document::Document,
    predicate::{Class, Name, Predicate},
};
use tokio::sync::Mutex;
use serde_json::map::Map;
use chrono::{DateTime, Utc};

type Error = Box<dyn std::error::Error>;

#[derive(Default)]
pub struct AccountDetails {
    address: String,
    balance: u64,
    owner: String,
    data_size: u64,
    executable: bool,
    transactions: Vec<Transactions>
}
struct Transactions {
    signature: String,
    block: u64,
    timestamp: DateTime<Utc>,
    success: bool,
    confirmation_status: String,
    confirmations: String,
    slot: u64,
    recent_blockhash: String,
    fee_lamports: u64,
    transaction_version: String,
    accounts: Vec<TxAccount>,
}

struct TxAccount {
    address: String,
    is_writable: bool,
    is_signer: bool,
    is_fee_payer: bool,
    is_program: bool,
    sol_change_lamports: u64,
}

pub enum Cluster {
    Devnet,
    Mainnet,
}

async fn new_webdriver_client() -> Result<Mutex<Client>, Error> {
    let mut caps = Map::new();
    let options = serde_json::json!({ "args": ["--headless", "--disable-gpu"] });
    caps.insert("goog:chromeOptions".to_string(), options);
    let webdriver_client = ClientBuilder::rustls()
        .capabilities(caps)
        .connect("http://localhost:4444")
        .await?;
    Ok(Mutex::new(webdriver_client))
}

pub async fn scrape_account_details(address: String, cluster: Cluster) -> Result<AccountDetails, Error> {
    let url = match cluster {
        Cluster::Mainnet => format!("https://explorer.solana.com/address/{}?cluster=mainnet-beta", address),
        Cluster::Devnet => format!("https://explorer.solana.com/address/{}?cluster=devnet", address)
    };
    log::debug!("url: {}", url);
    log::debug!("");
    let client = new_webdriver_client().await.expect("Client not created");
    let mut webdriver = client.lock().await;
    webdriver.goto(&url).await?;
    let html = webdriver.source().await?;

    let document = Document::from(html.as_str());
    log::info!("document: {:?}", document);

    Ok(AccountDetails::default())
}