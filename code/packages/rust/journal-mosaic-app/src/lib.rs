//! Journal on the standard Mosaic application ABI (J3c-1 of #14416; spec
//! `code/specs/journal-mosaic-app.md`).
//!
//! `journal-core` is the pure engine. This crate is the thin adapter every
//! generated Mosaic host loads as `libmosaic_app`: it turns the engine's state
//! into the `journal-app` package's slot values, and the package's events into
//! engine commands — the same job `task-mosaic-app` does for Trestle.
//!
//! ```text
//!   host ──event "onSaveEntry"──▶ dispatch ──Command::CreateEntry──▶ journal-core
//!   host ◀──props {timeline-rows, draft-title, …}── props() ◀── projections
//! ```
//!
//! ## The editor model
//!
//! The screen is a timeline beside one editor. The editor holds a **draft** for
//! a **target**: either a new entry, or an existing one. Typing only changes the
//! draft; nothing reaches the engine until Save. Select loads an entry's draft,
//! New starts an empty one, Cancel reloads the draft from its target.
//!
//! ## Search (J4a)
//!
//! A non-blank query turns the timeline into ranked search results from
//! `journal_core::projections::search`. The query is a way of LOOKING at the
//! journal, so it lives beside the persisted state, not in it: a restart
//! opens the full timeline.
//!
//! ## Stars (J4b)
//!
//! The editor's Star button sets `Entry::starred` directly (it does not save
//! the draft), and a *Starred only* filter narrows the timeline and search
//! alike. The filter is a view, like the query, and is not persisted.
//!
//! ## On this day (J4c)
//!
//! Above the timeline, the engine's `on_this_day` recall: what was written on
//! today's month and day in earlier years. Hidden while searching.
//!
//! ## Tags (J4d)
//!
//! Tags are part of the draft, typed comma-separated, and written on Save
//! AFTER being validated, so a bad tag leaves the journal untouched. A
//! SegmentedControl of the journal's tags filters every list; the choice is
//! a view, not persisted.
//!
//! ## An entry's day, and draft errors (J4e)
//!
//! The draft carries a `YYYY-MM-DD` day (blank = today). Save checks the day
//! and the tags before writing anything and, when either is wrong, says so in
//! `draft-error` instead of failing the event, so the person typing sees why.
//!
//! ## Journals (J4f)
//!
//! A switcher over the journal's journals ("All journals" first) filters every
//! list; new entries are filed into the selected one. A *New journal* field
//! creates one. The selection and the field are views, not persisted.
//!
//! ## An entry's journal (J4g)
//!
//! The editor's *Journal* picker says which journal the draft belongs to;
//! Save files a new entry there, or moves an existing one (`MoveEntry`).

use std::error::Error;
use std::fmt;

use journal_core::projections::{on_this_day, search, tag_counts, timeline, MAX_QUERY_CHARS};
use journal_core::tag::{normalize_tags, TagError, MAX_TAGS_PER_ENTRY, MAX_TAG_CHARS};
use journal_core::{
    apply, Command, Date, Entry, EntryFilter, EntryId, Journal, JournalId, JournalState, OpError,
    MAX_BODY_BYTES, MAX_ID_BYTES, MAX_JOURNALS, MAX_JOURNAL_NAME_CHARS, MAX_TITLE_CHARS,
};
use mosaic_app_runtime::{
    Announcement, AppUpdate, Event, MosaicApp, Politeness, Snapshot, StartContext,
    MAX_UTC_OFFSET_MINUTES, MIN_UTC_OFFSET_MINUTES,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const SNAPSHOT_SCHEMA: &str = "journal-mosaic-app/state";
const SNAPSHOT_VERSION: u32 = 1;
const DEFAULT_JOURNAL: &str = "journal-personal";
const MS_PER_DAY: u64 = 86_400_000;
/// Highest entry counter a snapshot may carry. Well past any real journal, and
/// far enough below `u64::MAX` that minting can never saturate and spin.
const MAX_NEXT_ENTRY: u64 = 1 << 53;
/// Longest subtitle (and body-derived title), in characters.
const EXCERPT_CHARS: usize = 100;

/// What the editor is editing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "id")]
enum Target {
    /// A new entry, created on Save.
    New,
    /// An existing entry.
    Entry(EntryId),
}

/// Everything persisted across restarts — including an unsaved draft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppState {
    journal: JournalState,
    target: Target,
    draft_title: String,
    draft_body: String,
    /// The editor's tags, as typed: `travel, family` (J4d). Defaults to empty
    /// so a version-1 snapshot written before tags still loads.
    #[serde(default)]
    draft_tags: String,
    /// The editor's day, as typed: `2026-09-23`, or blank for today (J4e).
    #[serde(default)]
    draft_date: String,
    /// The journal chosen in the editor's picker (J4g), or empty when none
    /// was chosen; see [`JournalMosaicApp::draft_journal`].
    #[serde(default)]
    draft_journal: String,
    next_entry: u64,
}

/// The Journal application.
#[derive(Clone)]
pub struct JournalMosaicApp {
    state: AppState,
    /// Milliseconds since the Unix epoch. A plain function so the app stays
    /// `Clone` (dispatch clones to roll back) and tests can pin time.
    clock: fn() -> u64,
    /// The host's UTC offset from `StartContext` (UI38 "Local time"), in
    /// minutes east of UTC; 0 (UTC) when the host did not say. Host context,
    /// not journal state: it is not in the snapshot.
    utc_offset_minutes: i32,
    /// The search field's text (J4a). Not in the snapshot; see the module
    /// docs. At most [`MAX_QUERY_CHARS`] characters, the most the engine reads.
    search_query: String,
    /// The *Starred only* filter (J4b). Not in the snapshot, like the query.
    starred_only: bool,
    /// The selected tag's key (J4d). Not in the snapshot, like the filter.
    tag_filter: Option<String>,
    /// Why the last Save refused the draft, or `""` (J4e). Not persisted: it
    /// describes the last attempt, not the journal.
    draft_error: String,
    /// The selected journal's id, or `None` for all (J4f). Not persisted.
    journal_filter: Option<String>,
    /// The *New journal* field, and why the engine refused its last name.
    new_journal_name: String,
    journal_error: String,
}

impl Default for JournalMosaicApp {
    fn default() -> Self {
        Self::with_clock(system_now_ms)
    }
}

/// Why an event, or a snapshot, was refused. The app is unchanged either way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalAppError {
    /// An event this app does not declare (UI38 §4.1: an error, not a no-op).
    UnknownEvent(String),
    /// A declared event with a missing or ill-typed payload field.
    InvalidPayload {
        /// The event.
        event: String,
        /// The field.
        field: &'static str,
    },
    /// The engine refused the command (e.g. a title over its limit).
    Engine(String),
    /// A snapshot this app did not write, cannot read, or that fails validation.
    InvalidSnapshot,
}

impl fmt::Display for JournalAppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEvent(event) => write!(f, "unknown Journal event `{event}`"),
            Self::InvalidPayload { event, field } => {
                write!(f, "Journal event `{event}` has an invalid `{field}`")
            }
            Self::Engine(message) => write!(f, "journal-core rejected the edit: {message}"),
            Self::InvalidSnapshot => f.write_str("invalid Journal snapshot"),
        }
    }
}

impl Error for JournalAppError {}

impl JournalMosaicApp {
    /// A fresh app with one empty "Personal" journal, reading time from `clock`.
    pub fn with_clock(clock: fn() -> u64) -> Self {
        let journal = JournalState::new(JournalId::from(DEFAULT_JOURNAL), "Personal", clock())
            .expect("the built-in journal id and name are valid");
        Self {
            utc_offset_minutes: 0,
            search_query: String::new(),
            starred_only: false,
            tag_filter: None,
            draft_error: String::new(),
            journal_filter: None,
            new_journal_name: String::new(),
            journal_error: String::new(),
            state: AppState {
                journal,
                target: Target::New,
                draft_title: String::new(),
                draft_body: String::new(),
                draft_tags: String::new(),
                draft_date: String::new(),
                draft_journal: String::new(),
                next_entry: 1,
            },
            clock,
        }
    }

    // ── props ────────────────────────────────────────────────────────────────

    /// The slot values, keyed exactly by the `journal-app` package's slot names.
    fn props(&self) -> Value {
        let searching = self.searching();
        let rows = self.timeline_rows();
        let journal_empty = self.state.journal.entries.is_empty();
        let recalled = self.on_this_day_rows();
        let tags = self.all_tags();
        let active = self.active_tag();
        let draft_journal = self.draft_journal();
        let draft_journal_index = self
            .journals_ordered()
            .iter()
            .position(|j| j.id == draft_journal)
            .map_or(0, |i| i as i64);
        json!({
            // "The journal has no entries", whatever the query. A search that
            // matches nothing is `no-matches`, so the two empty states can say
            // different things.
            "timeline-empty": journal_empty,
            "timeline-rows": rows,
            "search-query": self.search_query,
            "searching": searching,
            "no-matches": searching && rows.is_empty(),
            "starred-only": self.starred_only,
            // Only the plain timeline: a search that finds nothing is
            // `no-matches` whether or not the filter is on.
            "no-starred": self.starred_only && !searching && !journal_empty && rows.is_empty(),
            // A search takes the whole pane; the recall is a timeline companion.
            "has-on-this-day": !searching && !recalled.is_empty(),
            "on-this-day-rows": recalled,
            "draft-tags": self.state.draft_tags,
            "draft-date": self.state.draft_date,
            "journal-options": std::iter::once("All journals".to_string())
                .chain(self.journals_ordered().iter().map(|j| j.name.clone()))
                .collect::<Vec<_>>(),
            "selected-journal-index": self
                .active_journal()
                .and_then(|id| self.journals_ordered().iter().position(|j| j.id == id))
                .map_or(0, |i| i as i64 + 1),
            "new-journal-name": self.new_journal_name,
            "journal-error": self.journal_error,
            "draft-journal-options": self
                .journals_ordered()
                .iter()
                .map(|j| j.name.clone())
                .collect::<Vec<_>>(),
            "draft-journal-index": draft_journal_index,
            "has-journals": self.state.journal.journals.len() > 1,
            "draft-error": self.draft_error,
            "tag-options": tags.iter().map(|c| format!("#{} ({})", c.tag.display(), c.count)).collect::<Vec<_>>(),
            "selected-tag-index": active
                .as_ref()
                .and_then(|tag| tags.iter().position(|c| c.tag.key() == tag.key()))
                .map_or(-1, |i| i as i64),
            "has-tags": !tags.is_empty(),
            "star-label": match &self.state.target {
                Target::New => "",
                Target::Entry(id) => match self.state.journal.entry(id) {
                    Some(entry) if entry.starred => "Unstar",
                    _ => "Star",
                },
            },
            "selected-key": match &self.state.target {
                Target::New => "",
                Target::Entry(id) => id.as_str(),
            },
            "draft-title": self.state.draft_title,
            "draft-body": self.state.draft_body,
            "delete-label": match self.state.target {
                Target::New => "",
                Target::Entry(_) => "Delete",
            },
        })
    }

    /// The filter every projection shares: *Starred only* narrows the
    /// timeline and search alike.
    fn filter(&self) -> EntryFilter {
        EntryFilter {
            starred_only: self.starred_only,
            tag: self.active_tag(),
            journal: self.active_journal(),
        }
    }

    /// The journals in the order they were created (ties by id), as the
    /// switcher lists them after "All journals".
    fn journals_ordered(&self) -> Vec<&Journal> {
        let mut journals: Vec<&Journal> = self.state.journal.journals.values().collect();
        journals.sort_by(|a, b| a.created_at_ms.cmp(&b.created_at_ms).then(a.id.cmp(&b.id)));
        journals
    }

    /// The selected journal, if it still exists.
    fn active_journal(&self) -> Option<JournalId> {
        let id = JournalId::from(self.journal_filter.as_deref()?);
        self.state.journal.journal(&id).map(|j| j.id.clone())
    }

