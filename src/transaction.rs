use crate::Error;
use fantoccini::Client;
use select::{
    document::Document,
    node::Node,
    predicate::{Class, Name, Predicate},
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    thread,
    time::Duration,
};
use tokio::sync::Mutex;

#[derive(Debug, Default, Serialize)]
pub struct Transaction {
    pub overview: TxOverview,
    pub token_balances: Option<Vec<TokenAccountInfo>>,
    pub account_inputs: Vec<TxAccountInput>,
    pub instructions: Vec<Instruction>,
}

#[derive(Default, Debug, Serialize)]
pub struct Instruction {
    description: String,
    program: String,
    accounts: Vec<(String, IxAccountContext)>,
    additional_info: HashMap<String, String>,
    hex: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct IxAccountContext {
    address: String,
    attributes: Option<Vec<String>>,
}

#[derive(Default, Debug, Serialize)]
pub struct TxAccountInput {
    address: String,
    attributes: Vec<String>,
    sol_change: f64,
    post_balance: f64,
}

#[derive(Default, Debug, Serialize)]
pub struct TxOverview {
    signature: String,
    result: String,
    timestamp: String,
    confirmation_status: String,
    confirmations: String,
    slot: u64,
    recent_blockhash: String,
    fee: f64,
    transaction_version: String,
}

#[derive(Default, Debug, Serialize)]
pub struct TokenAccountInfo {
    address: String,
    token_name: String,
    token_url: String,
    change: f64,
    post_balance: String,
}

pub async fn get_transaction_info(url: &str, client: &Mutex<Client>) -> Result<Transaction, Error> {
    log::info!("url: {}", url);

    let mut webdriver = client.lock().await;
    webdriver.goto(url).await?;
    thread::sleep(Duration::from_secs(20));
    let html = webdriver.source().await?;

    let document = Document::from(html.as_str());

    let mut overview = TxOverview::default();
    let mut account_inputs = vec![];
    let mut token_balances: Option<Vec<TokenAccountInfo>> = None;
    let mut instructions = vec![];

    let cards = document
        .find(Class("card"))
        .filter(|x| x.parent().unwrap().attr("class").unwrap() != "inner-cards");

    for card in cards {
        let title = card.find(Class("card-header-title")).next().unwrap().text();
        match title.trim() {
            "Overview" => {
                log::info!("Parsing tx overview details...");
                overview = parse_overview(&card);
            }
            "Account Input(s)" => {
                log::info!("Parsing account inputs");
                account_inputs = parse_account_inputs(&card);
            }
            "Token Balances" => {
                log::info!("Parsing token balances...");
                token_balances = Some(parse_token_balances(&card));
            }
            "Program Instruction Logs" => {
                log::info!("Parsing Program Ix Logs is unimplemented. Skipping...");
            }
            _ => {
                log::info!("Parsing instruction...");
                instructions.push(parse_instruction(&card))
            }
        }
    }

    let transaction = Transaction {
        overview,
        token_balances,
        account_inputs,
        instructions,
    };

    Ok(transaction)
}

fn parse_overview(overview: &Node) -> TxOverview {
    let mut items = overview.find(Class("list").descendant(Name("tr")));
    let signature = items
        .next()
        .unwrap()
        .find(Class("font-monospace"))
        .next()
        .unwrap()
        .text();
    let result = items
        .next()
        .unwrap()
        .find(Class("bg-success-soft"))
        .next()
        .unwrap()
        .text();
    let timestamp = items
        .next()
        .unwrap()
        .find(Class("font-monospace"))
        .next()
        .unwrap()
        .text();
    let confirmation_status = items
        .next()
        .unwrap()
        .find(Class("text-lg-end"))
        .next()
        .unwrap()
        .text();
    let confirmations = items
        .next()
        .unwrap()
        .find(Class("text-lg-end"))
        .next()
        .unwrap()
        .text();
    let slot = items.next().unwrap().find(Name("a")).next().unwrap().text();
    let slot = slot.split(',').collect::<String>().parse().unwrap();
    let recent_blockhash = items
        .next()
        .unwrap()
        .find(Class("text-lg-end"))
        .next()
        .unwrap()
        .text();
    let fee: f64 = items
        .next()
        .unwrap()
        .find(Class("font-monospace"))
        .next()
        .unwrap()
        .text()
        .parse()
        .unwrap();
    let transaction_version = items
        .next()
        .unwrap()
        .find(Class("text-lg-end"))
        .next()
        .unwrap()
        .text();

    TxOverview {
        signature,
        result,
        timestamp,
        confirmation_status,
        confirmations,
        slot,
        recent_blockhash,
        fee,
        transaction_version,
    }
}

fn parse_token_balances(token_balances: &Node) -> Vec<TokenAccountInfo> {
    let token_balances = token_balances.find(Class("list").descendant(Name("tr")));
    let mut token_accounts_info = vec![];

    for info in token_balances {
        let mut child_nodes = info.children();
        let address = child_nodes
            .next()
            .unwrap()
            .find(Name("a"))
            .next()
            .unwrap()
            .text();
        let (token_name, token_url) = {
            let node = child_nodes.next().unwrap().find(Name("a")).next().unwrap();
            (node.text(), node.attr("href").unwrap().to_string())
        };
        let change: f64 = child_nodes
            .next()
            .unwrap()
            .first_child()
            .unwrap()
            .text()
            .parse()
            .unwrap();
        let post_balance = child_nodes.next().unwrap().text();

        let new_token_info = TokenAccountInfo {
            address,
            token_name,
            token_url: normalize_url(&token_url),
            change,
            post_balance,
        };
        token_accounts_info.push(new_token_info);
    }
    token_accounts_info
}

fn parse_account_inputs(account_inputs: &Node) -> Vec<TxAccountInput> {
    let mut accounts_vec: Vec<TxAccountInput> = vec![];
    let tx_accounts = account_inputs.find(Class("list").descendant(Name("tr")));

    for account in tx_accounts {
        let mut child_nodes = account.children();
        _ = child_nodes.next();
        let address = child_nodes
            .next()
            .unwrap()
            .find(Name("a"))
            .next()
            .unwrap()
            .text();

        let change_info = child_nodes.next().unwrap();
        let change_sign = change_info.find(Class("badge")).next().unwrap().text();
        let mut change_amount = change_info.find(Class("font-monospace"));
        let mut amount = String::from("0");
        if let Some(value) = change_amount.next() {
            amount = value.text();
        }

        let post_balance_text = child_nodes
            .next()
            .unwrap()
            .find(Class("font-monospace"))
            .next()
            .unwrap()
            .text();
        let post_balance = post_balance_text
            .split_whitespace()
            .next()
            .unwrap()
            .trim()
            .split(',')
            .collect::<String>()
            .parse::<f64>()
            .unwrap();
        let attribute_nodes = child_nodes.next().unwrap().find(Class("me-1"));

        let mut attributes = HashSet::new();
        for quality in attribute_nodes {
            attributes.insert(quality.text());
        }
        if address.split_whitespace().any(|x| x == "Program") {
            attributes.insert("Program".to_string());
        }
        let attributes = attributes.into_iter().collect::<Vec<String>>();

        let sol_change = {
            let multiplier: f64 = match change_sign.trim().chars().next().unwrap() {
                '+' | '0' => 1.0,
                _ => -1.0,
            };

            amount.parse::<f64>().unwrap() * multiplier
        };

        let new_account = TxAccountInput {
            address,
            attributes,
            sol_change,
            post_balance,
        };

        accounts_vec.push(new_account);
    }

    accounts_vec
}

fn parse_instruction(instructions: &Node) -> Instruction {
    let description = instructions
        .find(Class("card-header-title"))
        .next()
        .unwrap()
        .text();
    let mut account_nodes = instructions.find(Class("list").descendant(Name("tr")));
    let program = account_nodes
        .next()
        .unwrap()
        .find(Name("a"))
        .next()
        .unwrap()
        .text();

    let mut accounts = Vec::new();
    let mut additional_info = HashMap::new();
    let mut hex = None;

    for row in account_nodes {
        if row
            .first_child()
            .unwrap()
            .text()
            .split("<span")
            .next()
            .unwrap()
            .split_whitespace()
            .take(2)
            .collect::<Vec<&str>>()
            .join(" ")
            == "Instruction Data"
        {
            let data = row
                .find(
                    Class("text-lg-end")
                        .descendant(Class("mb-0"))
                        .descendant(Name("span")),
                )
                .map(|x| x.text().replace('\u{2003}', ""))
                .collect::<String>()
                .split_whitespace()
                .collect::<String>();
            let mid = data.len() / 2;
            hex = Some(data[0..mid].to_string());

            break;
        }

        let mut child_nodes = row.children();
        let first_child = child_nodes.next().unwrap();
        let maybe_title = first_child.find(Class("me-2")).next();
        let title = if let Some(title) = maybe_title {
            title.text()
        } else {
            first_child.text()
        };

        if let Some(address) = row.find(Name("a")).next() {
            let attributes = row
                .find(Class("badge"))
                .map(|x| x.text())
                .collect::<Vec<String>>();
            let attributes = if attributes.is_empty() {
                None
            } else {
                Some(attributes)
            };

            let context = IxAccountContext {
                address: address.text(),
                attributes,
            };
            accounts.push((title, context));
        } else {
            // we don't have an account, we get the extra information
            let value = row.find(Class("font-monospace")).next().unwrap().text();
            additional_info.insert(title, value);
        }
    }

    // Only attempt sort if we have accounts labelled as Account#1, Account#2, etc
    if accounts[0].0.contains("Account #") {
        accounts.sort_by(sort_accounts);
    }

    Instruction {
        description,
        program,
        accounts,
        additional_info,
        hex,
    }
}

fn sort_accounts(
    a: &(String, IxAccountContext),
    b: &(String, IxAccountContext),
) -> std::cmp::Ordering {
    let a_position =
        a.0.split('#')
            .last()
            .unwrap()
            .trim()
            .parse::<u64>()
            .unwrap();
    let b_position =
        b.0.split('#')
            .last()
            .unwrap()
            .trim()
            .parse::<u64>()
            .unwrap();

    a_position.cmp(&b_position)
}

fn normalize_url(url: &str) -> String {
    format!("https://explorer.solana.com{}", url)
}
