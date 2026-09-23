//! The stored state: journals, entries, and the root that holds them.
//!
//! Only **inputs** live here — what the user wrote and chose. Everything a screen
//! shows that can be derived (the timeline, on-this-day, search hits, tag counts)
//! is computed by [`crate::projections`] and never stored, so there is no second
//! copy of the truth to drift out of date.
//!
//! ```text
//!   JournalState
//!   ├── journals: JournalId → Journal      (always at least one)
//!   └── entries:  EntryId   → Entry        (each names its journal)
//! ```
//!
//! Both maps are `BTreeMap`s. Iteration order is then the id order, so every
//! projection is deterministic without needing a sort to rescue it — the same
//! state renders the same way on all nine backends.

use std::collections::BTreeMap;

use crate::{Date, EntryId, JournalId, Tag};

/// Longest journal name, in characters.
pub const MAX_JOURNAL_NAME_CHARS: usize = 128;
/// Longest entry title, in characters.
pub const MAX_TITLE_CHARS: usize = 512;
/// Most journals one state may hold. Day One users keep a handful; a thousand is
/// far past any real use, and the cap bounds what an imported file can make
/// `validate` and the journal pickers do.
pub const MAX_JOURNALS: usize = 1000;
/// Largest entry body, in **bytes** (1 MiB). Bytes, not characters, because this
/// limit exists to bound storage and the work a projection does per entry.
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

/// One journal — a named collection of entries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Journal {
    /// Stable id, minted by the host.
    pub id: JournalId,
    /// Display name, tidied; unique among journals case-insensitively.
    pub name: String,
    /// When the journal was created (ms since the Unix epoch).
    pub created_at_ms: u64,
}

/// One journal entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Entry {
    /// Stable id, minted by the host.
    pub id: EntryId,
    /// The journal this entry belongs to. Always names an existing journal.
    pub journal: JournalId,
    /// Heading; may be empty (many journal entries have none).
    pub title: String,
    /// Raw GFM markdown.
    pub body: String,
    /// The calendar day the entry is *about* — see [`crate::date`].
    pub date: Date,
    /// When the entry was first written (ms since the Unix epoch).
    pub created_at_ms: u64,
    /// When the entry was last changed (ms since the Unix epoch).
    pub updated_at_ms: u64,
    /// Tags, deduplicated case-insensitively, in the order they were added.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tags: Vec<Tag>,
    /// Day One's "favourite" flag.
    #[cfg_attr(feature = "serde", serde(default))]
    pub starred: bool,
}

impl Entry {
    /// True if the entry carries `tag` (case-insensitively).
    pub fn has_tag(&self, tag: &Tag) -> bool {
        self.tags.contains(tag)
    }
}

/// The whole stored world of one Journal installation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct JournalState {
    /// Every journal, by id. Never empty.
    pub journals: BTreeMap<JournalId, Journal>,
    /// Every entry, by id.
    pub entries: BTreeMap<EntryId, Entry>,
}

impl JournalState {
    /// A fresh state with one journal — the smallest *valid* state, because a
    /// journal app with no journal has nowhere to put the first entry.
    ///
    /// The id and name are checked exactly as `CreateJournal` checks them, so the
    /// very first journal cannot be one that a later command would have refused.
    pub fn new(
        first: JournalId,
        name: impl Into<String>,
        now_ms: u64,
    ) -> Result<JournalState, crate::OpError> {
        crate::ops::check_id(first.as_str())?;
        let raw = name.into();
        if crate::text::has_control(raw.trim()) {
            return Err(crate::OpError::InvalidJournalName);
        }
        let name = crate::text::tidy(&raw);
        crate::ops::check_journal_name(&name)?;
        let journal = Journal {
            id: first.clone(),
            name,
            created_at_ms: now_ms,
        };
        let mut journals = BTreeMap::new();
        journals.insert(first, journal);
        Ok(JournalState {
            journals,
            entries: BTreeMap::new(),
        })
    }

    /// Look up an entry.
    pub fn entry(&self, id: &EntryId) -> Option<&Entry> {
        self.entries.get(id)
    }

    /// Look up a journal.
    pub fn journal(&self, id: &JournalId) -> Option<&Journal> {
        self.journals.get(id)
    }

    /// Number of entries in `journal`.
    pub fn entry_count(&self, journal: &JournalId) -> usize {
        self.entries
            .values()
            .filter(|e| &e.journal == journal)
            .count()
    }

