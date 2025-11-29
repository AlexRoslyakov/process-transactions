use std::collections::HashMap;
use std::env;
use serde::Deserialize;
use serde::Serialize;

// Based on https://docs.rs/csv/latest/csv/tutorial/index.html

type ClientID = u32;

#[derive(Debug, Deserialize)]
struct Transaction {
    #[serde(rename = "type")]
    tr_type: String,
    client: ClientID, 
    tx: u64, 
    amount: f64
}

#[derive(Debug, Serialize)]
struct Client {
    id: ClientID,
    available: f64,
    held: f64,
    total: f64,
    locked: bool,
}



fn process_transaction(tr: &Transaction, clients: &mut HashMap<ClientID, Client>) {
    let client = clients.entry(tr.client).or_insert(Client {
        id: tr.client,
        available: 0.0,
        held: 0.0,
        total: 0.0,
        locked: false,
    });

    match tr.tr_type.as_str() {
        "deposit" => {
            client.available += tr.amount;
            client.total += tr.amount;
        }
        "withdrawal" => {
            if client.available >= tr.amount {
                client.available -= tr.amount;
                client.total -= tr.amount;
            } else {
                // println!("Insufficient funds for withdrawal: {:?}", tr);
            }
        }
        _ => {
            // println!("Unknown transaction type: {:?}", tr);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let input = &args[1];
    let csv_text = std::fs::read_to_string(input).expect("Error reading file");

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(csv_text.as_bytes());

    let mut clients: HashMap<ClientID, Client> = HashMap::new();

    for result in rdr.deserialize::<Transaction>() {
        let tr: Transaction = result?;
        process_transaction(&tr, &mut clients);
    }

    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    for client in clients.values() {
        wtr.serialize(client)?;
    }
    wtr.flush()?;

    Ok(())
}
