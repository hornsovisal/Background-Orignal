use quick_xml::events::BytesStart;
use std::{collections::HashSet, net::Ipv4Addr, str::FromStr, sync::Arc, time::Duration};
use tokio::{fs, process::Command, sync::watch, time::timeout};

use crate::{action_db::ActionDb, error::Error};

pub fn strip_trailing_newline(input: &str) -> &str {
    input
        .strip_suffix("\r\n")
        .or(input.strip_suffix("\n"))
        .unwrap_or(&input)
}

#[derive(Debug)]
pub struct DeviceManager {
    pub devices: Vec<Device>,
}

impl DeviceManager {
    pub async fn new(action_db: ActionDb) -> Self {
        let mut devices = Vec::new();
        for d in Command::new("adb")
            .arg("devices")
            .output()
            .await
            .unwrap()
            .stdout
            .split(|&c| c == b'\n')
            .skip(1)
            .filter(|d| !d.is_empty())
        {
            let d = String::from_utf8(d.to_vec()).unwrap();
            let serial = d.split('\t').next().unwrap();
            devices.push(Device::new(serial, action_db.clone()).await);
        }

        Self { devices }
    }
}




#[derive(Debug, Clone)]
pub struct Device {
    pub ipv4: Option<Ipv4Addr>,
    pub serial: String,
    pub action_db: ActionDb,
    pub last_deferred_broadcast: Arc<watch::Receiver<String>>,
}

