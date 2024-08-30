use controller::Controller;
use device::DeviceManager;
use error::Error;
use events::{EbpfEvent, Events};
use futures::StreamExt;
use service::{AsyncSocket, TcpService, WsStream};
use tls::TlsAcceptor;

mod action_db;
mod controller;
mod device;
mod error;
mod events;
mod service;
mod tls;

const MITMDUMP_PATH: &str = "/home/sajjad/.local/bin/mitmdump";
const MAX_PARALLEL: u8 = 30;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let path = std::env::args().nth(1).unwrap();
    let action_db = action_db::ActionDb::new("action.db")?;
    let device_manager = DeviceManager::new(action_db.clone()).await;
    let mut controller = Controller::new(device_manager.devices).await?;
    let event_sender = controller.event_sender.clone();
    let jobsender = controller.job_sender.clone();
    controller
        .load_apps(&path)
        .await?;
    let acceptor = TlsAcceptor::new_from_file(
        "ca.pem",
        "ca.key",
        vec!["172.30.143.86".to_string(),"172.30.143.74".to_string()],
    )?;
    let mut service = TcpService::new("0.0.0.0:8888".to_string()).await;
    controller.run().await?;
    while let Some(stream) = service.next().await {
        match stream {
            Ok(stream) => {
                let acceptor = acceptor.clone();
                let jobsender = jobsender.clone();
                let action_db = action_db.clone();
                let event_sender = event_sender.clone();
                tokio::spawn(async move {
                    let mut buffer = [0u8; 4];
                    if stream.peek(&mut buffer).await.is_err() {
                        return ();
                    }
                    let Some(peer_ip) =
                        stream
                            .peer_addr()
                            .map(|s| s.ip())
                            .ok()
                            .and_then(|ip| match ip {
                                std::net::IpAddr::V4(ip) => Some(ip),
                                _ => None,
                            })
                    else {
                        return ();
                    };
                    let stream: Box<dyn AsyncSocket> =
                        if !(buffer.starts_with(b"GET") || buffer.starts_with(b"POST")) {
                            // let stream = acceptor.accept(stream).await.unwrap();
                            let Ok(new_stream) = acceptor.accept(stream).await else {
                                return ();
                            };
                            Box::new(new_stream)
                        } else {
                            Box::new(stream)
                        };

                    let mut s = match WsStream::new(stream).await {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("Error: {:?}", e);
                            return ();
                        }
                    };
                    loop {
                        match s.next().await {
                            Some(Ok(event)) => {
                                event_sender.send(event.clone()).await.unwrap();

                                match event {
                                    Events::JobScheduler(jobinfo) => {
                                        _ = jobsender
                                            .send((peer_ip, jobinfo.uid, jobinfo.job_id))
                                            .await;
                                    }
                                    Events::OnStartJob(_e) => {}
                                    Events::Intent(e) => {
                                        if !e.data.is_empty() {
                                            _ = action_db.add_action(e.action, e.data);
                                        }
                                    }
                                    Events::OnCreate(_e) => {}
                                    Events::GetBroadcast(e) => {
                                        if !e.data.is_empty() {
                                            _ = action_db.add_action(e.action, e.data);
                                        }
                                    }
                                    Events::DoFinal(_e) => {}
                                    Events::DoFinalByte(_e) => {}
                                    Events::DoFinalByteInt(_e) => {}
                                    Events::DoFinalByteIntInt(_e) => {}
                                    Events::DoFinalByteIntIntByte(_e) => {}
                                    Events::DoFinalByteIntIntByteInt(_e) => {}
                                    Events::DoFinalByteBufferByteBuffer(_e) => {}
                                    Events::InitIntCertificate(_e) => {}
                                    Events::InitIntKey(_e) => {}
                                    Events::UpdateByte(_e) => {}
                                    Events::UpdateByteIntInt(_e) => {}
                                    Events::UpdateByteIntIntByte(_e) => {}
                                    Events::UpdateByteIntIntByteInt(_e) => {}
                                    Events::UpdateByteBufferByteBuffer(_e) => {}
                                    Events::UpdateAADByte(_e) => {}
                                    Events::UpdateAADByteIntInt(_e) => {}
                                    Events::UpdateAADByteBuffer(_e) => {}
                                    Events::StartForeground(_e) => {}
                                    Events::OnStartCommand(e) => {
                                        if !e.data.is_empty() {
                                            _ = action_db.add_action(e.action, e.data);
                                        }
                                    }
                                    Events::OnBind(e) => {
                                        if !e.data.is_empty() {
                                            _ = action_db.add_action(e.action, e.data);
                                        }
                                    }
                                    Events::SetImpl(_e) => {}
                                    Events::SetAlarmClock(_e) => {}
                                    Events::PendingIntentGet(e) => {
                                        if !e.data.is_empty() {
                                            _ = action_db.add_action(e.action, e.data);
                                        }
                                    }
                                    Events::ScheduleInternal(e) => {
                                        _ = jobsender.send((peer_ip, e.uid, e.job_id)).await;
                                    }
                                    Events::RequestAndDoWork(_e) => {}
                                    Events::GenericHook(_e) => {}
                                    Events::EbpfEvent(e) => match e {
                                        EbpfEvent::ProcessFork(_e, _ts) => {}
                                        EbpfEvent::SocketAddrV6(_e, _ts) => {}
                                        EbpfEvent::SocketAddrV4(_e, _ts) => {}
                                    },
                                    Events::Other(_e) => {}
                                }
                            }
                            Some(Err(e)) => {
                                eprintln!("Error: {:?}", e);
                            }
                            None => {
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        }
    }

    println!("Hello, world!");
    Ok(())
}
