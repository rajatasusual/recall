// api/src/classification.rs

use regex::Regex;
use std::path::Path;

use crate::core::class::ContentType;

pub fn classify_text(content: &str) -> ContentType {
    let content = content.trim();

    if content.is_empty() {
        return ContentType::Text;
    }

    if is_url(content) {
        return ContentType::Url;
    }

    if is_email(content) {
        return ContentType::Email;
    }

    if is_phone_number(content) {
        return ContentType::PhoneNumber;
    }

    if is_file_path(content) {
        return ContentType::FilePath;
    }

    if is_command(content) {
        return ContentType::Command;
    }

    if is_code(content) {
        return ContentType::Code;
    }

    ContentType::Text
}

fn is_url(text: &str) -> bool {
    let re = Regex::new(r"^(https?://)?([a-zA-Z0-9-]+\.)+[a-zA-Z]{2,}(/[^\s]*)?$").unwrap();

    re.is_match(text)
}

fn is_email(text: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();

    re.is_match(text)
}

fn is_phone_number(text: &str) -> bool {
    let re = Regex::new(r"^\+?[0-9\s\-\(\)]{7,20}$").unwrap();

    re.is_match(text)
}

fn is_file_path(text: &str) -> bool {
    // Windows
    let windows = Regex::new(r"^[a-zA-Z]:\\").unwrap();

    // Unix / macOS
    let unix = Regex::new(r"^/[^/]").unwrap();

    windows.is_match(text) || unix.is_match(text) || Path::new(text).exists()
}

fn is_command(text: &str) -> bool {
    let common_commands = [
        "git ", "npm ", "pnpm ", "yarn ", "cargo ", "docker ", "kubectl ", "brew ", "ssh ",
        "curl ", "wget ", "cd ", "ls", "mkdir ", "rm ", "cp ", "mv ",
    ];

    common_commands.iter().any(|cmd| text.starts_with(cmd))
}

fn is_code(text: &str) -> bool {
    let code_patterns = [
        "fn ",
        "function ",
        "class ",
        "struct ",
        "impl ",
        "interface ",
        "const ",
        "let ",
        "var ",
        "public ",
        "private ",
        "import ",
        "export ",
        "#include",
        "use ",
        "=>",
        "{}",
        "();",
    ];

    let hits = code_patterns.iter().filter(|p| text.contains(**p)).count();

    hits >= 2 || text.lines().count() > 3
}
