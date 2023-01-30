use crate::Error;
use clap::Parser;
use fantoccini::{Client, ClientBuilder};
use serde::Deserialize;
use serde_json::map::Map;
use tokio::sync::Mutex;

#[derive(Parser)]
#[clap(author, version, about, long_about=None)]
pub struct Args {
    #[clap(short, long, help = "account|transaction")]
    pub parse: String,

    #[clap(short, long, help = "Id of the account|tx to be parsed")]
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub cluster: String,
    pub wait_time: u64,
    pub tx_limit: u64,
    pub output_file_path: String,
}

#[derive(Debug)]
pub enum Cluster {
    Devnet,
    Mainnet,
    Testnet,
}

pub enum Command {
    Account,
    Transaction,
}

pub async fn new_webdriver_client() -> Result<Mutex<Client>, Error> {
    let mut caps = Map::new();
    let options = serde_json::json!({ "args": ["--headless", "--disable-gpu"] });
    caps.insert("goog:chromeOptions".to_string(), options);
    let webdriver_client = ClientBuilder::rustls()
        .capabilities(caps)
        .connect("http://localhost:4444")
        .await?;
    log::info!("Webdriver client constructed!");
    Ok(Mutex::new(webdriver_client))
}

pub fn construct_url(cluster: &Cluster, command: &Command, id: &str) -> String {
    let route = match command {
        Command::Account => "address",
        Command::Transaction => "tx",
    };
    match cluster {
        Cluster::Mainnet => format!("https://explorer.solana.com/{}/{}", route, id),
        Cluster::Devnet => format!(
            "https://explorer.solana.com/{}/{}?cluster=devnet",
            route, id
        ),
        Cluster::Testnet => format!(
            "https://explorer.solana.com/{}/{}?cluster=mainnet-beta",
            route, id
        ),
    }
}
