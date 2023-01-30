#![allow(dead_code)]

mod account;
mod transaction;
use account::Cluster;

use account::new_webdriver_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();

    let address = std::env::var("ADDRESS").expect("Failed getting account to scrape");
    let transaction = std::env::var("TX").expect("Failed getting transaction to scrape");

    let client = new_webdriver_client().await.expect("Client not created");

    account::get_account_details(address.to_string(), Cluster::Devnet, 10, &client).await?;
    transaction::get_transaction_info(transaction.to_string(), Cluster::Devnet, &client).await?;
    Ok(())
}
