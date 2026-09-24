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

use std::error::Error;
use std::fmt;

use journal_core::projections::timeline;
use journal_core::{
    apply, Command, Date, Entry, EntryFilter, EntryId, JournalId, JournalState, OpError,
    MAX_BODY_BYTES, MAX_TITLE_CHARS,
};
use mosaic_app_runtime::{
    Announcement, AppUpdate, Event, MosaicApp, Politeness, Snapshot, StartContext,
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
    next_entry: u64,
}

/// The Journal application.
#[derive(Clone)]
pub struct JournalMosaicApp {
    state: AppState,
    /// Milliseconds since the Unix epoch. A plain function so the app stays
    /// `Clone` (dispatch clones to roll back) and tests can pin time.
    clock: fn() -> u64,
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
            state: AppState {
                journal,
                target: Target::New,
                draft_title: String::new(),
                draft_body: String::new(),
                next_entry: 1,
            },
            clock,
        }
    }

    // ── props ────────────────────────────────────────────────────────────────

    /// The slot values, keyed exactly by the `journal-app` package's slot names.
    fn props(&self) -> Value {
        let rows = self.timeline_rows();
        json!({
            "timeline-empty": rows.is_empty(),
            "timeline-rows": rows,
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

    /// `RecordList` rows `[key, heading, title, subtitle, meta, badge]`, newest
    /// day first, the day heading only on each day's first row.
    fn timeline_rows(&self) -> Vec<[String; 6]> {
        let mut rows = Vec::new();
        for day in timeline(&self.state.journal, &EntryFilter::default()) {
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
                    String::new(),
                    if entry.starred {
                        "★".to_string()
                    } else {
                        String::new()
                    },
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
        match canonical_event_name(&event.name).as_ref() {
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
            "onNewEntry" => {
                self.state.target = Target::New;
                self.state.draft_title.clear();
                self.state.draft_body.clear();
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
                Ok(self.announced("Entry deleted"))
            }
            "onCancelEdit" => {
                match self.state.target.clone() {
                    Target::New => {
                        self.state.draft_title.clear();
                        self.state.draft_body.clear();
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
        match self.state.target.clone() {
            Target::New => {
                if title.trim().is_empty() && body.trim().is_empty() {
                    return Ok(self.announced("Nothing to save yet"));
                }
                let id = self.mint_entry_id()?;
                self.run(Command::CreateEntry {
                    id: id.clone(),
                    journal: JournalId::from(DEFAULT_JOURNAL),
                    date: today((self.clock)()),
                    title,
                    body,
                })?;
                self.state.target = Target::Entry(id);
            }
            Target::Entry(id) => {
                self.run(Command::EditEntry {
                    id,
                    title: Some(title),
                    body: Some(body),
                })?;
            }
        }
        Ok(self.announced("Entry saved"))
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
                self.state.target = Target::Entry(id);
            }
            None => {
                self.state.target = Target::New;
                self.state.draft_title.clear();
                self.state.draft_body.clear();
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
            self.state.next_entry,
        );
        self.dispatch_inner(&event).inspect_err(|_| {
            (
                self.state.target,
                self.state.draft_title,
                self.state.draft_body,
                self.state.next_entry,
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

/// Today's date in **UTC** — `StartContext` carries no time zone (see the spec).
fn today(now_ms: u64) -> Date {
    // Days since the epoch fit an i32 for any clock this side of year 5,000,000.
    let days = i32::try_from(now_ms / MS_PER_DAY).unwrap_or(i32::MAX);
    Date(days)
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
                "draft-title",
                "selected-key",
                "timeline-empty",
                "timeline-rows"
            ]
        );
        assert_eq!(props["timeline-empty"], true);
        assert_eq!(props["delete-label"], "");
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
    fn today_is_the_utc_date_and_headings_handle_pre_epoch_days() {
        assert_eq!(today(THU).to_iso(), "2026-09-24");
        assert_eq!(today(THU + MS_PER_DAY - 1).to_iso(), "2026-09-24");
        assert_eq!(day_heading(Date(0)), "Thursday, 1 January 1970");
        assert_eq!(day_heading(Date(-1)), "Wednesday, 31 December 1969");
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
