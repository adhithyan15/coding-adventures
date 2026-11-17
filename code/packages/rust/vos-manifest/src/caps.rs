//! # Caps (VOS02 section 6.8)
//!
//! Every parser here bounds its input **before** it allocates for it.  The numbers
//! are the table of section 6.8; the document cap is the Warrant source ceiling
//! (`warrant-grammar` refuses more than 1 MiB), so a generated document that fits the
//! tools is exactly a document the Warrant crate can admit.

/// `contract.vos`: bytes.
pub const CONTRACT_MAX_BYTES: usize = 64 * 1024;
/// `contract.vos`: bytes in a line (the LF is not counted).
pub const CONTRACT_MAX_LINE: usize = 1_024;
/// A `.part` or `impl.part`: bytes.
pub const PART_MAX_BYTES: usize = 1024 * 1024;
/// A `.part` or `impl.part`: bytes in a line.
pub const PART_MAX_LINE: usize = 4_096;
/// A generated document: bytes (the Warrant admission ceiling).
pub const DOCUMENT_MAX_BYTES: usize = 1024 * 1024;
/// A generated document: bytes in a line.
pub const DOCUMENT_MAX_LINE: usize = 4_096;
/// `tier` rows.
pub const MAX_TIER_ROWS: usize = 4;
/// `config` rows.
pub const MAX_CONFIG_ROWS: usize = 64;
/// `ceiling` rows.
pub const MAX_CEILING_ROWS: usize = 1_024;
/// The longest `IDENT`, `PKG`, `KEY` or `OPNAME`.
pub const NAME_MAX: usize = 32;
