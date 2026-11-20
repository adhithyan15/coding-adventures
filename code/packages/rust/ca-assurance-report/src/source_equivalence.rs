//! Refusal-only Rust-subset/WIR source-equivalence anchor validation.
//!
//! Selection and frontend execution are delegated to warrant-ir's shared
//! in-memory helper. The caller passes the already-hashed source frame, so no
//! second filesystem read can change the bytes being checked.

use coding_adventures_warrant_ir::source_equivalence::reproduce_refusal_from_snapshot;

use crate::evidence::EvidenceError;

pub(crate) fn reproduce_refusal(
    source_frame: &[u8],
    cargo_toml: &[u8],
    export: &str,
) -> Result<String, EvidenceError> {
    reproduce_refusal_from_snapshot(source_frame, cargo_toml, export).map_err(EvidenceError)
}
