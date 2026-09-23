//! Linear-memory WASM ABI over the **pure `journal-core` engine** (J2b of #14416).
//!
//! This is the JavaScript boundary for Journal: the web build, Electron, and the
//! Mosaic `journal-app`. It follows the repo's `*-wasm` convention, the same one
//! `task-wasm` uses:
//!
//! ```text
//!   JS                                   wasm linear memory
//!   ──                                   ──────────────────
//!   ptr = alloc(len); write UTF-8 JSON ─▶ [ JSON bytes … ]
//!   out = search(ptr, len)             ─▶ journal-core runs
//!   read [u32 LE len][UTF-8 JSON]      ◀─ out
//!   dealloc(ptr, len); dealloc(out, 4 + len)
//! ```
//!
//! It holds **one** `JournalState` for the page, and adds **no model logic**: every
//! rule lives in `journal-core`. This crate parses JSON, calls the engine, and
//! serialises the answer — nothing else.
//!
//! ## Nothing traps the boundary
//!
//! Every export returns a JSON envelope — `{"ok":true}`, `{"ok":true,"data":…}`, or
//! `{"ok":false,"error":…,"code":…}` — for success, a rejected command, a parse
//! failure, or a call before the state exists alike. A trap would kill the wasm
//! instance and every unsaved keystroke with it; an envelope is just a value the
//! host can show.
//!
//! ## `load` validates
//!
//! Deserialising checks only a snapshot's *shape*. `load` also runs
//! `JournalState::validate`, and keeps the current state unless both pass — the
//! loader obligation `code/specs/journal-core.md` records. See
//! `code/specs/journal-wasm.md` for the full contract.

use std::alloc::{alloc as raw_alloc, dealloc as raw_dealloc, Layout};
use std::cell::RefCell;

use journal_core::projections::{month_activity, on_this_day, search, tag_counts, timeline};
use journal_core::{
    apply, is_valid_id, Command, Date, EntryFilter, EntryId, JournalId, JournalState, OpError,
};
use serde::{Deserialize, Serialize};

thread_local! {
    /// The page's journal. `None` until the host calls `init` or `load`: there is
    /// no sensible default journal to invent, because its id must be host-minted.
    static STATE: RefCell<Option<JournalState>> = const { RefCell::new(None) };
}

// ── linear-memory plumbing (repo-standard, identical to task-wasm) ──────────────

/// Allocate `len` bytes for the host to write an argument into.
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return std::ptr::null_mut();
    }
    match Layout::from_size_align(len, 1) {
        // SAFETY: the layout has a non-zero size, which is all `alloc` requires.
        Ok(layout) => unsafe { raw_alloc(layout) },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free memory previously returned by [`alloc`] or by an export.
///
/// # Safety
/// `ptr`/`len` must exactly match a live allocation made by this module.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    if let Ok(layout) = Layout::from_size_align(len, 1) {
        // SAFETY: the caller guarantees `ptr` came from this module with this layout.
        unsafe { raw_dealloc(ptr, layout) };
    }
}

/// Read the host's argument as UTF-8 (lossily: a stray invalid byte becomes U+FFFD
/// and then, typically, a parse error envelope — never undefined behaviour).
///
/// # Safety
/// `ptr` must point to `len` readable bytes, or be null with a zero length.
unsafe fn read_input(ptr: *const u8, len: usize) -> String {
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    // SAFETY: the caller guarantees `ptr` addresses `len` readable bytes.
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).into_owned()
}

