use be_rcon::RConClient;
use std::io::stdin;

#[tokio::main]
async fn main() {
    let passwd = std::env::var("password").unwrap();
    let client = RConClient::new("138.201.129.116:2302", "0.0.0.0:0").await.expect("Failed to create client");
    client.start_keep_alive_task().await;
    if client.login(&passwd).await.expect("Failed to login") {
        println!("Logged in successfully");
        let mut listening_client = client.clone();
        tokio::spawn(async move {
            listening_client.start_listening().await.unwrap();
        });
        loop {
            let mut buf = String::new();
            stdin().read_line(&mut buf).expect("TODO: panic message");
            let sending_client = client.clone();
            tokio::spawn(async move {
                sending_client.send_command(&buf).await.unwrap();
            });
        }
    }
}
