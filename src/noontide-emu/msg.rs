pub enum UIMessage {
    Serial(u8),
    #[allow(dead_code)]
    Debug(u64, String),
    SetEIP(u64),
    CPUStarted(usize),
    CPUStopped(usize),
}