    /// The draft's journal (J4g), always one that exists: the one chosen in
    /// the picker; else the open entry's own; else, for a new draft, the
    /// pane's selected journal, or Personal under "All journals". So an
    /// unchosen new draft follows the pane, and a snapshot naming a journal
    /// that is gone falls back instead of failing.
    fn draft_journal(&self) -> JournalId {
        let chosen = JournalId::from(self.state.draft_journal.as_str());
        if self.state.journal.journal(&chosen).is_some() {
            return chosen;
        }
        if let Target::Entry(id) = &self.state.target {
            if let Some(entry) = self.state.journal.entry(id) {
                return entry.journal.clone();
            }
        }
        self.active_journal().unwrap_or_else(|| JournalId::from(DEFAULT_JOURNAL))
    }

    /// Every tag in the journal with its entry count, most used first. Counted
    /// over ALL entries, so an option never vanishes under the filter it sets.
    fn all_tags(&self) -> Vec<journal_core::projections::TagCount> {
        tag_counts(
            &self.state.journal,
            &EntryFilter {
                journal: self.active_journal(),
                ..EntryFilter::default()
            },
        )
    }

    /// The selected tag, if it still exists: a tag whose last entry was
    /// deleted or retagged simply stops filtering.
    fn active_tag(&self) -> Option<journal_core::Tag> {
        let key = self.tag_filter.as_deref()?;
        self.all_tags()
            .into_iter()
            .find(|count| count.tag.key() == key)
            .map(|count| count.tag)
    }

    /// Whether the query has anything to search for.
    fn searching(&self) -> bool {
        !self.search_query.trim().is_empty()
    }

    /// The rows `RecordList` shows, and that `onSelectEntry`'s index refers
    /// to: search results while [`searching`](Self::searching), else the
    /// timeline.
    fn timeline_rows(&self) -> Vec<[String; 6]> {
        if self.searching() {
            self.search_rows()
        } else {
            self.day_rows()
        }
    }

    /// The engine's recall for the user's today, as `RecordList` rows:
    ///
    /// ```text
    ///   [key, "1 year ago · 24 Sep 2025", title, subtitle, "", badge]
    ///   [key, "",                          title, subtitle, "", badge]   same year
    ///   [key, "3 years ago · 24 Sep 2023", title, subtitle, "", badge]
    /// ```
    ///
    /// The heading opens each year; `meta` stays empty because the heading
    /// already says when. Today is the LOCAL day, as for filing entries.
    fn on_this_day_rows(&self) -> Vec<[String; 6]> {
        let today = today((self.clock)(), self.utc_offset_minutes);
        let mut rows = Vec::new();
        for group in on_this_day(&self.state.journal, today, &self.filter()) {
            let heading = format!(
                "{} · {}",
                if group.years_ago == 1 {
                    "1 year ago".to_string()
                } else {
                    format!("{} years ago", group.years_ago)
                },
                short_date(group.date)
            );
            for (i, id) in group.entries.iter().enumerate() {
                let Some(entry) = self.state.journal.entry(id) else {
                    continue;
                };
                let (title, subtitle) = row_text(entry);
                rows.push([
                    id.as_str().to_string(),
                    if i == 0 { heading.clone() } else { String::new() },
                    title,
                    subtitle,
                    tags_meta(entry),
                    star_badge(entry),
                ]);
            }
        }
        rows
    }

    /// Search hits in rank order, as rows:
    ///
    /// ```text
    ///   [key, "", title, snippet, "24 Sep 2026", badge]
    /// ```
    ///
    /// No heading: results are ranked, not grouped by day, so the day moves to
    /// `meta` and a hit still says when it was written. Short, because `meta`
    /// shares a line with the title button in a 300px pane: rendered on
    /// Compose, the long "Thursday, 24 September 2026" did not fit beside a
    /// title and was drawn over it. The snippet is the
    /// engine's, at most `SNIPPET_CHARS` characters around the first body match.
    fn search_rows(&self) -> Vec<[String; 6]> {
        search(&self.state.journal, &self.search_query, &self.filter())
            .into_iter()
            .filter_map(|hit| {
                let entry = self.state.journal.entry(&hit.entry)?;
                let (title, _) = row_text(entry);
                Some([
                    hit.entry.as_str().to_string(),
                    String::new(),
                    title,
                    hit.snippet,
                    short_date(entry.date),
                    star_badge(entry),
                ])
            })
            .collect()
    }

    /// `RecordList` rows `[key, heading, title, subtitle, meta, badge]`, newest
    /// day first, the day heading only on each day's first row.
    fn day_rows(&self) -> Vec<[String; 6]> {
        let mut rows = Vec::new();
        for day in timeline(&self.state.journal, &self.filter()) {
            for (i, id) in day.entries.iter().enumerate() {
                let Some(entry) = self.state.journal.entry(id) else {
                    continue;
                };
                let (title, subtitle) = row_text(entry);
                rows.push([
                    id.as_str().to_string(),
                    if i == 0 {
                        day_heading(day.date)
                    } else {
                        String::new()
                    },
                    title,
                    subtitle,
                    tags_meta(entry),
                    star_badge(entry),
                ]);
            }
        }
        rows
    }

    fn update(&self) -> AppUpdate {
        AppUpdate::new(self.props())
    }

    fn announced(&self, message: &str) -> AppUpdate {
        let mut update = self.update();
        update.announcements.push(Announcement {
            politeness: Politeness::Polite,
            message: message.to_string(),
        });
        update
    }

    // ── events ───────────────────────────────────────────────────────────────

    fn dispatch_inner(&mut self, event: &Event) -> Result<AppUpdate, JournalAppError> {
        let name = canonical_event_name(&event.name);
        // A draft error describes the last Save; anything else the person does
        // moves on from it (and a new Save sets or clears it again).
        if name != "onSaveEntry" {
            self.draft_error.clear();
        }
        if name != "onAddJournal" {
            self.journal_error.clear();
        }
        match name.as_ref() {
            "onSelectEntry" => {
                let index = index_payload(event, "index")?;
                let rows = self.timeline_rows();
                let key = rows.get(index).map(|r| r[0].clone()).ok_or_else(|| {
                    JournalAppError::InvalidPayload {
                        event: event.name.clone(),
                        field: "index",
                    }
                })?;
                self.target_entry(EntryId::from_raw(key));
                Ok(self.update())
            }
            // Its own event because each RecordList indexes its own rows.
            "onSelectOnThisDay" => {
                let index = index_payload(event, "index")?;
                let key = self
                    .on_this_day_rows()
                    .get(index)
                    .map(|r| r[0].clone())
                    .ok_or_else(|| invalid(event, "index"))?;
                self.target_entry(EntryId::from_raw(key));
                Ok(self.update())
            }
            "onNewEntry" => {
                self.state.target = Target::New;
                self.state.draft_title.clear();
                self.state.draft_body.clear();
                self.state.draft_tags.clear();
                self.state.draft_date.clear();
                self.state.draft_journal.clear();
                Ok(self.update())
            }
            // Drafts are capped at the engine's own entry limits: an unsaveable
            // draft would otherwise be held in memory, echoed in every update and
            // written to the state file at any size.
            "onTitleChange" => {
                let value = text_payload(event, "value")?;
                if !title_fits(&value) {
                    return Err(invalid(event, "value"));
                }
                self.state.draft_title = value;
                Ok(self.update())
            }
            "onBodyChange" => {
                let value = text_payload(event, "value")?;
                if !body_fits(&value) {
                    return Err(invalid(event, "value"));
                }
                self.state.draft_body = value;
                Ok(self.update())
            }
            "onSaveEntry" => self.save(),
            "onDeleteEntry" => {
                let Target::Entry(id) = self.state.target.clone() else {
                    return Err(JournalAppError::Engine("nothing to delete".to_string()));
                };
                self.run(Command::DeleteEntry { id })?;
                self.state.target = Target::New;
                self.state.draft_title.clear();
                self.state.draft_body.clear();
                self.state.draft_tags.clear();
                self.state.draft_date.clear();
                self.state.draft_journal.clear();
                Ok(self.announced("Entry deleted"))
            }
            // The query is capped at what the engine reads: a longer one could
            // not change the results, and would be echoed back on every
            // keystroke at any size.
            "onSearchChange" => {
                let value = text_payload(event, "value")?;
                if value.chars().count() > MAX_QUERY_CHARS {
                    return Err(invalid(event, "value"));
                }
                self.search_query = value;
                Ok(self.update())
            }
            // Stars the entry in the editor. Only `starred` changes: the draft
            // is not saved with it, so an unsaved edit stays unsaved.
            "onToggleStar" => {
                let Target::Entry(id) = self.state.target.clone() else {
                    return Err(JournalAppError::Engine("nothing to star".to_string()));
                };
                let starred = self
                    .state
                    .journal
                    .entry(&id)
                    .is_some_and(|entry| entry.starred);
                self.run(Command::SetStarred {
                    id,
                    starred: !starred,
                })?;
                Ok(self.announced(if starred { "Unstarred" } else { "Starred" }))
            }
            // Capped at what 64 tags of 64 characters, with separators, could
            // take: longer could never save, and would be echoed at any size.
            // 0 is "All journals"; i is the i-th journal in creation order.
            "onSelectJournal" => {
                let index = index_payload(event, "index")?;
                self.journal_filter = match index {
                    0 => None,
                    i => Some(
                        self.journals_ordered()
                            .get(i - 1)
                            .map(|j| j.id.as_str().to_string())
                            .ok_or_else(|| invalid(event, "index"))?,
                    ),
                };
                Ok(self.update())
            }
            "onNewJournalNameChange" => {
                let value = text_payload(event, "value")?;
                if value.chars().count() > MAX_JOURNAL_NAME_CHARS {
                    return Err(invalid(event, "value"));
                }
                self.new_journal_name = value;
                Ok(self.update())
            }
            "onAddJournal" => self.add_journal(),
            // The editor's picker: only the draft changes; Save applies it.
            "onDraftJournalChange" => {
                let index = index_payload(event, "index")?;
                let id = self
                    .journals_ordered()
                    .get(index)
                    .map(|j| j.id.as_str().to_string())
                    .ok_or_else(|| invalid(event, "index"))?;
                self.state.draft_journal = id;
                Ok(self.update())
            }
            "onDateChange" => {
                let value = text_payload(event, "value")?;
                if value.chars().count() > MAX_DATE_CHARS {
                    return Err(invalid(event, "value"));
                }
                self.state.draft_date = value;
                Ok(self.update())
            }
            "onTagsChange" => {
                let value = text_payload(event, "value")?;
                if !tags_fit(&value) {
                    return Err(invalid(event, "value"));
                }
                self.state.draft_tags = value;
                Ok(self.update())
            }
            // An option of the tag-options last rendered; the selected one
            // clears the filter.
            "onSelectTag" => {
                let index = index_payload(event, "index")?;
                let key = self
                    .all_tags()
                    .get(index)
                    .map(|count| count.tag.key().to_string())
                    .ok_or_else(|| invalid(event, "index"))?;
                self.tag_filter = if self.active_tag().is_some_and(|t| t.key() == key) {
                    None
                } else {
                    Some(key)
                };
                Ok(self.update())
            }
            "onToggleStarredFilter" => {
                self.starred_only = !self.starred_only;
                Ok(self.update())
            }
            "onClearSearch" => {
                self.search_query.clear();
                Ok(self.update())
            }
            "onCancelEdit" => {
                match self.state.target.clone() {
                    Target::New => {
                        self.state.draft_title.clear();
                        self.state.draft_body.clear();
                        self.state.draft_tags.clear();
                        self.state.draft_date.clear();
                        self.state.draft_journal.clear();
                    }
                    Target::Entry(id) => self.target_entry(id),
                }
                Ok(self.update())
            }
            _ => Err(JournalAppError::UnknownEvent(event.name.clone())),
        }
    }

