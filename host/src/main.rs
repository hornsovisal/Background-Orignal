use controller::{BroadcastRequest, Controller, JobRequest};
use device::{Device, DeviceManager};
use error::Error;
use events::Events;
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
const MAX_PARALLEL: u8 = 8; // 10

#[tokio::main]
async fn main() -> Result<(), Error> {
    console_subscriber::init();
    let path = std::env::args().nth(1).unwrap();
    let action_db = action_db::ActionDb::new("action.db")?;
    let device_manager = DeviceManager::new(action_db.clone()).await;
    let mut controller = Controller::new(device_manager.devices).await?;
    let event_sender = controller.event_sender.clone();
    let jobsender = controller.job_sender.clone();
    controller.load_apps(&path).await?;
    let acceptor = TlsAcceptor::new_from_file(
        "ca.pem",
        "ca.key",
        vec!["172.30.143.86".to_string(), "172.30.143.74".to_string()],
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
                                        let job_request = JobRequest {
                                            ip: peer_ip,
                                            pkg: jobinfo.package_name,
                                            jid: jobinfo.job_id,
                                        };
                                        _ = jobsender
                                            .send(controller::RequestedAction::Job(job_request))
                                            .await;
                                    }
                                    Events::OnStartJob(_e) => {}
                                    Events::Intent(e) => {
                                        _ = action_db.add_action(
                                            e.action.clone(),
                                            e.data,
                                            e.extras,
                                        );
                                    }
                                    Events::OnReceiveHook(e) => {
                                        _ = action_db.add_action(e.action, e.data, e.extras);
                                    }
                                    Events::OnCreate(_e) => {}
                                    Events::GetBroadcast(e) => {
                                        _ = action_db.add_action(e.action, e.data, e.extras);
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
                                        _ = action_db.add_action(e.action, e.data, e.extras);
                                    }
                                    Events::OnBind(e) => {
                                        _ = action_db.add_action(e.action, e.data, e.extras);
                                    }
                                    Events::SetImpl(_e) => {}
                                    Events::SetAlarmClock(_e) => {}
                                    Events::PendingIntentGet(e) => {
                                        _ = action_db.add_action(
                                            e.action.clone(),
                                            e.data,
                                            e.extras,
                                        );
                                    }
                                    Events::ScheduleInternal(jobinfo) => {
                                        let job_request = JobRequest {
                                            ip: peer_ip,
                                            pkg: jobinfo.package_name,
                                            jid: jobinfo.job_id,
                                        };
                                        _ = jobsender
                                            .send(controller::RequestedAction::Job(job_request))
                                            .await;
                                    }
                                    Events::DoWork(_e) => {}
                                    Events::RequestWork(_e) => {}
                                    Events::GenericHook(_e) => {}
                                    Events::RegisterReceiver(e) => {
                                        let broadcast_request = BroadcastRequest {
                                            ip: peer_ip,
                                            pkg: e.package_name,
                                            class: e.register_class,
                                            actions: e.actions,
                                        };
                                        _ = jobsender
                                            .send(controller::RequestedAction::Broadcast(
                                                broadcast_request,
                                            ))
                                            .await;
                                    }
                                    Events::DeviceID(_e) => {}
                                    Events::StartThreadHook(_e) => {}
                                    Events::Canary(_e) => {}
                                    Events::SchedProcessFork(_e) => {}
                                    Events::SchedProcessExit(_e) => {}
                                    Events::TaskNewtask(_e) => {}
                                    // Events::TaskRename(_e) => {}
                                    Events::SocketAddrV6(_e) => {}
                                    Events::SocketAddrV4(_e) => {}
                                    Events::SocketHook(_e) => {}
                                    Events::ConnectHook(_e) => {}
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
