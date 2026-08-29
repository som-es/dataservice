use base64::{engine::general_purpose::STANDARD, Engine};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
pub struct IncomingReply {
    pub message_id: Option<String>,
    pub sender_email: Option<String>,
    pub related_message_ids: Vec<String>,
    pub body: String,
}

#[derive(Debug, PartialEq, Eq)]
enum ContentType {
    PlainText,
    Html,
    Multipart { boundary: String },
    Other,
}

impl ContentType {
    fn parse(value: Option<&str>) -> Self {
        let Some(value) = value else {
            return Self::Other;
        };
        let media_type = value.split(';').next().map(str::trim).unwrap_or_default();

        if media_type.eq_ignore_ascii_case("text/plain") {
            Self::PlainText
        } else if media_type.eq_ignore_ascii_case("text/html") {
            Self::Html
        } else if media_type.to_ascii_lowercase().starts_with("multipart/") {
            boundary_from_content_type(value)
                .map(|boundary| Self::Multipart { boundary })
                .unwrap_or(Self::Other)
        } else {
            Self::Other
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TransferEncoding {
    Base64,
    QuotedPrintable,
    Identity,
}

impl TransferEncoding {
    fn parse(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some(value) if value.eq_ignore_ascii_case("base64") => Self::Base64,
            Some(value) if value.eq_ignore_ascii_case("quoted-printable") => Self::QuotedPrintable,
            _ => Self::Identity,
        }
    }
}

pub fn new_question_message_id() -> String {
    format!("<question-{}@somes.at>", Uuid::new_v4())
}

pub fn parse_incoming_reply(raw_message: &[u8]) -> IncomingReply {
    let raw_message = String::from_utf8_lossy(raw_message);
    let (headers, body) = split_headers_and_body(&raw_message);
    let headers = parse_headers(headers);
    let content = extract_body(&headers, body);

    IncomingReply {
        message_id: header(&headers, "message-id").and_then(normalize_message_id),
        sender_email: header(&headers, "from").and_then(extract_email_address),
        related_message_ids: related_message_ids(&headers),
        body: strip_quoted_reply(&content),
    }
}

fn split_headers_and_body(message: &str) -> (&str, &str) {
    message
        .split_once("\r\n\r\n")
        .or_else(|| message.split_once("\n\n"))
        .unwrap_or((message, ""))
}

fn parse_headers(raw_headers: &str) -> HashMap<String, String> {
    let mut headers: HashMap<String, String> = HashMap::new();
    let mut current_name: Option<String> = None;

    for line in raw_headers.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(name) = &current_name {
                if let Some(value) = headers.get_mut(name) {
                    value.push(' ');
                    value.push_str(line.trim());
                }
            }
            continue;
        }

        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        headers.insert(name.clone(), value.trim().to_string());
        current_name = Some(name);
    }

    headers
}

fn header<'a>(headers: &'a HashMap<String, String>, name: &str) -> Option<&'a str> {
    headers.get(name).map(String::as_str)
}

fn related_message_ids(headers: &HashMap<String, String>) -> Vec<String> {
    let mut message_ids = Vec::new();
    for header_name in ["in-reply-to", "references"] {
        let Some(value) = header(headers, header_name) else {
            continue;
        };
        for message_id in extract_message_ids(value) {
            if !message_ids.contains(&message_id) {
                message_ids.push(message_id);
            }
        }
    }
    message_ids
}

fn extract_message_ids(value: &str) -> Vec<String> {
    let mut message_ids = Vec::new();
    let mut remaining = value;

    while let Some(start) = remaining.find('<') {
        let after_start = &remaining[start..];
        let Some(end) = after_start.find('>') else {
            break;
        };
        if let Some(message_id) = normalize_message_id(&after_start[..=end]) {
            if !message_ids.contains(&message_id) {
                message_ids.push(message_id);
            }
        }
        remaining = &after_start[end + 1..];
    }

    message_ids
}

fn normalize_message_id(value: &str) -> Option<String> {
    let value = value.trim();
    let start = value.find('<')?;
    let end = value[start..].find('>')? + start;
    Some(value[start..=end].to_ascii_lowercase())
}

fn extract_email_address(value: &str) -> Option<String> {
    let value = value.trim();
    let email = match (value.rfind('<'), value.rfind('>')) {
        (Some(start), Some(end)) if start < end => &value[start + 1..end],
        _ => value,
    };
    let email = email.trim();
    email.contains('@').then(|| email.to_ascii_lowercase())
}

fn extract_body(headers: &HashMap<String, String>, body: &str) -> String {
    let content_type = ContentType::parse(header(headers, "content-type"));
    let encoding = TransferEncoding::parse(header(headers, "content-transfer-encoding"));

    match content_type {
        ContentType::Multipart { boundary } => {
            extract_multipart_body(body, &boundary).unwrap_or_else(|| body.to_string())
        }
        ContentType::Html => somes_scraper::extract_text_from_html(&decode_body(body, encoding)),
        ContentType::PlainText | ContentType::Other => decode_body(body, encoding),
    }
}