    fn save(&mut self) -> Result<AppUpdate, JournalAppError> {
        let title = self.state.draft_title.clone();
        let body = self.state.draft_body.clone();
        // Tags are validated BEFORE anything is written: the entry and its
        // tags are two commands, and a bad tag must not leave an entry saved
        // without them.
        let tags = split_tags(&self.state.draft_tags);
        // Checked BEFORE anything is written, and reported in `draft-error`
        // rather than as a failed event: the journal must not change, and the
        // person typing must see why.
        let today = today((self.clock)(), self.utc_offset_minutes);
        let date = match self.state.draft_date.trim() {
            "" => None,
            typed => match Date::parse_iso(typed) {
                Some(date) => Some(date),
                None => return Ok(self.refused("Use a real date in YYYY-MM-DD format.")),
            },
        };
        if let Err((index, reason)) = normalize_tags(&tags) {
            return Ok(self.refused(&tag_error_text(index, reason)));
        }
        match self.state.target.clone() {
            Target::New => {
                if title.trim().is_empty() && body.trim().is_empty() {
                    return Ok(self.announced("Nothing to save yet"));
                }
                let id = self.mint_entry_id()?;
                self.run(Command::CreateEntry {
                    id: id.clone(),
                    // The picker's journal (J4g): where the person is looking
                    // unless they chose another; Personal from "All".
                    journal: self.draft_journal(),
                    date: date.unwrap_or(today),
                    title,
                    body,
                })?;
                self.run(Command::SetTags {
                    id: id.clone(),
                    tags,
                })?;
                self.state.target = Target::Entry(id);
            }
            Target::Entry(id) => {
                self.run(Command::EditEntry {
                    id: id.clone(),
                    title: Some(title),
                    body: Some(body),
                })?;
                // The picker always names a journal that exists, so the move
                // cannot be refused after the edit above was written.
                let journal = self.draft_journal();
                if self.state.journal.entry(&id).is_some_and(|e| e.journal != journal) {
                    self.run(Command::MoveEntry {
                        id: id.clone(),
                        journal,
                    })?;
                }
                // Blank keeps the entry's own day ("today" is for new ones).
                if let Some(date) = date {
                    if self.state.journal.entry(&id).is_some_and(|e| e.date != date) {
                        self.run(Command::SetEntryDate { id: id.clone(), date })?;
                    }
                }
                self.run(Command::SetTags { id, tags })?;
            }
        }
        // Show the tags as saved: the engine tidies and de-duplicates them, so
        // " Travel, travel " comes back as "Travel".
        if let Target::Entry(id) = &self.state.target {
            if let Some(entry) = self.state.journal.entry(id) {
                self.state.draft_tags = tags_text(entry);
                self.state.draft_date = entry.date.to_iso();
                self.state.draft_journal = entry.journal.as_str().to_string();
            }
        }
        self.draft_error.clear();
        Ok(self.announced("Entry saved"))
    }

    /// *Add journal*: create the named journal and select it. A blank name does
    /// nothing; a name the engine refuses is said in `journal-error`.
    fn add_journal(&mut self) -> Result<AppUpdate, JournalAppError> {
        let name = self.new_journal_name.trim().to_string();
        if name.is_empty() {
            return Ok(self.update());
        }
        let id = self.mint_journal_id();
        let now = (self.clock)();
        let command = Command::CreateJournal {
            id: id.clone(),
            name,
        };
        if let Err(error) = apply(&mut self.state.journal, command, now) {
            let reason = journal_error_text(&error);
            self.journal_error = reason.clone();
            return Ok(self.announced(&reason));
        }
        self.new_journal_name.clear();
        self.journal_filter = Some(id.as_str().to_string());
        Ok(self.announced("Journal added"))
    }

    /// `journal-{n}`, the first not in use. The engine caps journals at
    /// `MAX_JOURNALS`, so at most that many ids are ever tried.
    fn mint_journal_id(&self) -> JournalId {
        (1..=MAX_JOURNALS + 1)
            .map(|n| JournalId::from(format!("journal-{n}").as_str()))
            .find(|id| self.state.journal.journal(id).is_none())
            .unwrap_or_else(|| JournalId::from("journal-overflow"))
    }

    /// Save refused the draft: the journal is untouched, and the reason is
    /// shown and announced.
    fn refused(&mut self, reason: &str) -> AppUpdate {
        self.draft_error = reason.to_string();
        self.announced(reason)
    }

    fn run(&mut self, command: Command) -> Result<(), JournalAppError> {
        let now = (self.clock)();
        apply(&mut self.state.journal, command, now).map_err(engine_error)
    }

    /// Point the editor at `id` and load its saved text into the draft.
    fn target_entry(&mut self, id: EntryId) {
        match self.state.journal.entry(&id) {
            Some(entry) => {
                self.state.draft_title = entry.title.clone();
                self.state.draft_body = entry.body.clone();
                self.state.draft_tags = tags_text(entry);
                self.state.draft_date = entry.date.to_iso();
                self.state.draft_journal = entry.journal.as_str().to_string();
                self.state.target = Target::Entry(id);
            }
            None => {
                self.state.target = Target::New;
                self.state.draft_title.clear();
                self.state.draft_body.clear();
                self.state.draft_tags.clear();
                self.state.draft_date.clear();
                self.state.draft_journal.clear();
            }
        }
    }

    /// `entry-{n}`, skipping ids already in use (e.g. after a restore). Fails
    /// rather than spins if the counter is exhausted — `saturating_add` would
    /// repeat the last id forever once it reached `u64::MAX`.
    fn mint_entry_id(&mut self) -> Result<EntryId, JournalAppError> {
        loop {
            let n = self.state.next_entry;
            self.state.next_entry = n
                .checked_add(1)
                .ok_or_else(|| JournalAppError::Engine("entry ids exhausted".to_string()))?;
            let id = EntryId::from_raw(format!("entry-{n}"));
            if self.state.journal.entry(&id).is_none() {
                return Ok(id);
            }
        }
    }
}

impl MosaicApp for JournalMosaicApp {
    type Error = JournalAppError;

    fn start(&mut self, context: StartContext) -> Result<AppUpdate, Self::Error> {
        // The runtime validates the range; clamp anyway, for a caller that
        // drives the app directly.
        self.utc_offset_minutes = context
            .utc_offset_minutes
            .unwrap_or(0)
            .clamp(MIN_UTC_OFFSET_MINUTES, MAX_UTC_OFFSET_MINUTES);
        match context.restored_snapshot {
            Some(snapshot) => self.restore(snapshot),
            None => Ok(self.update()),
        }
    }

    /// Any error leaves the app exactly as it was, so the host can retry.
    ///
    /// Only the editor fields are saved for rollback, not the journal: every
    /// journal change goes through `journal_core::apply`, which validates before
    /// it writes and so is already all-or-nothing, and each event applies at most
    /// one command. Cloning the whole journal instead cost ~20 ms per keystroke
    /// at a few hundred large entries.
    fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error> {
        let before = (
            self.state.target.clone(),
            self.state.draft_title.clone(),
            self.state.draft_body.clone(),
            self.state.draft_tags.clone(),
            self.state.draft_date.clone(),
            self.state.next_entry,
            self.search_query.clone(),
            self.starred_only,
            self.tag_filter.clone(),
            self.draft_error.clone(),
            self.journal_filter.clone(),
            self.new_journal_name.clone(),
            self.journal_error.clone(),
        );
        self.dispatch_inner(&event).inspect_err(|_| {
            (
                self.state.target,
                self.state.draft_title,
                self.state.draft_body,
                self.state.draft_tags,
                self.state.draft_date,
                self.state.next_entry,
                self.search_query,
                self.starred_only,
                self.tag_filter,
                self.draft_error,
                self.journal_filter,
                self.new_journal_name,
                self.journal_error,
            ) = before;
        })
    }

    fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error> {
        let bytes =
            serde_json::to_vec(&self.state).map_err(|_| JournalAppError::InvalidSnapshot)?;
        Ok(Some(Snapshot {
            schema: SNAPSHOT_SCHEMA.to_string(),
            version: SNAPSHOT_VERSION,
            bytes,
        }))
    }

    fn restore(&mut self, snapshot: Snapshot) -> Result<AppUpdate, Self::Error> {
        if snapshot.schema != SNAPSHOT_SCHEMA || snapshot.version != SNAPSHOT_VERSION {
            return Err(JournalAppError::InvalidSnapshot);
        }
        let mut state: AppState = serde_json::from_slice(&snapshot.bytes)
            .map_err(|_| JournalAppError::InvalidSnapshot)?;
        // Deserialising checks only the shape; the engine's invariants are checked
        // here, as journal-core's spec requires of every loader.
        state
            .journal
            .validate()
            .map_err(|_| JournalAppError::InvalidSnapshot)?;
        if state
            .journal
            .journal(&JournalId::from(DEFAULT_JOURNAL))
            .is_none()
            || state.next_entry > MAX_NEXT_ENTRY
            || !title_fits(&state.draft_title)
            || !body_fits(&state.draft_body)
            || !tags_fit(&state.draft_tags)
            || state.draft_date.chars().count() > MAX_DATE_CHARS
            || state.draft_journal.len() > MAX_ID_BYTES
        {
            return Err(JournalAppError::InvalidSnapshot);
        }
        if let Target::Entry(id) = &state.target {
            if state.journal.entry(id).is_none() {
                state.target = Target::New;
            }
        }
        self.state = state;
        Ok(self.update())
    }
}

// ── helpers ─────────────────────────────────────────────────────────────────────

/// The last millisecond of 9999-12-31 (UTC). journal-core writes a date as a
/// four-digit ISO year and reads back only years 0 to 9999. An entry dated
/// later would snapshot fine and then make the WHOLE saved journal refuse to
/// restore. So no clock reading past this is ever used to date an entry.
const LAST_WRITABLE_MS: u64 = 253_402_300_799_999;

/// The system clock, natively. `SystemTime` is the platform's; a reading past
/// the last writable date (a badly set clock) reads as the epoch.
#[cfg(not(target_arch = "wasm32"))]
fn system_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
        .filter(|&ms| ms <= LAST_WRITABLE_MS)
        .unwrap_or(0)
}

// In the browser the clock is the HOST's. `wasm32-unknown-unknown` has no
// clock, and `SystemTime::now()` panics there, so the module imports one:
// `journal.now_ms() -> f64`, which the web host supplies as `Date.now`.
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "journal")]
extern "C" {
    fn now_ms() -> f64;
}

/// The host's clock, in the browser. The import crosses a trust boundary, so
/// anything that is not a writable time (NaN, infinite, negative, or past
/// 9999-12-31) reads as the epoch rather than panicking, wrapping, or dating
/// an entry the journal could never read back.
#[cfg(target_arch = "wasm32")]
fn system_now_ms() -> u64 {
    // SAFETY: the import takes no arguments and returns an f64, and a host
    // function of another signature fails at instantiation. The precondition
    // is that it RETURNS: a JS exception thrown out of it would unwind past
    // these Rust frames without running their destructors. The documented
    // host shim therefore catches everything and returns NaN (read as the
    // epoch below). A re-entrant call from it into the bridge is refused by
    // the bridge's RefCell (a trap, not aliasing).
    let ms = unsafe { now_ms() };
    clamp_host_ms(ms)
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn clamp_host_ms(ms: f64) -> u64 {
    // Exact: LAST_WRITABLE_MS < 2^53, so the bound and every value below it
    // convert between f64 and u64 without rounding.
    if ms.is_finite() && ms >= 0.0 && ms <= LAST_WRITABLE_MS as f64 {
        ms as u64
    } else {
        0
    }
}

/// The emit name every handler matches: `onSelectEntry`. Native hosts send it
/// as is. The generated React component dispatches the bare `selectEntry`,
/// which is read as `on` plus that name capitalised. Anything else passes
/// through unchanged, and is refused below as an unknown event.
fn canonical_event_name(name: &str) -> std::borrow::Cow<'_, str> {
    // Already an emit name: `on` then an upper-case letter.
    let prefixed = name
        .strip_prefix("on")
        .and_then(|rest| rest.chars().next())
        .is_some_and(|c| c.is_ascii_uppercase());
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if !prefixed && first.is_ascii_lowercase() => std::borrow::Cow::Owned(format!(
            "on{}{}",
            first.to_ascii_uppercase(),
            chars.as_str()
        )),
        _ => std::borrow::Cow::Borrowed(name),
    }
}

