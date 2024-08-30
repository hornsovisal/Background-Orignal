use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum EbpfEvent {
    ProcessFork(ProcessFork, u64),
    SocketAddrV6(SocketAddrV6, u64),
    SocketAddrV4(SocketAddrV4, u64),
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SocketAddr {
    ip: String,
    port: u16,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProcessFork {
    parent_pid: u32,
    parent_tid: u32,
    child_pid: u32,
    uid: u32,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SocketAddrV6 {
    uid: u32,
    pid: u32,
    tid: u32,
    protocol: u8,
    r#type: u8,
    left_socket_addr: SocketAddr,
    right_socket_addr: SocketAddr,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SocketAddrV4 {
    uid: u32,
    pid: u32,
    tid: u32,
    protocol: u8,
    r#type: u8,
    left_socket_addr: SocketAddr,
    right_socket_addr: SocketAddr,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged,deny_unknown_fields)]
pub enum Events {
    JobScheduler(JobScheduler),
    OnStartJob(OnStartJob),
    Intent(Intent),
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
    RequestAndDoWork(RequestAndDoWork),
    GenericHook(GenericHook),
    EbpfEvent(EbpfEvent),
    Other(String),
}

impl Events {
    pub fn get_uid(&self) -> u32 {
        match self {
            Events::JobScheduler(e) => e.uid,
            Events::OnStartJob(e) => e.uid,
            Events::Intent(e) => e.uid,
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
            Events::RequestAndDoWork(e) => e.uid,
            Events::GenericHook(e) => e.uid,
            Events::EbpfEvent(e) => match e {
                EbpfEvent::ProcessFork(e, _ts) => e.uid,
                EbpfEvent::SocketAddrV6(e, _ts) => e.uid,
                EbpfEvent::SocketAddrV4(e, _ts) => e.uid,
            },
            Events::Other(_) => 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobScheduler {
    class_name: String,
    method_name: String,
    hashcode: i32,
    pub job_id: u32,
    backoff_policy: i32,
    clip_data: String,
    clip_grant_flags: i32,
    estimated_network_download_bytes: i64,
    estimated_network_upload_bytes: i64,
    flex_millis: i64,
    initial_backoff_millis: i64,
    interval_millis: i64,
    max_execution_delay_millis: i64,
    min_latency_millis: i64,
    minimum_network_chunk_bytes: i64,
    network_type: i32,
    priority: i32,
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
    pid: i32,
    tid: i32,
    pub uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OnStartJob {
    class_name: String,
    method_name: String,
    hashcode: i32,
    job_id: i32,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Intent {
    class_name: String,
    method_name: String,
    hashcode: i32,
    pub action: String,
    pub component: String,
    pub data: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OnCreate {
    class_name: String,
    method_name: String,
    hashcode: i32,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetBroadcast {
    class_name: String,
    method_name: String,
    hashcode: i32,
    request_code: i32,
    pub action: String,
    pub component: String,
    pub data: String,
    flags: i32,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinal {
    class_name: String,
    method_name: String,
    hashcode: i32,
    ret: String,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinalByte {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_input: String,
    ret: String,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinalByteInt {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_output: String,
    arg_output_offset: i32,
    ret: String,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinalByteIntInt {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_input: String,
    arg_input_offset: i32,
    arg_input_len: i32,
    ret: String,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinalByteIntIntByte {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_input: String,
    arg_input_offset: i32,
    arg_input_len: i32,
    arg_output: String,
    ret: i32,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinalByteIntIntByteInt {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_input: String,
    arg_input_offset: i32,
    arg_input_len: i32,
    arg_output: String,
    arg_output_offset: i32,
    ret: i32,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DoFinalByteBufferByteBuffer {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_input: String,
    arg_output: String,
    ret: i32,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InitIntCertificate {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_opcode: i32,
    arg_certificate: String,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InitIntKey {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_opcode: i32,
    arg_key: String,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateByte {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_input: String,
    ret: String,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateByteIntInt {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_input: String,
    arg_input_offset: i32,
    arg_input_len: i32,
    ret: String,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateByteIntIntByte {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_input: String,
    arg_input_offset: i32,
    arg_input_len: i32,
    arg_output: String,
    ret: i32,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateByteIntIntByteInt {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_input: String,
    arg_input_offset: i32,
    arg_input_len: i32,
    arg_output: String,
    arg_output_offset: i32,
    ret: i32,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateByteBufferByteBuffer {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_input: String,
    arg_output: String,
    ret: i32,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateAADByte {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_input: String,
    ret: String,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateAADByteIntInt {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_input: String,
    arg_input_offset: i32,
    arg_input_len: i32,
    ret: String,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateAADByteBuffer {
    class_name: String,
    method_name: String,
    hashcode: i32,
    arg_src: String,
    ret: i32,
    algorithm: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StartForeground {
    class_name: String,
    method_name: String,
    hashcode: i32,
    id: i32,
    r#type: i32,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OnStartCommand {
    class_name: String,
    method_name: String,
    hashcode: i32,
    pub action: String,
    pub component: String,
    pub data: String,
    flags: i32,
    start_id: i32,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OnBind {
    class_name: String,
    method_name: String,
    hashcode: i32,
    pub action: String,
    pub component: String,
    pub data: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SetImpl {
    class_name: String,
    method_name: String,
    hashcode: i32,
    r#type: i32,
    trigger_at_millis: i64,
    window_millis: i64,
    interval_millis: i64,
    flags: i32,
    operation_hashcode: i32,
    listener_class_name: String,
    listener_tag: String,
    alarm_clock_intent_hash_code: i32,
    alarm_clock_get_trigger_time: i64,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SetAlarmClock {
    class_name: String,
    method_name: String,
    hashcode: i32,
    creator: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PendingIntentGet {
    class_name: String,
    method_name: String,
    hashcode: i32,
    request_code: i32,
    pub action: String,
    pub component: String,
    pub data: String,
    flags: i32,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScheduleInternal {
    class_name: String,
    method_name: String,
    hashcode: i32,
    pub job_id: u32,
    work_id: String,
    pid: i32,
    tid: i32,
    pub uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RequestAndDoWork {
    class_name: String,
    method_name: String,
    hashcode: i32,
    work_id: String,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GenericHook {
    class_name: String,
    method_name: String,
    hashcode: i32,
    pid: i32,
    tid: i32,
    uid: u32,
    package_name: String,
    ts: i64,
    stack_trace: String,
}
