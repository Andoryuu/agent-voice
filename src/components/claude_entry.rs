use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ClaudeEntry {
    pub r#type: String,
    pub message: ClaudeEntryMessage,
}

#[derive(Deserialize, Debug)]
pub struct ClaudeEntryMessage {
    pub r#type: String,
    pub role: String,
    pub content: Vec<ClaudeEntryContent>,
}

#[derive(Deserialize, Debug)]
pub struct ClaudeEntryContent {
    pub r#type: String,
    pub text: String,
}