/// The user's today: the local date at `now_ms`, `utc_offset_minutes` east
/// of UTC. A host that gives no offset gets the UTC date.
///
/// The offset is applied BEFORE the last-writable clamp: a reading at the end
/// of 9999 (UTC) plus a positive offset would otherwise fall on 10000-01-01,
/// a date the journal could not read back (see [`LAST_WRITABLE_MS`]). A
/// negative offset near the epoch gives 1969-12-31, which the engine stores
/// and reads like any other day.
fn today(now_ms: u64, utc_offset_minutes: i32) -> Date {
    const MS_PER_MINUTE: i64 = 60_000;
    let utc = i64::try_from(now_ms.min(LAST_WRITABLE_MS)).unwrap_or(i64::MAX);
    let local = utc
        .saturating_add(i64::from(utc_offset_minutes) * MS_PER_MINUTE)
        .min(LAST_WRITABLE_MS as i64);
    let days = local.div_euclid(MS_PER_DAY as i64);
    // Within 0..=9999 (plus one day before the epoch), days fit an i32.
    Date(i32::try_from(days).unwrap_or(i32::MAX))
}

/// `Thursday, 24 September 2026`.
fn day_heading(date: Date) -> String {
    const WEEKDAYS: [&str; 7] = [
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
    ];
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    // 1970-01-01 was a Thursday; rem_euclid keeps pre-epoch days in range.
    let weekday = WEEKDAYS[date.0.rem_euclid(7) as usize];
    let (year, month, day) = date.to_ymd();
    let month = MONTHS
        .get(usize::from(month).saturating_sub(1))
        .copied()
        .unwrap_or("");
    format!("{weekday}, {day} {month} {year}")
}

/// `24 Sep 2026`: a search hit's day, short enough to share a line with its
/// title (see `search_rows`).
fn short_date(date: Date) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let (year, month, day) = date.to_ymd();
    let month = MONTHS
        .get(usize::from(month).saturating_sub(1))
        .copied()
        .unwrap_or("");
    format!("{day} {month} {year}")
}

/// A row's `(title, subtitle)`. The title is never empty — `RecordList` draws
/// it as the row's button — so an untitled entry is named by its body.
fn row_text(entry: &Entry) -> (String, String) {
    let first_line = entry
        .body
        .lines()
        .map(|line| line.trim().trim_start_matches('#').trim())
        .find(|line| !line.is_empty())
        .map(|line| truncate(line, EXCERPT_CHARS));
    let title = entry.title.trim();
    if !title.is_empty() {
        (title.to_string(), first_line.unwrap_or_default())
    } else {
        match first_line {
            Some(line) => (line, String::new()),
            None => ("Untitled entry".to_string(), String::new()),
        }
    }
}

/// The editor's text for an entry's tags: `travel, family`.
fn tags_text(entry: &Entry) -> String {
    entry
        .tags
        .iter()
        .map(|t| t.display())
        .collect::<Vec<_>>()
        .join(", ")
}

/// A row's `meta`: `#travel #family`, at most 24 characters. Short because
/// `meta` shares a line with the title button in a 300px pane.
fn tags_meta(entry: &Entry) -> String {
    let text = entry
        .tags
        .iter()
        .map(|t| format!("#{}", t.display()))
        .collect::<Vec<_>>()
        .join(" ");
    truncate(&text, 24)
}

/// The draft's tags, split on commas, blanks dropped: `" a, ,b "` → `["a", "b"]`.
fn split_tags(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

/// The longest Date field kept: generous for `YYYY-MM-DD` plus stray spaces,
/// and bounded so a pasted essay is refused as it is typed.
const MAX_DATE_CHARS: usize = 32;

/// A journal the engine refused, in words for `journal-error`.
fn journal_error_text(error: &OpError) -> String {
    match error {
        OpError::DuplicateJournalName(name) => {
            format!("A journal named \u{201c}{name}\u{201d} already exists.")
        }
        OpError::TooManyJournals => format!("Journal can keep at most {MAX_JOURNALS} journals."),
        OpError::InvalidJournalName | OpError::EmptyJournalName => format!(
            "A journal name must be one line of at most {MAX_JOURNAL_NAME_CHARS} characters."
        ),
        _ => "That journal could not be created.".to_string(),
    }
}

/// A tag error in words for `draft-error`. `index` is the 0-based position
/// among the typed tags.
fn tag_error_text(index: usize, reason: TagError) -> String {
    let n = index + 1;
    match reason {
        TagError::TooLong => format!("Tag {n} is too long ({MAX_TAG_CHARS} characters at most)."),
        TagError::ControlCharacter => format!("Tag {n} contains a line break or other control character."),
        TagError::TooMany => format!("An entry can have at most {MAX_TAGS_PER_ENTRY} tags."),
        TagError::Empty => format!("Tag {n} is empty."),
        TagError::Duplicate => format!("Tag {n} repeats another tag."),
    }
}

/// The most the Tags field can hold: 64 tags of 64 characters, each with a
/// `, ` separator. Anything longer could never be saved.
fn tags_fit(value: &str) -> bool {
    value.chars().count() <= MAX_TAGS_PER_ENTRY * (MAX_TAG_CHARS + 2)
}

/// `★` for a starred entry, else `""`.
fn star_badge(entry: &Entry) -> String {
    if entry.starred {
        "★".to_string()
    } else {
        String::new()
    }
}

/// At most `max` characters, cut on a character boundary, with `…` if cut.
fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.truncate(out.trim_end().len());
    out.push('…');
    out
}

fn title_fits(value: &str) -> bool {
    value.chars().count() <= MAX_TITLE_CHARS
}

fn body_fits(value: &str) -> bool {
    value.len() <= MAX_BODY_BYTES
}

fn invalid(event: &Event, field: &'static str) -> JournalAppError {
    JournalAppError::InvalidPayload {
        event: event.name.clone(),
        field,
    }
}

fn text_payload(event: &Event, field: &'static str) -> Result<String, JournalAppError> {
    event
        .payload
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| JournalAppError::InvalidPayload {
            event: event.name.clone(),
            field,
        })
}

fn index_payload(event: &Event, field: &'static str) -> Result<usize, JournalAppError> {
    event
        .payload
        .get(field)
        .and_then(json_index)
        .ok_or_else(|| JournalAppError::InvalidPayload {
            event: event.name.clone(),
            field,
        })
}

/// A non-negative integral JSON number that fits `usize` (hosts may send 2.0).
fn json_index(value: &Value) -> Option<usize> {
    if let Some(value) = value.as_u64() {
        return usize::try_from(value).ok();
    }
    let value = value.as_f64()?;
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value >= usize::MAX as f64 {
        return None;
    }
    Some(value as usize)
}

fn engine_error(error: OpError) -> JournalAppError {
    JournalAppError::Engine(error.to_string())
}

