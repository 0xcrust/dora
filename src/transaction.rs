use crate::account::{Cluster, Error, TransactionSummary};
use fantoccini::Client;
use select::{
    document::Document,
    node::Node,
    predicate::{Class, Name, Predicate},
};
use std::{thread, time::Duration};
use tokio::sync::Mutex;

#[derive(Debug, Default)]
pub struct Transaction {
    summary: TransactionSummary,
    confirmation_status: String,
    confirmations: String,
    slot: u64,
    recent_blockhash: String,
    fee: f64,
    version: String,
    accounts: Vec<TxAccount>,
    token_balances: Vec<TokenAccountInfo>,
    instructions: Vec<Instruction>,
}

#[derive(Debug, Default)]
struct Instruction {
    pub description: String,
    pub program: String,
    pub accounts: Vec<IxAccount>,
    pub instruction_data: String,
}

#[derive(Default, Debug)]
struct TxAccount {
    address: String,
    is_writable: bool,
    is_signer: bool,
    is_fee_payer: bool,
    is_program: bool,
    sol_change: f64,
    post_balance: f64,
}

#[derive(Default, Debug)]
struct IxAccount {
    name: String,
    url: String,
    is_writable: bool,
    is_signer: bool,
}

#[derive(Default, Debug)]
struct TokenAccountInfo {
    address: String,
    token_name: String,
    token_url: String,
    change: f64,
    post_balance: String,
}

pub async fn get_transaction_info(
    tx_id: String,
    cluster: Cluster,
    client: &Mutex<Client>,
) -> Result<Transaction, Error> {
    let x = String::from("2.094908999");
    _ = x.parse::<f64>().unwrap();

    let url = match cluster {
        Cluster::Mainnet => format!(
            "https://explorer.solana.com/tx/{}?cluster=mainnet-beta",
            tx_id
        ),
        Cluster::Devnet => format!("https://explorer.solana.com/tx/{}?cluster=devnet", tx_id),
    };
    log::info!("url: {}", url);

    let mut webdriver = client.lock().await;
    webdriver.goto(&url).await?;
    thread::sleep(Duration::from_secs(20));
    let html = webdriver.source().await?;

    let document = Document::from(html.as_str());

    let mut list = document.find(Class("list"));
    let mut overview = list.next().unwrap().find(Name("tr"));
    let signature = overview
        .next()
        .unwrap()
        .find(Class("font-monospace"))
        .next()
        .unwrap()
        .text();
    let result = overview
        .next()
        .unwrap()
        .find(Class("bg-success-soft"))
        .next()
        .unwrap()
        .text();
    let time = overview
        .next()
        .unwrap()
        .find(Class("font-monospace"))
        .next()
        .unwrap()
        .text();
    let confirmation_status = overview
        .next()
        .unwrap()
        .find(Class("text-lg-end"))
        .next()
        .unwrap()
        .text();
    let confirmations = overview
        .next()
        .unwrap()
        .find(Class("text-lg-end"))
        .next()
        .unwrap()
        .text();
    let slot = overview
        .next()
        .unwrap()
        .find(Name("a"))
        .next()
        .unwrap()
        .text();
    let slot = slot.split(',').collect::<String>().parse().unwrap();
    let recent_blockhash = overview
        .next()
        .unwrap()
        .find(Class("text-lg-end"))
        .next()
        .unwrap()
        .text();
    let fee: f64 = overview
        .next()
        .unwrap()
        .find(Class("font-monospace"))
        .next()
        .unwrap()
        .text()
        .parse()
        .unwrap();
    let version = overview
        .next()
        .unwrap()
        .find(Class("text-lg-end"))
        .next()
        .unwrap()
        .text();

    let mut accounts_vec: Vec<TxAccount> = vec![];
    let tx_accounts = list.next().unwrap().find(Name("tr"));

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
        let attributes = child_nodes.next().unwrap().find(Class("me-1"));

        let mut account_attributes = vec![];
        for quality in attributes {
            account_attributes.push(quality.text());
        }

        let is_fee_payer = account_attributes.contains(&"Fee Payer".to_string());
        let is_signer = account_attributes.contains(&"Signer".to_string());
        let is_writable = account_attributes.contains(&"Writable".to_string());
        let mut is_program = account_attributes.contains(&"Program".to_string());
        is_program = if address.split_whitespace().any(|x| x == "Program") {
            true
        } else {
            is_program
        };

        let sol_change = {
            let multiplier: f64 = match change_sign.as_str() {
                "+" | "0" => 1.0,
                _ => -1.0,
            };

            amount.parse::<f64>().unwrap() * multiplier
        };

        let new_account = TxAccount {
            address,
            is_writable,
            is_signer,
            is_fee_payer,
            is_program,
            sol_change,
            post_balance,
        };

        accounts_vec.push(new_account);
    }

    let token_balances = list.next().unwrap().find(Name("tr"));
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
            token_url,
            change,
            post_balance,
        };
        token_accounts_info.push(new_token_info);
    }

    let mut instructions = document
        .find(Class("mt-n3"))
        .next()
        .unwrap()
        .children()
        .collect::<Vec<Node>>();
    _ = instructions.pop();
    instructions = instructions[5..].to_vec();
    let mut instructions_vec = vec![];

    for instruction in instructions.iter() {
        let description = instruction
            .find(Class("card-header").descendant(Class("me-2")))
            .next()
            .unwrap()
            .text();
        let mut ix_account_nodes = instruction.find(Class("list").child(Name("tr")));
        let program = ix_account_nodes
            .next()
            .unwrap()
            .find(Name("a"))
            .next()
            .unwrap()
            .text();
        let mut ix_accounts_vec = vec![];
        let mut instruction_data = String::new();

        for account in ix_account_nodes {
            if account
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
                instruction_data = account
                    .find(
                        Class("text-lg-end")
                            .descendant(Class("mb-0"))
                            .descendant(Name("span")),
                    )
                    .map(|x| {println!("what's popping\n"); x.text().replace("\u{2003}", "")})
                    .collect::<String>().split_whitespace().collect::<String>();
                let mid = instruction_data.len()/2;
                instruction_data = instruction_data[0..mid].to_string();
                
                break;
            }
            let (name, url) = {
                let info = account.find(Name("a")).next().unwrap();
                (info.text(), info.attr("href").unwrap().to_string())
            };

            let attributes: Vec<String> = account.find(Class("badge")).map(|x| x.text()).collect();
            let is_signer = attributes.contains(&"Signer".to_string());
            let is_writable = attributes.contains(&"Writable".to_string());

            let new_account = IxAccount {
                name,
                url,
                is_writable,
                is_signer,
            };

            ix_accounts_vec.push(new_account);
        }

        let new_instruction = Instruction {
            description,
            program,
            accounts: ix_accounts_vec,
            instruction_data,
        };

        instructions_vec.push(new_instruction);
    }

    let transaction = Transaction {
        summary: TransactionSummary {
            signature,
            block: slot,
            time,
            result,
        },
        confirmation_status,
        confirmations,
        slot,
        recent_blockhash,
        fee,
        version,
        accounts: accounts_vec,
        token_balances: token_accounts_info,
        instructions: instructions_vec,
    };

    log::info!("Transaction: {:#?}", transaction);
    Ok(transaction)
}
