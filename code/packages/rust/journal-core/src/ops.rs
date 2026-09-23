//! Commands — the only way the state changes.
//!
//! Every mutation is a [`Command`] value handed to [`apply`]. A command either
//! succeeds completely or leaves the state **exactly as it was**: each arm checks
//! everything first and writes only once nothing can fail. That is what lets a
//! host apply a command optimistically and simply keep the old state on error,
//! with no partial write to undo.
//!
//! ```text
//!   host ──Command──▶ apply(&mut state, cmd, now_ms)
//!                        │  validate … all checks pass?
//!                        │      no  ──▶ Err(OpError), state untouched
//!                        │      yes ──▶ write, Ok(())
//! ```
//!
//! Commands are plain data, so a host can log, replay, or send them across the
//! WebAssembly boundary as JSON (with the `serde` feature).

use crate::model::{MAX_BODY_BYTES, MAX_JOURNALS, MAX_JOURNAL_NAME_CHARS, MAX_TITLE_CHARS};
use crate::tag::{normalize_tags, TagError};
use crate::{text, Date, Entry, EntryId, Journal, JournalId, JournalState};

/// A request to change the state.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        tag = "type",
        rename_all = "camelCase",
        rename_all_fields = "camelCase"
    )
)]
pub enum Command {
    /// Add a journal.
    CreateJournal {
        /// Host-minted id.
        id: JournalId,
        /// Display name.
        name: String,
    },
    /// Rename a journal.
    RenameJournal {
        /// Which journal.
        id: JournalId,
        /// The new name.
        name: String,
    },
    /// Remove a journal. Its entries move to `move_entries_to`, or are deleted
    /// with it when that is `None`. The last journal cannot be deleted.
    DeleteJournal {
        /// Which journal.
        id: JournalId,
        /// Where its entries go; `None` deletes them.
        move_entries_to: Option<JournalId>,
    },
    /// Write a new entry.
    CreateEntry {
        /// Host-minted id.
        id: EntryId,
        /// Which journal it goes in.
        journal: JournalId,
        /// The day it is about.
        date: Date,
        /// Heading (may be empty).
        title: String,
        /// Markdown body.
        body: String,
    },
    /// Change an entry's title and/or body; `None` leaves that field alone.
    EditEntry {
        /// Which entry.
        id: EntryId,
        /// New title, if changing.
        title: Option<String>,
        /// New body, if changing.
        body: Option<String>,
    },
    /// Move an entry to another day.
    SetEntryDate {
        /// Which entry.
        id: EntryId,
        /// The new day.
        date: Date,
    },
    /// Move an entry to another journal.
    MoveEntry {
        /// Which entry.
        id: EntryId,
        /// The destination journal.
        journal: JournalId,
    },
    /// Replace an entry's tags (tidied, deduplicated case-insensitively).
    SetTags {
        /// Which entry.
        id: EntryId,
        /// The complete new tag list, as typed.
        tags: Vec<String>,
    },
    /// Star or unstar an entry.
    SetStarred {
        /// Which entry.
        id: EntryId,
        /// The new flag.
        starred: bool,
    },
    /// Remove an entry.
    DeleteEntry {
        /// Which entry.
        id: EntryId,
    },
}

