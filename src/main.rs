use clap::Parser;
use config::{Args, Cluster, Command, Config};
use std::{fs::File, io::Write};

mod account;
mod config;
mod transaction;

pub type Error = Box<dyn std::error::Error>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    env_logger::init();

    let config_file = File::open("config.yml").expect(
        "Missing config.yml with required fields: cluster, wait_time, tx_limit, output_file_path",
    );
    let config: Config = serde_yaml::from_reader(config_file).expect("Couldn't read config values");
    log::info!("Retrieved configuration from config.yml: {:?}", &config);

    let client = config::new_webdriver_client()
        .await
        .expect("Client not created");

    let args = Args::parse();
    let cluster = match config.cluster.to_lowercase().trim() {
        "mainnet" => Cluster::Mainnet,
        "devnet" => Cluster::Devnet,
        "testnet" => Cluster::Testnet,
        _ => {
            log::info!("Invalid cluster..Defaulting to mainnet");
            Cluster::Mainnet
        }
    };
    log::info!("Cluster detected: {:?}", cluster);

    let result = match args.parse.to_lowercase().trim() {
        "account" => {
            let url = config::construct_url(&cluster, &Command::Account, &args.id);
            let result = account::get_account_info(
                &url,
                config.tx_limit as usize,
                config.wait_time,
                &client,
            )
            .await
            .expect("Failed getting account info");
            log::info!("Retrieved results for account {}. Converting...", &args.id);
            serde_json::to_string_pretty(&result).expect("Failed converting result to json")
        }
        "transaction" => {
            let url = config::construct_url(&cluster, &Command::Transaction, &args.id);
            let result = transaction::get_transaction_info(&url, config.wait_time, &client)
                .await
                .expect("Failed getting transaction info");
            log::info!("Retrieved results for account {}. Converting...", &args.id);
            serde_json::to_string_pretty(&result).expect("Failed converting result to json")
        }
        _ => {
            panic!("Program shutdown, no command detected");
        }
    };

    let path = config.output_file_path;
    let mut handle = File::create(&path).expect("Invalid file path");
    handle
        .write_all(result.as_bytes())
        .unwrap_or_else(|_| panic!("Failed writing to {}", path));
    log::info!("Wrote results to {}", &path);

    Ok(())
}
