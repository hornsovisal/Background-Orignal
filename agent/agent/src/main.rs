use std::net::Ipv4Addr;
use std::process::Command;
use std::sync::Arc;
use std::{io::BufRead, net::SocketAddrV4};

use futures::{SinkExt, StreamExt};
use log::{debug, info, warn};
use tokio::signal;
use tokio_tungstenite::connect_async;

use crate::ebpf::{EbpfManager, Event};
mod ebpf;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    host: String,
    #[arg(long)]
    proxy: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Arc::new(Args::parse());
    let mut conn_stream = Ok(connect_async(&args.host).await.map(|(x, y)| x).unwrap());
    let forward_addr: SocketAddrV4 = (&args.proxy).parse().unwrap();
    dbg!(forward_addr);

    env_logger::init();
    let apps_uid = match Command::new("sh")
        .arg("-c")
        .arg("pm list packages -s -U | cut -d : -f 3")
        .output()
    {
        Ok(o) => o
            .stdout
            .lines()
            .filter_map(|x| x.ok().and_then(|x| x.parse::<u32>().ok()))
            .collect::<Vec<u32>>(),
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to execute command: {}", e));
        }
    };

    let mut e = EbpfManager::new(apps_uid, forward_addr);
    loop {
        let event = e.receiver.recv().await.unwrap();
        let o = serde_json::to_string(&event).unwrap();
        match conn_stream {
            Ok(ref mut stream) => {
                stream
                    .send(tokio_tungstenite::tungstenite::Message::Text(o))
                    .await
                    .unwrap();
            }
            Err(e) => {
                // let failiure = 0;
                loop {
                    warn!("Failed to connect to the server: {}", e);
                    conn_stream = connect_async(&args.host).await.map(|(x, y)| x);
                    if conn_stream.is_ok() {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    // if failiure == 5 {
                    //     return Err(anyhow::anyhow!("Failed to connect to the server: {}", e));
                    // }
                }
            }
        }
    }
    Ok(())
}
