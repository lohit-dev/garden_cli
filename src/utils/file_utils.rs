use eyre::Result;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderData {
    pub order_id: String,
    pub secret: String,
}

pub fn save_order_data(order_id: &str, secret: &str) -> Result<()> {
    // Create data directory if it doesn't exist
    std::fs::create_dir_all("data")?;

    let order_data = OrderData {
        order_id: order_id.to_string(),
        secret: secret.to_string(),
    };

    // Save to data/order_secrets.json
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open("data/order_secrets.json")?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, &order_data)?;
    writer.write_all(b"\n")?;

    // Save order_id to data/order_ids.json
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open("data/order_ids.json")?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, &order_id)?;
    writer.write_all(b"\n")?;

    Ok(())
}

pub fn load_order_data() -> Result<Vec<OrderData>> {
    let file = File::open("data/order_secrets.json")?;
    let reader = BufReader::new(file);
    let order_data: Vec<OrderData> = serde_json::from_reader(reader)?;
    Ok(order_data)
}

pub fn load_order_ids() -> Result<Vec<String>> {
    let file = File::open("data/order_ids.json")?;
    let reader = BufReader::new(file);
    let order_ids: Vec<String> = serde_json::from_reader(reader)?;
    Ok(order_ids)
}
