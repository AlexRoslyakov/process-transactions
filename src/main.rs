use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use serde::Deserialize;
use serde::Serialize;
use log::{info, warn};
use env_logger;

type ClientID = u16;
type TransactionID = u32;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize)]
struct Transaction {
    #[serde(rename = "type")]
    tr_type: TransactionType,
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

    fn process_revertable_transaction(&mut self, tr: Transaction) {
        let client = self.clients.entry(tr.client).or_insert(Client {
            client: tr.client,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        });

        let sign = if tr.tr_type == TransactionType::Deposit { 1.0 } else { -1.0 };

        let Some(amount) = tr.amount else {
            warn!("Transaction missing amount: {:?}", tr);
            return;
        };

        // TBD: likely should check for locked account here, especially for withdrawal (no requirement in spec)
        if client.available + sign*amount > 0.0 {
            client.available += sign*amount;
            client.total += sign*amount;
        }
        else {
            info!("Insufficient funds for withdrawal: {:?}", tr);
        }

        self.revertable_transactions.insert(tr.tx, tr);
    }

    fn process_dispute_resolve_chargeback(&mut self, tr: Transaction) {
        let Some(original_tr) = self.revertable_transactions.get(&tr.tx) else {
            warn!("Dispute/Resolve/Chargeback on unknown transaction: {:?}", tr);
            return;
        };

        if original_tr.client != tr.client {
            warn!("Dispute/Resolve/Chargeback transaction client mismatch: {:?}, {:?}", tr, original_tr);
            return;
        }
        if original_tr.tr_type != TransactionType::Deposit {
            warn!("Dispute/Resolve/Chargeback on non-deposit transaction: {:?}", tr);
            return;
        }
        if tr.tr_type == TransactionType::Dispute {
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

        let Some(amount) = original_tr.amount else {
            warn!("Dispute/Resolve/Chargeback {:?} on transaction without amount: {:?}", tr, original_tr);
            return;
        };

        let Some(client) = self.clients.get_mut(&tr.client) else {
            warn!("Client not found for Dispute/Resolve/Chargeback: {:?}", tr);
            return;
        };

        match tr.tr_type {
            TransactionType::Dispute => {
                client.available -= amount;
                client.held += amount;
                self.disputed_transactions.insert(tr.tx);
            }
            TransactionType::Resolve => {
                client.held -= amount;
                client.available += amount;
                self.disputed_transactions.remove(&tr.tx);
            }
            TransactionType::Chargeback => {
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

    fn process_transaction(&mut self, tr: Transaction) {
        match tr.tr_type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                self.process_revertable_transaction(tr);
            }
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => {
                self.process_dispute_resolve_chargeback(tr);
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
            if let Ok(tr) = result {
                self.process_transaction(tr);
            } else {
                warn!("Error deserializing transaction: {:?}", result);
            }
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

    #[test]
    fn test_unexpected() {
        run_case("06-transactions-unexpected", "06-accounts-unexpected")
    }

    #[test]
    fn test_dispute_wrong_client() {
        run_case("07-transactions-dispute-wrong-client", "07-accounts-dispute-wrong-client")
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