mosaic_app_capi::export_mosaic_app!(JournalMosaicApp, JournalMosaicApp::default());
mosaic_app_wasm::export_mosaic_wasm!(JournalMosaicApp, JournalMosaicApp::default());

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_app_runtime::Platform;
    use std::cell::Cell;

    /// Thursday, 24 September 2026 00:00 UTC, in milliseconds.
    const THU: u64 = 20_720 * MS_PER_DAY;

    thread_local! {
        static NOW: Cell<u64> = const { Cell::new(THU + 3_600_000) };
    }
    fn test_clock() -> u64 {
        NOW.with(Cell::get)
    }
    fn set_now(ms: u64) {
        NOW.with(|n| n.set(ms));
    }

    fn app() -> JournalMosaicApp {
        set_now(THU + 3_600_000);
        let mut app = JournalMosaicApp::with_clock(test_clock);
        app.start(StartContext::new("en-US", Platform::Linux))
            .unwrap();
        app
    }

    fn send(
        app: &mut JournalMosaicApp,
        name: &str,
        payload: Value,
    ) -> Result<AppUpdate, JournalAppError> {
        app.dispatch(Event::new(1, name, payload))
    }

    fn write(app: &mut JournalMosaicApp, title: &str, body: &str) {
        send(app, "onNewEntry", json!({})).unwrap();
        send(app, "onTitleChange", json!({ "value": title })).unwrap();
        send(app, "onBodyChange", json!({ "value": body })).unwrap();
        send(app, "onSaveEntry", json!({})).unwrap();
    }

    fn rows(app: &JournalMosaicApp) -> Vec<Vec<String>> {
        serde_json::from_value(app.props()["timeline-rows"].clone()).unwrap()
    }

    #[test]
    fn start_provides_every_slot_and_nothing_else() {
        let props = app().props();
        let mut keys: Vec<&str> = props
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "delete-label",
                "draft-body",
                "draft-date",
                "draft-error",
                "draft-journal-index",
                "draft-journal-options",
                "draft-tags",
                "draft-title",
                "has-journals",
                "has-on-this-day",
                "has-tags",
                "journal-error",
                "journal-options",
                "new-journal-name",
                "no-matches",
                "no-starred",
                "on-this-day-rows",
                "search-query",
                "searching",
                "selected-journal-index",
                "selected-key",
                "selected-tag-index",
                "star-label",
                "starred-only",
                "tag-options",
                "timeline-empty",
                "timeline-rows"
            ]
        );
        assert_eq!(props["timeline-empty"], true);
        assert_eq!(props["delete-label"], "");
        assert_eq!(props["searching"], false);
        assert_eq!(props["no-matches"], false);
    }

    // ── an entry's journal (J4g) ──────────────────────────────────────────────

    fn journal_of(app: &JournalMosaicApp, title: &str) -> String {
        let entry = app.state.journal.entries.values().find(|e| e.title == title).unwrap();
        entry.journal.as_str().to_string()
    }

    #[test]
    fn the_picker_lists_the_journals_and_follows_the_pane_until_chosen() {
        let mut a = app();
        let props = a.props();
        assert_eq!(props["has-journals"], false, "one journal: no picker");
        assert_eq!(props["draft-journal-options"], json!(["Personal"]));
        set_now(THU + 7_200_000);
        add_journal(&mut a, "Work");
        let props = a.props();
        assert_eq!(props["has-journals"], true);
        assert_eq!(props["draft-journal-options"], json!(["Personal", "Work"]));
        assert_eq!(props["draft-journal-index"], 1, "a new draft starts in the pane's journal");
        send(&mut a, "onSelectJournal", json!({ "index": 0 })).unwrap();
        assert_eq!(a.props()["draft-journal-index"], 0, "All journals: Personal");

        send(&mut a, "onSelectJournal", json!({ "index": 2 })).unwrap();
        send(&mut a, "onDraftJournalChange", json!({ "index": 0 })).unwrap();
        send(&mut a, "onTitleChange", json!({ "value": "Chosen" })).unwrap();
        send(&mut a, "onSaveEntry", json!({})).unwrap();
        assert_eq!(journal_of(&a, "Chosen"), DEFAULT_JOURNAL, "the choice beats the pane");
        assert!(titles(&a).is_empty(), "filed outside the Work filter");
        assert_eq!(a.props()["draft-title"], "Chosen", "but still open in the editor");
        assert!(send(&mut a, "onDraftJournalChange", json!({ "index": 2 })).is_err());
    }

    #[test]
    fn save_moves_an_entry_and_cancel_reverts_the_picker() {
        let mut a = app();
        write(&mut a, "Standup", "body");
        set_now(THU + 7_200_000);
        add_journal(&mut a, "Work");
        send(&mut a, "onSelectJournal", json!({ "index": 1 })).unwrap();
        send(&mut a, "onSelectEntry", json!({ "index": 0 })).unwrap();
        assert_eq!(a.props()["draft-journal-index"], 0, "the entry's own journal");

        send(&mut a, "onDraftJournalChange", json!({ "index": 1 })).unwrap();
        send(&mut a, "onCancelEdit", json!({})).unwrap();
        assert_eq!(a.props()["draft-journal-index"], 0);
        assert_eq!(journal_of(&a, "Standup"), DEFAULT_JOURNAL);

        send(&mut a, "onDraftJournalChange", json!({ "index": 1 })).unwrap();
        let props = send(&mut a, "onSaveEntry", json!({})).unwrap().props;
        assert_eq!(journal_of(&a, "Standup"), "journal-1");
        assert_eq!(props["draft-journal-index"], 1);
        assert!(titles(&a).is_empty(), "it left the Personal filter");
        send(&mut a, "onSelectJournal", json!({ "index": 2 })).unwrap();
        assert_eq!(titles(&a), ["Standup"]);
    }

    #[test]
    fn a_refused_save_does_not_move_the_entry() {
        let mut a = app();
        write(&mut a, "Stay", "body");
        set_now(THU + 7_200_000);
        add_journal(&mut a, "Work");
        send(&mut a, "onSelectJournal", json!({ "index": 0 })).unwrap();
        send(&mut a, "onSelectEntry", json!({ "index": 0 })).unwrap();
        send(&mut a, "onDraftJournalChange", json!({ "index": 1 })).unwrap();
        send(&mut a, "onDateChange", json!({ "value": "2026-02-30" })).unwrap();
        let props = send(&mut a, "onSaveEntry", json!({})).unwrap().props;
        assert_ne!(props["draft-error"], "");
        assert_eq!(journal_of(&a, "Stay"), DEFAULT_JOURNAL);
    }

    #[test]
    fn a_snapshot_naming_a_missing_or_overlong_journal() {
        let mut a = app();
        write(&mut a, "Kept", "body");
        let snapshot = a.snapshot().unwrap().unwrap();
        let with = |journal: &str| {
            let mut state: Value = serde_json::from_slice(&snapshot.bytes).unwrap();
            state["draftJournal"] = json!(journal);
            Snapshot { bytes: serde_json::to_vec(&state).unwrap(), ..snapshot.clone() }
        };
        let mut b = JournalMosaicApp::with_clock(test_clock);
        let mut ctx = StartContext::new("en-US", Platform::Linux);
        ctx.restored_snapshot = Some(with("journal-gone"));
        let props = b.start(ctx).unwrap().props;
        assert_eq!(props["draft-journal-index"], 0, "falls back to the entry's journal");
        send(&mut b, "onSaveEntry", json!({})).unwrap();
        assert_eq!(journal_of(&b, "Kept"), DEFAULT_JOURNAL);

        let mut c = JournalMosaicApp::with_clock(test_clock);
        let mut ctx = StartContext::new("en-US", Platform::Linux);
        ctx.restored_snapshot = Some(with(&"j".repeat(MAX_ID_BYTES + 1)));
        assert!(c.start(ctx).is_err(), "an overlong id is not ours");
    }

    // ── journals (J4f) ────────────────────────────────────────────────────────

    fn add_journal(app: &mut JournalMosaicApp, name: &str) -> Value {
        send(app, "onNewJournalNameChange", json!({ "value": name })).unwrap();
        send(app, "onAddJournal", json!({})).unwrap().props
    }

    fn journal_options(app: &JournalMosaicApp) -> Vec<String> {
        serde_json::from_value(app.props()["journal-options"].clone()).unwrap()
    }

    #[test]
    fn adding_a_journal_selects_it_and_files_new_entries_there() {
        let mut a = app();
        assert_eq!(journal_options(&a), ["All journals", "Personal"]);
        assert_eq!(a.props()["selected-journal-index"], 0);
        write(&mut a, "Home", "body");

        set_now(THU + 7_200_000);
        let props = add_journal(&mut a, "  Work ");
        assert_eq!(props["journal-error"], "");
        assert_eq!(props["new-journal-name"], "", "the field clears");
        assert_eq!(props["selected-journal-index"], 2);
        assert_eq!(journal_options(&a), ["All journals", "Personal", "Work"]);
        assert!(titles(&a).is_empty(), "the new journal starts empty");

        write(&mut a, "Standup", "body");
        assert_eq!(titles(&a), ["Standup"]);
        let entry = a.state.journal.entries.values().find(|e| e.title == "Standup").unwrap();
        assert_eq!(entry.journal.as_str(), "journal-1");

        send(&mut a, "onSelectJournal", json!({ "index": 1 })).unwrap();
        assert_eq!(titles(&a), ["Home"]);
        send(&mut a, "onSelectJournal", json!({ "index": 0 })).unwrap();
        assert_eq!(titles(&a).len(), 2);
        write(&mut a, "Unfiled", "body");
        let entry = a.state.journal.entries.values().find(|e| e.title == "Unfiled").unwrap();
        assert_eq!(entry.journal.as_str(), DEFAULT_JOURNAL, "All files into Personal");
    }

    #[test]
    fn the_journal_filter_narrows_search_and_tag_counts() {
        let mut a = app();
        write_tagged(&mut a, "Beach trip", "travel");
        add_journal(&mut a, "Work");
        write_tagged(&mut a, "Work trip", "work");
        let options: Vec<String> =
            serde_json::from_value(a.props()["tag-options"].clone()).unwrap();
        assert_eq!(options, ["#work (1)"]);
        search_for(&mut a, "trip");
        assert_eq!(titles(&a), ["Work trip"], "search stays inside the journal");
        send(&mut a, "onSelectJournal", json!({ "index": 0 })).unwrap();
        assert_eq!(titles(&a).len(), 2);
    }

    #[test]
    fn blank_and_refused_names_change_no_journals() {
        let mut a = app();
        let props = add_journal(&mut a, "   ");
        assert_eq!(props["journal-error"], "", "a blank name does nothing");
        assert_eq!(journal_options(&a).len(), 2);

        let props = add_journal(&mut a, "personal");
        assert_eq!(
            props["journal-error"],
            "A journal named \u{201c}personal\u{201d} already exists."
        );
        assert_eq!(props["new-journal-name"], "personal", "the name stays to fix");
        assert_eq!(props["selected-journal-index"], 0);
        assert_eq!(journal_options(&a).len(), 2);

        let props = send(&mut a, "onNewJournalNameChange", json!({ "value": "Pers" }))
            .unwrap()
            .props;
        assert_eq!(props["journal-error"], "", "typing clears the error");

        let long = "x".repeat(MAX_JOURNAL_NAME_CHARS + 1);
        assert!(send(&mut a, "onNewJournalNameChange", json!({ "value": long })).is_err());
        assert_eq!(a.props()["new-journal-name"], "Pers");
        let props = add_journal(&mut a, "Two\nlines");
        assert_eq!(
            props["journal-error"],
            format!(
                "A journal name must be one line of at most {MAX_JOURNAL_NAME_CHARS} characters."
            )
        );
        assert!(send(&mut a, "onSelectJournal", json!({ "index": 2 })).is_err());
    }

    #[test]
    fn journal_ids_skip_ones_in_use_and_the_selection_is_not_persisted() {
        let mut a = app();
        apply(
            &mut a.state.journal,
            Command::CreateJournal {
                id: JournalId::from("journal-1"),
                name: "Seeded".into(),
            },
            THU + 7_200_000,
        )
        .unwrap();
        set_now(THU + 10_800_000);
        add_journal(&mut a, "Travel");
        assert!(a.state.journal.journal(&JournalId::from("journal-2")).is_some());
        assert_eq!(a.props()["selected-journal-index"], 3);

        let snapshot = a.snapshot().unwrap().unwrap();
        let mut b = JournalMosaicApp::with_clock(test_clock);
        let mut ctx = StartContext::new("en-US", Platform::Linux);
        ctx.restored_snapshot = Some(snapshot);
        let props = b.start(ctx).unwrap().props;
        assert_eq!(props["selected-journal-index"], 0);
        assert_eq!(journal_options(&b), ["All journals", "Personal", "Seeded", "Travel"]);
    }

    #[test]
    fn a_selected_journal_that_disappears_stops_filtering() {
        let mut a = app();
        write(&mut a, "Home", "body");
        add_journal(&mut a, "Gone");
        assert!(titles(&a).is_empty());
        apply(
            &mut a.state.journal,
            Command::DeleteJournal {
                id: JournalId::from("journal-1"),
                move_entries_to: None,
            },
            THU,
        )
        .unwrap();
        assert_eq!(a.props()["selected-journal-index"], 0);
        assert_eq!(titles(&a), ["Home"]);
    }


    #[test]
    fn a_new_entry_can_be_filed_under_another_day() {
        let mut a = app();
        send(&mut a, "onTitleChange", json!({ "value": "About yesterday" })).unwrap();
        send(&mut a, "onDateChange", json!({ "value": " 2026-09-23 " })).unwrap();
        let props = send(&mut a, "onSaveEntry", json!({})).unwrap().props;
        assert_eq!(props["draft-error"], "");
        assert_eq!(props["draft-date"], "2026-09-23", "the field shows the saved day");
        assert_eq!(rows(&a)[0][1], "Wednesday, 23 September 2026");
    }

    #[test]
    fn a_blank_date_is_today_for_a_new_entry_and_unchanged_for_an_old_one() {
        let mut a = app();
        write(&mut a, "Today's", "body");
        assert_eq!(a.props()["draft-date"], "2026-09-24");
        send(&mut a, "onDateChange", json!({ "value": "" })).unwrap();
        send(&mut a, "onTitleChange", json!({ "value": "Still today's" })).unwrap();
        send(&mut a, "onSaveEntry", json!({})).unwrap();
        assert_eq!(rows(&a)[0][1], "Thursday, 24 September 2026");
        assert_eq!(a.props()["draft-date"], "2026-09-24");
    }

    #[test]
    fn moving_an_entry_refiles_it_and_cancel_reverts_the_field() {
        let mut a = app();
        write(&mut a, "Movable", "body");
        send(&mut a, "onDateChange", json!({ "value": "2020-02-29" })).unwrap();
        let props = send(&mut a, "onCancelEdit", json!({})).unwrap().props;
        assert_eq!(props["draft-date"], "2026-09-24", "Cancel reverts the day");

        send(&mut a, "onDateChange", json!({ "value": "2020-02-29" })).unwrap();
        send(&mut a, "onSaveEntry", json!({})).unwrap();
        assert_eq!(rows(&a)[0][1], "Saturday, 29 February 2020");
        let props = send(&mut a, "onNewEntry", json!({})).unwrap().props;
        assert_eq!(props["draft-date"], "", "a new draft starts on today (blank)");
    }

    #[test]
    fn a_bad_date_is_shown_and_writes_nothing() {
        let mut a = app();
        send(&mut a, "onTitleChange", json!({ "value": "Draft" })).unwrap();
        for bad in ["2026-02-30", "24/09/2026", "10000-01-01", "yesterday"] {
            send(&mut a, "onDateChange", json!({ "value": bad })).unwrap();
            let props = send(&mut a, "onSaveEntry", json!({})).unwrap().props;
            assert_eq!(props["draft-error"], "Use a real date in YYYY-MM-DD format.", "{bad}");
            assert_eq!(props["timeline-empty"], true, "{bad}");
        }
        let props = send(&mut a, "onDateChange", json!({ "value": "2026-09-01" }))
            .unwrap()
            .props;
        assert_eq!(props["draft-error"], "", "typing again clears the message");
        let long = "9".repeat(MAX_DATE_CHARS + 1);
        assert!(send(&mut a, "onDateChange", json!({ "value": long })).is_err());
    }

    // ── tags (J4d) ────────────────────────────────────────────────────────────

    fn write_tagged(app: &mut JournalMosaicApp, title: &str, tags: &str) {
        send(app, "onNewEntry", json!({})).unwrap();
        send(app, "onTitleChange", json!({ "value": title })).unwrap();
        send(app, "onTagsChange", json!({ "value": tags })).unwrap();
        send(app, "onSaveEntry", json!({})).unwrap();
    }

    fn titles(app: &JournalMosaicApp) -> Vec<String> {
        rows(app).iter().map(|r| r[2].clone()).collect()
    }

    #[test]
    fn tags_are_saved_with_the_entry_and_shown_as_row_meta() {
        let mut a = app();
        write_tagged(&mut a, "Trip", " Travel, family ,, travel ");
        let props = a.props();
        assert_eq!(rows(&a)[0][4], "#Travel #family", "tidied and deduplicated");
        assert_eq!(props["draft-tags"], "Travel, family", "the editor shows what was saved");
        assert_eq!(props["has-tags"], true);
        assert_eq!(props["tag-options"], json!(["#family (1)", "#Travel (1)"]));
        assert_eq!(props["selected-tag-index"], -1);
    }

    #[test]
    fn tags_belong_to_the_draft_cancel_reverts_them() {
        let mut a = app();
        write_tagged(&mut a, "Trip", "travel");
        send(&mut a, "onTagsChange", json!({ "value": "work" })).unwrap();
        assert_eq!(rows(&a)[0][4], "#travel", "unsaved tags change nothing yet");
        let props = send(&mut a, "onCancelEdit", json!({})).unwrap().props;
        assert_eq!(props["draft-tags"], "travel");
        let props = send(&mut a, "onNewEntry", json!({})).unwrap().props;
        assert_eq!(props["draft-tags"], "");
    }

    #[test]
    fn a_bad_tag_fails_the_whole_save_and_writes_nothing() {
        let mut a = app();
        send(&mut a, "onTitleChange", json!({ "value": "Never saved" })).unwrap();
        let long = "x".repeat(MAX_TAG_CHARS + 1);
        send(&mut a, "onTagsChange", json!({ "value": format!("ok, {long}") })).unwrap();
        // Refused in words (J4e), not as a failed event the person never sees.
        let props = send(&mut a, "onSaveEntry", json!({})).unwrap().props;
        assert_eq!(props["draft-error"], "Tag 2 is too long (64 characters at most).");
        assert_eq!(props["timeline-empty"], true, "no entry without its tags");
        assert_eq!(props["draft-title"], "Never saved", "the draft is kept to fix");

        write_tagged(&mut a, "Saved", "fine");
        send(&mut a, "onTagsChange", json!({ "value": "bad\ntag" })).unwrap();
        send(&mut a, "onTitleChange", json!({ "value": "Renamed" })).unwrap();
        let props = send(&mut a, "onSaveEntry", json!({})).unwrap().props;
        assert_eq!(props["draft-error"], "Tag 1 contains a line break or other control character.");
        assert_eq!(titles(&a), ["Saved"], "an edit with a bad tag writes nothing either");
    }

    #[test]
    fn an_overlong_tags_field_is_refused_as_typed() {
        let mut a = app();
        let long = "x".repeat(MAX_TAGS_PER_ENTRY * (MAX_TAG_CHARS + 2) + 1);
        assert!(send(&mut a, "onTagsChange", json!({ "value": long })).is_err());
        assert_eq!(a.props()["draft-tags"], "");
    }

    #[test]
    fn selecting_a_tag_filters_every_list_and_again_clears_it() {
        let mut a = app();
        write_tagged(&mut a, "Beach", "travel");
        write_tagged(&mut a, "Office", "work");
        write_tagged(&mut a, "Airport", "travel, work");
        let options: Vec<String> =
            serde_json::from_value(a.props()["tag-options"].clone()).unwrap();
        assert_eq!(options, ["#travel (2)", "#work (2)"]);

        let props = send(&mut a, "onSelectTag", json!({ "index": 0 })).unwrap().props;
        assert_eq!(props["selected-tag-index"], 0);
        let mut got = titles(&a);
        got.sort();
        assert_eq!(got, ["Airport", "Beach"]);
        let options: Vec<String> =
            serde_json::from_value(a.props()["tag-options"].clone()).unwrap();
        assert_eq!(options.len(), 2, "options count every entry, not the filtered list");

        send(&mut a, "onSearchChange", json!({ "value": "office" })).unwrap();
        assert_eq!(a.props()["no-matches"], true, "search honours the tag filter");
        send(&mut a, "onClearSearch", json!({})).unwrap();

        let props = send(&mut a, "onSelectTag", json!({ "index": 0 })).unwrap().props;
        assert_eq!(props["selected-tag-index"], -1);
        assert_eq!(rows(&a).len(), 3);
        assert!(send(&mut a, "onSelectTag", json!({ "index": 9 })).is_err());
    }

    #[test]
    fn a_selected_tag_that_disappears_stops_filtering() {
        let mut a = app();
        write_tagged(&mut a, "Only", "rare");
        write_tagged(&mut a, "Other", "");
        send(&mut a, "onSelectTag", json!({ "index": 0 })).unwrap();
        assert_eq!(titles(&a), ["Only"]);
        send(&mut a, "onSelectEntry", json!({ "index": 0 })).unwrap();
        send(&mut a, "onTagsChange", json!({ "value": "" })).unwrap();
        let props = send(&mut a, "onSaveEntry", json!({})).unwrap().props;
        assert_eq!(props["has-tags"], false);
        assert_eq!(props["selected-tag-index"], -1);
        assert_eq!(rows(&a).len(), 2, "the vanished tag no longer filters");
    }

    #[test]
    fn draft_tags_persist_but_the_tag_filter_does_not() {
        let mut a = app();
        write_tagged(&mut a, "Trip", "travel");
        send(&mut a, "onSelectTag", json!({ "index": 0 })).unwrap();
        send(&mut a, "onTagsChange", json!({ "value": "travel, unsaved" })).unwrap();
        let snapshot = a.snapshot().unwrap().unwrap();
        let mut b = JournalMosaicApp::with_clock(test_clock);
        let mut ctx = StartContext::new("en-US", Platform::Linux);
        ctx.restored_snapshot = Some(snapshot);
        let props = b.start(ctx).unwrap().props;
        assert_eq!(props["draft-tags"], "travel, unsaved");
        assert_eq!(props["selected-tag-index"], -1);
    }

    #[test]
    fn a_snapshot_written_before_tags_still_loads() {
        let mut a = app();
        write(&mut a, "Old", "body");
        let snapshot = a.snapshot().unwrap().unwrap();
        let mut state: Value = serde_json::from_slice(&snapshot.bytes).unwrap();
        state.as_object_mut().unwrap().remove("draftTags");
        state.as_object_mut().unwrap().remove("draftJournal");
        let old = Snapshot { bytes: serde_json::to_vec(&state).unwrap(), ..snapshot };
        let mut b = JournalMosaicApp::with_clock(test_clock);
        let mut ctx = StartContext::new("en-US", Platform::Linux);
        ctx.restored_snapshot = Some(old);
        let props = b.start(ctx).unwrap().props;
        assert_eq!(props["draft-tags"], "");
        assert_eq!(props["draft-journal-index"], 0);
        assert_eq!(rows(&b).len(), 1);
    }

    // ── on this day (J4c) ─────────────────────────────────────────────────────

    const YEAR_MS: u64 = 365 * MS_PER_DAY;

    fn on_this_day_rows_of(app: &JournalMosaicApp) -> Vec<Vec<String>> {
        serde_json::from_value(app.props()["on-this-day-rows"].clone()).unwrap()
    }

    #[test]
    fn earlier_years_on_this_date_are_recalled_above_the_timeline() {
        let mut a = app();
        assert_eq!(a.props()["has-on-this-day"], false);
        // 24 Sep 2025 and 24 Sep 2023 (2024 is a leap year: 366 days back).
        set_now(THU + 3_600_000 - YEAR_MS);
        write(&mut a, "A year ago", "the old harbour");
        set_now(THU + 3_600_000 - 3 * YEAR_MS - MS_PER_DAY);
        write(&mut a, "Three years ago", "first entry");
        set_now(THU + 3_600_000);
        write(&mut a, "Today", "not recalled: this year");

        let props = a.props();
        assert_eq!(props["has-on-this-day"], true);
        let recalled = on_this_day_rows_of(&a);
        let titles: Vec<&str> = recalled.iter().map(|r| r[2].as_str()).collect();
        assert_eq!(titles, ["A year ago", "Three years ago"], "most recent year first");
        assert_eq!(recalled[0][1], "1 year ago · 24 Sep 2025");
        assert_eq!(recalled[1][1], "3 years ago · 24 Sep 2023");
        assert_eq!(recalled[0][4], "", "the heading already says when");
    }

    #[test]
    fn selecting_a_recalled_entry_opens_it_and_bad_indexes_change_nothing() {
        let mut a = app();
        set_now(THU + 3_600_000 - YEAR_MS);
        write(&mut a, "Last year", "body");
        set_now(THU + 3_600_000);
        send(&mut a, "onNewEntry", json!({})).unwrap();
        let props = send(&mut a, "onSelectOnThisDay", json!({ "index": 0 }))
            .unwrap()
            .props;
        assert_eq!(props["draft-title"], "Last year");
        assert!(send(&mut a, "onSelectOnThisDay", json!({ "index": 1 })).is_err());
        assert!(send(&mut a, "onSelectOnThisDay", json!({ "index": -1 })).is_err());
        assert_eq!(a.props()["draft-title"], "Last year");
    }

    #[test]
    fn the_recall_hides_during_a_search_and_follows_the_starred_filter() {
        let mut a = app();
        set_now(THU + 3_600_000 - YEAR_MS);
        write(&mut a, "Last year", "harbour");
        set_now(THU + 3_600_000);
        send(&mut a, "onSearchChange", json!({ "value": "harbour" })).unwrap();
        assert_eq!(a.props()["has-on-this-day"], false);
        send(&mut a, "onClearSearch", json!({})).unwrap();
        send(&mut a, "onToggleStarredFilter", json!({})).unwrap();
        assert_eq!(a.props()["has-on-this-day"], false, "nothing recalled is starred");
    }

    #[test]
    fn the_recall_follows_the_local_day() {
        // 23:30 on 23 Sep in New York is 03:30 on 24 Sep UTC.
        let mut a = JournalMosaicApp::with_clock(test_clock);
        let mut ctx = StartContext::new("en-US", Platform::Linux);
        ctx.utc_offset_minutes = Some(-240);
        set_now(THU - YEAR_MS - MS_PER_DAY + 23 * 3_600_000);
        a.start(ctx).unwrap();
        write(&mut a, "23 Sep, local", "body");
        // 03:30 on 24 Sep UTC is still 23:30 on 23 Sep in New York, so the
        // entry filed on 23 Sep 2025 is recalled...
        set_now(THU + 3 * 3_600_000 + 1_800_000);
        assert_eq!(a.props()["has-on-this-day"], true);
        // ...but a host that gives no offset is on 24 Sep (UTC) already.
        let snapshot = a.snapshot().unwrap().unwrap();
        let mut utc = JournalMosaicApp::with_clock(test_clock);
        let mut ctx = StartContext::new("en-US", Platform::Linux);
        ctx.restored_snapshot = Some(snapshot);
        utc.start(ctx).unwrap();
        assert_eq!(utc.props()["has-on-this-day"], false);
    }

    // ── stars (J4b) ───────────────────────────────────────────────────────────

    #[test]
    fn star_toggles_the_open_entry_without_saving_the_draft() {
        let mut a = app();
        assert_eq!(a.props()["star-label"], "", "a new draft has nothing to star");
        assert!(send(&mut a, "onToggleStar", json!({})).is_err());

        write(&mut a, "Kept", "body");
        assert_eq!(a.props()["star-label"], "Star");
        send(&mut a, "onTitleChange", json!({ "value": "Unsaved edit" })).unwrap();
        let props = send(&mut a, "onToggleStar", json!({})).unwrap().props;
        assert_eq!(props["star-label"], "Unstar");
        assert_eq!(rows(&a)[0][5], "★");
        assert_eq!(rows(&a)[0][2], "Kept", "the draft was not saved by starring");
        assert_eq!(props["draft-title"], "Unsaved edit", "and it is still in the editor");

        let props = send(&mut a, "onToggleStar", json!({})).unwrap().props;
        assert_eq!(props["star-label"], "Star");
        assert_eq!(rows(&a)[0][5], "");
    }

    #[test]
    fn starred_only_narrows_the_timeline_and_search_alike() {
        let mut a = app();
        write(&mut a, "Harbour, starred", "fog");
        send(&mut a, "onToggleStar", json!({})).unwrap();
        write(&mut a, "Harbour, plain", "fog");

        let props = send(&mut a, "onToggleStarredFilter", json!({})).unwrap().props;
        assert_eq!(props["starred-only"], true);
        let titles: Vec<String> = rows(&a).iter().map(|r| r[2].clone()).collect();
        assert_eq!(titles, ["Harbour, starred"]);

        send(&mut a, "onSearchChange", json!({ "value": "harbour" })).unwrap();
        let titles: Vec<String> = rows(&a).iter().map(|r| r[2].clone()).collect();
        assert_eq!(titles, ["Harbour, starred"], "search honours the filter");

        send(&mut a, "onClearSearch", json!({})).unwrap();
        let props = send(&mut a, "onToggleStarredFilter", json!({})).unwrap().props;
        assert_eq!(props["starred-only"], false);
        assert_eq!(rows(&a).len(), 2);
    }

    #[test]
    fn the_empty_states_say_which_list_is_empty() {
        let mut a = app();
        let props = send(&mut a, "onToggleStarredFilter", json!({})).unwrap().props;
        assert_eq!(props["timeline-empty"], true);
        assert_eq!(props["no-starred"], false, "an empty journal is timeline-empty");

        write(&mut a, "Plain", "fog");
        let props = a.props();
        assert_eq!(props["no-starred"], true);
        assert_eq!(props["no-matches"], false);

        let props = send(&mut a, "onSearchChange", json!({ "value": "fog" }))
            .unwrap()
            .props;
        assert_eq!(props["no-starred"], false, "a search owns its own empty state");
        assert_eq!(props["no-matches"], true);
    }

    #[test]
    fn unstarring_under_the_filter_keeps_the_entry_in_the_editor() {
        let mut a = app();
        write(&mut a, "Only", "body");
        send(&mut a, "onToggleStar", json!({})).unwrap();
        send(&mut a, "onToggleStarredFilter", json!({})).unwrap();
        let props = send(&mut a, "onToggleStar", json!({})).unwrap().props;
        assert_eq!(props["no-starred"], true);
        assert_eq!(props["draft-title"], "Only");
        assert_eq!(props["star-label"], "Star");
    }

    #[test]
    fn stars_persist_but_the_filter_does_not() {
        let mut a = app();
        write(&mut a, "Kept", "body");
        send(&mut a, "onToggleStar", json!({})).unwrap();
        send(&mut a, "onToggleStarredFilter", json!({})).unwrap();
        let snapshot = a.snapshot().unwrap().unwrap();
        let mut b = JournalMosaicApp::with_clock(test_clock);
        let mut ctx = StartContext::new("en-US", Platform::Linux);
        ctx.restored_snapshot = Some(snapshot);
        let props = b.start(ctx).unwrap().props;
        assert_eq!(props["starred-only"], false);
        assert_eq!(rows(&b)[0][5], "★");
    }

    // ── search (J4a) ──────────────────────────────────────────────────────────

    fn search_for(app: &mut JournalMosaicApp, query: &str) -> Value {
        send(app, "onSearchChange", json!({ "value": query }))
            .unwrap()
            .props
    }

    #[test]
    fn a_query_turns_the_timeline_into_ranked_results() {
        let mut a = app();
        write(&mut a, "Harbour walk", "Fog over the water, then sun by noon.");
        write(&mut a, "Groceries", "Bread, and a walk to the harbour after.");
        write(&mut a, "Unrelated", "Nothing to see here.");

        let props = search_for(&mut a, "harbour");
        assert_eq!(props["searching"], true);
        assert_eq!(props["no-matches"], false);
        assert_eq!(props["timeline-empty"], false);
        let hits: Vec<Vec<String>> = serde_json::from_value(props["timeline-rows"].clone()).unwrap();
        // Title hit outranks the body hit; the unrelated entry is gone.
        let titles: Vec<&str> = hits.iter().map(|r| r[2].as_str()).collect();
        assert_eq!(titles, ["Harbour walk", "Groceries"]);
        for row in &hits {
            assert_eq!(row[1], "", "results are ranked, not grouped: no heading");
            assert_eq!(row[4], "24 Sep 2026", "the day moves to meta, short");
        }
        assert!(hits[1][3].contains("harbour"), "the snippet shows the body match");
    }

    #[test]
    fn every_term_must_match_and_blank_is_not_a_search() {
        let mut a = app();
        write(&mut a, "Harbour walk", "Fog over the water.");
        let props = search_for(&mut a, "harbour sunshine");
        assert_eq!(props["searching"], true);
        assert_eq!(props["no-matches"], true);
        assert_eq!(props["timeline-empty"], false, "the journal is not empty");
        assert_eq!(props["timeline-rows"], json!([]));

        let props = search_for(&mut a, "   ");
        assert_eq!(props["searching"], false);
        assert_eq!(props["no-matches"], false);
        assert_eq!(rows(&a)[0][1], "Thursday, 24 September 2026", "the timeline is back");
    }

    #[test]
    fn selecting_a_result_opens_that_entry_and_clear_restores_the_timeline() {
        let mut a = app();
        write(&mut a, "First", "alpha");
        write(&mut a, "Second", "beta");
        write(&mut a, "Third", "alpha again");
        search_for(&mut a, "alpha");
        let hits = rows(&a);
        let target = hits.iter().position(|r| r[2] == "First").unwrap();
        let props = send(&mut a, "onSelectEntry", json!({ "index": target }))
            .unwrap()
            .props;
        assert_eq!(props["draft-title"], "First");
        assert_eq!(props["search-query"], "alpha", "selecting keeps the query");

        let props = send(&mut a, "onClearSearch", json!({})).unwrap().props;
        assert_eq!(props["search-query"], "");
        assert_eq!(props["searching"], false);
        assert_eq!(rows(&a).len(), 3);
    }

    #[test]
    fn an_edit_that_stops_matching_drops_out_of_the_results() {
        let mut a = app();
        write(&mut a, "Harbour", "fog");
        search_for(&mut a, "harbour");
        assert_eq!(rows(&a).len(), 1);
        send(&mut a, "onSelectEntry", json!({ "index": 0 })).unwrap();
        send(&mut a, "onTitleChange", json!({ "value": "Hill" })).unwrap();
        let props = send(&mut a, "onSaveEntry", json!({})).unwrap().props;
        assert_eq!(props["no-matches"], true);
    }

    #[test]
    fn an_overlong_query_is_refused_and_changes_nothing() {
        let mut a = app();
        search_for(&mut a, "kept");
        let long = "x".repeat(MAX_QUERY_CHARS + 1);
        let err = send(&mut a, "onSearchChange", json!({ "value": long })).unwrap_err();
        assert!(matches!(err, JournalAppError::InvalidPayload { field: "value", .. }));
        assert_eq!(a.props()["search-query"], "kept");
        // Exactly the engine's limit is fine.
        search_for(&mut a, &"y".repeat(MAX_QUERY_CHARS));
        assert!(send(&mut a, "onSearchChange", json!({ "value": 7 })).is_err());
    }

    #[test]
    fn the_query_is_not_persisted() {
        let mut a = app();
        write(&mut a, "Harbour", "fog");
        search_for(&mut a, "harbour");
        let snapshot = a.snapshot().unwrap().unwrap();
        assert!(
            !String::from_utf8_lossy(&snapshot.bytes).contains("harbour\""),
            "the query must not be in the snapshot"
        );
        let mut b = JournalMosaicApp::with_clock(test_clock);
        let mut ctx = StartContext::new("en-US", Platform::Linux);
        ctx.restored_snapshot = Some(snapshot);
        let props = b.start(ctx).unwrap().props;
        assert_eq!(props["search-query"], "");
        assert_eq!(props["searching"], false);
        assert_eq!(rows(&b).len(), 1);
    }

    #[test]
    fn writing_and_saving_an_entry_files_it_under_today() {
        let mut a = app();
        send(&mut a, "onTitleChange", json!({ "value": "First light" })).unwrap();
        send(
            &mut a,
            "onBodyChange",
            json!({ "value": "# Dawn\nWalked to the lighthouse." }),
        )
        .unwrap();
        assert!(rows(&a).is_empty(), "typing alone does not save");
        let update = send(&mut a, "onSaveEntry", json!({})).unwrap();
        assert_eq!(update.announcements[0].message, "Entry saved");
        let r = rows(&a);
        assert_eq!(
            r,
            vec![vec![
                "entry-1".to_string(),
                "Thursday, 24 September 2026".to_string(),
                "First light".to_string(),
                "Dawn".to_string(),
                String::new(),
                String::new(),
            ]]
        );
        let props = a.props();
        assert_eq!(props["selected-key"], "entry-1");
        assert_eq!(props["delete-label"], "Delete");
        assert_eq!(props["timeline-empty"], false);
    }

    #[test]
    fn an_empty_new_draft_is_not_saved() {
        let mut a = app();
        let update = send(&mut a, "onSaveEntry", json!({})).unwrap();
        assert_eq!(update.announcements[0].message, "Nothing to save yet");
        assert!(rows(&a).is_empty());
    }

    #[test]
    fn select_edit_save_and_cancel() {
        let mut a = app();
        write(&mut a, "One", "first");
        write(&mut a, "Two", "second");
        // Newest first: entry-2 was written later the same day.
        set_now(THU + 7_200_000);
        let r = rows(&a);
        assert_eq!(r.len(), 2);
        let one = r.iter().position(|row| row[0] == "entry-1").unwrap();
        send(&mut a, "onSelectEntry", json!({ "index": one })).unwrap();
        assert_eq!(a.props()["draft-title"], "One");

        send(&mut a, "onTitleChange", json!({ "value": "One, revised" })).unwrap();
        send(&mut a, "onCancelEdit", json!({})).unwrap();
        assert_eq!(a.props()["draft-title"], "One", "cancel discards the edit");

        send(&mut a, "onTitleChange", json!({ "value": "One, revised" })).unwrap();
        send(&mut a, "onSaveEntry", json!({})).unwrap();
        assert!(rows(&a).iter().any(|row| row[2] == "One, revised"));
        assert_eq!(
            a.state.journal.entries.len(),
            2,
            "editing does not create an entry"
        );
    }

    #[test]
    fn deleting_returns_to_a_new_draft() {
        let mut a = app();
        write(&mut a, "Gone soon", "");
        let update = send(&mut a, "onDeleteEntry", json!({})).unwrap();
        assert_eq!(update.announcements[0].message, "Entry deleted");
        assert!(rows(&a).is_empty());
        assert_eq!(a.props()["selected-key"], "");
        assert_eq!(a.props()["draft-title"], "");
        // Nothing to delete for a new draft: refused, state unchanged.
        assert!(send(&mut a, "onDeleteEntry", json!({})).is_err());
    }

    #[test]
    fn day_headings_group_entries_newest_day_first() {
        let mut a = app();
        write(&mut a, "Wednesday entry", "");
        set_now(THU + MS_PER_DAY + 60_000);
        write(&mut a, "Friday morning", "");
        set_now(THU + MS_PER_DAY + 120_000);
        write(&mut a, "Friday noon", "");
        let headings: Vec<(String, String)> = rows(&a)
            .into_iter()
            .map(|r| (r[1].clone(), r[2].clone()))
            .collect();
        assert_eq!(
            headings,
            [
                (
                    "Friday, 25 September 2026".to_string(),
                    "Friday noon".to_string()
                ),
                (String::new(), "Friday morning".to_string()),
                (
                    "Thursday, 24 September 2026".to_string(),
                    "Wednesday entry".to_string()
                ),
            ]
        );
    }

    #[test]
    fn an_untitled_entry_is_still_named() {
        let mut a = app();
        write(&mut a, "  ", "\n\n## Rain all day\nStayed in.");
        // Saved (not blank) but nothing to show: a lone markdown marker.
        write(&mut a, "", "#");
        let titles: Vec<(String, String)> = rows(&a)
            .into_iter()
            .map(|r| (r[2].clone(), r[3].clone()))
            .collect();
        assert!(titles.contains(&("Rain all day".to_string(), String::new())));
        assert!(titles.contains(&("Untitled entry".to_string(), String::new())));
    }

    #[test]
    fn long_excerpts_are_cut_on_character_boundaries() {
        let long = "é".repeat(150);
        let cut = truncate(&long, EXCERPT_CHARS);
        assert_eq!(cut.chars().count(), EXCERPT_CHARS);
        assert!(cut.ends_with('…'));
        assert_eq!(truncate("short", EXCERPT_CHARS), "short");
    }

    #[test]
    fn bad_events_are_errors_that_change_nothing() {
        let mut a = app();
        write(&mut a, "Kept", "body");
        let before = a.props();
        for (name, payload) in [
            ("onSelectEntry", json!({ "index": 9 })),
            ("onSelectEntry", json!({ "index": -1 })),
            ("onSelectEntry", json!({})),
            ("onTitleChange", json!({ "value": 3 })),
            ("onFly", json!({})),
        ] {
            assert!(send(&mut a, name, payload).is_err(), "{name}");
            assert_eq!(a.props(), before, "{name} left no trace");
        }
        assert_eq!(
            send(&mut a, "onFly", json!({})).unwrap_err(),
            JournalAppError::UnknownEvent("onFly".to_string())
        );
        // A float index that is integral is accepted, as hosts may send one.
        send(&mut a, "onSelectEntry", json!({ "index": 0.0 })).unwrap();
    }

    #[test]
    fn an_engine_rejection_rolls_the_whole_event_back() {
        // The engine refuses to edit an entry that is gone (a stale target the
        // adapter did not repair); the event must leave no trace.
        let mut a = app();
        a.state.target = Target::Entry(EntryId::from_raw("entry-9"));
        send(&mut a, "onTitleChange", json!({ "value": "orphaned edit" })).unwrap();
        let before = a.state.clone();
        let err = send(&mut a, "onSaveEntry", json!({})).unwrap_err();
        assert!(matches!(err, JournalAppError::Engine(_)), "{err:?}");
        assert_eq!(a.state, before);
    }

    #[test]
    fn a_snapshot_round_trips_including_an_unsaved_draft() {
        let mut a = app();
        write(&mut a, "Saved", "body");
        send(&mut a, "onNewEntry", json!({})).unwrap();
        send(&mut a, "onBodyChange", json!({ "value": "half a thought" })).unwrap();
        let snap = a.snapshot().unwrap().unwrap();

        let mut b = JournalMosaicApp::with_clock(test_clock);
        let mut ctx = StartContext::new("en-US", Platform::Linux);
        ctx.restored_snapshot = Some(snap);
        b.start(ctx).unwrap();
        assert_eq!(b.props(), a.props());
        assert_eq!(b.props()["draft-body"], "half a thought");
    }

    #[test]
    fn foreign_corrupt_or_invalid_snapshots_are_refused() {
        let good = app().snapshot().unwrap().unwrap();
        let mut a = app();
        let variants = [
            Snapshot {
                schema: "other/state".into(),
                ..good.clone()
            },
            Snapshot {
                version: 2,
                ..good.clone()
            },
            Snapshot {
                bytes: b"{not json".to_vec(),
                ..good.clone()
            },
        ];
        for snap in variants {
            assert_eq!(a.restore(snap), Err(JournalAppError::InvalidSnapshot));
        }
        // Well-formed but invalid: an entry pointing at a missing journal.
        let mut v: Value = serde_json::from_slice(&good.bytes).unwrap();
        v["journal"]["entries"] = json!({ "e": {
            "id": "e", "journal": "ghost", "title": "", "body": "",
            "date": "2026-01-01", "createdAtMs": 0, "updatedAtMs": 0 } });
        let bad = Snapshot {
            bytes: serde_json::to_vec(&v).unwrap(),
            ..good.clone()
        };
        assert_eq!(a.restore(bad), Err(JournalAppError::InvalidSnapshot));
    }

    #[test]
    fn a_target_naming_a_missing_entry_is_repaired_on_restore() {
        let a = app();
        let good = a.snapshot().unwrap().unwrap();
        let mut v: Value = serde_json::from_slice(&good.bytes).unwrap();
        v["target"] = json!({ "kind": "entry", "id": "entry-99" });
        let snap = Snapshot {
            bytes: serde_json::to_vec(&v).unwrap(),
            ..good
        };
        let mut b = JournalMosaicApp::with_clock(test_clock);
        b.restore(snap).unwrap();
        assert_eq!(b.props()["selected-key"], "");
    }

    #[test]
    fn an_exhausted_counter_is_an_error_not_a_hang() {
        // Security review: next_entry = u64::MAX made saturating_add repeat one id
        // forever. Minting now fails cleanly, and restore refuses such a counter.
        let mut a = app();
        a.state.next_entry = u64::MAX;
        send(&mut a, "onTitleChange", json!({ "value": "x" })).unwrap();
        let before = a.state.clone();
        assert!(send(&mut a, "onSaveEntry", json!({})).is_err());
        assert_eq!(a.state, before);

        let good = app().snapshot().unwrap().unwrap();
        let mut v: Value = serde_json::from_slice(&good.bytes).unwrap();
        v["nextEntry"] = json!(u64::MAX);
        let bad = Snapshot {
            bytes: serde_json::to_vec(&v).unwrap(),
            ..good
        };
        assert_eq!(app().restore(bad), Err(JournalAppError::InvalidSnapshot));
    }

    #[test]
    fn drafts_are_capped_at_the_engines_limits() {
        let mut a = app();
        let err = send(
            &mut a,
            "onBodyChange",
            json!({ "value": "b".repeat(MAX_BODY_BYTES + 1) }),
        );
        assert!(err.is_err());
        assert_eq!(a.props()["draft-body"], "");
        let err = send(
            &mut a,
            "onTitleChange",
            json!({ "value": "t".repeat(MAX_TITLE_CHARS + 1) }),
        );
        assert!(err.is_err());
        send(
            &mut a,
            "onTitleChange",
            json!({ "value": "t".repeat(MAX_TITLE_CHARS) }),
        )
        .unwrap();

        let good = app().snapshot().unwrap().unwrap();
        let mut v: Value = serde_json::from_slice(&good.bytes).unwrap();
        v["draftBody"] = json!("b".repeat(MAX_BODY_BYTES + 1));
        let bad = Snapshot {
            bytes: serde_json::to_vec(&v).unwrap(),
            ..good
        };
        assert_eq!(app().restore(bad), Err(JournalAppError::InvalidSnapshot));
    }

    #[test]
    fn minted_ids_skip_ones_already_in_use() {
        let mut a = app();
        write(&mut a, "one", "");
        a.state.next_entry = 1; // as if restored from an older counter
        write(&mut a, "two", "");
        let ids: Vec<&str> = a
            .state
            .journal
            .entries
            .keys()
            .map(EntryId::as_str)
            .collect();
        assert_eq!(ids, ["entry-1", "entry-2"]);
    }

    #[test]
    fn today_is_the_local_date_and_headings_handle_pre_epoch_days() {
        // No offset: the UTC date.
        assert_eq!(today(THU, 0).to_iso(), "2026-09-24");
        assert_eq!(today(THU + MS_PER_DAY - 1, 0).to_iso(), "2026-09-24");
        // 01:00 UTC on the 24th is still the 23rd in New York (UTC-5) ...
        assert_eq!(today(THU + 3_600_000, -300).to_iso(), "2026-09-23");
        // ... and 23:00 UTC on the 24th is already the 25th in India (UTC+5:30).
        assert_eq!(today(THU + 23 * 3_600_000, 330).to_iso(), "2026-09-25");
        // The extremes, either side of midnight UTC.
        assert_eq!(today(THU, MIN_UTC_OFFSET_MINUTES).to_iso(), "2026-09-23");
        assert_eq!(
            today(THU - 1, MAX_UTC_OFFSET_MINUTES).to_iso(),
            "2026-09-24"
        );
        // Near the epoch, west of Greenwich: the day before it.
        assert_eq!(today(0, -300), Date(-1));
        // Never past the last day the journal can read back, whatever the offset.
        assert_eq!(
            today(LAST_WRITABLE_MS, MAX_UTC_OFFSET_MINUTES).to_iso(),
            "9999-12-31"
        );
        assert_eq!(
            today(u64::MAX, MAX_UTC_OFFSET_MINUTES).to_iso(),
            "9999-12-31"
        );
        assert_eq!(day_heading(Date(0)), "Thursday, 1 January 1970");
        assert_eq!(day_heading(Date(-1)), "Wednesday, 31 December 1969");
    }

    /// End to end: the host's offset from `StartContext` decides the day an
    /// entry is filed under, and survives neither into nor out of a snapshot.
    #[test]
    fn an_entry_is_filed_under_the_hosts_local_day() {
        set_now(THU + 3_600_000); // 01:00 UTC, Thursday 24 September
        let mut a = JournalMosaicApp::with_clock(test_clock);
        let mut context = StartContext::new("en-US", Platform::Linux);
        context.utc_offset_minutes = Some(-300); // New York: 20:00 on the 23rd
        a.start(context).unwrap();
        send(&mut a, "onNewEntry", json!({})).unwrap();
        send(&mut a, "onTitleChange", json!({ "value": "Evening" })).unwrap();
        let update = send(&mut a, "onSaveEntry", json!({})).unwrap();
        assert_eq!(
            update.props["timeline-rows"][0][1],
            "Wednesday, 23 September 2026"
        );

        let snapshot = a.snapshot().unwrap().unwrap();
        assert!(!String::from_utf8(snapshot.bytes)
            .unwrap()
            .contains("utcOffset"));
    }

    #[test]
    fn bare_event_names_from_the_react_host_are_read_as_emit_names() {
        assert_eq!(canonical_event_name("onSelectEntry"), "onSelectEntry");
        assert_eq!(canonical_event_name("selectEntry"), "onSelectEntry");
        assert_eq!(canonical_event_name("newEntry"), "onNewEntry");
        // Not an emit name either way: passed through, refused as unknown.
        assert_eq!(canonical_event_name("SelectEntry"), "SelectEntry");
        assert_eq!(canonical_event_name(""), "");
        assert_eq!(canonical_event_name("é"), "é");

        let mut a = app();
        send(&mut a, "newEntry", json!({})).unwrap();
        send(&mut a, "titleChange", json!({ "value": "Bare" })).unwrap();
        let update = send(&mut a, "saveEntry", json!({})).unwrap();
        assert_eq!(update.props["timeline-rows"][0][2], "Bare");
        let error = send(&mut a, "notAJournalEvent", json!({})).unwrap_err();
        assert!(error.to_string().contains("notAJournalEvent"), "{error}");
    }

    #[test]
    fn an_implausible_host_clock_reads_as_the_epoch() {
        assert_eq!(clamp_host_ms(1_790_000_000_000.0), 1_790_000_000_000);
        assert_eq!(clamp_host_ms(12.9), 12);
        // The last writable instant, and the first one past it.
        assert_eq!(clamp_host_ms(LAST_WRITABLE_MS as f64), LAST_WRITABLE_MS);
        assert_eq!(clamp_host_ms(LAST_WRITABLE_MS as f64 + 1.0), 0);
        // The largest value `Date.now()` can return (year 275760).
        assert_eq!(clamp_host_ms(8.64e15), 0);
        for bad in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -1.0,
            9_007_199_254_740_992.0,
            1e300,
        ] {
            assert_eq!(clamp_host_ms(bad), 0, "{bad}");
        }
    }

    /// The latest date the clock can give an entry still round-trips: an
    /// entry dated 9999-12-31 snapshots and restores.
    #[test]
    fn an_entry_on_the_last_writable_day_survives_a_restore() {
        let mut a = app();
        set_now(LAST_WRITABLE_MS);
        send(&mut a, "onNewEntry", json!({})).unwrap();
        send(&mut a, "onTitleChange", json!({ "value": "Far" })).unwrap();
        send(&mut a, "onSaveEntry", json!({})).unwrap();
        let snapshot = a.snapshot().unwrap().unwrap();
        let mut b = app();
        let props = b.restore(snapshot).unwrap().props;
        assert_eq!(props["timeline-rows"][0][2], "Far");
        assert!(props["timeline-rows"][0][1]
            .as_str()
            .unwrap()
            .contains("9999"));
    }
}
