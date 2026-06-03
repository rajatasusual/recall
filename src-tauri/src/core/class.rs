
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentType {
    Code,
    Url,
    Email,
    PhoneNumber,
    FilePath,
    Command,
    Image,
    Text,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Code => "code",
            ContentType::Url => "url",
            ContentType::Email => "email",
            ContentType::PhoneNumber => "phone_number",
            ContentType::FilePath => "file_path",
            ContentType::Command => "command",
            ContentType::Image => "image",
            ContentType::Text => "text",
        }
    }
}