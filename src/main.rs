mod account;
use account::Cluster;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();
    
    let address = std::env::var("ADDRESS").expect("Failed getting account to scrape");
    account::scrape_account_details(address.to_string(), Cluster::Devnet).await?;
    Ok(())
}




