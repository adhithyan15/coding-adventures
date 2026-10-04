//! Native UUID creation. The renderer cannot supply project or document IDs.

use uuid::Uuid;

/// Create one lowercase, hyphenated RFC 9562 UUIDv7.
pub fn create_document_identity() -> String {
    Uuid::now_v7().hyphenated().to_string()
}
