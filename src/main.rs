use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use serde::Deserialize;
use serde::Serialize;
use log::{info, warn};
use env_logger;


// CSV & Serde usage is based on https://docs.rs/csv/latest/csv/tutorial/index.html

type ClientID = u16;
type TransactionID = u32;

#[derive(Debug, Deserialize)]
struct Transaction {
    #[serde(rename = "type")]
    tr_type: String,
    client: ClientID, 
    tx: TransactionID, 
    amount: Option<f64>
}

#[derive(Debug, Serialize)]
struct Client {
    client: ClientID,
    available: f64,
    held: f64,
    total: f64,
    locked: bool,
}

fn process_transaction(tr: &Transaction, clients: &mut HashMap<ClientID, Client>, processed_transactions: &HashMap<TransactionID, Transaction>, disputed_transactions: &mut HashSet<TransactionID>) {
    let client = clients.entry(tr.client).or_insert(Client {
        client: tr.client,
        available: 0.0,
        held: 0.0,
        total: 0.0,
        locked: false,
    });

    match tr.tr_type.as_str() {
        "deposit" => {
            let amount = match tr.amount {
                Some(a) => a,
                None => {
                    warn!("Withdrawal transaction missing amount: {:?}", tr);
                    return;
                }
            };
            // TBD: likely should check for locked account here (no requirement in spec)
            client.available += amount;
            client.total += amount;
        }
        "withdrawal" => {
            let amount = match tr.amount {
                Some(a) => a,
                None => {
                    warn!("Withdrawal transaction missing amount: {:?}", tr);
                    return;
                }
            };
            if client.available >= amount {
                // TBD: likely should check for locked account here (no requirement in spec)
                client.available -= amount;
                client.total -= amount;
            } else {
                info!("Insufficient funds for withdrawal: {:?}", tr);
            }
        }
        "dispute" => {
            if let Some(original_tr) = processed_transactions.get(&tr.tx) {
                if original_tr.client != tr.client {
                    warn!("Dispute transaction client mismatch: {:?}, {:?}", tr, original_tr);
                    return;
                }
                if disputed_transactions.contains(&tr.tx) {
                    warn!("Transaction already disputed: {:?}", tr);
                    return;
                }
                if let Some(amount) = original_tr.amount {
                    client.available -= amount;
                    client.held += amount;
                    disputed_transactions.insert(tr.tx);
                } else {
                    warn!("Dispute on transaction without amount: {:?}", tr);
                }
            } else {
                warn!("Dispute on unknown transaction: {:?}", tr);
            }
        }
        "resolve" => {
            if let Some(original_tr) = processed_transactions.get(&tr.tx) {
                if original_tr.client != tr.client {
                    warn!("Resolve transaction client mismatch: {:?}, {:?}", tr, original_tr);
                    return;
                }   
                if !disputed_transactions.contains(&tr.tx) {
                    warn!("Resolve on non-disputed transaction: {:?}", tr);
                    return;
                }
                if let Some(amount) = original_tr.amount {
                    client.held -= amount;
                    client.available += amount;
                    disputed_transactions.remove(&tr.tx);
                } else {
                    warn!("Resolve on transaction without amount: {:?}", tr);
                }
            } else {
                warn!("Resolve on unknown transaction: {:?}", tr);
            }
        }
        "chargeback" => {
            if let Some(original_tr) = processed_transactions.get(&tr.tx) {
                if original_tr.client != tr.client {
                    warn!("Chargeback transaction client mismatch: {:?}, {:?}", tr, original_tr);
                    return;
                }
                if !disputed_transactions.contains(&tr.tx) {
                    warn!("Chargeback on non-disputed transaction: {:?}", tr);
                    return;
                }
                if let Some(amount) = original_tr.amount {
                    client.held -= amount;
                    client.total -= amount;
                    client.locked = true;
                    disputed_transactions.remove(&tr.tx);
                } else {
                    warn!("Chargeback on transaction without amount: {:?}, {:?}", tr, original_tr);
                }
            } else {
                warn!("Chargeback on unknown transaction: {:?}", tr);
            }
        }
        _ => {
            warn!("Unknown transaction type: {:?}", tr);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let input = &args[1];
    let csv_text = std::fs::read_to_string(input).expect("Error reading file");

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(csv_text.as_bytes());

    let mut clients: HashMap<ClientID, Client> = HashMap::new();
    let mut processed_transactions: HashMap<TransactionID, Transaction> = HashMap::new();
    let mut disputed_transactions: HashSet<TransactionID> = HashSet::new();
    for result in rdr.deserialize::<Transaction>() {
        let tr: Transaction = result?;
        process_transaction(&tr, &mut clients, &processed_transactions, &mut disputed_transactions);
        if tr.tr_type == "deposit" || tr.tr_type == "withdrawal" {
            processed_transactions.insert(tr.tx, tr);
        }
    }

    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    for client in clients.values() {
        wtr.serialize(client)?;
    }
    wtr.flush()?;

    Ok(())
}