/// Pack a string as `[u32 little-endian length][UTF-8 bytes]` in a fresh allocation.
/// Returns null if the payload cannot be described by a `u32` or allocation fails.
fn pack(value: String) -> *mut u8 {
    let bytes = value.into_bytes();
    let payload_len = bytes.len();
    let Ok(prefix) = u32::try_from(payload_len) else {
        return std::ptr::null_mut();
    };
    let Some(total) = payload_len.checked_add(4) else {
        return std::ptr::null_mut();
    };
    let Ok(layout) = Layout::from_size_align(total, 1) else {
        return std::ptr::null_mut();
    };
    // SAFETY: `layout` is non-zero-sized; we write exactly `total` bytes into the
    // fresh allocation, from buffers of the stated lengths.
    unsafe {
        let ptr = raw_alloc(layout);
        if ptr.is_null() {
            return ptr;
        }
        std::ptr::copy_nonoverlapping(prefix.to_le_bytes().as_ptr(), ptr, 4);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), payload_len);
        ptr
    }
}

// ── envelopes ───────────────────────────────────────────────────────────────────

/// A stable, camelCase name for each `OpError` variant. Hosts branch on this; the
/// human-readable `error` string may be reworded freely.
fn op_code(err: &OpError) -> &'static str {
    match err {
        OpError::JournalNotFound(_) => "journalNotFound",
        OpError::EntryNotFound(_) => "entryNotFound",
        OpError::DuplicateId(_) => "duplicateId",
        OpError::InvalidId => "invalidId",
        OpError::TooManyJournals => "tooManyJournals",
        OpError::IdMismatch(_) => "idMismatch",
        OpError::DuplicateJournalName(_) => "duplicateJournalName",
        OpError::EmptyJournalName => "emptyJournalName",
        OpError::InvalidJournalName => "invalidJournalName",
        OpError::TitleTooLong => "titleTooLong",
        OpError::BodyTooLarge => "bodyTooLarge",
        OpError::InvalidTag { .. } => "invalidTag",
        OpError::LastJournal => "lastJournal",
        OpError::MoveTargetIsDeleted => "moveTargetIsDeleted",
    }
}

fn ok() -> String {
    r#"{"ok":true}"#.to_string()
}

fn ok_data<T: Serialize>(value: &T) -> String {
    serde_json::json!({ "ok": true, "data": value }).to_string()
}

fn fail(code: &str, error: &str) -> String {
    serde_json::json!({ "ok": false, "error": error, "code": code }).to_string()
}

fn op_fail(err: &OpError) -> String {
    fail(op_code(err), &err.to_string())
}

fn uninitialised() -> String {
    fail("uninitialised", "call init or load first")
}

/// A parse-failure envelope that says *where* the input broke, never *what* it
/// said. serde_json's own message quotes the offending value — unbounded in
/// length, and unescaped for an unknown enum tag — so repeating it would let a
/// hostile snapshot put a megabyte, or a newline and a terminal escape, into a
/// host's log. The category and position are enough to debug a file.
fn parse_fail(e: &serde_json::Error) -> String {
    let kind = match e.classify() {
        serde_json::error::Category::Io => "io",
        serde_json::error::Category::Syntax => "syntax",
        serde_json::error::Category::Data => "data",
        serde_json::error::Category::Eof => "eof",
    };
    fail(
        "parse",
        &format!(
            "parse error ({kind}) at line {} column {}",
            e.line(),
            e.column()
        ),
    )
}

/// Parse `json` as `A`. An empty argument parses as `{}` so optional-only inputs
/// (a filter with nothing set) can be sent as nothing at all.
fn parse<A: for<'de> Deserialize<'de>>(json: &str) -> Result<A, String> {
    let text = if json.trim().is_empty() { "{}" } else { json };
    serde_json::from_str(text).map_err(|e| parse_fail(&e))
}

/// Run a read-only query against the state.
fn read(f: impl FnOnce(&JournalState) -> String) -> String {
    STATE.with(|s| match s.borrow().as_ref() {
        Some(state) => f(state),
        None => uninitialised(),
    })
}

/// Parse an argument and run a query with it.
fn query<A, F>(json: &str, f: F) -> String
where
    A: for<'de> Deserialize<'de>,
    F: FnOnce(&JournalState, A) -> String,
{
    match parse::<A>(json) {
        Ok(args) => read(|state| f(state, args)),
        Err(envelope) => envelope,
    }
}

