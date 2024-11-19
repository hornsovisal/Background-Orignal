use std::net::SocketAddrV4;

use crate::events::{DoWork, Events, OnBind, OnCreate, OnReceiveHook, OnStartCommand, OnStartJob};

#[derive(Debug, Clone)]
pub enum BackgroundWorks {
    OnBind(OnBind),
    OnReceiveHook(OnReceiveHook),
    OnStartCommand(OnStartCommand),
    OnStartJob(OnStartJob),
    DoWork(DoWork),
    OnCreate(OnCreate),
}

impl BackgroundWorks {
    pub fn from_event(e: &Events) -> Option<Self> {
        match e {
            Events::OnBind(e) => Some(BackgroundWorks::OnBind(e.clone())),
            Events::OnReceiveHook(e) => Some(BackgroundWorks::OnReceiveHook(e.clone())),
            Events::OnStartCommand(e) => Some(BackgroundWorks::OnStartCommand(e.clone())),
            Events::OnStartJob(e) => Some(BackgroundWorks::OnStartJob(e.clone())),
            Events::DoWork(e) => Some(BackgroundWorks::DoWork(e.clone())),
            Events::OnCreate(e) => Some(BackgroundWorks::OnCreate(e.clone())),
            _ => None,
        }
    }
    pub fn get_work_name(&self) -> (String,String) {
        match self {
            BackgroundWorks::OnBind(e) => (e.class_name.clone(),e.method_name.clone()),
            BackgroundWorks::OnReceiveHook(e) => (e.class_name.clone(),e.method_name.clone()),
            BackgroundWorks::OnStartCommand(e) => (e.class_name.clone(),e.method_name.clone()),
            BackgroundWorks::OnStartJob(e) => (e.class_name.clone(),e.method_name.clone()),
            BackgroundWorks::DoWork(e) => (e.class_name.clone(),e.method_name.clone()),
            BackgroundWorks::OnCreate(e) => (e.class_name.clone(),e.method_name.clone()),
        }
    }
}

// pub struct BackgroundSocketInfo {
//     socket_addr: SocketAddrV4,
//     canary: String,
//     ts: u128,
//     importance: u64,
// }
