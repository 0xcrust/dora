use chrono::{DateTime, NaiveDateTime, Utc};
use fantoccini::{Client, ClientBuilder};
use select::{
    document::Document,
    predicate::{Class, Name, Predicate},
};
use serde_json::map::Map;
use std::thread;
use std::time::Duration;
use tokio::sync::Mutex;

pub type Error = Box<dyn std::error::Error>;

#[derive(Default, Debug)]
pub struct AccountDetails {
    pub address: String,
    pub balance: f64,
    pub owner: String,
    pub data_size: f64,
    pub executable: bool,
    pub recent_transactions: Vec<TransactionSummary>,
}

#[derive(Default, Debug)]
pub struct TransactionSummary {
    pub signature: String,
    pub block: u64,
    pub time: String,
    pub result: String,
}

pub enum Cluster {
    Devnet,
    Mainnet,
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

pub async fn get_account_details(
    address: String,
    cluster: Cluster,
    txns_count: usize,
    client: &Mutex<Client>,
) -> Result<AccountDetails, Error> {
    let url = match cluster {
        Cluster::Mainnet => format!(
            "https://explorer.solana.com/address/{}?cluster=mainnet-beta",
            address
        ),
        Cluster::Devnet => format!(
            "https://explorer.solana.com/address/{}?cluster=devnet",
            address
        ),
    };

    log::info!("url: {}", url);
    let mut webdriver = client.lock().await;
    webdriver.goto(&url).await?;
    thread::sleep(Duration::from_secs(20));
    let html = webdriver.source().await?;

    let document = Document::from(html.as_str());
    let mut table = document.find(Class("table-responsive").descendant(Name("tr")));
    let address = table
        .next()
        .unwrap()
        .find(Class("font-monospace").descendant(Name("span")))
        .next()
        .unwrap()
        .text();
    let balance = table
        .next()
        .unwrap()
        .find(Class("font-monospace"))
        .next()
        .unwrap()
        .text()
        .parse::<f64>()
        .unwrap();
    let data_size = table
        .next()
        .unwrap()
        .find(Class("text-lg-end"))
        .next()
        .unwrap()
        .text()
        .split_whitespace()
        .next()
        .unwrap()
        .parse::<f64>()
        .unwrap();
    let owner = table
        .next()
        .unwrap()
        .find(Class("font-monospace").descendant(Name("a")))
        .next()
        .unwrap()
        .text();
    let executable = table
        .next()
        .unwrap()
        .find(Class("text-lg-end"))
        .next()
        .unwrap()
        .text();
    let executable = match executable.trim().to_ascii_lowercase().as_str() {
        "no" => false,
        "yes" => true,
        _ => panic!("Unexpected result"),
    };

    let mut txns: Vec<TransactionSummary> = vec![];

    let mut list = document.find(Class("list"));

    _ = list.next();
    let transactions = list.next().unwrap().children(); //.find(Name("tr"));

    for transaction in transactions {
        if txns.len() == txns_count {
            break;
        }
        let mut details = transaction.find(Name("td"));
        let signature = details
            .next()
            .unwrap()
            .find(Name("a"))
            .next()
            .unwrap()
            .text();
        let block = details.next().unwrap().find(Name("a")).next().unwrap();
        let block: u64 = block.text().split(',').collect::<String>().parse().unwrap();
        let timestamp = details.next().unwrap().find(Name("time")).next().unwrap();
        let _ = details.next();
        let result = details.next().unwrap().first_child().unwrap().text();

        let time = {
            let timestamp: i64 = timestamp.attr("datetime").unwrap().parse().unwrap();
            let utc = DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp_opt(timestamp, 0).unwrap(),
                Utc,
            );
            format!("{}", utc.format("%Y-%m-%d %H:%M:%S"))
        };

        let new_transaction = TransactionSummary {
            signature,
            block,
            time,
            result,
        };

        txns.push(new_transaction);
    }

    let details = AccountDetails {
        address,
        balance,
        owner,
        data_size,
        executable,
        recent_transactions: txns,
    };
    log::info!("details: {:#?}", details);

    let details = AccountDetails {
        ..Default::default()
    };

    Ok(details)
}
