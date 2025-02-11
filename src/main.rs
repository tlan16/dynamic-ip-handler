extern crate dotenv;

use std::env;
use polars::df;
use polars::frame::DataFrame;
use polars::prelude::{
    CsvReadOptions, CsvWriter, IntoLazy, SerReader, SerWriter, SortMultipleOptions,
};
use std::net::Ipv4Addr;
use dotenv::dotenv;
use mail_send::mail_builder::MessageBuilder;
use mail_send::SmtpClientBuilder;

#[tokio::main]
async fn main() {
    dotenv().ok();
    
    let ip_v4 = get_public_ip_v4().await.unwrap();
    println!("public ip v4 address: {}", ip_v4);

    let data_file_name = "data.csv";
    if !std::path::Path::new(data_file_name).exists() {
        create_data_file(data_file_name, ip_v4.to_string()).await;
    }
    let (last_ip_v4, df) = get_last_recorded_ip_v4(data_file_name).await;
    println!("last recorded ip v4 address: {}", last_ip_v4);

    if ip_v4.to_string() != last_ip_v4 {
        on_ip_v4_change(last_ip_v4, ip_v4.to_string(), data_file_name, df).await;
    } else {
        println!("IP remained the same as {}", last_ip_v4);
    }
}

async fn send_email(from_ip_v4: String, to_ip_v4: String) {
    println!("Sending email notification");
    let sender = env::vars().find(|(key, _)| key == "APP_EMAIL_FROM").unwrap().1;
    let receiver = env::vars().find(|(key, _)| key == "APP_EMAIL_TO").unwrap().1;
    
    let message = MessageBuilder::new()
        .from(("Ip change bot", sender.as_str()))
        .to(vec![
            ("Frank Lan", receiver.as_str()),
        ])
        .subject("IP address changed")
        .text_body(format!("IP address changed from {} to {}", from_ip_v4, to_ip_v4));

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

async fn on_ip_v4_change(from_ip_v4: String, to_ip_v4: String, data_file_name: &str, df: DataFrame) {
    println!("IP address changed from {} to {}", from_ip_v4, to_ip_v4);
    record_ip_v4(data_file_name, to_ip_v4.clone(), Option::from(df)).await;
    send_email(from_ip_v4, to_ip_v4.clone()).await;
}

async fn create_data_file(data_file_name: &str, ip_v4: String) {
    println!("Creating new file: {}", data_file_name);
    record_ip_v4(data_file_name, ip_v4, None).await;
    println!("File created: {}", data_file_name);
}

async fn record_ip_v4(data_file_name: &str, ip_v4: String, df: Option<DataFrame>) {
    let mut df2: DataFrame = df!(
        "ip_v4" => [ip_v4],
        "timestamp" => [chrono::Utc::now().to_rfc3339()]
    )
    .unwrap();
    if df.is_none() {
        let mut file = std::fs::File::create(data_file_name).unwrap();
        CsvWriter::new(&mut file).finish(&mut df2).unwrap();
    } else if let Some(df) = df {
        println!("Appending new record to file: {}", data_file_name);
        let mut df3= df.vstack(&df2).unwrap().sort(
            ["timestamp"],
            SortMultipleOptions::default()
                .with_order_descending(true),
        ).unwrap();
        let mut file = std::fs::File::create(data_file_name).unwrap();
        CsvWriter::new(&mut file).finish(&mut df3).unwrap();
    }
}

async fn get_last_recorded_ip_v4(data_file_name: &str) -> (String, DataFrame) {
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some(data_file_name.into()))
        .unwrap()
        .finish()
        .unwrap();
    let last_ip_v4_df = df
        .clone()
        .lazy()
        .sort(
            ["timestamp"],
            SortMultipleOptions::default()
                .with_order_descending(true),
        )
        .limit(1)
        .collect()
        .unwrap();
    let last_ip_v4 = last_ip_v4_df.column("ip_v4").unwrap().get(0).unwrap();
    return (last_ip_v4.get_str().unwrap().to_string(), df);
}

async fn get_public_ip_v4() -> Option<Ipv4Addr> {
    // Attempt to get an IP address and return it.
    if let Some(ip) = public_ip::addr_v4().await {
        Some(ip)
    } else {
        None
    }
}
