use args::{Args, Command};
use clap::Parser;
use fantoccini::{Client, ClientBuilder};
use serde_json::map::Map;
use std::{fs::File, io::Write};
use tokio::sync::Mutex;

mod account;
mod args;
mod transaction;

pub type Error = Box<dyn std::error::Error>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    env_logger::init();

    let client = new_webdriver_client().await.expect("Client not created");

    let args = Args::parse();
    let cluster = match args.cluster.to_lowercase().trim() {
        "mainnet" => Cluster::Mainnet,
        "devnet" => Cluster::Devnet,
        "testnet" => Cluster::Testnet,
        _ => {
            log::info!("Invalid cluster..Defaulting to mainnet");
            Cluster::Mainnet
        }
    };

    let result = match args.command {
        Some(Command::Account { tx_limit }) => {
            let url = construct_url(&cluster, &Command::Account { tx_limit }, &args.id);
            let result = account::get_account_info(&url, tx_limit as usize, &client)
                .await
                .expect("Failed getting account info");
            serde_json::to_string_pretty(&result).expect("Failed converting result to json")
        }
        Some(Command::Transaction) => {
            let url = construct_url(&cluster, &Command::Transaction, &args.id);
            let result = transaction::get_transaction_info(&url, &client)
                .await
                .expect("Failed getting transaction info");
            serde_json::to_string_pretty(&result).expect("Failed converting result to json")
        }
        None => {
            panic!("Program shutdown, no command detected");
        }
    };

    let path = match args.output {
        Some(file_path) => file_path,
        None => "results.json".to_string(),
    };

    let mut handle = File::create(&path).expect("Invalid file path");
    handle
        .write_all(result.as_bytes())
        .unwrap_or_else(|_| panic!("Failed writing to {}", path));

    log::info!("Results retrieved!. Check them out at {}", &path);

    Ok(())
}

pub enum Cluster {
    Devnet,
    Mainnet,
    Testnet,
}

pub async fn new_webdriver_client() -> Result<Mutex<Client>, Error> {
    let mut caps = Map::new();
    let options = serde_json::json!({ "args": ["--headless", "--disable-gpu"] });
    caps.insert("goog:chromeOptions".to_string(), options);
    let webdriver_client = ClientBuilder::rustls()
        .capabilities(caps)
        .connect("http://localhost:4444")
        .await?;
    Ok(Mutex::new(webdriver_client))
}

fn construct_url(cluster: &Cluster, command: &Command, id: &str) -> String {
    let route = match command {
        Command::Account { tx_limit: _ } => "address",
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
