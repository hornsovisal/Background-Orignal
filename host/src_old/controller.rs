use std::{
    collections::HashMap, net::Ipv4Addr, path::PathBuf, process::exit, sync::Arc, time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::{
    fs::{self, File}, io::AsyncWriteExt, process::Command, sync::{Mutex, RwLock, Semaphore}, time::Instant
};

use crate::{device::Device, error::Error, events::Events};

pub struct Controller {
    devices: Vec<Device>,
    app_base_dir: PathBuf,
    apps: Arc<RwLock<Vec<String>>>,
    pub ongoing: Arc<RwLock<HashMap<u32, String>>>,
    lspd_scope: Arc<HashMap<String, Vec<String>>>,
    pub job_sender: tokio::sync::mpsc::Sender<(Ipv4Addr, u32, u32)>,
    pub event_sender: tokio::sync::mpsc::Sender<Events>,
    pub event_files: Arc<RwLock<HashMap<u32, File>>>,
    pub global_event_files: Arc<Mutex<Option<File>>>,
    pub start_time: u64,
}

impl Controller {
    pub async fn new(devices: Vec<Device>) -> Result<Self, Error> {
        let mut lspd_scope = HashMap::new();
        let ongoing: Arc<RwLock<HashMap<u32, String>>> = Default::default();
        for device in devices.iter() {
            for apps in device.list_of_3rd_party_apps().await? {
                device.uninstall(&apps).await?;
            }
            let ids = device.execute_as_root("sqlite3 /data/adb/lspd/config/modules_config.db \\\"DELETE FROM scope\\\";sqlite3 /data/adb/lspd/config/modules_config.db \\\"select mid from modules where mid !=1\\\"",10).await?.lines().map(|s|s.to_owned()).collect::<Vec<String>>();
                lspd_scope.insert(device.serial.clone(), ids);
        }
        let (job_sender, mut job_receiver) =
            tokio::sync::mpsc::channel::<(Ipv4Addr, u32, u32)>(100);
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::channel::<Events>(1000);
        let inner_devices = devices.clone();
        let inner_ongoing = ongoing.clone();
        let event_files: Arc<RwLock<HashMap<u32, File>>> = Arc::new(RwLock::new(HashMap::new()));
        let _event_files = event_files.clone();
        let global_events =Arc::new(Mutex::new(None::<File>));
        let _global_events = global_events.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                let global_events = _global_events.clone();
                let _event_files = _event_files.clone();
                tokio::select! {
                    _ = interval.tick() => {
                        for d in &inner_devices{
                            d.execute("input keyevent 3;input keyevent 4;input keyevent KEYCODE_WAKEUP")
                                .await
                                .unwrap();
                        }
                    }
                    Some((ip,uid,jid)) = job_receiver.recv() => {
                        for d in &inner_devices{
                            if let Some(pkg) = inner_ongoing.read().await.get(&uid){
                                if let Some(ipv4) = d.ipv4{
                                    if ipv4 == ip{
                                        d.execute(&format!("cmd jobscheduler run -f {} {}",pkg,jid)).await.unwrap();
                                    }
                                }
                            }
                        }
                    }
                    Some(event) = event_receiver.recv() => {
                        let msg = serde_json::to_string(&event).map(|s| format!("{}\n", s)).unwrap();
                        if let Some(f) = _event_files.write().await.get_mut(&event.get_uid()){
                            f.write_all(msg.as_bytes()).await.unwrap();
                        } else{
                            if let Some(f) = global_events.lock().await.as_mut(){
                                f.write_all(msg.as_bytes()).await.unwrap();
                            }
                        }
                    }
                };
            }
        });
        let start = SystemTime::now();
        let ts = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards").as_secs();
        
        Ok(Self {
            devices,
            app_base_dir: PathBuf::new(),
            apps: Default::default(),
            ongoing,
            lspd_scope: Arc::new(lspd_scope),
            job_sender,
            event_sender,
            event_files,
            global_event_files: global_events,
            start_time:ts,
        })
    }
    pub async fn load_apps(&mut self, base_dir: &str) -> Result<(), Error> {
        self.app_base_dir = PathBuf::from(&base_dir);
        {
            self.global_event_files.lock().await.replace(
            File::options()
                .write(true)
                .create(true)
                .append(true)
                .open(self.app_base_dir.join(format!("events-{}.txt",self.start_time)))
                .await?,
            );
        }
        let mut dir = tokio::fs::read_dir(base_dir).await?;
        while let Ok(entry) = dir.next_entry().await {
            let Some((path, file_name)) = entry.as_ref().map(|x| (x.path(), x.file_name())) else {
                break;
            };

            if !path.join("AndroidManifest.xml").exists() {
                continue;
            }
            if ["done.txt", "fail.txt", "lock.txt"]
                .iter()
                .any(|&file| path.join("analysis_result").join(file).exists())
            {
                continue;
            }

            _ = fs::remove_dir_all(path.join("analysis_result")).await;
            fs::create_dir_all(path.join("analysis_result")).await?;
            {
                self.apps
                    .write()
                    .await
                    .push(file_name.to_string_lossy().to_string());
            }
        }
        Ok(())
    }
    pub async fn run(&self) -> Result<(), Error> {
        let mitm_proxy = Arc::new(Mutex::new(Command::new(crate::MITMDUMP_PATH)
        .arg("-w")
        .arg(format!("{}/dump-{}.mitm", self.app_base_dir.to_string_lossy(),self.start_time))
        .arg("--anticomp")
        .arg("-q")
        .arg("--listen-port")
        .arg("8080")
        .spawn()?));
        let apps = self.apps.clone();
        let app_base_dir = self.app_base_dir.clone();
        let devices = self.devices.clone();
        let ongoing = self.ongoing.clone();
        let event_files = self.event_files.clone();
        for device in devices {
            let apps = apps.clone();
            let app_base_dir = app_base_dir.clone();
            let lspd_scope = self.lspd_scope.clone();
            let ongoing = ongoing.clone();
            let mut device_tasks = Vec::new();
            let event_files = event_files.clone();
            let start_time = self.start_time;
            device_tasks.push(tokio::spawn(async move {
                let mut tasks = Vec::new();
                let device = device.clone();
                let apps = apps.clone();
                let semaphore = Arc::new(Semaphore::new(crate::MAX_PARALLEL.into()));
                let all_install_token = Arc::new(Semaphore::new(1));
                let parallel_events = Arc::new(Semaphore::new((crate::MAX_PARALLEL/2).into()));
                
                loop {
                    let parallel_events = parallel_events.clone();
                    let lspd_scope = lspd_scope.clone();
                    let apps = apps.clone();
                    let app_base_dir = app_base_dir.clone();
                    let device = device.clone();
                    let ongoing = ongoing.clone();
                    let event_files = event_files.clone();
                    if { ongoing.read().await.len() == 0 } && { apps.read().await.len() == 0 } {
                        break;
                    }
                    let permit = semaphore.clone().acquire_owned().await.unwrap();
                    let all_install_token = all_install_token.clone();
                    tasks.push(tokio::spawn(async move {
                        let app = {
                            match apps.write().await.pop() {
                                Some(app) => app,
                                None => {
                                    drop(permit);
                                    return;
                                }
                            }
                        };
                        let path = app_base_dir.join(app);//PathBuf::from(&format!("{}/{}", app_base_dir.to_string_lossy(), app));
                        let install_permit = all_install_token.clone().acquire_owned().await.unwrap();
                        println!("Installing {}", path.to_string_lossy());
                        let info = match device.install_apk(&path.to_string_lossy()).await {
                            Ok(info) => info,
                            Err(e) => {
                                if let Ok(mut f) =
                                    File::create(path.join("analysis_result").join("fail.txt"))
                                        .await
                                {
                                    let _ = f.write_all(e.to_string().as_bytes()).await;
                                }
                                drop(install_permit);
                                drop(permit);
                                return;
                            }
                        };
                        println!("Installing {} done", path.to_string_lossy());
                        drop(install_permit);
                        if let Ok(mut f) =
                            File::create(path.join("analysis_result").join("lock.txt")).await
                        {
                            let _ = f.write_all(device.serial.as_bytes()).await;
                        }
                        
                        if let Some(uid) = info.uid {
                            {ongoing.write().await.insert(uid, info.package_name.clone());}
                            
                            let f = File::options()
                            .write(true)
                            .create(true)
                            .append(true)
                            .open(
                                app_base_dir.join(&info.package_name).join(
                                    "analysis_result",
                                )
                                .join(format!("events-{}.txt",start_time)),
                            )
                            .await
                            .unwrap();
                        {
                            event_files.write().await.insert(uid, f);
                        }
                        }

                        if let Err(e) = device.disable(&info).await {
                            eprintln!("Error disabling app {}", e);
                            _ = fs::remove_dir_all(path.join("analysis_result")).await;
                            if let Some(uid) = info.uid {
                                ongoing.write().await.remove(&uid);
                                if let Some(mut f) = event_files.write().await.remove(&uid){
                                    f.sync_all().await.unwrap();
                                    f.shutdown().await.unwrap();
                                }
                            }
                            drop(permit);
                            return;
                        }
                        let mut ids = Vec::new();
                        if let Some(_ids) = lspd_scope.get(&device.serial) {
                            ids.extend(_ids);
                        }
                        for id in ids {
                            let res = device.execute_as_root(&format!("sqlite3 /data/adb/lspd/config/modules_config.db \\\"INSERT INTO scope(mid,app_pkg_name,user_id) SELECT {1}, '{0}',0 WHERE NOT EXISTS(SELECT 1 FROM scope WHERE mid = {1} AND app_pkg_name = '{0}');\\\"",info.package_name,id),10).await;
                            match res {
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!("Error inserting scope {}", e);
                                }
                            }
                        }
                        device
                            .execute_as_root(
                                &format!(
                                    "/data/adb/lspd/bin/cli scope set -a just.trust.me {}/0",
                                    info.package_name
                                ),
                                10,
                            )
                            .await
                            .unwrap();

                        device.enable(&info).await.unwrap();
                        device.grant_permissions(&info).await.unwrap();
                        device
                            .execute(&format!("monkey -p {} -v 1", info.package_name))
                            .await
                            .unwrap();
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        let final_ts = Instant::now() + Duration::from_secs(60 * 5);
                        device
                            .execute(
                                "input keyevent 3;input keyevent 4;input keyevent KEYCODE_WAKEUP",
                            )
                            .await
                            .unwrap();
                        let lock = parallel_events.clone().acquire_owned().await.unwrap();
                        if let Err(e) = device.start_services(&info).await {
                            eprintln!("Failed to start services for {}: {}", info.package_name, e);
                        }
                        drop(lock);
                        device
                            .execute(
                                "input keyevent 3;input keyevent 4;input keyevent KEYCODE_WAKEUP",
                            )
                            .await
                            .unwrap();
                        // let mut action = Vec::new();
                        for r in info.receivers.iter() {
                            for a in r.actions.to_owned() {
                                let lock = parallel_events.clone().acquire_owned().await.unwrap();
                                _ = device.send_action(&info.package_name, &a).await;
                                drop(lock);
                            }
                        }
                        for r in info.services.iter() {
                            
                            for a in r.actions.to_owned() {
                                let lock = parallel_events.clone().acquire_owned().await.unwrap();
                                _ = device.send_action(&info.package_name, &a).await;
                                drop(lock);
                            }
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                        tokio::time::sleep_until(final_ts).await;
                        _ = tokio::fs::remove_file(path.join("analysis_result").join("lock.txt"))
                            .await;
                        if let Ok(mut f) =
                            File::create(path.join("analysis_result").join("done.txt")).await
                        {
                            let _ = f.write_all(device.serial.as_bytes()).await;
                        }
                        _ = device.execute_as_root(&format!("sqlite3 /data/adb/lspd/config/modules_config.db \\\"DELETE FROM scope WHERE app_pkg_name = '{0}';\\\"",info.package_name), 10).await;
                        if let Some(uid) = info.uid {
                            ongoing.write().await.remove(&uid);
                            if let Some(mut f) = event_files.write().await.remove(&uid){
                                f.sync_all().await.unwrap();
                                f.shutdown().await.unwrap();
                            }
                        }
                        drop(permit);
                    }));
                }
                for task in tasks {
                    _ = task.await;
                }
            }));
            let mitm_proxy= mitm_proxy.clone();
            tokio::spawn(async move {
                for task in device_tasks {
                    _ = task.await;
                }
                mitm_proxy.lock().await.kill().await.unwrap();
                exit(0);
            });
        }
        Ok(())
    }
}