pub enum UIMessage {
    Serial(char),
    #[allow(dead_code)]
    Debug(String),
    SetEIP(i64),
}
