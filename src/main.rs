use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use serde::Deserialize;
use serde::Serialize;
use log::{info, warn};
use env_logger;

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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Client {
    client: ClientID,
    available: f64,
    held: f64,
    total: f64,
    locked: bool,
}

struct Model {
    clients: HashMap<ClientID, Client>,
    revertable_transactions: HashMap<TransactionID, Transaction>,
    disputed_transactions: HashSet<TransactionID>
}

impl Model {
    fn new() -> Self {
        Model {
            clients: HashMap::new(),
            revertable_transactions: HashMap::new(),
            disputed_transactions: HashSet::new(),
        }
    }

    fn process_revertable_transaction(&mut self, tr: Transaction, sign: f64) {
        let client = self.clients.entry(tr.client).or_insert(Client {
            client: tr.client,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        });

        if let Some(amount) = tr.amount {
            // TBD: likely should check for locked account here, especially for withdrawal (no requirement in spec)
            if client.available + sign*amount > 0.0 {
                client.available += sign*amount;
                client.total += sign*amount;
            }
            else {
                info!("Insufficient funds for withdrawal: {:?}", tr);
            }
        } else {
            warn!("Transaction missing amount: {:?}", tr);
        }
        self.revertable_transactions.insert(tr.tx, tr);
    }

    fn process_dispute_resolve_chargeback(&mut self, tr: Transaction) {
        if let Some(original_tr) = self.revertable_transactions.get(&tr.tx) {
            if original_tr.client != tr.client {
                warn!("Dispute/Resolve/Chargeback transaction client mismatch: {:?}, {:?}", tr, original_tr);
                return;
            }
            if original_tr.tr_type != "deposit" {
                warn!("Dispute/Resolve/Chargeback on non-deposit transaction: {:?}", tr);
                return;
            }
            if tr.tr_type == "dispute" {
                if self.disputed_transactions.contains(&tr.tx) {
                    warn!("Transaction already disputed: {:?}", tr);
                    return;
                }
            } else {
                if !self.disputed_transactions.contains(&tr.tx) {
                    warn!("Resolve/Chargeback on non-disputed transaction: {:?}", tr);
                    return;
                }
            }
            if let Some(amount) = original_tr.amount {
                if let Some(client) = self.clients.get_mut(&tr.client) {
                    match tr.tr_type.as_str() {
                        "dispute" => {
                            client.available -= amount;
                            client.held += amount;
                            self.disputed_transactions.insert(tr.tx);
                        }
                        "resolve" => {
                            client.held -= amount;
                            client.available += amount;
                            self.disputed_transactions.remove(&tr.tx);
                        }
                        "chargeback" => {
                            client.held -= amount;
                            client.total -= amount;
                            self.disputed_transactions.remove(&tr.tx);
                            client.locked = true;
                        }
                        _ => {
                            warn!("Unexpected transaction type: {:?}", tr);
                        }
                    }
                }
                else {
                    warn!("Client not found for Dispute/Resolve/Chargeback: {:?}", tr);
                    return;
                }
            } else {
                warn!("Dispute/Resolve/Chargeback on transaction without amount: {:?}", tr);
            }
        } else {
            warn!("Dispute/Resolve/Chargeback on unknown transaction: {:?}", tr);
        }
    }

    fn process_transaction(&mut self, tr: Transaction) {
        match tr.tr_type.as_str() {
            "deposit" => {
                self.process_revertable_transaction(tr, 1.0);
            }
            "withdrawal" => {
                self.process_revertable_transaction(tr, -1.0);
            }
            "dispute" => {
                self.process_dispute_resolve_chargeback(tr);
            }
            "resolve" => {
                self.process_dispute_resolve_chargeback(tr);
            }
            "chargeback" => {
                self.process_dispute_resolve_chargeback(tr);
            }
            _ => {
                warn!("Unknown transaction type: {:?}", tr);
            }
        }
    }

    fn process_transactions(&mut self, input: &String) -> Result<(), Box<dyn std::error::Error>> {
        let csv_text = std::fs::read_to_string(input).expect("Error reading file");
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .trim(csv::Trim::All)
            .from_reader(csv_text.as_bytes());

        for result in rdr.deserialize::<Transaction>() {
            let tr: Transaction = result?;
            self.process_transaction(tr);
        }

        Ok(())
    }

    fn print_to_stdout(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut wtr = csv::Writer::from_writer(std::io::stdout());
        for client in self.clients.values() {
            wtr.serialize(client)?;
        }
        wtr.flush()?;

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let input = &args[1];

    let mut model = Model::new();
    model.process_transactions(input)?;
    model.print_to_stdout()
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_base() {
        run_case("01-transactions-base", "01-accounts-base")
    }

    #[test]
    fn test_dispute() {
        run_case("02-transactions-dispute", "02-accounts-dispute")
    }

    #[test]
    fn test_resolve() {
        run_case("03-transactions-resolve", "03-accounts-resolve")
    }

    #[test]
    fn test_resolve_no_dispute() {
        run_case("04-transactions-resolve-no-dispute", "04-accounts-resolve-no-dispute")
    }

    #[test]
    fn test_chargeback() {
        run_case("05-transactions-chargeback", "05-accounts-chargeback")
    }

    fn run_case(input_name: &str, output_name: &str) {
        let input = format!("cases/{}.csv", input_name);
        let mut model = Model::new();
        model.process_transactions(&input).expect("Processing failed");

        let output = format!("cases/{}.csv", output_name);
        let expected_csv = std::fs::read_to_string(output).expect("Error reading expected");
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .trim(csv::Trim::All)
            .from_reader(expected_csv.as_bytes());

        let mut record_count = 0;
        for result in rdr.deserialize::<Client>() {
            let expected_client: Client = result.expect("Error deserializing client");
            let actual_client = model.clients.get(&expected_client.client).expect("Client missing");
            assert_eq!(&expected_client, actual_client, "Client data mismatch for client {}", expected_client.client);
            record_count += 1;
        }
        assert_eq!(model.clients.len(), record_count, "Number of clients mismatch");
    }
}