    /// Check the invariants a well-formed state keeps. Commands preserve these by
    /// construction; this exists for state that arrives from outside — a file on
    /// disk, an import — where nothing has checked it yet.
    pub fn validate(&self) -> Result<(), crate::OpError> {
        use crate::OpError;
        if self.journals.is_empty() {
            return Err(OpError::LastJournal);
        }
        if self.journals.len() > MAX_JOURNALS {
            return Err(OpError::TooManyJournals);
        }
        // A set, not a list: checking each name against a list is quadratic in
        // the journal count, and this runs on files nobody has checked yet.
        let mut names = std::collections::BTreeSet::new();
        for (id, j) in &self.journals {
            // Validate the id before echoing it into any error.
            crate::ops::check_id(id.as_str())?;
            if &j.id != id {
                return Err(OpError::IdMismatch(id.to_string()));
            }
            crate::ops::check_journal_name(&j.name)?;
            if !names.insert(crate::text::fold(&j.name)) {
                return Err(OpError::DuplicateJournalName(j.name.clone()));
            }
        }
        for (id, e) in &self.entries {
            crate::ops::check_id(id.as_str())?;
            if &e.id != id {
                return Err(OpError::IdMismatch(id.to_string()));
            }
            if !self.journals.contains_key(&e.journal) {
                return Err(OpError::JournalNotFound(e.journal.clone()));
            }
            crate::ops::check_title(&e.title)?;
            crate::ops::check_body(&e.body)?;
            if e.tags.len() > crate::tag::MAX_TAGS_PER_ENTRY {
                return Err(OpError::InvalidTag {
                    index: crate::tag::MAX_TAGS_PER_ENTRY,
                    reason: crate::TagError::TooMany,
                });
            }
            for (i, t) in e.tags.iter().enumerate() {
                if e.tags[..i].contains(t) {
                    return Err(OpError::InvalidTag {
                        index: i,
                        reason: crate::TagError::Duplicate,
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_state_has_exactly_one_journal_and_no_entries() {
        let s = JournalState::new(JournalId::from("j1"), "  My   Journal ", 5).unwrap();
        assert_eq!(s.journals.len(), 1);
        assert!(s.entries.is_empty());
        let j = s.journal(&JournalId::from("j1")).unwrap();
        assert_eq!(j.name, "My Journal");
        assert_eq!(j.created_at_ms, 5);
        assert_eq!(s.entry_count(&JournalId::from("j1")), 0);
        assert!(s.validate().is_ok());
    }

    #[test]
    fn new_checks_its_first_journal_like_create_journal_does() {
        use crate::OpError;
        let n = |id: &str, name: &str| JournalState::new(JournalId::from(id), name, 0);
        assert_eq!(n("", "Ok"), Err(OpError::InvalidId));
        assert_eq!(n("j", "   "), Err(OpError::EmptyJournalName));
        assert_eq!(n("j", "a\nb"), Err(OpError::InvalidJournalName));
        assert_eq!(n("j", "Wo\u{200B}rk"), Err(OpError::InvalidJournalName));
    }

    #[test]
    fn validate_catches_what_only_outside_state_can_break() {
        use crate::{OpError, TagError};
        let base = JournalState::new(JournalId::from("j1"), "One", 0).unwrap();

        let mut empty = base.clone();
        empty.journals.clear();
        assert_eq!(empty.validate(), Err(OpError::LastJournal));

        let mut mismatch = base.clone();
        mismatch
            .journals
            .get_mut(&JournalId::from("j1"))
            .unwrap()
            .id = JournalId::from("x");
        assert_eq!(mismatch.validate(), Err(OpError::IdMismatch("j1".into())));

        let mut bad_key = base.clone();
        let mut j = bad_key.journals.values().next().unwrap().clone();
        j.id = JournalId::from("bad key");
        bad_key.journals.insert(j.id.clone(), j);
        assert_eq!(bad_key.validate(), Err(OpError::InvalidId));

        let mut crowded = base.clone();
        for i in 0..MAX_JOURNALS {
            let id = JournalId::from(format!("x{i}").as_str());
            crowded.journals.insert(
                id.clone(),
                Journal {
                    id,
                    name: format!("X{i}"),
                    created_at_ms: 0,
                },
            );
        }
        assert_eq!(crowded.validate(), Err(OpError::TooManyJournals));

        let mut dup_name = base.clone();
        dup_name.journals.insert(
            JournalId::from("j2"),
            Journal {
                id: JournalId::from("j2"),
                name: "ONE".into(),
                created_at_ms: 0,
            },
        );
        assert_eq!(
            dup_name.validate(),
            Err(OpError::DuplicateJournalName("ONE".into()))
        );

        let entry = Entry {
            id: EntryId::from("e"),
            journal: JournalId::from("j1"),
            title: String::new(),
            body: String::new(),
            date: Date(0),
            created_at_ms: 0,
            updated_at_ms: 0,
            tags: vec![Tag::new("a").unwrap(), Tag::new("A").unwrap()],
            starred: false,
        };
        let mut dup_tag = base.clone();
        dup_tag.entries.insert(entry.id.clone(), entry.clone());
        assert_eq!(
            dup_tag.validate(),
            Err(OpError::InvalidTag {
                index: 1,
                reason: TagError::Duplicate
            })
        );

        let mut too_many = base.clone();
        let mut many = entry.clone();
        many.tags = (0..65)
            .map(|i| Tag::new(&format!("t{i}")).unwrap())
            .collect();
        too_many.entries.insert(many.id.clone(), many);
        assert!(matches!(
            too_many.validate(),
            Err(OpError::InvalidTag {
                reason: TagError::TooMany,
                ..
            })
        ));

        let mut wrong_key = base.clone();
        wrong_key
            .entries
            .insert(EntryId::from("other"), entry.clone());
        assert_eq!(
            wrong_key.validate(),
            Err(OpError::IdMismatch("other".into()))
        );

        let mut long_title = base;
        let mut t = entry;
        t.tags.clear();
        t.title = "x".repeat(MAX_TITLE_CHARS + 1);
        long_title.entries.insert(t.id.clone(), t);
        assert_eq!(long_title.validate(), Err(OpError::TitleTooLong));
    }
}