// ── argument shapes ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitArgs {
    journal_id: JournalId,
    name: String,
    #[serde(default)]
    now_ms: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyArgs {
    command: Command,
    #[serde(default)]
    now_ms: u64,
}

#[derive(Deserialize)]
struct IdArgs {
    id: EntryId,
}

#[derive(Deserialize)]
struct OnThisDayArgs {
    today: Date,
    #[serde(default)]
    filter: EntryFilter,
}

#[derive(Deserialize)]
struct SearchArgs {
    query: String,
    #[serde(default)]
    filter: EntryFilter,
}

#[derive(Deserialize)]
struct MonthArgs {
    year: i32,
    month: u32,
    #[serde(default)]
    filter: EntryFilter,
}

/// One entry as the TypeScript Journal stored it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyEntry {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    content: String,
    /// Kept as a string so one impossible date skips one row instead of failing
    /// the whole file at parse time.
    created_at: String,
    #[serde(default)]
    updated_at: u64,
}

#[derive(Deserialize)]
struct ImportArgs {
    journal: JournalId,
    entries: Vec<LegacyEntry>,
}

#[derive(Serialize)]
struct Skipped {
    index: usize,
    code: &'static str,
}

#[derive(Serialize)]
struct ImportReport {
    imported: usize,
    skipped: Vec<Skipped>,
}

// ── lifecycle ───────────────────────────────────────────────────────────────────

/// Start a fresh journal store with one journal, replacing any state.
///
/// # Safety
/// `ptr`/`len` must describe a readable buffer (see [`read_input`]).
#[no_mangle]
pub unsafe extern "C" fn init(ptr: *const u8, len: usize) -> *mut u8 {
    let json = unsafe { read_input(ptr, len) };
    pack(init_json(&json))
}

fn init_json(json: &str) -> String {
    let args = match parse::<InitArgs>(json) {
        Ok(a) => a,
        Err(envelope) => return envelope,
    };
    match JournalState::new(args.journal_id, args.name, args.now_ms) {
        Ok(state) => {
            STATE.with(|s| *s.borrow_mut() = Some(state));
            ok()
        }
        Err(e) => op_fail(&e),
    }
}

/// Replace the state with a snapshot — only if it parses **and** validates.
///
/// # Safety
/// `ptr`/`len` must describe a readable buffer (see [`read_input`]).
#[no_mangle]
pub unsafe extern "C" fn load(ptr: *const u8, len: usize) -> *mut u8 {
    let json = unsafe { read_input(ptr, len) };
    pack(load_json(&json))
}

fn load_json(json: &str) -> String {
    let state: JournalState = match serde_json::from_str(json) {
        Ok(s) => s,
        Err(e) => return parse_fail(&e),
    };
    if let Err(e) = state.validate() {
        return op_fail(&e);
    }
    STATE.with(|s| *s.borrow_mut() = Some(state));
    ok()
}

/// The whole state as raw JSON (not an envelope), for host-owned persistence;
/// `null` before `init`/`load`.
#[no_mangle]
pub extern "C" fn snapshot() -> *mut u8 {
    pack(snapshot_json())
}

fn snapshot_json() -> String {
    STATE.with(|s| serde_json::to_string(&*s.borrow()).unwrap_or_else(|_| "null".to_string()))
}

// ── commands ────────────────────────────────────────────────────────────────────

/// Apply one `journal-core` command. Exported to wasm as `apply` (the Rust name
/// differs only to avoid shadowing `journal_core::apply`).
///
/// # Safety
/// `ptr`/`len` must describe a readable buffer (see [`read_input`]).
#[export_name = "apply"]
pub unsafe extern "C" fn apply_command(ptr: *const u8, len: usize) -> *mut u8 {
    let json = unsafe { read_input(ptr, len) };
    pack(apply_json(&json))
}