impl Device {
    pub async fn execute(&self, command: &str) -> Result<String, Error> {
        self.execute_with_timeout(command, 20).await
    }
    pub async fn execute_with_timeout_nth(
        &self,
        command: &str,
        sec: u64,
        n: u8,
    ) -> Result<String, Error> {
        for _ in 0..n {
            match self.execute_with_timeout(command, sec).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    println!("Error: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
        Ok(String::new())
    }

    pub async fn execute_with_timeout(&self, command: &str, sec: u64) -> Result<String, Error> {
        let mut freez_try = 0;
        let mut r#try = 0;
        // let random = rand::random::<u64>();

        loop {
            // println!(">{}:{}:{}", command, random,sec);
            let exec = timeout(
                Duration::from_secs(sec),
                Command::new("adb")
                    .arg("-s")
                    .arg(&self.serial)
                    .arg("shell")
                    // .arg("echo")
                    .arg(format!("{command}"))
                    .output(),
            )
            .await;

            // println!("<{}:{}:{}", command, random,sec);
            match exec {
                Ok(Ok(res)) => {
                    // dbg!(&res);
                    let res = String::from_utf8(res.stdout)
                        .map_err(|e| Error::AdbError(e.to_string()))?;
                    // println!("<{}:{}:{} -> {}", command, random,sec,res);
                    if res.starts_with("cmd: Can't find service")
                        || res.starts_with("cmd: Failure calling service")
                    {
                        freez_try += 1;
                        dbg!("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
                        dbg!("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
                        dbg!("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
                        dbg!("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
                        if freez_try >= 5 {
                            dbg!("ISSSUEEE");
                            std::process::exit(0);
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                        continue;
                    } else if res.starts_with("error: device") && res.ends_with("not found") {
                        // println!("Device lost/not found:{}",random);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        r#try += 1;
                        if r#try == 3 {
                            return Err(Error::AdbError(res));
                        }
                        continue;
                    }

                    return Ok(res);
                }
                Ok(Err(e)) => {
                    // println!("{}:{} -> {}", command,random, &e);
                    return Err(e)?;
                }
                Err(e) => {
                    // println!("{}:{} -> {}", command,random, &e);
                    r#try += 1;
                    if r#try == 2 {
                        return Err(Error::AdbError(e.to_string()));
                    }
                    continue;
                }
            }
        }
        // let res = self.d.execute_host_shell_command(command).await.unwrap();
        // println!("{} -> {}", command, &res);
        // Ok(res)
        // let child = Command::new("adb")
        //     .arg("-s")
        //     .arg(&self.serial)
        //     .arg("shell")
        //     .arg("sh")
        //     .arg("-c")
        //     .arg(format!("\"{}\"", command))
        //     .output()
        //     .await?;
        // Ok(String::from_utf8(child.stdout).unwrap())
    }
    pub async fn execute_as_root(
        &self,
        command: &str,
        timeout: impl Into<Option<u64>>,
    ) -> Result<String, Error> {
        if let Some(sec) = timeout.into() {
            self.execute_with_timeout(&format!("su -c \"{}\"", command), sec)
                .await
        } else {
            self.execute(&format!("su -c \"{}\"", command)).await
        }
    }
    pub async fn execute_with_timeout_nth_as_root(
        &self,
        command: &str,
        timeout: impl Into<Option<u64>>,
        n: u8,
    ) -> Result<String, Error> {
        if let Some(sec) = timeout.into() {
            self.execute_with_timeout_nth(&format!("su -c \"{}\"", command), sec, n)
                .await
        } else {
            self.execute(&format!("su -c \"{}\"", command)).await
        }
    }
    pub async fn new(serial: &str, action_db: ActionDb) -> Self {
        let (watch_sender, watch_receiver) = watch::channel(String::new());

        let mut device = Self {
            serial: serial.to_string(),
            ipv4: None,
            action_db,
            last_deferred_broadcast: Arc::new(watch_receiver),
        };
        let ip = device
            .execute("ifconfig wlan0 | grep Bcast | cut -d : -f 2 | cut -d \" \" -f 1")
            .await
            .ok()
            .as_ref()
            .map(|s| strip_trailing_newline(s))
            .and_then(|ip| Ipv4Addr::from_str(ip).ok());
        device.ipv4 = ip;
        let d = device.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if let Ok(res) = d
                    .execute_with_timeout(
                        "dumpsys activity broadcasts | grep \"Deferred Ordered\" -A 3",
                        10,
                    )
                    .await
                {
                    watch_sender.send(res).unwrap();
                }
            }
        });
        device
    }
    pub fn get_apk_info(manifest_path: &str) -> Result<AppInfo, Error> {
        let extract_attr = |tag: &BytesStart, attr: &str| -> Option<String> {
            let Some(Ok(name)) = tag
                .attributes()
                .find(|a| a.as_ref().map(|a| a.key.as_ref()) == Ok(attr.as_bytes()))
            else {
                return None;
            };
            String::from_utf8(name.value.to_vec()).ok()
        };
        let extract_tag_name = |tag: &BytesStart| -> Option<String> {
            String::from_utf8(tag.name().into_inner().to_vec()).ok()
        };
        let mut services = HashSet::<Service>::new();
        let mut receivers = HashSet::new();
        let mut permissions = HashSet::new();
        let mut package_name = String::new();
        let mut parent = Vec::new();
        let mut reader = quick_xml::Reader::from_file(manifest_path).unwrap();
        let mut buf = Vec::new();
        let mut ongoing_service = None::<Service>;
        let mut ongoing_receiver = None::<Receiver>;
        let mut actions = Vec::<String>::new();
        loop {
            match reader.read_event_into(&mut buf).unwrap() {
                quick_xml::events::Event::Start(t) => {
                    let tag_name = extract_tag_name(&t).unwrap();
                    if tag_name == "manifest" {
                        if let Some(name) = extract_attr(&t, "package") {
                            package_name = name;
                        };
                    } else if parent == ["manifest", "application"]
                        && vec!["service", "receiver"].contains(&tag_name.as_str())
                    {
                        let Some(name) =
                            extract_attr(&t, "android:name").or(extract_attr(&t, "name"))
                        else {
                            continue;
                        };
                        let permission = extract_attr(&t, "android:permission")
                            .or(extract_attr(&t, "permission"));
                        if tag_name == "receiver" {
                            ongoing_receiver.replace(Receiver {
                                name,
                                permission,
                                actions: Default::default(),
                            });
                        } else {
                            ongoing_service.replace(Service {
                                name,
                                permission,
                                exported: extract_attr(&t, "android:exported")
                                    .or(extract_attr(&t, "exported"))
                                    .unwrap_or("false".to_string())
                                    == "true",
                                actions: Default::default(),
                                foreground_types: extract_attr(&t, "android:foregroundServiceType")
                                    .or(extract_attr(&t, "foregroundServiceType")),
                            });
                        }
                    }
                    parent.push(tag_name);
                }
                quick_xml::events::Event::End(_) => {
                    if parent == ["manifest", "application", "service"] {
                        if let Some(mut service) = ongoing_service.take() {
                            service.actions.extend(actions.drain(..));
                            services.insert(service);
                        }
                    } else if parent == ["manifest", "application", "receiver"] {
                        if let Some(mut receiver) = ongoing_receiver.take() {
                            receiver.actions.extend(actions.drain(..));
                            receivers.insert(receiver);
                        }
                    }
                    parent.pop();
                }
                quick_xml::events::Event::Empty(t) => {
                    let tag_name = extract_tag_name(&t).unwrap();

                    if (parent == ["manifest", "application", "service", "intent-filter"]
                        || parent == ["manifest", "application", "receiver", "intent-filter"])
                        && tag_name == "action"
                    {
                        let action = extract_attr(&t, "android:name")
                            .or(extract_attr(&t, "name"))
                            .unwrap();
                        actions.push(action);
                    }
                    if ["uses-permission", "permission"].contains(&tag_name.as_str()) {
                        if let Some(p) =
                            extract_attr(&t, "android:name").or(extract_attr(&t, "name"))
                        {
                            permissions.insert(p);
                        }
                    } else if parent == ["manifest", "application"]
                        && ["service", "receiver"].contains(&tag_name.as_str())
                    {
                        let Some(name) =
                            extract_attr(&t, "android:name").or(extract_attr(&t, "name"))
                        else {
                            continue;
                        };
                        let permission = extract_attr(&t, "android:permission")
                            .or(extract_attr(&t, "permission"));
                        if tag_name == "receiver" {
                            receivers.insert(Receiver {
                                name,
                                permission,
                                actions: Default::default(),
                            });
                        } else {
                            services.insert(Service {
                                name,
                                permission,
                                exported: extract_attr(&t, "android:exported")
                                    .or(extract_attr(&t, "exported"))
                                    .unwrap_or("false".to_string())
                                    == "true",
                                actions: Default::default(),
                                foreground_types: extract_attr(&t, "android:foregroundServiceType")
                                    .or(extract_attr(&t, "foregroundServiceType")),
                            });
                        }
                    }
                }
                // quick_xml::events::Event::Text(d) => {}
                // quick_xml::events::Event::CData(_) => {}
                // quick_xml::events::Event::Comment(_) => {}
                // quick_xml::events::Event::Decl(_) => {}
                // quick_xml::events::Event::PI(_) => {}
                // quick_xml::events::Event::DocType(_) => {}
                quick_xml::events::Event::Eof => break,
                _ => continue,
            }

            buf.clear();
        }
        Ok(AppInfo {
            package_name,
            services,
            receivers,
            permissions,
            uid: None,
        })
    }
    // pub async fn pull(&self, path: &str, dest: &mut tokio::fs::File) -> Result<(), Error> {
    //     // self.d.pull(UnixPath::new(path), dest).await?; todo
    //     Ok(())
    // }

    pub async fn install_apk(&self, apk_folder: &str) -> Result<AppInfo, Error> {
        // dbg!(1);
        let manifest_path = format!("{}/{}", apk_folder, "AndroidManifest.xml");
        let mut app_info = Self::get_apk_info(&manifest_path)?;
        let mut dir_content = fs::read_dir(apk_folder).await.unwrap();
        let mut args = vec!["-s".to_string(), self.serial.clone(), "install-multiple".to_string(), "-g".to_string(), format!("{}/base.apk", apk_folder)];

        loop {
            // dbg!("loop");
            let entry = dir_content.next_entry().await.unwrap();
            if entry.is_none() {
                break;
            }
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "apk")
                && path.file_name().unwrap() != "base.apk"
            {
                if path.file_name().unwrap() != "base.apk" {
                    args.push(format!(
                        "{}/{}",
                        apk_folder,
                        path.file_name().unwrap().to_str().unwrap()
                    ));
                }
            }
        }
        

        let install_result = Command::new("adb")
            .args(args)
            .output()
            .await?;
        if install_result.stdout.starts_with(b"Success") {
            // dbg!(7.1);
            let x = self.get_package_uid(&app_info.package_name).await.unwrap();
            app_info.uid.replace(x);
            // dbg!(8);
            Ok(app_info)
        } else {
            // dbg!(9);
            return Err(Error::AdbError(format!(
                "Error installing APKs {}",
                apk_folder
            )));
        }

        // let mut args = Vec::new();

        // let mut args = Vec::new();
        // let base = format!("{}/{}", apk_folder, "base.apk");
        // let manifest_path = format!("{}/{}", apk_folder, "AndroidManifest.xml");
        // let mut app_info = Self::get_apk_info(&manifest_path)?;
        // args.push(base);
        // let mut dir_content = fs::read_dir(apk_folder).await.unwrap();
        // loop {
        //     let entry = dir_content.next_entry().await.unwrap();
        //     if entry.is_none() {
        //         break;
        //     }

        //     let entry = entry.unwrap();
        //     let path = entry.path();
        //     if path.extension().is_some_and(|e| e == "apk") {
        //         if path.file_name().unwrap() == "base.apk" {
        //             continue;
        //         }
        //         args.push(path.to_str().unwrap().to_string());
        //     }
        // }
        // dbg!(&args);
        // let child = Command::new("adb")
        //     .arg("-s")
        //     .arg(&self.serial)
        //     .arg("install-multiple")
        //     .arg("-g")
        //     .args(&args)
        //     .output()
        //     .await
        //     .unwrap();
        // if child.stdout == b"Success\n" {
        //     let x = self.get_package_uid(&app_info.package_name).await.unwrap();
        //     app_info.uid.replace(x);
        //     println!("APKs installed successfully");
        //     Ok(app_info)
        // } else {
        //     println!(
        //         "Error installing APKs: {}",
        //         String::from_utf8(child.stderr).unwrap()
        //     );
        //     return Err(anyhow!("Error installing APKs"));
        // }
    }
    pub async fn uninstall(&self, package_name: &str) -> Result<(), Error> {
        self.execute_with_timeout(&format!("pm clear {0};pm uninstall {0}", package_name), 30)
            .await?;
        Ok(())
    }
    pub async fn disable(&self, app_info: &AppInfo) -> Result<(), Error> {
        self.execute_as_root(&format!("pm disable {}", app_info.package_name), 10)
            .await?;
        Ok(())
    }
    pub async fn enable(&self, app_info: &AppInfo) -> Result<(), Error> {
        self.execute_as_root(&format!("pm enable {}", app_info.package_name), 10)
            .await?;
        Ok(())
    }

    pub async fn grant_permissions(&self, app_info: &AppInfo) -> Result<(), Error> {
        let mut normal_perm_command = app_info
            .permissions
            .iter()
            .map(|p| format!("pm grant {} {}", p, app_info.package_name))
            .collect::<Vec<String>>();
        let appops_perms = [
            "COARSE_LOCATION",
            "FINE_LOCATION",
            "READ_CONTACTS",
            "POST_NOTIFICATION",
            "CALL_PHONE",
            "CAMERA",
            "RECORD_AUDIO",
            "READ_EXTERNAL_STORAGE",
            "WRITE_EXTERNAL_STORAGE",
            "READ_MEDIA_AUDIO",
            "READ_MEDIA_VIDEO",
            "READ_MEDIA_IMAGES",
            "LEGACY_STORAGE",
            "FINE_LOCATION",
            "WRITE_SMS",
            "SYSTEM_ALERT_WINDOW",
            "WAKE_LOCK",
            "READ_PHONE_STATE",
            "WRITE_MEDIA_AUDIO",
            "WRITE_MEDIA_VIDEO",
            "WRITE_MEDIA_IMAGES",
            "READ_DEVICE_IDENTIFIERS",
            "MANAGE_EXTERNAL_STORAGE",
            "NO_ISOLATED_STORAGE",
            "USE_ICC_AUTH_WITH_DEVICE_IDENTIFIER",
        ];
        let appops_perm_command = appops_perms
            .iter()
            .map(|p| format!("appops set {} {} allow", app_info.package_name, p))
            .collect::<Vec<String>>();
        normal_perm_command.extend(appops_perm_command);

        self.execute(&normal_perm_command.join(";")).await?;
        let mut service_receiver_perms = app_info
            .services
            .iter()
            .filter_map(|s| s.permission.as_ref().map(|p| (s.name.clone(), p)))
            .collect::<HashSet<_>>();
        service_receiver_perms.extend(
            app_info
                .receivers
                .iter()
                .filter_map(|s| s.permission.as_ref().map(|p| (s.name.clone(), p)))
                .collect::<HashSet<_>>(),
        );
        for (name, perm) in service_receiver_perms {
            if perm == "android.permission.BIND_NOTIFICATION_LISTENER_SERVICE" {
                self.execute(&format!(
                    "cmd notification allow_listener {}/{}",
                    app_info.package_name, name
                ))
                .await?;
            }
            if perm == "android.permission.BIND_ACCESSIBILITY_SERVICE" {
                self.execute(&format!(
                    "settings put secure enabled_accessibility_services {}/{}",
                    app_info.package_name, name
                ))
                .await?;
            }
        }
        Ok(())
    }
    // pub async fn send_actions(&self, app_info: &AppInfo) -> Result<(), Error> {
    //     // adb shell am broadcast -p p -a com.whereismywifeserver.intent.TEST --es sms_body "test from adb"
    //     // adb shell am start-foreground-service com.larini.sunriseapp/.NewService
    //     // adb shell am startservice com.larini.sunriseapp/.NewService
    //     let mut actions = app_info
    //         .services
    //         .iter()
    //         .flat_map(|s| s.actions.iter())
    //         .collect::<Vec<_>>();
    //     actions.extend(app_info.receivers.iter().flat_map(|r| r.actions.iter()));
    //     let commands = actions
    //         .iter()
    //         .map(|a| format!("am broadcast -p {} -a {}", app_info.package_name, a))
    //         .collect::<Vec<String>>()
    //         .join(";");
    //     println!("{:?}", commands);
    //     println!("{}", self.execute_as_root(&commands, 10).await?);
    //     Ok(())
    // }
    pub async fn send_action(
        &self,
        pkg_name: &str,
        action: &str,
        component_name: &str,
    ) -> Result<(), Error> {
        let last_deferred_broadcast = self.last_deferred_broadcast.clone();
        loop {
            if last_deferred_broadcast
                .borrow()
                .clone()
                .contains(format!("pkg={}", pkg_name).as_str())
            {
                println!("{} is still has deferred action", pkg_name);
                tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
                continue;
            } else {
                self.execute_with_timeout_nth_as_root(&format!("am broadcast --allow-background-activity-starts --async --user current -p {} -a {} -n {}", pkg_name, action,component_name), 20,5)
            .await?;
                if let Ok(args) = self.action_db.get_action_args(action.into()) {
                    for arg in args {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        self.execute_with_timeout_nth_as_root(
                    &format!("am broadcast --allow-background-activity-starts --async --user current -p {} -a {} -d {} -n {}", pkg_name, action, arg,component_name),
                    20,5
                )
                .await?;
                    }
                }
                break;
            }
        }

        Ok(())
    }
    pub async fn start_services(&self, app_info: &AppInfo) -> Result<(), Error> {
        let mut commands = Vec::new();
        for service in app_info.services.iter().filter(|s| s.exported) {
            commands.push(format!(
                "am start-foreground-service -n {}/{}",
                app_info.package_name, service.name
            ));
        }
        self.execute_with_timeout(&commands.join(";"), 30).await?;
        Ok(())
    }
    async fn get_package_uid(&self, package_name: &str) -> Result<i64, Error> {
        let output = self
            .execute(&format!("pm list packages -U {}", package_name))
            .await?;
        let uid = output
            .lines()
            .find(|l| l.starts_with("package:"))
            .unwrap()
            .split(':')
            .nth(2)
            .unwrap()
            .parse::<i64>()
            .unwrap();
        Ok(uid)
    }
    pub async fn list_of_3rd_party_apps(&self) -> Result<Vec<String>, Error> {
        let output = self.execute("pm list packages -3").await?;
        let packages = output
            .split('\n')
            .filter(|l| !l.is_empty())
            .map(|l| l.trim_start_matches("package:"))
            .collect::<Vec<_>>();
        let mut apps = Vec::new();
        for package in packages {
            if [
                "just.trust.me",
                "com.topjohnwu.magisk",
                "com.gauravssnl.bypassrootcheck.pro",
                "ca.concordia.madiba.cryptologger",
            ]
            .iter()
            .any(|p| package.starts_with(p))
            {
                continue;
            }
            apps.push(package.to_string());
        }
        Ok(apps)
    }
}

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct Service {
    pub name: String,
    permission: Option<String>,
    exported: bool,
    pub actions: Vec<String>,
    foreground_types: Option<String>,
}
#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct Receiver {
    pub name: String,
    permission: Option<String>,
    pub actions: Vec<String>,
}

#[derive(Debug)]
pub struct AppInfo {
    pub package_name: String,
    pub services: HashSet<Service>,
    pub receivers: HashSet<Receiver>,
    pub permissions: HashSet<String>,
    pub uid: Option<i64>,
}

impl Clone for AppInfo {
    fn clone(&self) -> Self {
        let services = self
            .services
            .iter()
            .map(|s| s.clone())
            .collect::<HashSet<_>>();
        let receivers = self
            .receivers
            .iter()
            .map(|r| r.clone())
            .collect::<HashSet<_>>();
        Self {
            package_name: self.package_name.clone(),
            services,
            receivers,
            permissions: self.permissions.clone(),
            uid: self.uid,
        }
    }
}
