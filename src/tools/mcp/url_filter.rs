use crate::tools::tool::ToolError;

const FILE_URL_PREFIX: &str = "file://";

const BLOCK_MSG: &str =
    "file:// URLs are blocked for security. Use the local_search tool for filesystem access.";

/// Strip ASCII control characters (`\t`, `\n`, `\r`) and leading/trailing
/// whitespace from a URL string.  WHATWG-compliant parsers (used by browsers,
/// `reqwest`, Node.js `fetch`) silently remove these characters anywhere inside
/// a URL, so `"f\nile:///etc/passwd"` reaches the HTTP client as
/// `"file:///etc/passwd"` even though the raw string doesn't start with
/// `file://`.  Normalising before the prefix check closes this bypass.
#[inline]
fn strip_url_noise(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    cleaned.trim().to_string()
}

#[inline]
fn is_file_url(s: &str) -> bool {
    let s = strip_url_noise(s);
    if s.len() >= FILE_URL_PREFIX.len()
        && s.as_bytes()[..FILE_URL_PREFIX.len()].eq_ignore_ascii_case(FILE_URL_PREFIX.as_bytes())
    {
        return true;
    }
    let decoded = percent_decode(&s);
    decoded.len() >= FILE_URL_PREFIX.len()
        && decoded.as_bytes()[..FILE_URL_PREFIX.len()]
            .eq_ignore_ascii_case(FILE_URL_PREFIX.as_bytes())
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

/// Recursively walk a JSON value and return the first `file://` string found.
fn find_file_url(value: &serde_json::Value) -> Option<&str> {
    match value {
        serde_json::Value::String(s) if is_file_url(s) => Some(s.as_str()),
        serde_json::Value::Array(arr) => arr.iter().find_map(find_file_url),
        serde_json::Value::Object(map) => map.values().find_map(find_file_url),
        _ => None,
    }
}

/// Inspect `params` for any `file://` URL in any string field (at any nesting
/// depth). Returns `Err(ToolError::InvalidParameters)` with a user-facing message
/// if one is found, otherwise returns `Ok(())`.
pub fn reject_if_file_url(params: &serde_json::Value) -> Result<(), ToolError> {
    if find_file_url(params).is_some() {
        return Err(ToolError::InvalidParameters(BLOCK_MSG.to_string()));
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
