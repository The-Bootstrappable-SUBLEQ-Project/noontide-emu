pub enum UIMessage {
    Serial(char),
    #[allow(dead_code)]
    Debug(String),
    SetEIP(u64),
}