fn apply_json(json: &str) -> String {
    let args = match parse::<ApplyArgs>(json) {
        Ok(a) => a,
        Err(envelope) => return envelope,
    };
    STATE.with(|s| match s.borrow_mut().as_mut() {
        Some(state) => match apply(state, args.command, args.now_ms) {
            Ok(()) => ok(),
            Err(e) => op_fail(&e),
        },
        None => uninitialised(),
    })
}

/// Import the TypeScript Journal's stored `Entry[]`, skipping (and reporting) rows
/// the core refuses.
///
/// # Safety
/// `ptr`/`len` must describe a readable buffer (see [`read_input`]).
#[no_mangle]
pub unsafe extern "C" fn import_legacy(ptr: *const u8, len: usize) -> *mut u8 {
    let json = unsafe { read_input(ptr, len) };
    pack(import_json(&json))
}

fn import_json(json: &str) -> String {
    let args = match parse::<ImportArgs>(json) {
        Ok(a) => a,
        Err(envelope) => return envelope,
    };
    STATE.with(|s| {
        let mut guard = s.borrow_mut();
        let Some(state) = guard.as_mut() else {
            return uninitialised();
        };
        // Check the id before an error can echo it (journal-core's rule for every
        // lookup: an id that was never validated must not reach a "not found").
        if !is_valid_id(args.journal.as_str()) {
            return op_fail(&OpError::InvalidId);
        }
        if state.journal(&args.journal).is_none() {
            return op_fail(&OpError::JournalNotFound(args.journal));
        }
        let mut report = ImportReport {
            imported: 0,
            skipped: Vec::new(),
        };
        for (index, row) in args.entries.into_iter().enumerate() {
            let Some(date) = Date::parse_iso(&row.created_at) else {
                report.skipped.push(Skipped {
                    index,
                    code: "invalidDate",
                });
                continue;
            };
            let cmd = Command::CreateEntry {
                id: EntryId::from_raw(row.id),
                journal: args.journal.clone(),
                date,
                title: row.title,
                body: row.content,
            };
            // `now_ms = updatedAt` sets both instants to the only one the TS app
            // recorded.
            match apply(state, cmd, row.updated_at) {
                Ok(()) => report.imported += 1,
                Err(e) => report.skipped.push(Skipped {
                    index,
                    code: op_code(&e),
                }),
            }
        }
        ok_data(&report)
    })
}

// ── queries ─────────────────────────────────────────────────────────────────────

/// Every journal, alphabetical by name (case-insensitively), for a picker.
#[no_mangle]
pub extern "C" fn journals() -> *mut u8 {
    pack(journals_json())
}

fn journals_json() -> String {
    read(|state| {
        let mut list: Vec<_> = state.journals.values().collect();
        list.sort_by_cached_key(|j| (j.name.to_lowercase(), j.id.clone()));
        ok_data(&list)
    })
}

/// One entry by id.
///
/// # Safety
/// `ptr`/`len` must describe a readable buffer (see [`read_input`]).
#[no_mangle]
pub unsafe extern "C" fn entry(ptr: *const u8, len: usize) -> *mut u8 {
    let json = unsafe { read_input(ptr, len) };
    pack(entry_json(&json))
}

fn entry_json(json: &str) -> String {
    query(json, |state, args: IdArgs| match state.entry(&args.id) {
        _ if !is_valid_id(args.id.as_str()) => op_fail(&OpError::InvalidId),
        Some(e) => ok_data(e),
        None => op_fail(&OpError::EntryNotFound(args.id)),
    })
}

