pub enum UIMessage {
    Serial(u8),
    #[allow(dead_code)]
    Debug(String),
    SetEIP(u64),
    CPUStarted(usize),
    CPUStopped(usize),
}