/// Why a command (or a loaded state) was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpError {
    /// No journal has this id.
    JournalNotFound(JournalId),
    /// No entry has this id.
    EntryNotFound(EntryId),
    /// The host tried to create something with an id already in use.
    DuplicateId(String),
    /// An id was empty, longer than [`crate::MAX_ID_BYTES`], or not printable
    /// ASCII. The id itself is deliberately *not* carried, so a hostile id cannot
    /// ride an error message into a host's log.
    InvalidId,
    /// Creating another journal would exceed [`MAX_JOURNALS`].
    TooManyJournals,
    /// A map key and the id stored under it disagree (loaded state only).
    IdMismatch(String),
    /// Another journal already has this name (case-insensitively).
    DuplicateJournalName(String),
    /// A journal name was empty after tidying.
    EmptyJournalName,
    /// A journal name exceeded [`MAX_JOURNAL_NAME_CHARS`] or contained a control
    /// character.
    InvalidJournalName,
    /// A title exceeded [`MAX_TITLE_CHARS`].
    TitleTooLong,
    /// A body exceeded [`MAX_BODY_BYTES`].
    BodyTooLarge,
    /// A tag was invalid; `index` is its position in the input list.
    InvalidTag {
        /// Position of the offending tag in the input.
        index: usize,
        /// What was wrong with it.
        reason: TagError,
    },
    /// Deleting this journal would leave none.
    LastJournal,
    /// `DeleteJournal` was asked to move entries into the journal being deleted.
    MoveTargetIsDeleted,
}

impl core::fmt::Display for OpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OpError::JournalNotFound(id) => write!(f, "journal {id} not found"),
            OpError::EntryNotFound(id) => write!(f, "entry {id} not found"),
            OpError::DuplicateId(id) => write!(f, "id {id} is already in use"),
            OpError::InvalidId => write!(
                f,
                "an id must be 1 to {} printable ASCII bytes",
                crate::MAX_ID_BYTES
            ),
            OpError::TooManyJournals => write!(f, "at most {MAX_JOURNALS} journals are allowed"),
            OpError::IdMismatch(id) => write!(f, "record stored under {id} carries a different id"),
            OpError::DuplicateJournalName(n) => write!(f, "a journal named {n:?} already exists"),
            OpError::EmptyJournalName => f.write_str("a journal name cannot be empty"),
            OpError::InvalidJournalName => write!(
                f,
                "a journal name must be at most {MAX_JOURNAL_NAME_CHARS} characters on one line"
            ),
            OpError::TitleTooLong => {
                write!(f, "a title must be at most {MAX_TITLE_CHARS} characters")
            }
            OpError::BodyTooLarge => {
                write!(f, "an entry body must be at most {MAX_BODY_BYTES} bytes")
            }
            OpError::InvalidTag { index, reason } => {
                write!(f, "tag {index} is invalid: {reason:?}")
            }
            OpError::LastJournal => f.write_str("the last journal cannot be deleted"),
            OpError::MoveTargetIsDeleted => {
                f.write_str("entries cannot be moved into the journal being deleted")
            }
        }
    }
}

impl std::error::Error for OpError {}

// ── validation helpers (shared with JournalState::validate) ─────────────────────

pub(crate) fn check_journal_name(name: &str) -> Result<(), OpError> {
    if name.is_empty() {
        return Err(OpError::EmptyJournalName);
    }
    if text::has_control(name) || name.chars().count() > MAX_JOURNAL_NAME_CHARS {
        return Err(OpError::InvalidJournalName);
    }
    Ok(())
}

pub(crate) fn check_id(id: &str) -> Result<(), OpError> {
    if crate::is_valid_id(id) {
        Ok(())
    } else {
        Err(OpError::InvalidId)
    }
}

pub(crate) fn check_title(title: &str) -> Result<(), OpError> {
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(OpError::TitleTooLong);
    }
    Ok(())
}

pub(crate) fn check_body(body: &str) -> Result<(), OpError> {
    if body.len() > MAX_BODY_BYTES {
        return Err(OpError::BodyTooLarge);
    }
    Ok(())
}

/// Tidy and check a journal name, and refuse one another journal (other than
/// `except`) already uses.
fn journal_name(
    state: &JournalState,
    raw: &str,
    except: Option<&JournalId>,
) -> Result<String, OpError> {
    // Control characters are checked on the raw input: tidying would turn a
    // newline into a space and hide it.
    if text::has_control(raw.trim()) {
        return Err(OpError::InvalidJournalName);
    }
    let name = text::tidy(raw);
    check_journal_name(&name)?;
    let key = text::fold(&name);
    let clash = state
        .journals
        .values()
        .any(|j| Some(&j.id) != except && text::fold(&j.name) == key);
    if clash {
        return Err(OpError::DuplicateJournalName(name));
    }
    Ok(name)
}

