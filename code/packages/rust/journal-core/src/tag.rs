//! Tags — names, not entities.
//!
//! Day One has no tag objects: a tag is the text you typed, and two entries share
//! a tag when they share the text. So `Tag` carries no id. It carries:
//!
//! - a **display form** — what you typed, tidied (trimmed, inner whitespace runs
//!   collapsed), with its capitalisation kept; and
//! - a **key** — the case-folded display form, which is what equality, hashing,
//!   and ordering use.
//!
//! ```text
//!   typed            display        key
//!   "  Road  Trip "  "Road Trip"    "road trip"
//!   "road trip"      "road trip"    "road trip"   ← the same tag
//! ```
//!
//! When an entry is given both spellings, the **first one wins** and the second is
//! dropped as a duplicate — see [`normalize_tags`].

use crate::text;

/// Longest allowed tag, in characters (not bytes).
pub const MAX_TAG_CHARS: usize = 64;
/// Most tags one entry may carry.
pub const MAX_TAGS_PER_ENTRY: usize = 64;

/// A tag: a display string compared case-insensitively.
#[derive(Debug, Clone)]
pub struct Tag {
    display: String,
    key: String,
}

/// Why a string is not a valid tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagError {
    /// Nothing left after trimming.
    Empty,
    /// Longer than [`MAX_TAG_CHARS`].
    TooLong,
    /// Contains a newline, tab, or other control character.
    ControlCharacter,
    /// More than [`MAX_TAGS_PER_ENTRY`] distinct tags.
    TooMany,
    /// The same tag (case-insensitively) appears twice. [`normalize_tags`] drops
    /// duplicates silently; this is only reported for state loaded from outside.
    Duplicate,
}

impl Tag {
    /// Validate and tidy a user-typed tag.
    pub fn new(raw: &str) -> Result<Tag, TagError> {
        if text::has_control(raw.trim()) {
            return Err(TagError::ControlCharacter);
        }
        let display = text::tidy(raw);
        if display.is_empty() {
            return Err(TagError::Empty);
        }
        if display.chars().count() > MAX_TAG_CHARS {
            return Err(TagError::TooLong);
        }
        let key = text::fold(&display);
        Ok(Tag { display, key })
    }

    /// The tag as the user spelled it (tidied).
    pub fn display(&self) -> &str {
        &self.display
    }

    /// The case-folded comparison key.
    pub fn key(&self) -> &str {
        &self.key
    }
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for Tag {}

impl core::hash::Hash for Tag {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl PartialOrd for Tag {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Tag {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

impl core::fmt::Display for Tag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.display)
    }
}

/// Validate a list of raw tags: tidy each, drop case-insensitive duplicates (first
/// spelling wins, order kept), and enforce [`MAX_TAGS_PER_ENTRY`] on the result.
///
/// Returns the index of the first offending input alongside the error, so a host
/// can point at the bad chip rather than rejecting the whole field vaguely.
pub fn normalize_tags<S: AsRef<str>>(raw: &[S]) -> Result<Vec<Tag>, (usize, TagError)> {
    let mut out: Vec<Tag> = Vec::new();
    for (i, r) in raw.iter().enumerate() {
        let tag = Tag::new(r.as_ref()).map_err(|e| (i, e))?;
        if !out.contains(&tag) {
            if out.len() == MAX_TAGS_PER_ENTRY {
                return Err((i, TagError::TooMany));
            }
            out.push(tag);
        }
    }
    Ok(out)
}

// A tag's wire form is just its display string; the key is re-derived on load, so
// stored JSON can never carry a key that disagrees with its display.
#[cfg(feature = "serde")]
impl serde::Serialize for Tag {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.display)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Tag {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        Tag::new(&s).map_err(|e| serde::de::Error::custom(format!("invalid tag {s:?}: {e:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_tidy_and_compare_case_insensitively() {
        let a = Tag::new("  Road   Trip ").unwrap();
        let b = Tag::new("road trip").unwrap();
        assert_eq!(a.display(), "Road Trip");
        assert_eq!(a.key(), "road trip");
        assert_eq!(a, b);
        assert_eq!(a.to_string(), "Road Trip");
        assert_eq!(a.cmp(&b), core::cmp::Ordering::Equal);
    }

    #[test]
    fn invalid_tags_are_rejected() {
        assert_eq!(Tag::new("   "), Err(TagError::Empty));
        assert_eq!(Tag::new("a\nb"), Err(TagError::ControlCharacter));
        assert_eq!(Tag::new(&"x".repeat(65)), Err(TagError::TooLong));
        // 64 multi-byte characters is fine: the limit counts characters.
        assert!(Tag::new(&"é".repeat(64)).is_ok());
    }

    #[test]
    fn normalize_dedupes_keeping_first_spelling_and_order() {
        let tags = normalize_tags(&["Travel", "food", "TRAVEL", " Food "]).unwrap();
        let shown: Vec<&str> = tags.iter().map(Tag::display).collect();
        assert_eq!(shown, ["Travel", "food"]);
    }

    #[test]
    fn normalize_reports_the_offending_index() {
        assert_eq!(normalize_tags(&["ok", " "]), Err((1, TagError::Empty)));
    }

    #[test]
    fn normalize_caps_the_count_after_dedup() {
        let many: Vec<String> = (0..65).map(|i| format!("t{i}")).collect();
        assert_eq!(normalize_tags(&many), Err((64, TagError::TooMany)));
        // 65 spellings of 64 distinct tags is fine: dedup happens first.
        let mut dup: Vec<String> = (0..64).map(|i| format!("t{i}")).collect();
        dup.push("T0".into());
        assert_eq!(normalize_tags(&dup).unwrap().len(), 64);
    }

    #[test]
    fn tag_hash_follows_key() {
        use std::collections::HashSet;
        let set: HashSet<Tag> = [Tag::new("A").unwrap(), Tag::new("a").unwrap()]
            .into_iter()
            .collect();
        assert_eq!(set.len(), 1);
    }
}
