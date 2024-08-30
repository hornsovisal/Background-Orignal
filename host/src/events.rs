use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SocketAddr {
    ip: String,
    port: u16,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SchedProcessFork {
    parent_pid: i64,
    parent_tid: i64,
    child_pid: i64,
    uid: i64,
    pub ts: u128,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TaskInfo {
    parent_pid: i64,
    parent_tid: i64,
    uid: i64,
    pub ts: u128,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SocketAddrV6 {
    uid: i64,
    pid: i64,
    tid: i64,
    protocol: u8,
    r#type: u8,
    left_socket_addr: SocketAddr,
    right_socket_addr: SocketAddr,
    pub ts: u128,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SocketAddrV4 {
    uid: i64,
    pid: i64,
    tid: i64,
    protocol: u8,
    r#type: u8,
    left_socket_addr: SocketAddr,
    right_socket_addr: SocketAddr,
    pub ts: u128,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Events {
    JobScheduler(JobScheduler),
    OnStartJob(OnStartJob),
    Intent(Intent),
    OnReceiveHook(OnReceiveHook),
    OnCreate(OnCreate),
    GetBroadcast(GetBroadcast),
    DoFinal(DoFinal),
    DoFinalByte(DoFinalByte),
    DoFinalByteInt(DoFinalByteInt),
    DoFinalByteIntInt(DoFinalByteIntInt),
    DoFinalByteIntIntByte(DoFinalByteIntIntByte),
    DoFinalByteIntIntByteInt(DoFinalByteIntIntByteInt),
    DoFinalByteBufferByteBuffer(DoFinalByteBufferByteBuffer),
    InitIntCertificate(InitIntCertificate),
    InitIntKey(InitIntKey),
    UpdateByte(UpdateByte),
    UpdateByteIntInt(UpdateByteIntInt),
    UpdateByteIntIntByte(UpdateByteIntIntByte),
    UpdateByteIntIntByteInt(UpdateByteIntIntByteInt),
    UpdateByteBufferByteBuffer(UpdateByteBufferByteBuffer),
    UpdateAADByte(UpdateAADByte),
    UpdateAADByteIntInt(UpdateAADByteIntInt),
    UpdateAADByteBuffer(UpdateAADByteBuffer),
    StartForeground(StartForeground),
    OnStartCommand(OnStartCommand),
    OnBind(OnBind),
    SetImpl(SetImpl),
    SetAlarmClock(SetAlarmClock),
    PendingIntentGet(PendingIntentGet),
    ScheduleInternal(ScheduleInternal),
    RequestWork(RequestWork),
    DoWork(DoWork),
    GenericHook(GenericHook),
    RegisterReceiver(RegisterReceiver),
    DeviceID(DeviceID),
    Canary(Canary),
    StartThreadHook(StartThreadHook),
    SchedProcessFork(SchedProcessFork),
    SchedProcessExit(TaskInfo),
    TaskNewtask(TaskInfo),
    // TaskRename(TaskInfo),
    SocketAddrV6(SocketAddrV6),
    SocketAddrV4(SocketAddrV4),
    SocketHook(SocketHook),
    ConnectHook(ConnectHook),
    #[serde(skip_deserializing)]
    Other(String),
}

impl Events {
    pub fn get_uid(&self) -> i64 {
        match self {
            Events::JobScheduler(e) => e.uid,
            Events::OnStartJob(e) => e.uid,
            Events::Intent(e) => e.uid,
            Events::OnReceiveHook(e) => e.uid,
            Events::OnCreate(e) => e.uid,
            Events::GetBroadcast(e) => e.uid,
            Events::DoFinal(e) => e.uid,
            Events::DoFinalByte(e) => e.uid,
            Events::DoFinalByteInt(e) => e.uid,
            Events::DoFinalByteIntInt(e) => e.uid,
            Events::DoFinalByteIntIntByte(e) => e.uid,
            Events::DoFinalByteIntIntByteInt(e) => e.uid,
            Events::DoFinalByteBufferByteBuffer(e) => e.uid,
            Events::InitIntCertificate(e) => e.uid,
            Events::InitIntKey(e) => e.uid,
            Events::UpdateByte(e) => e.uid,
            Events::UpdateByteIntInt(e) => e.uid,
            Events::UpdateByteIntIntByte(e) => e.uid,
            Events::UpdateByteIntIntByteInt(e) => e.uid,
            Events::UpdateByteBufferByteBuffer(e) => e.uid,
            Events::UpdateAADByte(e) => e.uid,
            Events::UpdateAADByteIntInt(e) => e.uid,
            Events::UpdateAADByteBuffer(e) => e.uid,
            Events::StartForeground(e) => e.uid,
            Events::OnStartCommand(e) => e.uid,
            Events::OnBind(e) => e.uid,
            Events::SetImpl(e) => e.uid,
            Events::SetAlarmClock(e) => e.uid,
            Events::PendingIntentGet(e) => e.uid,
            Events::ScheduleInternal(e) => e.uid,
            Events::RequestWork(e) => e.uid,
            Events::DoWork(e) => e.uid,
            Events::GenericHook(e) => e.uid,
            Events::RegisterReceiver(e) => e.uid,
            Events::DeviceID(e) => e.uid,
            Events::Canary(e) => e.uid,
            Events::StartThreadHook(e) => e.uid,
            Events::SchedProcessFork(e) => e.uid,
            Events::SchedProcessExit(e) => e.uid,
            Events::TaskNewtask(e) => e.uid,
            // Events::TaskRename(e) => e.uid,
            Events::SocketAddrV6(e) => e.uid,
            Events::SocketAddrV4(e) => e.uid,
            Events::SocketHook(e) => e.uid,
            Events::ConnectHook(e) => e.uid,
            Events::Other(_) => 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobScheduler {
    class_name: String,
    method_name: String,
    hashcode: i64,
    pub job_id: i64,
    backoff_policy: i64,
    clip_data: String,
    clip_grant_flags: i64,
    estimated_network_download_bytes: i64,
    estimated_network_upload_bytes: i64,
    flex_millis: i64,
    initial_backoff_millis: i64,
    interval_millis: i64,
    max_execution_delay_millis: i64,
    min_latency_millis: i64,
    minimum_network_chunk_bytes: i64,
    network_type: i64,
    priority: i64,
    service: String,
    required_network_capabilities: Vec<i32>,
    required_network_transport_types: Vec<i32>,
    trigger_content_max_delay: i64,
    trigger_content_update_delay: i64,
    trigger_content_uris: String,
    is_expedited: bool,
    is_important_while_foreground: bool,
    is_periodic: bool,
    is_persisted: bool,
    is_prefetch: bool,
    is_require_battery_not_low: bool,
    is_require_charging: bool,
    is_require_device_idle: bool,
    is_require_storage_not_low: bool,
    pid: i64,
    tid: i64,
    pub uid: i64,
    pub package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OnStartJob {
    class_name: String,
    method_name: String,
    hashcode: i64,
    job_id: i64,
    pid: i64,
    ts: u128,
    tid: i64,
    uid: i64,
    package_name: String,
    start_ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Intent {
    pub class_name: String,
    pub method_name: String,
    hashcode: i64,
    pub action: String,
    pub component: String,
    pub data: String,
    pub extras: Vec<(String, String)>,
    pid: i64,
    tid: i64,
    pub uid: i64,
    pub package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OnReceiveHook {
    class_name: String,
    method_name: String,
    hashcode: i64,
    pub action: String,
    pub component: String,
    pub data: String,
    pub extras: Vec<(String, String)>,
    start_ts: u128,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OnCreate {
    class_name: String,
    method_name: String,
    hashcode: i64,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetBroadcast {
    class_name: String,
    method_name: String,
    hashcode: i64,
    request_code: i64,
    pub action: String,
    pub component: String,
    pub data: String,
    pub extras: Vec<(String, String)>,
    flags: i64,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinal {
    class_name: String,
    method_name: String,
    hashcode: i64,
    ret: String,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinalByte {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_input: String,
    ret: String,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinalByteInt {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_output: String,
    arg_output_offset: i64,
    ret: i64,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinalByteIntInt {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_input: String,
    arg_input_offset: i64,
    arg_input_len: i64,
    ret: String,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinalByteIntIntByte {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_input: String,
    arg_input_offset: i64,
    arg_input_len: i64,
    arg_output: String,
    ret: i64,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinalByteIntIntByteInt {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_input: String,
    arg_input_offset: i64,
    arg_input_len: i64,
    arg_output: String,
    arg_output_offset: i64,
    ret: i64,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinalByteBufferByteBuffer {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_input: String,
    arg_output: String,
    ret: i64,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InitIntCertificate {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_opcode: i64,
    arg_certificate: String,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InitIntKey {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_opcode: i64,
    arg_key: String,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateByte {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_input: String,
    ret: String,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateByteIntInt {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_input: String,
    arg_input_offset: i64,
    arg_input_len: i64,
    ret: String,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateByteIntIntByte {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_input: String,
    arg_input_offset: i64,
    arg_input_len: i64,
    arg_output: String,
    ret: i64,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateByteIntIntByteInt {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_input: String,
    arg_input_offset: i64,
    arg_input_len: i64,
    arg_output: String,
    arg_output_offset: i64,
    ret: i64,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateByteBufferByteBuffer {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_input: String,
    arg_output: String,
    ret: i64,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateAADByte {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_input: String,
    ret: String,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateAADByteIntInt {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_input: String,
    arg_input_offset: i64,
    arg_input_len: i64,
    ret: String,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateAADByteBuffer {
    class_name: String,
    method_name: String,
    hashcode: i64,
    arg_src: String,
    ret: i64,
    algorithm: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StartForeground {
    class_name: String,
    method_name: String,
    hashcode: i64,
    id: i64,
    r#type: i64,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OnStartCommand {
    class_name: String,
    method_name: String,
    hashcode: i64,
    pub action: String,
    pub component: String,
    pub data: String,
    pub extras: Vec<(String, String)>,
    flags: i64,
    start_id: i64,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OnBind {
    class_name: String,
    method_name: String,
    hashcode: i64,
    pub action: String,
    pub component: String,
    pub data: String,
    pub extras: Vec<(String, String)>,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SetImpl {
    class_name: String,
    method_name: String,
    hashcode: i64,
    r#type: i64,
    trigger_at_millis: i64,
    window_millis: i64,
    interval_millis: i64,
    flags: i64,
    operation_hashcode: i64,
    listener_class_name: String,
    listener_tag: String,
    alarm_clock_intent_hash_code: i64,
    alarm_clock_get_trigger_time: i64,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SetAlarmClock {
    class_name: String,
    method_name: String,
    hashcode: i64,
    creator: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PendingIntentGet {
    class_name: String,
    method_name: String,
    hashcode: i64,
    request_code: i64,
    pub action: String,
    pub component: String,
    pub data: String,
    pub extras: Vec<(String, String)>,
    flags: i64,
    pid: i64,
    tid: i64,
    uid: i64,
    pub package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScheduleInternal {
    class_name: String,
    method_name: String,
    hashcode: i64,
    pub job_id: i64,
    work_id: String,
    pid: i64,
    tid: i64,
    pub uid: i64,
    pub package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RequestWork {
    class_name: String,
    method_name: String,
    hashcode: i64,
    work_id: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoWork {
    class_name: String,
    method_name: String,
    hashcode: i64,
    work_id: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GenericHook {
    class_name: String,
    method_name: String,
    hashcode: i64,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RegisterReceiver {
    class_name: String,
    method_name: String,
    hashcode: i64,
    pub register_class: String,
    pub actions: Vec<String>,
    pid: i64,
    tid: i64,
    uid: i64,
    pub package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeviceID {
    class_name: String,
    method_name: String,
    hashcode: i64,
    device_id: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StartThreadHook {
    class_name: String,
    method_name: String,
    hashcode: i64,
    new_jtid: u64,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Canary {
    class_name: String,
    method_name: String,
    hashcode: i64,
    canary: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SocketHook {
    class_name: String,
    method_name: String,
    hashcode: i64,
    local_addr: String,
    remote_addr: String,
    canary: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConnectHook {
    class_name: String,
    method_name: String,
    hashcode: i64,
    local_addr: String,
    remote_addr: String,
    pid: i64,
    tid: i64,
    uid: i64,
    package_name: String,
    ts: u128,
    jtid: u64,
    importance: u64,
    stack_trace: String,
}