// Every lookup checks the id first: a "not found" error echoes the id it was
// given, so an id that was never validated must not reach one.
fn entry_mut<'a>(state: &'a mut JournalState, id: &EntryId) -> Result<&'a mut Entry, OpError> {
    check_id(id.as_str())?;
    state
        .entries
        .get_mut(id)
        .ok_or_else(|| OpError::EntryNotFound(id.clone()))
}

fn require_journal(state: &JournalState, id: &JournalId) -> Result<(), OpError> {
    check_id(id.as_str())?;
    if state.journals.contains_key(id) {
        Ok(())
    } else {
        Err(OpError::JournalNotFound(id.clone()))
    }
}

fn require_entry(state: &JournalState, id: &EntryId) -> Result<(), OpError> {
    check_id(id.as_str())?;
    if state.entries.contains_key(id) {
        Ok(())
    } else {
        Err(OpError::EntryNotFound(id.clone()))
    }
}

/// Apply `cmd` to `state` at time `now_ms`. On `Err`, `state` is unchanged.
pub fn apply(state: &mut JournalState, cmd: Command, now_ms: u64) -> Result<(), OpError> {
    match cmd {
        Command::CreateJournal { id, name } => {
            check_id(id.as_str())?;
            if state.journals.len() >= MAX_JOURNALS {
                return Err(OpError::TooManyJournals);
            }
            if state.journals.contains_key(&id) {
                return Err(OpError::DuplicateId(id.to_string()));
            }
            let name = journal_name(state, &name, None)?;
            state.journals.insert(
                id.clone(),
                Journal {
                    id,
                    name,
                    created_at_ms: now_ms,
                },
            );
        }
        Command::RenameJournal { id, name } => {
            require_journal(state, &id)?;
            let name = journal_name(state, &name, Some(&id))?;
            if let Some(j) = state.journals.get_mut(&id) {
                j.name = name;
            }
        }
        Command::DeleteJournal {
            id,
            move_entries_to,
        } => {
            require_journal(state, &id)?;
            if state.journals.len() == 1 {
                return Err(OpError::LastJournal);
            }
            if let Some(target) = &move_entries_to {
                if target == &id {
                    return Err(OpError::MoveTargetIsDeleted);
                }
                require_journal(state, target)?;
            }
            // All checks passed — now write.
            match move_entries_to {
                Some(target) => {
                    for e in state.entries.values_mut().filter(|e| e.journal == id) {
                        e.journal = target.clone();
                        e.updated_at_ms = now_ms;
                    }
                }
                None => state.entries.retain(|_, e| e.journal != id),
            }
            state.journals.remove(&id);
        }
        Command::CreateEntry {
            id,
            journal,
            date,
            title,
            body,
        } => {
            check_id(id.as_str())?;
            if state.entries.contains_key(&id) {
                return Err(OpError::DuplicateId(id.to_string()));
            }
            require_journal(state, &journal)?;
            check_title(&title)?;
            check_body(&body)?;
            state.entries.insert(
                id.clone(),
                Entry {
                    id,
                    journal,
                    title,
                    body,
                    date,
                    created_at_ms: now_ms,
                    updated_at_ms: now_ms,
                    tags: Vec::new(),
                    starred: false,
                },
            );
        }
        Command::EditEntry { id, title, body } => {
            require_entry(state, &id)?;
            if let Some(t) = &title {
                check_title(t)?;
            }
            if let Some(b) = &body {
                check_body(b)?;
            }
            let e = entry_mut(state, &id)?;
            if let Some(t) = title {
                e.title = t;
            }
            if let Some(b) = body {
                e.body = b;
            }
            e.updated_at_ms = now_ms;
        }
        Command::SetEntryDate { id, date } => {
            let e = entry_mut(state, &id)?;
            e.date = date;
            e.updated_at_ms = now_ms;
        }
        Command::MoveEntry { id, journal } => {
            require_entry(state, &id)?;
            require_journal(state, &journal)?;
            let e = entry_mut(state, &id)?;
            e.journal = journal;
            e.updated_at_ms = now_ms;
        }
        Command::SetTags { id, tags } => {
            require_entry(state, &id)?;
            let tags = normalize_tags(&tags)
                .map_err(|(index, reason)| OpError::InvalidTag { index, reason })?;
            let e = entry_mut(state, &id)?;
            e.tags = tags;
            e.updated_at_ms = now_ms;
        }
        Command::SetStarred { id, starred } => {
            let e = entry_mut(state, &id)?;
            e.starred = starred;
            e.updated_at_ms = now_ms;
        }
        Command::DeleteEntry { id } => {
            check_id(id.as_str())?;
            if state.entries.remove(&id).is_none() {
                return Err(OpError::EntryNotFound(id));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn j(s: &str) -> JournalId {
        JournalId::from(s)
    }
    fn e(s: &str) -> EntryId {
        EntryId::from(s)
    }
    fn day(s: &str) -> Date {
        Date::parse_iso(s).unwrap()
    }

    fn state() -> JournalState {
        JournalState::new(j("personal"), "Personal", 1).unwrap()
    }

    fn with_entry() -> JournalState {
        let mut s = state();
        apply(
            &mut s,
            Command::CreateEntry {
                id: e("e1"),
                journal: j("personal"),
                date: day("2026-09-23"),
                title: "Hello".into(),
                body: "First *entry*".into(),
            },
            10,
        )
        .unwrap();
        s
    }

    /// Apply a command that must fail, and prove the state did not move.
    fn rejects(s: &mut JournalState, cmd: Command, want: OpError) {
        let before = s.clone();
        assert_eq!(apply(s, cmd, 999), Err(want));
        assert_eq!(
            *s, before,
            "a rejected command must leave the state untouched"
        );
    }

    #[test]
    fn create_entry_stamps_both_instants() {
        let s = with_entry();
        let en = s.entry(&e("e1")).unwrap();
        assert_eq!(en.created_at_ms, 10);
        assert_eq!(en.updated_at_ms, 10);
        assert_eq!(en.journal, j("personal"));
        assert!(en.tags.is_empty());
        assert!(!en.starred);
    }

    #[test]
    fn create_entry_refuses_bad_input() {
        let mut s = with_entry();
        let mk = |id: &str, journal: &str, title: String, body: String| Command::CreateEntry {
            id: e(id),
            journal: j(journal),
            date: day("2026-09-23"),
            title,
            body,
        };
        rejects(
            &mut s,
            mk("e1", "personal", "".into(), "".into()),
            OpError::DuplicateId("e1".into()),
        );
        rejects(
            &mut s,
            mk("e2", "nope", "".into(), "".into()),
            OpError::JournalNotFound(j("nope")),
        );
        rejects(
            &mut s,
            mk("e2", "personal", "t".repeat(513), "".into()),
            OpError::TitleTooLong,
        );
        rejects(
            &mut s,
            mk("e2", "personal", "".into(), "b".repeat(MAX_BODY_BYTES + 1)),
            OpError::BodyTooLarge,
        );
        // Exactly at the limits is fine.
        apply(
            &mut s,
            mk(
                "e2",
                "personal",
                "t".repeat(512),
                "b".repeat(MAX_BODY_BYTES),
            ),
            1,
        )
        .unwrap();
    }

    #[test]
    fn edit_entry_changes_only_what_is_given() {
        let mut s = with_entry();
        apply(
            &mut s,
            Command::EditEntry {
                id: e("e1"),
                title: None,
                body: Some("Rewritten".into()),
            },
            20,
        )
        .unwrap();
        let en = s.entry(&e("e1")).unwrap();
        assert_eq!(en.title, "Hello");
        assert_eq!(en.body, "Rewritten");
        assert_eq!(en.created_at_ms, 10);
        assert_eq!(en.updated_at_ms, 20);
    }

    #[test]
    fn edit_entry_validates_before_writing() {
        let mut s = with_entry();
        // A valid title paired with an oversized body must not half-apply.
        rejects(
            &mut s,
            Command::EditEntry {
                id: e("e1"),
                title: Some("New".into()),
                body: Some("b".repeat(MAX_BODY_BYTES + 1)),
            },
            OpError::BodyTooLarge,
        );
        rejects(
            &mut s,
            Command::EditEntry {
                id: e("e1"),
                title: Some("t".repeat(513)),
                body: None,
            },
            OpError::TitleTooLong,
        );
        rejects(
            &mut s,
            Command::EditEntry {
                id: e("zz"),
                title: None,
                body: None,
            },
            OpError::EntryNotFound(e("zz")),
        );
    }

    #[test]
    fn set_date_move_star_and_tags() {
        let mut s = with_entry();
        apply(
            &mut s,
            Command::CreateJournal {
                id: j("work"),
                name: "Work".into(),
            },
            11,
        )
        .unwrap();
        apply(
            &mut s,
            Command::SetEntryDate {
                id: e("e1"),
                date: day("2020-01-02"),
            },
            12,
        )
        .unwrap();
        apply(
            &mut s,
            Command::MoveEntry {
                id: e("e1"),
                journal: j("work"),
            },
            13,
        )
        .unwrap();
        apply(
            &mut s,
            Command::SetStarred {
                id: e("e1"),
                starred: true,
            },
            14,
        )
        .unwrap();
        apply(
            &mut s,
            Command::SetTags {
                id: e("e1"),
                tags: vec!["Travel".into(), "travel".into(), " food ".into()],
            },
            15,
        )
        .unwrap();
        let en = s.entry(&e("e1")).unwrap();
        assert_eq!(en.date, day("2020-01-02"));
        assert_eq!(en.journal, j("work"));
        assert!(en.starred);
        let tags: Vec<&str> = en.tags.iter().map(|t| t.display()).collect();
        assert_eq!(tags, ["Travel", "food"]);
        assert!(en.has_tag(&crate::Tag::new("FOOD").unwrap()));
        assert_eq!(en.updated_at_ms, 15);
    }

    #[test]
    fn entry_commands_on_missing_targets_fail_cleanly() {
        let mut s = with_entry();
        rejects(
            &mut s,
            Command::SetEntryDate {
                id: e("x"),
                date: day("2020-01-01"),
            },
            OpError::EntryNotFound(e("x")),
        );
        rejects(
            &mut s,
            Command::MoveEntry {
                id: e("e1"),
                journal: j("nope"),
            },
            OpError::JournalNotFound(j("nope")),
        );
        rejects(
            &mut s,
            Command::MoveEntry {
                id: e("x"),
                journal: j("personal"),
            },
            OpError::EntryNotFound(e("x")),
        );
        rejects(
            &mut s,
            Command::SetStarred {
                id: e("x"),
                starred: true,
            },
            OpError::EntryNotFound(e("x")),
        );
        rejects(
            &mut s,
            Command::SetTags {
                id: e("x"),
                tags: vec![],
            },
            OpError::EntryNotFound(e("x")),
        );
        rejects(
            &mut s,
            Command::SetTags {
                id: e("e1"),
                tags: vec!["ok".into(), "bad\ntag".into()],
            },
            OpError::InvalidTag {
                index: 1,
                reason: TagError::ControlCharacter,
            },
        );
        rejects(
            &mut s,
            Command::DeleteEntry { id: e("x") },
            OpError::EntryNotFound(e("x")),
        );
    }

    #[test]
    fn unusable_ids_are_refused_without_echoing_them() {
        let mut s = state();
        for bad in ["", "two words", "\u{1b}[31mred", &"x".repeat(65)] {
            rejects(
                &mut s,
                Command::CreateJournal {
                    id: j(bad),
                    name: "N".into(),
                },
                OpError::InvalidId,
            );
            rejects(
                &mut s,
                Command::CreateEntry {
                    id: e(bad),
                    journal: j("personal"),
                    date: day("2026-01-01"),
                    title: String::new(),
                    body: String::new(),
                },
                OpError::InvalidId,
            );
        }
    }

    #[test]
    fn looked_up_ids_are_checked_before_any_error_can_echo_them() {
        let mut s = with_entry();
        let evil = "\u{1b}[31mred\n";
        rejects(
            &mut s,
            Command::MoveEntry {
                id: e("e1"),
                journal: j(evil),
            },
            OpError::InvalidId,
        );
        rejects(
            &mut s,
            Command::MoveEntry {
                id: e(evil),
                journal: j("personal"),
            },
            OpError::InvalidId,
        );
        rejects(
            &mut s,
            Command::DeleteEntry { id: e(evil) },
            OpError::InvalidId,
        );
        rejects(
            &mut s,
            Command::SetStarred {
                id: e(evil),
                starred: true,
            },
            OpError::InvalidId,
        );
        rejects(
            &mut s,
            Command::DeleteJournal {
                id: j(evil),
                move_entries_to: None,
            },
            OpError::InvalidId,
        );
        rejects(
            &mut s,
            Command::RenameJournal {
                id: j(evil),
                name: "X".into(),
            },
            OpError::InvalidId,
        );
    }

    #[test]
    fn the_journal_count_is_capped() {
        let mut s = state();
        for i in 1..MAX_JOURNALS {
            apply(
                &mut s,
                Command::CreateJournal {
                    id: j(&format!("j{i}")),
                    name: format!("J{i}"),
                },
                1,
            )
            .unwrap();
        }
        assert_eq!(s.journals.len(), MAX_JOURNALS);
        rejects(
            &mut s,
            Command::CreateJournal {
                id: j("one-more"),
                name: "One more".into(),
            },
            OpError::TooManyJournals,
        );
    }

    #[test]
    fn delete_entry_removes_it() {
        let mut s = with_entry();
        apply(&mut s, Command::DeleteEntry { id: e("e1") }, 20).unwrap();
        assert!(s.entries.is_empty());
    }

    #[test]
    fn journal_names_are_tidied_and_unique_case_insensitively() {
        let mut s = state();
        apply(
            &mut s,
            Command::CreateJournal {
                id: j("w"),
                name: "  Work   Log ".into(),
            },
            2,
        )
        .unwrap();
        assert_eq!(s.journal(&j("w")).unwrap().name, "Work Log");
        assert_eq!(s.journal(&j("w")).unwrap().created_at_ms, 2);
        rejects(
            &mut s,
            Command::CreateJournal {
                id: j("w2"),
                name: "work log".into(),
            },
            OpError::DuplicateJournalName("work log".into()),
        );
        rejects(
            &mut s,
            Command::CreateJournal {
                id: j("w"),
                name: "Other".into(),
            },
            OpError::DuplicateId("w".into()),
        );
        rejects(
            &mut s,
            Command::CreateJournal {
                id: j("x"),
                name: "   ".into(),
            },
            OpError::EmptyJournalName,
        );
        rejects(
            &mut s,
            Command::CreateJournal {
                id: j("x"),
                name: "two\nlines".into(),
            },
            OpError::InvalidJournalName,
        );
        rejects(
            &mut s,
            Command::CreateJournal {
                id: j("x"),
                name: "n".repeat(129),
            },
            OpError::InvalidJournalName,
        );
    }

    #[test]
    fn rename_may_keep_its_own_name_but_not_take_another() {
        let mut s = state();
        apply(
            &mut s,
            Command::CreateJournal {
                id: j("w"),
                name: "Work".into(),
            },
            2,
        )
        .unwrap();
        // Re-casing your own name is not a clash with yourself.
        apply(
            &mut s,
            Command::RenameJournal {
                id: j("w"),
                name: "WORK".into(),
            },
            3,
        )
        .unwrap();
        assert_eq!(s.journal(&j("w")).unwrap().name, "WORK");
        rejects(
            &mut s,
            Command::RenameJournal {
                id: j("w"),
                name: "personal".into(),
            },
            OpError::DuplicateJournalName("personal".into()),
        );
        rejects(
            &mut s,
            Command::RenameJournal {
                id: j("nope"),
                name: "X".into(),
            },
            OpError::JournalNotFound(j("nope")),
        );
    }

    #[test]
    fn delete_journal_moves_or_drops_its_entries() {
        let mut s = with_entry();
        apply(
            &mut s,
            Command::CreateJournal {
                id: j("w"),
                name: "Work".into(),
            },
            2,
        )
        .unwrap();
        apply(
            &mut s,
            Command::CreateJournal {
                id: j("t"),
                name: "Travel".into(),
            },
            2,
        )
        .unwrap();

        // Move: personal → work.
        apply(
            &mut s,
            Command::DeleteJournal {
                id: j("personal"),
                move_entries_to: Some(j("w")),
            },
            30,
        )
        .unwrap();
        assert!(s.journal(&j("personal")).is_none());
        let en = s.entry(&e("e1")).unwrap();
        assert_eq!(en.journal, j("w"));
        assert_eq!(en.updated_at_ms, 30);

        // Drop: deleting work with no target deletes e1 with it.
        apply(
            &mut s,
            Command::DeleteJournal {
                id: j("w"),
                move_entries_to: None,
            },
            31,
        )
        .unwrap();
        assert!(s.entries.is_empty());
        assert_eq!(s.journals.len(), 1);
    }

    #[test]
    fn delete_journal_guards() {
        let mut s = with_entry();
        rejects(
            &mut s,
            Command::DeleteJournal {
                id: j("personal"),
                move_entries_to: None,
            },
            OpError::LastJournal,
        );
        apply(
            &mut s,
            Command::CreateJournal {
                id: j("w"),
                name: "Work".into(),
            },
            2,
        )
        .unwrap();
        rejects(
            &mut s,
            Command::DeleteJournal {
                id: j("personal"),
                move_entries_to: Some(j("personal")),
            },
            OpError::MoveTargetIsDeleted,
        );
        rejects(
            &mut s,
            Command::DeleteJournal {
                id: j("personal"),
                move_entries_to: Some(j("nope")),
            },
            OpError::JournalNotFound(j("nope")),
        );
        rejects(
            &mut s,
            Command::DeleteJournal {
                id: j("nope"),
                move_entries_to: None,
            },
            OpError::JournalNotFound(j("nope")),
        );
    }

    #[test]
    fn errors_have_readable_messages() {
        let msgs = [
            OpError::JournalNotFound(j("a")).to_string(),
            OpError::EntryNotFound(e("a")).to_string(),
            OpError::DuplicateId("a".into()).to_string(),
            OpError::InvalidId.to_string(),
            OpError::TooManyJournals.to_string(),
            OpError::IdMismatch("a".into()).to_string(),
            OpError::DuplicateJournalName("A".into()).to_string(),
            OpError::EmptyJournalName.to_string(),
            OpError::InvalidJournalName.to_string(),
            OpError::TitleTooLong.to_string(),
            OpError::BodyTooLarge.to_string(),
            OpError::InvalidTag {
                index: 0,
                reason: TagError::Empty,
            }
            .to_string(),
            OpError::LastJournal.to_string(),
            OpError::MoveTargetIsDeleted.to_string(),
        ];
        for m in msgs {
            assert!(!m.is_empty());
        }
    }
}
