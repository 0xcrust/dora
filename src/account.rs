use crate::Error;
use chrono::{DateTime, NaiveDateTime, Utc};
use fantoccini::Client;
use select::{
    document::Document,
    predicate::{Class, Name, Predicate},
};
use serde::Serialize;
use std::{thread, time::Duration};
use tokio::sync::Mutex;

#[derive(Default, Debug, Serialize)]
pub struct AccountDetails {
    pub address: String,
    pub balance: f64,
    pub owner: String,
    pub data_size: f64,
    pub executable: bool,
    pub recent_transactions: Vec<Transaction>,
}

#[derive(Default, Debug, Serialize)]
pub struct Transaction {
    pub signature: String,
    pub block: u64,
    pub time: String,
    pub result: String,
}

pub async fn get_account_info(
    url: &str,
    txns_limit: usize,
    wait_time: u64,
    client: &Mutex<Client>,
) -> Result<AccountDetails, Error> {
    log::info!("Parsing data for url: {}", url);
    let mut webdriver = client.lock().await;
    webdriver.goto(url).await?;
    log::info!("Hold on. Waiting for page load...");
    thread::sleep(Duration::from_secs(wait_time));
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

    let mut list = document.find(Class("list"));
    _ = list.next();
    let transaction_nodes = list.next().unwrap().children();
    let mut transactions: Vec<Transaction> = vec![];

    for transaction in transaction_nodes {
        if transactions.len() == txns_limit {
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

        let new_transaction = Transaction {
            signature,
            block,
            time,
            result,
        };

        transactions.push(new_transaction);
    }

    let details = AccountDetails {
        address,
        balance,
        owner,
        data_size,
        executable,
        recent_transactions: transactions,
    };

    Ok(details)
}
