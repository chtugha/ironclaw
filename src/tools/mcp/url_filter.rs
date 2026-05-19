use crate::tools::tool::ToolError;

const BLOCK_MSG: &str =
    "file:// URLs are blocked for security. Use the local_search tool for filesystem access.";

const DATA_BLOCK_MSG: &str =
    "data: URLs are blocked for security.";

const BLOCKED_SCHEME_MSG: &str =
    "This URL scheme is blocked for security.";

const BLOCKED_SCHEMES: &[&str] = &[
    "file:", "data:", "javascript:", "vbscript:",
    "ftp:", "sftp:", "gopher:", "dict:", "ldap:",
    "blob:",
];

fn strip_url_noise(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| {
            let cp = *c as u32;
            !(cp <= 0x1F                        // C0 controls
                || cp == 0x7F                   // DEL
                || cp == 0xFEFF                 // BOM / ZWNBSP
                || cp == 0x00A0                 // NBSP
                || cp == 0x00AD                 // soft hyphen
                || (0x200B..=0x200F).contains(&cp) // ZWS, ZWNJ, ZWJ, LRM, RLM
                || cp == 0x2028                 // line separator
                || cp == 0x2029                 // paragraph separator
                || cp == 0x2060                 // word joiner
                || (0x2061..=0x2064).contains(&cp) // invisible math operators
                || cp == 0xFFF9 || cp == 0xFFFA || cp == 0xFFFB) // interlinear annotations
        })
        .collect();
    cleaned.trim().to_string()
}

fn normalize_slashes(s: &str) -> String {
    s.replace('\\', "/")
}

fn fully_percent_decode(s: &str) -> String {
    let mut cur = s.to_string();
    for _ in 0..5 {
        let next = percent_decode(&cur);
        if next == cur {
            break;
        }
        cur = next;
    }
    cur
}

fn is_blocked_scheme(s: &str) -> Option<&'static str> {
    let cleaned = strip_url_noise(s);
    let normalized = normalize_slashes(&cleaned);
    let decoded = fully_percent_decode(&normalized);
    let decoded_cleaned = strip_url_noise(&decoded);
    for check in [&normalized, &decoded_cleaned] {
        let lower = check.to_ascii_lowercase();
        for &scheme in BLOCKED_SCHEMES {
            if lower.starts_with(scheme) {
                return match scheme {
                    "file:" => Some(BLOCK_MSG),
                    "data:" => Some(DATA_BLOCK_MSG),
                    _ => Some(BLOCKED_SCHEME_MSG),
                };
            }
        }
    }
    None
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push(hi << 4 | lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[inline]
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn find_blocked_url(value: &serde_json::Value) -> Option<&'static str> {
    match value {
        serde_json::Value::String(s) => is_blocked_scheme(s),
        serde_json::Value::Array(arr) => arr.iter().find_map(find_blocked_url),
        serde_json::Value::Object(map) => map.values().find_map(find_blocked_url),
        _ => None,
    }
}

pub fn reject_if_file_url(params: &serde_json::Value) -> Result<(), ToolError> {
    if let Some(msg) = find_blocked_url(params) {
        return Err(ToolError::InvalidParameters(msg.to_string()));
    }
    Ok(())
}

/// Check whether `binary` is reachable on the current `PATH` without spawning
/// a child process.
///
/// Uses the same logic as the Unix `which` command: iterates `PATH` entries
/// and checks for an executable file at `<dir>/<binary>`.
pub fn binary_available(binary: &str) -> bool {
    if binary.is_empty()
        || binary.contains('/')
        || binary.contains('\\')
        || binary.contains('\0')
    {
        return false;
    }
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(binary);
        if candidate.is_file() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&candidate)
                    && meta.permissions().mode() & 0o111 != 0
                {
                    return true;
                }
            }
            #[cfg(not(unix))]
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn file_url_at_top_level_is_rejected() {
        let params = json!({"url": "file:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn file_url_nested_is_rejected() {
        let params = json!({"options": {"source": "file:///home/user/secret.txt"}});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn file_url_in_array_is_rejected() {
        let params = json!({"urls": ["https://example.com", "file:///tmp/x"]});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn file_url_single_slash_is_rejected() {
        let params = json!({"url": "file:/etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "file:etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn file_url_backslash_is_rejected() {
        let params = json!({"url": "file:\\\\\\etc\\passwd"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "file:%5C%5C%5Cetc%5Cpasswd"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn data_url_is_rejected() {
        let params = json!({"url": "data:text/html,<script>alert(1)</script>"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "DATA:text/plain,hello"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn javascript_url_is_rejected() {
        let params = json!({"url": "javascript:alert(1)"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn loopback_url_passes_through() {
        let params = json!({"url": "http://localhost:9222/json"});
        assert!(reject_if_file_url(&params).is_ok());
    }

    #[test]
    fn loopback_127_url_passes_through() {
        let params = json!({"url": "http://127.0.0.1:3000/api"});
        assert!(reject_if_file_url(&params).is_ok());
    }

    #[test]
    fn https_url_passes_through() {
        let params = json!({"url": "https://example.com/page"});
        assert!(reject_if_file_url(&params).is_ok());
    }

    #[test]
    fn non_url_params_pass_through() {
        let params = json!({"query": "search term", "count": 5, "enabled": true});
        assert!(reject_if_file_url(&params).is_ok());
    }

    #[test]
    fn file_url_mixed_case_is_rejected() {
        let params = json!({"url": "FILE:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "File:///etc/shadow"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "fIlE:///tmp/secret"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn percent_encoded_file_url_is_rejected() {
        let params = json!({"url": "file%3A///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "file%3a%2F%2F/etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "FILE%3A%2F%2Fetc/passwd"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn control_chars_in_file_url_are_rejected() {
        let params = json!({"url": "f\nile:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "fi\tle:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "\r\nfile:///etc/shadow"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "  file:///etc/hosts"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn c0_control_prefix_stripped() {
        let params = json!({"url": "\x01file:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn bom_prefix_does_not_bypass() {
        let params = json!({"url": "\u{FEFF}file:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn zero_width_space_prefix_does_not_bypass() {
        let params = json!({"url": "\u{200B}file:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "\u{200C}file:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "\u{200D}file:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "\u{2060}file:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());

        let params = json!({"url": "\u{00AD}file:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn zero_width_space_percent_encoded_does_not_bypass() {
        let params = json!({"url": "%E2%80%8Bfile:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn double_percent_encoding_is_rejected() {
        let params = json!({"url": "%2566ile:///etc/passwd"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn ftp_url_is_rejected() {
        let params = json!({"url": "ftp://attacker.com/file"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn gopher_url_is_rejected() {
        let params = json!({"url": "gopher://localhost:25/"});
        assert!(reject_if_file_url(&params).is_err());
    }

    #[test]
    fn binary_available_rejects_path_traversal() {
        assert!(!binary_available("../../../etc/passwd"));
        assert!(!binary_available("foo/bar"));
        assert!(!binary_available("foo\\bar"));
        assert!(!binary_available(""));
    }

    #[test]
    fn error_message_contains_guidance() {
        let params = json!({"path": "file:///etc/hosts"});
        match reject_if_file_url(&params) {
            Err(ToolError::InvalidParameters(msg)) => {
                assert!(msg.contains("file://"));
                assert!(msg.contains("local_search"));
            }
            _ => panic!("expected InvalidParameters error"),
        }
    }
}