/// Define a query export whose argument is a JSON value of type `$args`, plus the
/// natively-testable `$json` function behind it. The wasm symbol is `$export`; the
/// Rust names carry an `_export` suffix only so they do not shadow the
/// `journal_core::projections` functions they call.
macro_rules! query_export {
    ($(#[$doc:meta])* $export:literal, $name:ident, $json:ident, $args:ty, $f:expr) => {
        $(#[$doc])*
        ///
        /// # Safety
        /// `ptr`/`len` must describe a readable buffer (see [`read_input`]).
        #[export_name = $export]
        pub unsafe extern "C" fn $name(ptr: *const u8, len: usize) -> *mut u8 {
            let json = unsafe { read_input(ptr, len) };
            pack($json(&json))
        }

        fn $json(json: &str) -> String {
            query(json, $f)
        }
    };
}

query_export!(
    /// Entries grouped by day, newest first.
    "timeline",
    timeline_export,
    timeline_json,
    EntryFilter,
    |state, f: EntryFilter| ok_data(&timeline(state, &f))
);

query_export!(
    /// Entries from this month and day in earlier years.
    "on_this_day",
    on_this_day_export,
    on_this_day_json,
    OnThisDayArgs,
    |state, a: OnThisDayArgs| ok_data(&on_this_day(state, a.today, &a.filter))
);

query_export!(
    /// Ranked full-text search.
    "search",
    search_export,
    search_json,
    SearchArgs,
    |state, a: SearchArgs| ok_data(&search(state, &a.query, &a.filter))
);

query_export!(
    /// Every tag with its entry count.
    "tag_counts",
    tag_counts_export,
    tag_counts_json,
    EntryFilter,
    |state, f: EntryFilter| ok_data(&tag_counts(state, &f))
);

query_export!(
    /// Days of a month that have entries.
    "month_activity",
    month_activity_export,
    month_activity_json,
    MonthArgs,
    |state, a: MonthArgs| ok_data(&month_activity(state, a.year, a.month, &a.filter))
);

/// Drop the state (tests and "sign out" flows). The host must `init` or `load`
/// again before anything else succeeds.
#[no_mangle]
pub extern "C" fn reset() {
    STATE.with(|s| *s.borrow_mut() = None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn v(s: String) -> Value {
        serde_json::from_str(&s).unwrap()
    }

    fn fresh() {
        reset();
        let r = v(init_json(
            r#"{"journalId":"p","name":"Personal","nowMs":1}"#,
        ));
        assert_eq!(r, json!({ "ok": true }));
    }

    fn cmd(c: Value) -> Value {
        v(apply_json(&json!({ "command": c, "nowMs": 5 }).to_string()))
    }

    fn create(id: &str, date: &str, title: &str, body: &str) -> Value {
        cmd(json!({ "type": "createEntry", "id": id, "journal": "p",
                    "date": date, "title": title, "body": body }))
    }

    #[test]
    fn everything_before_init_is_an_uninitialised_envelope() {
        reset();
        for out in [
            journals_json(),
            timeline_json(""),
            entry_json(r#"{"id":"x"}"#),
            apply_json(r#"{"command":{"type":"deleteEntry","id":"x"}}"#),
            import_json(r#"{"journal":"p","entries":[]}"#),
        ] {
            assert_eq!(v(out)["code"], "uninitialised");
        }
        assert_eq!(snapshot_json(), "null");
    }

    #[test]
    fn init_refuses_what_create_journal_would() {
        reset();
        let r = v(init_json(r#"{"journalId":"bad id","name":"P"}"#));
        assert_eq!(r["code"], "invalidId");
        let r = v(init_json(r#"{"journalId":"p","name":"   "}"#));
        assert_eq!(r["code"], "emptyJournalName");
        assert_eq!(v(init_json("{not json")).get("code").unwrap(), "parse");
    }

    #[test]
    fn commands_and_queries_round_trip_through_json() {
        fresh();
        assert_eq!(
            create("e1", "2025-09-23", "Lighthouse", "a walk")["ok"],
            true
        );
        assert_eq!(create("e2", "2026-09-23", "Today", "rain")["ok"], true);
        assert_eq!(
            cmd(json!({ "type": "setTags", "id": "e1", "tags": ["Travel"] }))["ok"],
            true
        );

        let t = v(timeline_json(""));
        assert_eq!(t["data"][0]["date"], "2026-09-23");
        assert_eq!(t["data"][1]["entries"], json!(["e1"]));

        let otd = v(on_this_day_json(r#"{"today":"2026-09-23"}"#));
        assert_eq!(otd["data"][0]["yearsAgo"], 1);

        let hits = v(search_json(r#"{"query":"LIGHTHOUSE"}"#));
        assert_eq!(hits["data"][0]["entry"], "e1");
        assert_eq!(hits["data"][0]["score"], 3);

        let tags = v(tag_counts_json(r#"{"journal":"p"}"#));
        assert_eq!(tags["data"], json!([{ "tag": "Travel", "count": 1 }]));

        let month = v(month_activity_json(r#"{"year":2026,"month":9}"#));
        assert_eq!(month["data"], json!([{ "day": 23, "count": 1 }]));

        let e = v(entry_json(r#"{"id":"e1"}"#));
        assert_eq!(e["data"]["title"], "Lighthouse");
        assert_eq!(e["data"]["updatedAtMs"], 5);
    }

    #[test]
    fn rejections_carry_stable_codes() {
        fresh();
        create("e1", "2026-01-01", "", "");
        assert_eq!(create("e1", "2026-01-01", "", "")["code"], "duplicateId");
        assert_eq!(
            cmd(json!({ "type": "deleteJournal", "id": "p" }))["code"],
            "lastJournal"
        );
        assert_eq!(v(entry_json(r#"{"id":"nope"}"#))["code"], "entryNotFound");
        assert_eq!(
            cmd(json!({ "type": "setTags", "id": "e1", "tags": ["a\nb"] }))["code"],
            "invalidTag"
        );
        assert_eq!(cmd(json!({ "type": "noSuchCommand" }))["code"], "parse");
        // A bad date inside a command is a parse failure, not a panic.
        assert_eq!(create("e2", "2026-02-30", "", "")["code"], "parse");
    }

    #[test]
    fn every_op_error_has_a_distinct_code() {
        use journal_core::TagError;
        let all = [
            OpError::JournalNotFound(JournalId::from("a")),
            OpError::EntryNotFound(EntryId::from("a")),
            OpError::DuplicateId("a".into()),
            OpError::InvalidId,
            OpError::TooManyJournals,
            OpError::IdMismatch("a".into()),
            OpError::DuplicateJournalName("a".into()),
            OpError::EmptyJournalName,
            OpError::InvalidJournalName,
            OpError::TitleTooLong,
            OpError::BodyTooLarge,
            OpError::InvalidTag {
                index: 0,
                reason: TagError::Empty,
            },
            OpError::LastJournal,
            OpError::MoveTargetIsDeleted,
        ];
        let mut codes: Vec<&str> = all.iter().map(op_code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), all.len());
    }

    #[test]
    fn snapshot_then_load_restores_the_state() {
        fresh();
        create("e1", "2026-01-01", "Kept", "");
        let snap = snapshot_json();
        reset();
        assert_eq!(v(load_json(&snap)), json!({ "ok": true }));
        assert_eq!(v(entry_json(r#"{"id":"e1"}"#))["data"]["title"], "Kept");
    }

    #[test]
    fn load_refuses_invalid_state_and_keeps_the_old_one() {
        fresh();
        create("keep", "2026-01-01", "", "");
        // Well-formed JSON whose entry points at a journal that does not exist.
        let hostile = json!({
            "journals": { "p": { "id": "p", "name": "P", "createdAtMs": 0 } },
            "entries": { "e": { "id": "e", "journal": "ghost", "title": "", "body": "",
                                "date": "2020-01-01", "createdAtMs": 0, "updatedAtMs": 0 } }
        });
        assert_eq!(
            v(load_json(&hostile.to_string()))["code"],
            "journalNotFound"
        );
        assert_eq!(v(load_json("[1,2"))["code"], "parse");
        // The old state survived both refusals.
        assert_eq!(v(entry_json(r#"{"id":"keep"}"#))["ok"], true);
    }

    #[test]
    fn journals_list_alphabetically() {
        fresh();
        cmd(json!({ "type": "createJournal", "id": "w", "name": "work" }));
        cmd(json!({ "type": "createJournal", "id": "a", "name": "Art" }));
        let names: Vec<String> = v(journals_json())["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|j| j["name"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(names, ["Art", "Personal", "work"]);
    }

    #[test]
    fn legacy_import_skips_and_reports_bad_rows() {
        fresh();
        create("taken", "2026-01-01", "", "");
        let args = json!({ "journal": "p", "entries": [
            { "id": "a", "title": "Old", "content": "# Hi", "createdAt": "2024-05-01", "updatedAt": 99 },
            { "id": "b", "content": "no title", "createdAt": "2024-02-30", "updatedAt": 1 },
            { "id": "taken", "createdAt": "2024-05-02", "updatedAt": 1 },
            { "id": "bad id", "createdAt": "2024-05-03", "updatedAt": 1 },
            { "id": "c", "createdAt": "2024-05-04" }
        ]});
        let r = v(import_json(&args.to_string()));
        assert_eq!(r["data"]["imported"], 2);
        assert_eq!(
            r["data"]["skipped"],
            json!([
                { "index": 1, "code": "invalidDate" },
                { "index": 2, "code": "duplicateId" },
                { "index": 3, "code": "invalidId" }
            ])
        );
        let a = v(entry_json(r#"{"id":"a"}"#))["data"].clone();
        assert_eq!(a["date"], "2024-05-01");
        assert_eq!(a["body"], "# Hi");
        assert_eq!(a["createdAtMs"], 99);
        assert_eq!(a["updatedAtMs"], 99);

        assert_eq!(
            v(import_json(r#"{"journal":"ghost","entries":[]}"#))["code"],
            "journalNotFound"
        );
        // A row that is not shaped like an entry fails the whole file.
        assert_eq!(
            v(import_json(
                r#"{"journal":"p","entries":[{"title":"no id"}]}"#
            ))["code"],
            "parse"
        );
    }

    #[test]
    fn errors_never_echo_unchecked_input() {
        fresh();
        let evil = "x\n\u{1b}[31mFAKE\u{202e}";
        let r = v(entry_json(&json!({ "id": evil }).to_string()));
        assert_eq!(r["code"], "invalidId");
        let r = v(import_json(
            &json!({ "journal": evil, "entries": [] }).to_string(),
        ));
        assert_eq!(r["code"], "invalidId");

        // A parse error reports a position, not the offending text.
        let huge = "d".repeat(100_000);
        let snap = json!({ "journals": {}, "entries": { "e": { "date": huge } } }).to_string();
        let r = v(load_json(&snap));
        assert_eq!(r["code"], "parse");
        assert!(r["error"].as_str().unwrap().len() < 100, "{r}");
        let r = cmd(json!({ "type": "a\n\u{1b}[31mb" }));
        let msg = r["error"].as_str().unwrap();
        assert!(!msg.contains('\n') && !msg.contains('\u{1b}'), "{msg}");
    }

    #[test]
    fn linear_memory_round_trip() {
        fresh();
        let arg = br#"{"query":"x"}"#;
        let p = alloc(arg.len());
        assert!(!p.is_null());
        unsafe {
            std::ptr::copy_nonoverlapping(arg.as_ptr(), p, arg.len());
            let out = search_export(p, arg.len());
            dealloc(p, arg.len());
            let len = u32::from_le_bytes([*out, *out.add(1), *out.add(2), *out.add(3)]) as usize;
            let body = std::str::from_utf8(std::slice::from_raw_parts(out.add(4), len)).unwrap();
            assert_eq!(v(body.to_string()), json!({ "ok": true, "data": [] }));
            dealloc(out, 4 + len);
        }
        assert!(alloc(0).is_null());
        unsafe {
            dealloc(std::ptr::null_mut(), 0);
            // Null input reads as the empty argument `{}`.
            let out = timeline_export(std::ptr::null(), 0);
            let len = u32::from_le_bytes([*out, *out.add(1), *out.add(2), *out.add(3)]) as usize;
            dealloc(out, 4 + len);
        }
    }
}
