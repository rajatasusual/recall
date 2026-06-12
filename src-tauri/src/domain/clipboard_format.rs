use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardFormat {
    Original,
    PlainText,
    Uppercase,
    Lowercase,
    RemoveFormatting,
    ConvertQuotes,
    StripTrackingParams,
}

pub fn apply_text_format(content: &str, format: ClipboardFormat) -> String {
    match format {
        ClipboardFormat::Original | ClipboardFormat::PlainText => content.to_string(),
        ClipboardFormat::Uppercase => content.to_uppercase(),
        ClipboardFormat::Lowercase => content.to_lowercase(),
        ClipboardFormat::RemoveFormatting => remove_formatting(content),
        ClipboardFormat::ConvertQuotes => convert_quotes(content),
        ClipboardFormat::StripTrackingParams => strip_tracking_params(content),
    }
}

fn remove_formatting(content: &str) -> String {
    let tag_re = regex::Regex::new(r"<[^>]+>").expect("valid tag regex");
    let whitespace_re = regex::Regex::new(r"[ \t]{2,}").expect("valid whitespace regex");
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| tag_re.replace_all(line, ""))
        .map(|line| whitespace_re.replace_all(&line, " ").into_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

fn convert_quotes(content: &str) -> String {
    content
        .replace(&['\u{2018}', '\u{2019}', '\u{201A}', '\u{201B}'][..], "'")
        .replace(&['\u{201C}', '\u{201D}', '\u{201E}', '\u{201F}'][..], "\"")
}

fn strip_tracking_params(content: &str) -> String {
    content
        .split_whitespace()
        .map(strip_tracking_from_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_tracking_from_token(token: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(token) else {
        return token.to_string();
    };

    let filtered_pairs = parsed
        .query_pairs()
        .filter(|(key, _)| {
            let key = key.to_ascii_lowercase();
            !key.starts_with("utm_")
                && !matches!(
                    key.as_str(),
                    "fbclid"
                        | "gclid"
                        | "dclid"
                        | "msclkid"
                        | "mc_cid"
                        | "mc_eid"
                        | "igshid"
                        | "ref"
                        | "ref_src"
                )
        })
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<Vec<_>>();

    parsed.set_query(None);
    if !filtered_pairs.is_empty() {
        parsed
            .query_pairs_mut()
            .extend_pairs(filtered_pairs.iter().map(|(key, value)| (&**key, &**value)));
    }

    parsed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms_case() {
        assert_eq!(
            apply_text_format("Recall", ClipboardFormat::Uppercase),
            "RECALL"
        );
        assert_eq!(
            apply_text_format("Recall", ClipboardFormat::Lowercase),
            "recall"
        );
    }

    #[test]
    fn converts_smart_quotes() {
        assert_eq!(
            apply_text_format("\u{201C}Hello\u{201D}", ClipboardFormat::ConvertQuotes),
            "\"Hello\""
        );
    }

    #[test]
    fn strips_tracking_params() {
        assert_eq!(
            apply_text_format(
                "https://example.com?a=1&utm_source=x&fbclid=y",
                ClipboardFormat::StripTrackingParams
            ),
            "https://example.com/?a=1"
        );
    }
}
