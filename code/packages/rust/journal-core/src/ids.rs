//! Typed entity identifiers.
//!
//! Journals and entries are addressed by stable, **string-backed** ids. Strings
//! rather than 128-bit integers for the same reason `task-core` gives: this core is
//! consumed from JavaScript through WebAssembly, and a JSON number above 2^53 loses
//! precision in JS. A string (canonical UUID-v7 text) round-trips losslessly on
//! every platform.
//!
//! **Ids are not generated here.** Minting a UUID v7 needs a clock and a random
//! source, and this crate is deliberately clock-free and deterministic. The host
//! mints ids and passes them in on the create-commands; inside the core an id is an
//! opaque, ordered key.
//!
//! Each entity gets its *own* newtype so the compiler rejects an `EntryId` where a
//! `JournalId` is expected — a mix-up that would otherwise surface only as a
//! "journal not found" at runtime.

/// Define a string-backed id newtype with the standard trait set and optional serde.
///
/// `serde(transparent)` makes the id serialise as a bare string (`"018f…"`), not a
/// wrapper object, which keeps the JSON contract clean for host adapters.
macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(pub String);

        impl $name {
            /// Wrap an externally-minted id string (the host mints UUID v7s).
            pub fn from_raw(s: impl Into<String>) -> Self {
                Self(s.into())
            }
            /// Borrow the id as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
    };
}

/// Longest id, in bytes. A canonical UUID v7 is 36; the slack allows other
/// schemes without letting an id grow into a storage or log problem.
pub const MAX_ID_BYTES: usize = 64;

/// True if `s` is a usable id: 1..=[`MAX_ID_BYTES`] bytes of printable ASCII.
///
/// Ids are echoed into error messages and host logs, used as map keys, and
/// repeated in every record that points at them. Printable ASCII keeps them from
/// carrying newlines or terminal escape sequences into a log, and the length cap
/// keeps a hostile import from making every lookup compare megabytes.
pub fn is_valid_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_ID_BYTES && s.bytes().all(|b| b.is_ascii_graphic())
}

id_type!(
    /// Identifies one journal ("Personal", "Work", "Travel").
    JournalId
);
id_type!(
    /// Identifies one entry.
    EntryId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_wrap_and_display_their_string() {
        let id = EntryId::from_raw("018f-abc");
        assert_eq!(id.as_str(), "018f-abc");
        assert_eq!(id.to_string(), "018f-abc");
        assert_eq!(EntryId::from("018f-abc"), id);
    }

    #[test]
    fn id_validity() {
        assert!(is_valid_id("018f7c2e-8b1a-7c3d-9e4f-0a1b2c3d4e5f"));
        assert!(!is_valid_id(""));
        assert!(!is_valid_id(&"a".repeat(MAX_ID_BYTES + 1)));
        assert!(!is_valid_id("has space"));
        assert!(!is_valid_id("line\nbreak"));
        assert!(!is_valid_id("\u{1b}[31m"));
        assert!(!is_valid_id("é"));
    }

    #[test]
    fn ids_order_lexically() {
        // UUID v7 text sorts by creation time, so lexical order is the useful one.
        assert!(JournalId::from("a") < JournalId::from("b"));
    }
}