fn boundary_from_content_type(content_type: &str) -> Option<String> {
    let boundary_start = content_type.to_ascii_lowercase().find("boundary=")? + "boundary=".len();
    let boundary = content_type[boundary_start..].trim_start();
    if let Some(stripped) = boundary.strip_prefix('"') {
        return stripped.split_once('"').map(|(value, _)| value.to_string());
    }
    boundary
        .split(';')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn extract_multipart_body(body: &str, boundary: &str) -> Option<String> {
    let marker = format!("--{boundary}");
    let mut html_body = None;

    for part in body.split(&marker).skip(1) {
        if part.trim_start().starts_with("--") {
            break;
        }
        let (part_headers, part_body) =
            split_headers_and_body(part.trim_start_matches(['\r', '\n']));
        let part_headers = parse_headers(part_headers);
        let content_type = ContentType::parse(header(&part_headers, "content-type"));
        let decoded = decode_body(
            part_body,
            TransferEncoding::parse(header(&part_headers, "content-transfer-encoding")),
        );

        match content_type {
            ContentType::PlainText => return Some(decoded),
            ContentType::Html => {
                html_body = Some(somes_scraper::extract_text_from_html(&decoded));
            }
            ContentType::Multipart { .. } | ContentType::Other => {}
        }
    }

    html_body
}

fn decode_body(body: &str, transfer_encoding: TransferEncoding) -> String {
    match transfer_encoding {
        TransferEncoding::Base64 => STANDARD
            .decode(body.replace(['\r', '\n'], ""))
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_else(|_| body.to_string()),
        TransferEncoding::QuotedPrintable => decode_quoted_printable(body),
        TransferEncoding::Identity => body.to_string(),
    }
}

fn decode_quoted_printable(value: &str) -> String {
    let mut bytes = Vec::new();
    let mut input = value.as_bytes();

    while let Some((&byte, rest)) = input.split_first() {
        input = rest;
        if byte == b'=' {
            if input.starts_with(b"\r\n") {
                input = &input[2..];
                continue;
            }
            if input.starts_with(b"\n") {
                input = &input[1..];
                continue;
            }
            if input.len() >= 2 {
                if let (Some(high), Some(low)) = (hex_value(input[0]), hex_value(input[1])) {
                    bytes.push(high << 4 | low);
                    input = &input[2..];
                    continue;
                }
            }
        }
        bytes.push(byte);
    }

    String::from_utf8_lossy(&bytes).into_owned()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn strip_quoted_reply(value: &str) -> String {
    let mut reply = Vec::new();
    for line in value.lines() {
        let is_quoted_reply_start = line.starts_with("On ") && line.contains(" wrote:");
        if is_quoted_reply_start || line.starts_with('>') {
            break;
        }
        reply.push(line);
    }
    reply.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{new_question_message_id, parse_incoming_reply};

    #[test]
    fn creates_rfc_message_ids() {
        let message_id = new_question_message_id();
        assert!(message_id.starts_with("<question-"));
        assert!(message_id.ends_with("@somes.at>"));
    }

    #[test]
    fn parses_thunderbird_reply_headers_and_body() {
        let raw_message = b"From: Parliament Club <parlamentsklub@neos.eu>\r\nMessage-ID: <reply-123@neos.eu>\r\nIn-Reply-To: <question-abc@somes.at>\r\nReferences: <question-abc@somes.at>\r\nContent-Type: multipart/alternative; boundary=part\r\n\r\n--part\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\nDanke fuer die Antwort.\r\n\r\nOn 6/26/26, info@somes.at wrote:\r\n> Alte Frage\r\n--part--\r\n";

        let reply = parse_incoming_reply(raw_message);

        assert_eq!(reply.message_id.as_deref(), Some("<reply-123@neos.eu>"));
        assert_eq!(
            reply.sender_email.as_deref(),
            Some("parlamentsklub@neos.eu")
        );
        assert_eq!(reply.related_message_ids, vec!["<question-abc@somes.at>"]);
        assert_eq!(reply.body, "Danke fuer die Antwort.");
    }

    #[test]
    fn parses_html_reply_with_html_entities() {
        let raw_message = b"From: Club <club@example.com>\r\nMessage-ID: <reply-html@example.com>\r\nIn-Reply-To: <question-html@somes.at>\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<p>Hallo &amp; danke</p>";

        let reply = parse_incoming_reply(raw_message);

        assert_eq!(reply.body, "Hallo & danke");
    }

    #[test]
    fn parses_base64_reply() {
        let raw_message = b"From: Club <club@example.com>\r\nMessage-ID: <reply-base64@example.com>\r\nIn-Reply-To: <question-base64@somes.at>\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\nSGFsbG8=";

        let reply = parse_incoming_reply(raw_message);

        assert_eq!(reply.body, "Hallo");
    }
}
