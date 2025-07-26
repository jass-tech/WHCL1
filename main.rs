use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::env;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use qrcode::QrCode;
use image::Luma;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DrugBatch {
    id: Uuid,
    name: String,
    manufacturer: String,
    batch_number: String,
    expiry_date: String,
    recalled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PharmaChain {
    batches: HashMap<Uuid, DrugBatch>,
    recalls: HashSet<Uuid>,
}

impl PharmaChain {
    fn new() -> Self {
        if Path::new("pharma_data.json").exists() {
            let file = File::open("pharma_data.json").expect("Failed to open data file");
            let reader = BufReader::new(file);
            serde_json::from_reader(reader).unwrap_or_else(|_| Self::empty())
        } else {
            Self::empty()
        }
    }

    fn empty() -> Self {
        PharmaChain {
            batches: HashMap::new(),
            recalls: HashSet::new(),
        }
    }

    fn save(&self) {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("pharma_data.json")
            .expect("Failed to open data file");
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self).expect("Failed to write data");
    }

    fn mint_batch(&mut self, name: &str, manufacturer: &str, batch_number: &str, expiry_date: &str) -> Uuid {
        let id = Uuid::new_v4();
        let batch = DrugBatch {
            id,
            name: name.to_string(),
            manufacturer: manufacturer.to_string(),
            batch_number: batch_number.to_string(),
            expiry_date: expiry_date.to_string(),
            recalled: false,
        };
        self.batches.insert(id, batch.clone());
        self.save();
        self.generate_qr_code(&batch);
        id
    }

    fn verify_batch(&self, id: &Uuid) -> Option<&DrugBatch> {
        self.batches.get(id)
    }

    fn recall_batch(&mut self, id: &Uuid) -> bool {
        if let Some(batch) = self.batches.get_mut(id) {
            batch.recalled = true;
            self.recalls.insert(*id);
            self.save();
            true
        } else {
            false
        }
    }

    fn is_recalled(&self, id: &Uuid) -> bool {
        self.recalls.contains(id)
    }

    fn generate_qr_code(&self, batch: &DrugBatch) {
        let data = format!(
            "Batch ID: {}\nName: {}\nManufacturer: {}\nBatch No: {}\nExpiry: {}",
            batch.id, batch.name, batch.manufacturer, batch.batch_number, batch.expiry_date
        );
        let code = QrCode::new(data.as_bytes()).unwrap();
        let image = code.render::<Luma<u8>>().build();
        image.save(format!("qr_{}.png", batch.id)).unwrap();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut chain = PharmaChain::new();

    if args.len() < 2 {
        println!("Usage: pharmachain [mint|verify|recall] [params...]");
        return;
    }

    match args[1].as_str() {
        "mint" => {
            if args.len() != 6 {
                println!("Usage: pharmachain mint <name> <manufacturer> <batch_number> <expiry_date>");
                return;
            }
            let id = chain.mint_batch(&args[2], &args[3], &args[4], &args[5]);
            println!("✅ Minted batch with ID: {}\n📦 QR saved to: qr_{}.png", id, id);
        },
        "verify" => {
            if args.len() != 3 {
                println!("Usage: pharmachain verify <uuid>");
                return;
            }
            let id = Uuid::parse_str(&args[2]).expect("Invalid UUID");
            match chain.verify_batch(&id) {
                Some(batch) => println!("🔍 Batch Info:\n{:#?}", batch),
                None => println!("❌ Batch not found."),
            }
        },
        "recall" => {
            if args.len() != 3 {
                println!("Usage: pharmachain recall <uuid>");
                return;
            }
            let id = Uuid::parse_str(&args[2]).expect("Invalid UUID");
            if chain.recall_batch(&id) {
                println!("⚠️ Batch {} has been recalled.", id);
            } else {
                println!("❌ Batch not found.");
            }
        },
        _ => {
            println!("Unknown command. Use: mint, verify, or recall.");
        }
    }
}
