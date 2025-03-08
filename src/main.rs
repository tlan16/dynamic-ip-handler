extern crate dotenv;

use arrow::array::{ArrayRef, StringArray};
use arrow::compute::{sort_to_indices, take, SortOptions};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use arrow_csv::ReaderBuilder;
use dotenv::dotenv;
use mail_send::mail_builder::MessageBuilder;
use mail_send::SmtpClientBuilder;
use std::env;
use std::fs::File;
use std::net::Ipv4Addr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let data_file_name = "data.csv";
    if !std::path::Path::new(data_file_name).exists() {
        init_data_file(data_file_name);
    }

    let ip_v4 = get_public_ip_v4().await.unwrap();
    println!("public ip v4 address: {}", ip_v4.clone().to_string());

    let last_ip_v4 = get_last_recorded_ip_v4(data_file_name);
    let has_last_ip_v4 = !last_ip_v4.clone().is_none();
    if !has_last_ip_v4 {
        println!("No record found in file: {}", data_file_name);
    } else {
        println!("Last ip v4: {:} at {:}", last_ip_v4.clone().unwrap().clone().ip_v4, last_ip_v4.clone().unwrap().clone().timestamp);
    }
    let should_record_ip_v4 = last_ip_v4.clone().is_none() || last_ip_v4.clone().unwrap().ip_v4 != ip_v4.to_string();
    println!("Should record ip v4: {}", should_record_ip_v4);
    if should_record_ip_v4 == false {
        return;
    }

    if should_record_ip_v4 {
        record_ip_v4(data_file_name, ip_v4.clone()).await;
        println!("IP address recorded: {}", ip_v4);
        let from_ip_v4: Option<String> = if has_last_ip_v4 { Some(last_ip_v4.clone().unwrap().ip_v4) } else { None };
        send_email(from_ip_v4, ip_v4.clone().to_string()).await;
    }
}

#[derive(Debug, Clone)]
struct DataRow {
    ip_v4: String,
    timestamp: String,
}

async fn send_email(from_ip_v4: Option<String>, to_ip_v4: String) {
    println!("Sending email notification");
    let sender = env::vars().find(|(key, _)| key == "APP_EMAIL_FROM").unwrap().1;
    let receiver = env::vars().find(|(key, _)| key == "APP_EMAIL_TO").unwrap().1;
    
    let message = MessageBuilder::new()
        .from(("Ip change bot", sender.as_str()))
        .to(vec![
            ("Frank Lan", receiver.as_str()),
        ])
        .subject("IP address changed")
        .text_body(format!("IP address changed from {} to {}", from_ip_v4.unwrap_or("None".to_string()), to_ip_v4));

    let host = env::vars().find(|(key, _)| key == "APP_SMTP_HOST").unwrap().1;
    let username = env::vars().find(|(key, _)| key == "APP_SMTP_USERNAME").unwrap().1;
    let password = env::vars().find(|(key, _)| key == "APP_SMTP_PASSWORD").unwrap().1;
    let port = env::vars().find(|(key, _)| key == "APP_SMTP_PORT").unwrap().1;
    SmtpClientBuilder::new(host.as_str(), port.parse::<u16>().unwrap())
        .implicit_tls(false)
        .credentials((username.as_str(), password.as_str()))
        .connect()
        .await
        .unwrap()
        .send(message)
        .await
        .unwrap();
}

fn init_data_file(data_file_name: &str) {
    println!("Creating new file: {}", data_file_name);
    File::create(data_file_name).unwrap();
    insert_string_at_second_line(data_file_name, "ip_v4,timestamp").unwrap();
    println!("File initiated: {}", data_file_name);
}

async fn record_ip_v4(data_file_name: &str, ip_v4: Ipv4Addr) {
    let new_line = format!("{},{}", ip_v4.to_string(), chrono::Utc::now().to_rfc3339());
    insert_string_at_second_line(data_file_name, new_line.as_str()).unwrap();
}


fn insert_string_at_second_line(file_path: &str, string_to_insert: &str) -> std::io::Result<()> {
    let content = std::fs::read_to_string(file_path)?;
    let mut lines: Vec<&str> = content.lines().collect();

    if !lines.is_empty() {
        lines.insert(1, string_to_insert);
    } else {
        lines.push(string_to_insert);
    }

    let new_content = lines.join("\n");

    std::fs::write(file_path, new_content)?;
    Ok(())
}

fn get_last_recorded_ip_v4(data_file_name: &str) -> Option<DataRow> {
    let file = File::open(data_file_name).unwrap();
    let schema = Arc::new(Schema::new(vec![
        Field::new("ip_v4", DataType::Utf8, false),
        Field::new("timestamp", DataType::Utf8, false),
    ]));
    let mut csv_reader = ReaderBuilder::new(schema.clone()).with_header(true).build(file).unwrap();

    let batch = csv_reader.next();
    if batch.is_none() {
        return None;
    }
    let batch = batch.unwrap().unwrap();

    let timestamp_array = batch.column(1);
    let sort_options = SortOptions {
        descending: true,
        nulls_first: false,
    };
    let sorted_indices = sort_to_indices(timestamp_array, Some(sort_options), Some(1)).unwrap();
    let sorted_columns: Vec<ArrayRef> = batch.columns()
        .iter()
        .map(|column| take(column, &sorted_indices, None))
        .collect::<Result<Vec<_>, ArrowError>>()
        .unwrap();
    let sorted_batch = RecordBatch::try_new(batch.schema(), sorted_columns).unwrap();

    let ip_array = sorted_batch.column(0)
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("Column 0 should be a StringArray");
    let timestamp_array = sorted_batch.column(1)
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("Column 1 should be a StringArray");

    // Get the first row values.
    let first_ip = ip_array.value(0);
    let first_timestamp = timestamp_array.value(0);

    Some(DataRow {
        ip_v4: first_ip.to_string(),
        timestamp: first_timestamp.to_string(),
    })
}

async fn get_public_ip_v4() -> Option<Ipv4Addr> {
    // Attempt to get an IP address and return it.
    if let Some(ip) = public_ip::addr_v4().await {
        Some(ip)
    } else {
        None
    }
}
