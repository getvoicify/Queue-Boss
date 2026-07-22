use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::error::BackendError;
use crate::model::JobId;

/// One page of a cursor-paginated result (repo pagination pattern).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

/// Keyset position encoded into an opaque cursor: `{createdAt, id}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cursor {
    pub created_at: u64,
    pub id: JobId,
}

/// Encode a keyset position as a base64url (no-pad) JSON cursor.
pub fn encode_cursor(cursor: &Cursor) -> String {
    let json = serde_json::to_vec(cursor).expect("Cursor is always serializable");
    URL_SAFE_NO_PAD.encode(json)
}

/// Decode a cursor produced by [`encode_cursor`]; malformed input is a typed error.
pub fn decode_cursor(encoded: &str) -> Result<Cursor, BackendError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| BackendError::Internal("malformed cursor encoding".to_owned()))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| BackendError::Internal("malformed cursor payload".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trips() {
        let cursor = Cursor {
            created_at: 1_700_000_000_123,
            id: JobId("job-42".to_owned()),
        };
        let encoded = encode_cursor(&cursor);
        let decoded = decode_cursor(&encoded).unwrap();
        assert_eq!(decoded, cursor);
    }

    #[test]
    fn encoded_cursor_is_non_empty_url_safe_base64() {
        let cursor = Cursor {
            created_at: 42,
            id: JobId("a/b+c".to_owned()),
        };
        let encoded = encode_cursor(&cursor);
        assert!(!encoded.is_empty());
        assert!(!encoded.contains('='), "no padding: {encoded}");
        assert!(!encoded.contains('+'), "url-safe alphabet: {encoded}");
        assert!(!encoded.contains('/'), "url-safe alphabet: {encoded}");
    }

    #[test]
    fn decode_rejects_malformed_cursor() {
        assert!(decode_cursor("!!!not-base64!!!").is_err());
    }

    #[test]
    fn decode_rejects_valid_base64_with_garbage_json() {
        // "AAAA" is valid base64url but decodes to three zero bytes, not Cursor JSON.
        assert!(matches!(
            decode_cursor("AAAA"),
            Err(crate::error::BackendError::Internal(_))
        ));
    }

    #[test]
    fn page_round_trips_and_serializes_camel_case() {
        let page = Page {
            items: vec![1_i32, 2, 3],
            next_cursor: Some("abc".to_owned()),
            has_more: true,
        };
        let json = serde_json::to_string(&page).unwrap();
        assert!(json.contains("\"nextCursor\""), "{json}");
        assert!(json.contains("\"hasMore\""), "{json}");
        let back: Page<i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, page);
    }

    #[test]
    fn empty_page_round_trips() {
        let page: Page<i32> = Page {
            items: vec![],
            next_cursor: None,
            has_more: false,
        };
        let json = serde_json::to_string(&page).unwrap();
        let back: Page<i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, page);
        assert!(!back.has_more);
    }
}
