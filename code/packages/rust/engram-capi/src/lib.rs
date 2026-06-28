//! Stable C ABI over `engram_core_wasm::EngramSession`.
//!
//! Native shells call this crate through an opaque session handle and
//! NUL-terminated UTF-8 strings. All string results are allocated by Rust and
//! must be released with `eg_string_free`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use engram_anki_package::{
    analyze_engram_media_references, inspect_apkg, read_media_file,
    read_v11_collection_as_engram_state, write_legacy_apkg_from_engram_state,
};
use engram_core_wasm::EngramSession;
use serde_json::{json, Value};

pub struct EgSession {
    inner: EngramSession,
}

#[no_mangle]
pub extern "C" fn eg_session_new() -> *mut EgSession {
    Box::into_raw(Box::new(EgSession {
        inner: EngramSession::new(),
    }))
}

/// # Safety
/// `session` must be a pointer returned by `eg_session_new` and not already freed.
#[no_mangle]
pub unsafe extern "C" fn eg_session_free(session: *mut EgSession) {
    if !session.is_null() {
        drop(Box::from_raw(session));
    }
}

/// # Safety
/// `value` must be a pointer returned by this library and not already freed.
#[no_mangle]
pub unsafe extern "C" fn eg_string_free(value: *mut c_char) {
    if !value.is_null() {
        drop(CString::from_raw(value));
    }
}

/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn eg_snapshot(session: *mut EgSession) -> *mut c_char {
    with_session(session, |session| session.snapshot())
}

/// # Safety
/// `session` must be valid; `snapshot_json` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn eg_load_snapshot(
    session: *mut EgSession,
    snapshot_json: *const c_char,
) -> *mut c_char {
    let snapshot_json = read_cstr(snapshot_json);
    with_session(session, |session| session.load_snapshot(&snapshot_json))
}

/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn eg_export_backup(
    session: *mut EgSession,
    exported_at: u64,
) -> *mut c_char {
    with_session(session, |session| session.export_backup(exported_at))
}

/// # Safety
/// `session` must be valid; `backup_json` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn eg_import_backup(
    session: *mut EgSession,
    backup_json: *const c_char,
) -> *mut c_char {
    let backup_json = read_cstr(backup_json);
    with_session(session, |session| session.import_backup(&backup_json))
}

/// # Safety
/// `session` must be valid; `command_json` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn eg_dispatch(
    session: *mut EgSession,
    command_json: *const c_char,
) -> *mut c_char {
    let command_json = read_cstr(command_json);
    with_session(session, |session| session.dispatch(&command_json))
}

/// # Safety
/// `session` must be valid; `deck_id` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn eg_build_queue(
    session: *mut EgSession,
    deck_id: *const c_char,
    now: u64,
) -> *mut c_char {
    let deck_id = read_cstr(deck_id);
    with_session(session, |session| session.build_queue(&deck_id, now))
}

/// # Safety
/// `session` must be valid; strings must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn eg_daily_limit_usage(
    session: *mut EgSession,
    deck_id: *const c_char,
    day_start: u64,
    day_end: u64,
    deck_options_json: *const c_char,
) -> *mut c_char {
    let deck_id = read_cstr(deck_id);
    let deck_options_json = read_cstr(deck_options_json);
    with_session(session, |session| {
        session.daily_limit_usage(&deck_id, day_start, day_end, &deck_options_json)
    })
}

/// # Safety
/// `session` must be valid; strings must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn eg_build_queue_with_daily_limits(
    session: *mut EgSession,
    deck_id: *const c_char,
    now: u64,
    day_start: u64,
    day_end: u64,
    deck_options_json: *const c_char,
) -> *mut c_char {
    let deck_id = read_cstr(deck_id);
    let deck_options_json = read_cstr(deck_options_json);
    with_session(session, |session| {
        session.build_queue_with_daily_limits(&deck_id, now, day_start, day_end, &deck_options_json)
    })
}

/// # Safety
/// `session` must be valid; `deck_id` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn eg_deck_stats(
    session: *mut EgSession,
    deck_id: *const c_char,
    now: u64,
) -> *mut c_char {
    let deck_id = read_cstr(deck_id);
    with_session(session, |session| session.deck_stats(&deck_id, now))
}

/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn eg_session_progress(session: *mut EgSession) -> *mut c_char {
    with_session(session, |session| session.session_progress())
}

/// # Safety
/// `session` must be valid; `deck_id` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn eg_engram_app_props(
    session: *mut EgSession,
    deck_id: *const c_char,
    now: u64,
) -> *mut c_char {
    let deck_id = read_cstr(deck_id);
    with_session(session, |session| session.engram_app_props(&deck_id, now))
}

/// # Safety
/// `session` must be valid; strings must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn eg_handle_engram_app_event(
    session: *mut EgSession,
    event: *const c_char,
    deck_id: *const c_char,
    now: u64,
) -> *mut c_char {
    let event = read_cstr(event);
    let deck_id = read_cstr(deck_id);
    with_session(session, |session| {
        session.handle_engram_app_event(&event, &deck_id, now)
    })
}

/// # Safety
/// `session` must be valid; `deck_id` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn eg_review_history(
    session: *mut EgSession,
    deck_id: *const c_char,
    reviewed_after: u64,
    reviewed_before: u64,
) -> *mut c_char {
    let deck_id = read_cstr(deck_id);
    with_session(session, |session| {
        session.review_history(&deck_id, reviewed_after, reviewed_before)
    })
}

/// # Safety
/// `session` must be valid; arguments must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn eg_generated_cards(
    session: *mut EgSession,
    note_type_id: *const c_char,
    note_id: *const c_char,
) -> *mut c_char {
    let note_type_id = read_cstr(note_type_id);
    let note_id = read_cstr(note_id);
    with_session(session, |session| {
        session.generated_cards(&note_type_id, &note_id)
    })
}

/// # Safety
/// `session` must be valid; string arguments must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn eg_materialized_cards(
    session: *mut EgSession,
    note_type_id: *const c_char,
    note_id: *const c_char,
    created_at: u64,
) -> *mut c_char {
    let note_type_id = read_cstr(note_type_id);
    let note_id = read_cstr(note_id);
    with_session(session, |session| {
        session.materialized_cards(&note_type_id, &note_id, created_at)
    })
}

/// # Safety
/// `session` must be valid; `query` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn eg_search_cards(
    session: *mut EgSession,
    query: *const c_char,
    now: u64,
) -> *mut c_char {
    let query = read_cstr(query);
    with_session(session, |session| session.search_cards(&query, now))
}

/// # Safety
/// `session` must be valid; `deck_id` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn eg_export_cards_csv(
    session: *mut EgSession,
    deck_id: *const c_char,
) -> *mut c_char {
    let deck_id = read_cstr(deck_id);
    with_session(session, |session| session.export_cards_csv(&deck_id))
}

/// # Safety
/// `session` must be valid; strings must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn eg_export_anki_basic_tsv(
    session: *mut EgSession,
    deck_id: *const c_char,
    deck_name: *const c_char,
    note_type_name: *const c_char,
    html: u8,
) -> *mut c_char {
    let deck_id = read_cstr(deck_id);
    let deck_name = read_cstr(deck_name);
    let note_type_name = read_cstr(note_type_name);
    with_session(session, |session| {
        session.export_anki_basic_tsv(&deck_id, &deck_name, &note_type_name, html != 0)
    })
}

/// # Safety
/// `session` must be valid; strings must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn eg_export_anki_notes_tsv(
    session: *mut EgSession,
    note_type_id: *const c_char,
    deck_id: *const c_char,
    deck_name: *const c_char,
    note_type_name: *const c_char,
    html: u8,
) -> *mut c_char {
    let note_type_id = read_cstr(note_type_id);
    let deck_id = read_cstr(deck_id);
    let deck_name = read_cstr(deck_name);
    let note_type_name = read_cstr(note_type_name);
    with_session(session, |session| {
        session.export_anki_notes_tsv(
            &note_type_id,
            &deck_id,
            &deck_name,
            &note_type_name,
            html != 0,
        )
    })
}

/// # Safety
/// `session` must be valid; `csv` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn eg_parse_cards_csv(
    session: *mut EgSession,
    csv: *const c_char,
) -> *mut c_char {
    let csv = read_cstr(csv);
    with_session(session, |session| session.parse_cards_csv(&csv))
}

/// # Safety
/// `session` must be valid; string arguments must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn eg_parse_basic_cards_csv(
    session: *mut EgSession,
    csv: *const c_char,
    deck_id: *const c_char,
    id_prefix: *const c_char,
    created_at: u64,
) -> *mut c_char {
    let csv = read_cstr(csv);
    let deck_id = read_cstr(deck_id);
    let id_prefix = read_cstr(id_prefix);
    with_session(session, |session| {
        session.parse_basic_cards_csv(&csv, &deck_id, &id_prefix, created_at)
    })
}

/// # Safety
/// `session` must be valid; string arguments must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn eg_parse_anki_basic_tsv(
    session: *mut EgSession,
    tsv: *const c_char,
    deck_id: *const c_char,
    id_prefix: *const c_char,
    created_at: u64,
) -> *mut c_char {
    let tsv = read_cstr(tsv);
    let deck_id = read_cstr(deck_id);
    let id_prefix = read_cstr(id_prefix);
    with_session(session, |session| {
        session.parse_anki_basic_tsv(&tsv, &deck_id, &id_prefix, created_at)
    })
}

/// # Safety
/// `session` must be valid; string arguments must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn eg_parse_anki_notes_tsv(
    session: *mut EgSession,
    tsv: *const c_char,
    deck_id: *const c_char,
    note_type_id: *const c_char,
    note_type_name: *const c_char,
    note_id_prefix: *const c_char,
    created_at: u64,
) -> *mut c_char {
    let tsv = read_cstr(tsv);
    let deck_id = read_cstr(deck_id);
    let note_type_id = read_cstr(note_type_id);
    let note_type_name = read_cstr(note_type_name);
    let note_id_prefix = read_cstr(note_id_prefix);
    with_session(session, |session| {
        session.parse_anki_notes_tsv(
            &tsv,
            &deck_id,
            &note_type_id,
            &note_type_name,
            &note_id_prefix,
            created_at,
        )
    })
}

/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn eg_export_anki_apkg(session: *mut EgSession) -> *mut c_char {
    if session.is_null() {
        return ptr::null_mut();
    }
    into_cstr(export_anki_apkg_json(&(*session).inner))
}

/// # Safety
/// `session` must be a valid session pointer.
#[no_mangle]
pub unsafe extern "C" fn eg_analyze_media_references(session: *mut EgSession) -> *mut c_char {
    if session.is_null() {
        return ptr::null_mut();
    }
    into_cstr(analyze_media_references_json(&(*session).inner))
}

/// # Safety
/// `session` must be valid; `data` must point to `data_len` APKG bytes.
#[no_mangle]
pub unsafe extern "C" fn eg_inspect_anki_apkg(
    session: *mut EgSession,
    data: *const u8,
    data_len: usize,
) -> *mut c_char {
    if session.is_null() {
        return ptr::null_mut();
    }
    into_cstr(match read_ffi_bytes(data, data_len) {
        Ok(bytes) => inspect_anki_apkg_json(bytes),
        Err(message) => error_json(&message),
    })
}

/// # Safety
/// `session` must be valid; `data` must point to `data_len` APKG bytes;
/// `archive_name` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn eg_read_anki_apkg_media(
    session: *mut EgSession,
    data: *const u8,
    data_len: usize,
    archive_name: *const c_char,
) -> *mut c_char {
    if session.is_null() {
        return ptr::null_mut();
    }
    let archive_name = read_cstr(archive_name);
    into_cstr(match read_ffi_bytes(data, data_len) {
        Ok(bytes) => read_anki_apkg_media_json(bytes, &archive_name),
        Err(message) => error_json(&message),
    })
}

/// # Safety
/// `session` must be valid; `data` must point to `data_len` APKG bytes.
#[no_mangle]
pub unsafe extern "C" fn eg_parse_anki_apkg(
    session: *mut EgSession,
    data: *const u8,
    data_len: usize,
) -> *mut c_char {
    if session.is_null() {
        return ptr::null_mut();
    }
    into_cstr(match read_ffi_bytes(data, data_len) {
        Ok(bytes) => parse_anki_apkg_json(bytes),
        Err(message) => error_json(&message),
    })
}

/// # Safety
/// `session` must be valid; `data` must point to `data_len` APKG bytes.
#[no_mangle]
pub unsafe extern "C" fn eg_import_anki_apkg(
    session: *mut EgSession,
    data: *const u8,
    data_len: usize,
) -> *mut c_char {
    if session.is_null() {
        return ptr::null_mut();
    }
    into_cstr(match read_ffi_bytes(data, data_len) {
        Ok(bytes) => import_anki_apkg_json(&mut (*session).inner, bytes),
        Err(message) => error_json(&message),
    })
}

unsafe fn read_cstr(value: *const c_char) -> String {
    if value.is_null() {
        return String::new();
    }
    CStr::from_ptr(value).to_string_lossy().into_owned()
}

unsafe fn read_ffi_bytes<'a>(data: *const u8, data_len: usize) -> Result<&'a [u8], String> {
    if data.is_null() {
        if data_len == 0 {
            Ok(&[])
        } else {
            Err("APKG data pointer is null".to_string())
        }
    } else {
        Ok(std::slice::from_raw_parts(data, data_len))
    }
}

unsafe fn with_session(
    session: *mut EgSession,
    run: impl FnOnce(&mut EngramSession) -> String,
) -> *mut c_char {
    if session.is_null() {
        return ptr::null_mut();
    }
    into_cstr(run(&mut (*session).inner))
}

fn inspect_anki_apkg_json(bytes: &[u8]) -> String {
    match inspect_apkg(bytes) {
        Ok(manifest) => ok_json_with(
            "manifest",
            serde_json::to_value(manifest).unwrap_or(Value::Null),
        ),
        Err(error) => error_json(&error.message),
    }
}

fn read_anki_apkg_media_json(bytes: &[u8], archive_name: &str) -> String {
    match read_media_file(bytes, archive_name) {
        Ok(media) => ok_json_with("media", serde_json::to_value(media).unwrap_or(Value::Null)),
        Err(error) => error_json(&error.message),
    }
}

fn parse_anki_apkg_json(bytes: &[u8]) -> String {
    match read_v11_collection_as_engram_state(bytes) {
        Ok(state) => ok_json_with("state", serde_json::to_value(state).unwrap_or(Value::Null)),
        Err(error) => error_json(&error.message),
    }
}

fn import_anki_apkg_json(session: &mut EngramSession, bytes: &[u8]) -> String {
    match read_v11_collection_as_engram_state(bytes) {
        Ok(state) => match serde_json::to_string(&state) {
            Ok(snapshot_json) => session.load_snapshot(&snapshot_json),
            Err(error) => error_json(&format!("failed to serialize imported Anki state: {error}")),
        },
        Err(error) => error_json(&error.message),
    }
}

fn export_anki_apkg_json(session: &EngramSession) -> String {
    match write_legacy_apkg_from_engram_state(session.state(), &[]) {
        Ok(apkg) => ok_json_with("apkg", serde_json::to_value(apkg).unwrap_or(Value::Null)),
        Err(error) => error_json(&error.message),
    }
}

fn analyze_media_references_json(session: &EngramSession) -> String {
    ok_json_with(
        "mediaReferences",
        serde_json::to_value(analyze_engram_media_references(session.state()))
            .unwrap_or(Value::Null),
    )
}

fn ok_json_with(key: &str, value: Value) -> String {
    let mut object = serde_json::Map::new();
    object.insert("ok".to_string(), Value::Bool(true));
    object.insert(key.to_string(), value);
    Value::Object(object).to_string()
}

fn error_json(message: &str) -> String {
    json!({ "ok": false, "error": message }).to_string()
}

fn into_cstr(value: String) -> *mut c_char {
    match CString::new(value) {
        Ok(value) => value.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_anki_package::{write_legacy_apkg, MediaAsset};
    use rusqlite::Connection;
    use serde_json::Value;

    const NOW: u64 = 1_700_000_000_000;

    unsafe fn take(value: *mut c_char) -> String {
        let result = CStr::from_ptr(value).to_string_lossy().into_owned();
        eg_string_free(value);
        result
    }

    fn cstr(value: &str) -> CString {
        CString::new(value).unwrap()
    }

    fn v11_apkg_fixture() -> Vec<u8> {
        let sqlite = tempfile::NamedTempFile::new().unwrap();
        {
            let connection = Connection::open(sqlite.path()).unwrap();
            connection
                .execute_batch(
                    r#"
CREATE TABLE col (
  id integer primary key,
  crt integer not null,
  mod integer not null,
  scm integer not null,
  ver integer not null,
  dty integer not null,
  usn integer not null,
  ls integer not null,
  conf text not null,
  models text not null,
  decks text not null,
  dconf text not null,
  tags text not null
);
CREATE TABLE notes (
  id integer primary key,
  guid text not null,
  mid integer not null,
  mod integer not null,
  usn integer not null,
  tags text not null,
  flds text not null,
  sfld integer not null,
  csum integer not null,
  flags integer not null,
  data text not null
);
CREATE TABLE cards (
  id integer primary key,
  nid integer not null,
  did integer not null,
  ord integer not null,
  mod integer not null,
  usn integer not null,
  type integer not null,
  queue integer not null,
  due integer not null,
  ivl integer not null,
  factor integer not null,
  reps integer not null,
  lapses integer not null,
  left integer not null,
  odue integer not null,
  odid integer not null,
  flags integer not null,
  data text not null
);
CREATE TABLE revlog (
  id integer primary key,
  cid integer not null,
  usn integer not null,
  ease integer not null,
  ivl integer not null,
  lastIvl integer not null,
  factor integer not null,
  time integer not null,
  type integer not null
);
CREATE TABLE graves (
  usn integer not null,
  oid integer not null,
  type integer not null
);
"#,
                )
                .unwrap();

            let decks = r#"{"2":{"id":2,"name":"Spanish::Latin","desc":"Story deck"}}"#;
            let models = r#"{"100":{"id":100,"name":"Basic","type":0,"css":"","flds":[{"name":"Front","ord":0},{"name":"Back","ord":1}],"tmpls":[{"name":"Card 1","ord":0,"qfmt":"{{Front}}","afmt":"{{Back}}","did":2}]}}"#;
            connection
                .execute(
                    "INSERT INTO col VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    rusqlite::params![
                        1_i64,
                        19_000_i64,
                        1_700_000_000_i64,
                        1_700_000_001_i64,
                        11_i64,
                        0_i64,
                        -1_i64,
                        1_700_000_002_i64,
                        r#"{}"#,
                        models,
                        decks,
                        r#"{
                            "1": {
                                "id": 1,
                                "name": "Default",
                                "new": {"perDay": 12, "delays": [3, 12], "ints": [2, 5]},
                                "rev": {"perDay": 80},
                                "lapse": {"delays": [20], "mult": 0.5}
                            }
                        }"#,
                        r#"{}"#
                    ],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO notes VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    rusqlite::params![
                        1000_i64,
                        "guid-1000",
                        100_i64,
                        1_700_000_010_i64,
                        -1_i64,
                        " spanish roots ",
                        "hola\u{1f}hello",
                        "hola",
                        123_i64,
                        0_i64,
                        ""
                    ],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO cards VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                    rusqlite::params![
                        2000_i64,
                        1000_i64,
                        2_i64,
                        0_i64,
                        1_700_000_020_i64,
                        -1_i64,
                        2_i64,
                        2_i64,
                        42_i64,
                        7_i64,
                        2500_i64,
                        3_i64,
                        1_i64,
                        0_i64,
                        0_i64,
                        0_i64,
                        4_i64,
                        ""
                    ],
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO revlog VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        3000_i64, 2000_i64, -1_i64, 3_i64, 7_i64, 3_i64, 2500_i64, 12_000_i64,
                        1_i64
                    ],
                )
                .unwrap();
        }

        let sqlite_bytes = std::fs::read(sqlite.path()).unwrap();
        write_legacy_apkg(
            &sqlite_bytes,
            &[
                MediaAsset {
                    filename: "audio/hola.mp3",
                    data: b"mp3",
                },
                MediaAsset {
                    filename: "images/card.png",
                    data: b"png",
                },
            ],
        )
    }

    fn media_apkg_fixture() -> Vec<u8> {
        write_legacy_apkg(
            b"sqlite collection",
            &[
                MediaAsset {
                    filename: "audio/hola.mp3",
                    data: b"mp3",
                },
                MediaAsset {
                    filename: "images/card.png",
                    data: b"png",
                },
            ],
        )
    }

    #[test]
    fn c_abi_dispatches_and_snapshots_state() {
        unsafe {
            let session = eg_session_new();
            let command = cstr(
                r#"{
                    "type": "createDeck",
                    "id": "deck",
                    "name": "Tamil",
                    "description": "Script",
                    "createdAt": 1700000000000
                }"#,
            );

            let result = take(eg_dispatch(session, command.as_ptr()));
            assert!(result.contains(r#""ok":true"#));

            let snapshot = take(eg_snapshot(session));
            assert!(snapshot.contains(r#""id":"deck""#));

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_dispatches_shared_media_asset_commands() {
        unsafe {
            let session = eg_session_new();
            let upsert = cstr(
                r#"{
                    "type": "upsertMediaAsset",
                    "asset": {
                        "id": "anki-media:0",
                        "archiveName": "0",
                        "filename": "audio/hola.mp3",
                        "data": [109, 112, 51]
                    }
                }"#,
            );

            let result = take(eg_dispatch(session, upsert.as_ptr()));
            let result: Value = serde_json::from_str(&result).unwrap();
            assert_eq!(result["ok"], true);
            assert_eq!(
                result["state"]["mediaAssets"][0]["filename"],
                "audio/hola.mp3"
            );

            let delete = cstr(
                r#"{
                    "type": "deleteMediaAsset",
                    "assetId": "anki-media:0"
                }"#,
            );
            let result = take(eg_dispatch(session, delete.as_ptr()));
            let result: Value = serde_json::from_str(&result).unwrap();
            assert!(result["state"]["mediaAssets"]
                .as_array()
                .unwrap()
                .is_empty());

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_parses_and_imports_anki_apkg_state() {
        unsafe {
            let session = eg_session_new();
            let apkg = v11_apkg_fixture();

            let parsed = take(eg_parse_anki_apkg(session, apkg.as_ptr(), apkg.len()));
            let parsed: Value = serde_json::from_str(&parsed).unwrap();
            assert_eq!(parsed["ok"], true);
            assert_eq!(parsed["state"]["decks"][0]["id"], "2");
            assert_eq!(parsed["state"]["decks"][0]["name"], "Spanish::Latin");
            assert_eq!(parsed["state"]["cards"][0]["front"], "hola");
            assert_eq!(parsed["state"]["cards"][0]["back"], "hello");
            assert_eq!(parsed["state"]["cardProgress"][0]["flag"], "blue");
            assert_eq!(
                parsed["state"]["mediaAssets"][0]["filename"],
                "audio/hola.mp3"
            );
            assert_eq!(
                parsed["state"]["mediaAssets"][0]["data"],
                json!([109, 112, 51])
            );
            assert_eq!(
                parsed["state"]["deckOptions"][0]["options"]["newCardsPerDay"],
                12
            );

            let imported = take(eg_import_anki_apkg(session, apkg.as_ptr(), apkg.len()));
            let imported: Value = serde_json::from_str(&imported).unwrap();
            assert_eq!(imported["ok"], true);
            assert_eq!(imported["state"]["cards"][0]["id"], "2000");
            assert_eq!(imported["state"]["reviews"][0]["rating"], "good");
            assert_eq!(
                imported["state"]["mediaAssets"][1]["filename"],
                "images/card.png"
            );

            let snapshot = take(eg_snapshot(session));
            let snapshot: Value = serde_json::from_str(&snapshot).unwrap();
            assert_eq!(snapshot["state"]["cards"][0]["front"], "hola");
            assert_eq!(snapshot["state"]["sessions"][0]["status"], "completed");
            assert_eq!(snapshot["state"]["mediaAssets"][0]["archiveName"], "0");

            let exported = take(eg_export_anki_apkg(session));
            let exported: Value = serde_json::from_str(&exported).unwrap();
            let exported_apkg = exported["apkg"]
                .as_array()
                .unwrap()
                .iter()
                .map(|byte| byte.as_u64().unwrap() as u8)
                .collect::<Vec<_>>();
            let inspected = take(eg_inspect_anki_apkg(
                session,
                exported_apkg.as_ptr(),
                exported_apkg.len(),
            ));
            let inspected: Value = serde_json::from_str(&inspected).unwrap();
            assert_eq!(
                inspected["manifest"]["media"]["mapping"]["0"],
                "audio/hola.mp3"
            );

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_exports_anki_apkg_bytes() {
        unsafe {
            let session = eg_session_new();
            let snapshot = cstr(
                r#"{
                    "decks": [{"id":"2","name":"Spanish::Latin","description":"Story deck","createdAt":1641600000000}],
                    "noteTypes": [{
                        "id":"100",
                        "name":"Basic",
                        "fields":[
                            {"id":"100:field:0","name":"Front","required":true,"ordinal":0},
                            {"id":"100:field:1","name":"Back","required":true,"ordinal":1}
                        ],
                        "templates":[{
                            "id":"100:template:0",
                            "name":"Card 1",
                            "frontTemplate":"{{Front}}",
                            "backTemplate":"{{Back}}",
                            "requiredFieldNames":["Front"],
                            "ordinal":0
                        }],
                        "createdAt":1641600000000,
                        "updatedAt":1641600000000
                    }],
                    "notes": [{
                        "id":"1000",
                        "noteTypeId":"100",
                        "deckId":"2",
                        "fields":[
                            {"fieldId":"100:field:0","value":"hola"},
                            {"fieldId":"100:field:1","value":"hello"}
                        ],
                        "tags":["spanish"],
                        "createdAt":1700000010000,
                        "updatedAt":1700000010000
                    }],
                    "cards": [{
                        "id":"2000",
                        "deckId":"2",
                        "front":"hola",
                        "back":"hello",
                        "createdAt":1700000020000,
                        "lineage":{
                            "noteId":"1000",
                            "noteTypeId":"100",
                            "templateId":"100:template:0",
                            "ordinal":0,
                            "clozeOrdinal":null
                        }
                    }],
                    "cardProgress": [],
                    "sessions": [],
                    "reviews": [],
                    "activeSession": null
                }"#,
            );
            let loaded = take(eg_load_snapshot(session, snapshot.as_ptr()));
            assert!(loaded.contains(r#""ok":true"#));

            let exported = take(eg_export_anki_apkg(session));
            let exported: Value = serde_json::from_str(&exported).unwrap();
            assert_eq!(exported["ok"], true);
            let apkg = exported["apkg"]
                .as_array()
                .unwrap()
                .iter()
                .map(|byte| byte.as_u64().unwrap() as u8)
                .collect::<Vec<_>>();

            let parsed = take(eg_parse_anki_apkg(session, apkg.as_ptr(), apkg.len()));
            let parsed: Value = serde_json::from_str(&parsed).unwrap();
            assert_eq!(parsed["ok"], true);
            assert_eq!(parsed["state"]["decks"][0]["name"], "Spanish::Latin");
            assert_eq!(parsed["state"]["cards"][0]["front"], "hola");
            assert_eq!(parsed["state"]["cards"][0]["back"], "hello");

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_inspects_and_reads_anki_apkg_media() {
        unsafe {
            let session = eg_session_new();
            let apkg = media_apkg_fixture();

            let inspected = take(eg_inspect_anki_apkg(session, apkg.as_ptr(), apkg.len()));
            let inspected: Value = serde_json::from_str(&inspected).unwrap();
            assert_eq!(inspected["ok"], true);
            assert_eq!(
                inspected["manifest"]["collection"]["format"],
                "legacySqlite"
            );
            assert_eq!(inspected["manifest"]["media"]["mapPresent"], true);
            assert_eq!(
                inspected["manifest"]["media"]["mapping"]["0"],
                "audio/hola.mp3"
            );
            assert_eq!(
                inspected["manifest"]["media"]["mediaFiles"][1]["filename"],
                "images/card.png"
            );

            let archive_name = cstr("0");
            let media = take(eg_read_anki_apkg_media(
                session,
                apkg.as_ptr(),
                apkg.len(),
                archive_name.as_ptr(),
            ));
            let media: Value = serde_json::from_str(&media).unwrap();
            assert_eq!(media["ok"], true);
            assert_eq!(media["media"]["archiveName"], "0");
            assert_eq!(media["media"]["filename"], "audio/hola.mp3");
            assert_eq!(media["media"]["data"], serde_json::json!([109, 112, 51]));

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_analyzes_state_media_references() {
        unsafe {
            let session = eg_session_new();
            let snapshot = cstr(
                r#"{
                    "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":0}],
                    "noteTypes": [],
                    "notes": [{
                        "id":"note",
                        "noteTypeId":"basic",
                        "deckId":"deck",
                        "fields":[{"fieldId":"front","value":"[sound:audio/hola.mp3]"}],
                        "tags":[],
                        "createdAt":0,
                        "updatedAt":0
                    }],
                    "cards": [{
                        "id":"card",
                        "deckId":"deck",
                        "front":"<img src='images/card.png'>",
                        "back":"<img src=\"missing.png\">",
                        "createdAt":0,
                        "lineage":null
                    }],
                    "cardProgress": [],
                    "sessions": [],
                    "reviews": [],
                    "mediaAssets": [
                        {"id":"audio","archiveName":"0","filename":"audio/hola.mp3","data":[109,112,51]},
                        {"id":"image","archiveName":"1","filename":"images/card.png","data":[112,110,103]},
                        {"id":"unused","archiveName":"2","filename":"audio/unused.mp3","data":[117]}
                    ],
                    "activeSession": null
                }"#,
            );
            let loaded = take(eg_load_snapshot(session, snapshot.as_ptr()));
            assert!(loaded.contains(r#""ok":true"#));

            let analyzed = take(eg_analyze_media_references(session));
            let analyzed: Value = serde_json::from_str(&analyzed).unwrap();

            assert_eq!(analyzed["ok"], true);
            assert_eq!(
                analyzed["mediaReferences"]["referencedFilenames"],
                json!(["audio/hola.mp3", "images/card.png", "missing.png"])
            );
            assert_eq!(
                analyzed["mediaReferences"]["referencedAssetIds"],
                json!(["audio", "image"])
            );
            assert_eq!(
                analyzed["mediaReferences"]["missingFilenames"],
                json!(["missing.png"])
            );
            assert_eq!(
                analyzed["mediaReferences"]["unreferencedAssetIds"],
                json!(["unused"])
            );

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_search_and_csv_helpers_return_json() {
        unsafe {
            let session = eg_session_new();
            let snapshot = cstr(
                r#"{
                    "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                    "noteTypes": [],
                    "notes": [],
                    "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
                    "cardProgress": [],
                    "sessions": [],
                    "reviews": [],
                    "activeSession": null
                }"#,
            );
            take(eg_load_snapshot(session, snapshot.as_ptr()));

            let query = cstr("deck:tamil");
            let search = take(eg_search_cards(session, query.as_ptr(), NOW));
            assert!(search.contains(r#""results":[{"card""#));

            let deck_id = cstr("deck");
            let csv = take(eg_export_cards_csv(session, deck_id.as_ptr()));
            assert!(csv.contains("id,deckId,front,back,createdAt"));

            let deck_name = cstr("Tamil::Script");
            let note_type = cstr("Basic");
            let tsv = take(eg_export_anki_basic_tsv(
                session,
                deck_id.as_ptr(),
                deck_name.as_ptr(),
                note_type.as_ptr(),
                0,
            ));
            assert!(tsv.contains("#separator:tab"));
            assert!(tsv.contains("#deck:Tamil::Script"));
            assert!(tsv.contains(r#"letter-a\ta"#));

            let basic = cstr("front,back\nletter-aa,aa\n");
            let imported = take(eg_parse_basic_cards_csv(
                session,
                basic.as_ptr(),
                deck_id.as_ptr(),
                cstr("import").as_ptr(),
                NOW,
            ));
            assert!(imported.contains(r#""id":"import-1""#));

            let anki = cstr("#separator:tab\n#columns:Front\tBack\nletter-aa\taa\n");
            let anki_imported = take(eg_parse_anki_basic_tsv(
                session,
                anki.as_ptr(),
                deck_id.as_ptr(),
                cstr("anki").as_ptr(),
                NOW,
            ));
            assert!(anki_imported.contains(r#""id":"anki-1""#));
            assert!(anki_imported.contains(r#""front":"letter-aa""#));

            let anki_notes = cstr(
                "#separator:tab\n#notetype:Basic (and reversed card)\n#columns:Front\tBack\tTags\nhola\thello\tspanish common\n",
            );
            let note_imported = take(eg_parse_anki_notes_tsv(
                session,
                anki_notes.as_ptr(),
                deck_id.as_ptr(),
                cstr("basic-reversed").as_ptr(),
                cstr("").as_ptr(),
                cstr("note").as_ptr(),
                NOW,
            ));
            assert!(note_imported.contains(r#""noteTypes":["#));
            assert!(note_imported.contains(r#""id":"basic-reversed""#));
            assert!(note_imported.contains(r#""id":"note-1::forward""#));
            assert!(note_imported.contains(r#""id":"note-1::reverse""#));

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_materialized_cards_return_lineage_json() {
        unsafe {
            let session = eg_session_new();
            let snapshot = cstr(
                r#"{
                    "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                    "noteTypes": [{
                        "id": "basic",
                        "name": "Basic",
                        "fields": [
                            {"id": "front", "name": "Front", "required": true, "ordinal": 0},
                            {"id": "back", "name": "Back", "required": true, "ordinal": 1}
                        ],
                        "templates": [{
                            "id": "forward",
                            "name": "Forward",
                            "frontTemplate": "{{Front}}",
                            "backTemplate": "{{Back}}",
                            "requiredFieldNames": ["Front", "Back"],
                            "ordinal": 0
                        }],
                        "createdAt": 1700000000000,
                        "updatedAt": 1700000000000
                    }],
                    "notes": [{
                        "id": "note",
                        "noteTypeId": "basic",
                        "deckId": "deck",
                        "fields": [
                            {"fieldId": "front", "value": "letter-a"},
                            {"fieldId": "back", "value": "a"}
                        ],
                        "tags": ["tamil"],
                        "createdAt": 1700000000000,
                        "updatedAt": 1700000000000
                    }],
                    "cards": [],
                    "cardProgress": [],
                    "sessions": [],
                    "reviews": [],
                    "activeSession": null
                }"#,
            );
            take(eg_load_snapshot(session, snapshot.as_ptr()));

            let note_type_id = cstr("basic");
            let note_id = cstr("note");
            let cards = take(eg_materialized_cards(
                session,
                note_type_id.as_ptr(),
                note_id.as_ptr(),
                NOW + 1,
            ));

            assert!(cards.contains(r#""id":"note::forward""#));
            assert!(cards.contains(r#""createdAt":1700000000001"#));
            assert!(cards.contains(r#""lineage":{"#));
            assert!(cards.contains(r#""templateId":"forward""#));

            let exported = take(eg_export_anki_notes_tsv(
                session,
                note_type_id.as_ptr(),
                cstr("deck").as_ptr(),
                cstr("Tamil::Script").as_ptr(),
                cstr("").as_ptr(),
                0,
            ));
            assert!(exported.contains(r#"#columns:Front\tBack\tTags"#));
            assert!(exported.contains(r#"letter-a\ta\ttamil"#));

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_generated_cards_return_cloze_json() {
        unsafe {
            let session = eg_session_new();
            let snapshot = cstr(
                r#"{
                    "decks": [{"id":"deck","name":"Spanish","description":"Grammar","createdAt":1700000000000}],
                    "noteTypes": [{
                        "id": "cloze",
                        "name": "Cloze",
                        "fields": [
                            {"id": "text", "name": "Text", "required": true, "ordinal": 0},
                            {"id": "extra", "name": "Extra", "required": false, "ordinal": 1}
                        ],
                        "templates": [{
                            "id": "cloze",
                            "name": "Cloze",
                            "frontTemplate": "{{cloze:Text}}",
                            "backTemplate": "{{cloze:Text}}<hr>{{Extra}}",
                            "requiredFieldNames": ["Text"],
                            "ordinal": 0
                        }],
                        "createdAt": 1700000000000,
                        "updatedAt": 1700000000000
                    }],
                    "notes": [{
                        "id": "note",
                        "noteTypeId": "cloze",
                        "deckId": "deck",
                        "fields": [
                            {"fieldId": "text", "value": "{{c1::root::base}} plus {{c2::suffix}}"},
                            {"fieldId": "extra", "value": "etymology"}
                        ],
                        "tags": ["grammar"],
                        "createdAt": 1700000000000,
                        "updatedAt": 1700000000000
                    }],
                    "cards": [],
                    "cardProgress": [],
                    "sessions": [],
                    "reviews": [],
                    "activeSession": null
                }"#,
            );
            take(eg_load_snapshot(session, snapshot.as_ptr()));

            let note_type_id = cstr("cloze");
            let note_id = cstr("note");
            let cards = take(eg_materialized_cards(
                session,
                note_type_id.as_ptr(),
                note_id.as_ptr(),
                NOW + 1,
            ));

            assert!(cards.contains(r#""id":"note::cloze::c1""#));
            assert!(cards.contains(r#""front":"[base] plus suffix""#));
            assert!(cards.contains(r#""clozeOrdinal":1"#));
            assert!(cards.contains(r#""id":"note::cloze::c2""#));

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_session_progress_returns_json() {
        unsafe {
            let session = eg_session_new();
            let snapshot = cstr(
                r#"{
                    "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                    "noteTypes": [],
                    "notes": [],
                    "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
                    "cardProgress": [],
                    "sessions": [],
                    "reviews": [],
                    "activeSession": null
                }"#,
            );
            take(eg_load_snapshot(session, snapshot.as_ptr()));

            let command = cstr(
                r#"{
                    "type": "startSession",
                    "sessionId": "session",
                    "deckId": "deck",
                    "queue": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
                    "startedAt": 1700000000000
                }"#,
            );
            take(eg_dispatch(session, command.as_ptr()));

            let progress = take(eg_session_progress(session));
            assert!(progress.contains(r#""progress":{"#));
            assert!(progress.contains(r#""totalCards":1"#));
            assert!(progress.contains(r#""remainingCards":1"#));

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_engram_app_props_return_mosaic_slot_json() {
        unsafe {
            let session = eg_session_new();
            let snapshot = cstr(
                r#"{
                    "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                    "noteTypes": [],
                    "notes": [],
                    "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
                    "cardProgress": [],
                    "sessions": [],
                    "reviews": [],
                    "activeSession": null
                }"#,
            );
            take(eg_load_snapshot(session, snapshot.as_ptr()));

            let deck_id = cstr("deck");
            let props = take(eg_engram_app_props(session, deck_id.as_ptr(), NOW));
            let props: Value = serde_json::from_str(&props).unwrap();

            assert_eq!(props["ok"], true);
            assert_eq!(props["props"]["app-title"], "Engram");
            assert_eq!(props["props"]["deck-name"], "Tamil");
            assert_eq!(props["props"]["deck-total-value"], "1");
            assert_eq!(props["props"]["answer-visible"], false);
            assert_eq!(props["props"]["action-undo-label"], "Undo");
            assert_eq!(props["props"]["action-bury-card-label"], "Bury card");
            assert_eq!(
                props["props"]["action-bury-siblings-label"],
                "Bury siblings"
            );
            assert_eq!(props["props"]["action-suspend-card-label"], "Suspend");
            assert_eq!(props["props"]["action-mark-label"], "Mark");

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_handles_engram_app_events_for_native_shells() {
        unsafe {
            let session = eg_session_new();
            let snapshot = cstr(
                r#"{
                    "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                    "noteTypes": [],
                    "notes": [],
                    "cards": [
                        {"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
                        {"id":"other","deckId":"deck","front":"letter-aa","back":"aa","createdAt":1700000000000}
                    ],
                    "cardProgress": [],
                    "sessions": [],
                    "reviews": [],
                    "activeSession": null
                }"#,
            );
            take(eg_load_snapshot(session, snapshot.as_ptr()));

            let start_session = cstr(
                r#"{
                    "type": "startSession",
                    "sessionId": "session",
                    "deckId": "deck",
                    "queue": [
                        {"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000},
                        {"id":"other","deckId":"deck","front":"letter-aa","back":"aa","createdAt":1700000000000}
                    ],
                    "startedAt": 1700000000000
                }"#,
            );
            take(eg_dispatch(session, start_session.as_ptr()));

            let deck_id = cstr("deck");
            let reveal = cstr("reveal");
            let revealed = take(eg_handle_engram_app_event(
                session,
                reveal.as_ptr(),
                deck_id.as_ptr(),
                NOW,
            ));
            let revealed: Value = serde_json::from_str(&revealed).unwrap();
            assert_eq!(revealed["ok"], true);
            assert_eq!(revealed["event"], "onReveal");
            assert_eq!(revealed["props"]["answer-visible"], true);

            let good = cstr("onGood");
            let rated = take(eg_handle_engram_app_event(
                session,
                good.as_ptr(),
                deck_id.as_ptr(),
                NOW + 1,
            ));
            let rated: Value = serde_json::from_str(&rated).unwrap();
            assert_eq!(rated["ok"], true);
            assert_eq!(rated["event"], "onGood");
            assert_eq!(rated["props"]["prompt"], "letter-aa");
            assert_eq!(rated["state"]["reviews"][0]["rating"], "good");

            let undo = cstr("undo");
            let undone = take(eg_handle_engram_app_event(
                session,
                undo.as_ptr(),
                deck_id.as_ptr(),
                NOW + 2,
            ));
            let undone: Value = serde_json::from_str(&undone).unwrap();
            assert_eq!(undone["ok"], true);
            assert_eq!(undone["event"], "onUndo");
            assert!(undone["state"]["reviews"].as_array().unwrap().is_empty());
            assert_eq!(undone["props"]["prompt"], "letter-a");

            let mark = cstr("onToggleMark");
            let marked = take(eg_handle_engram_app_event(
                session,
                mark.as_ptr(),
                deck_id.as_ptr(),
                NOW + 3,
            ));
            let marked: Value = serde_json::from_str(&marked).unwrap();
            assert_eq!(marked["ok"], true);
            assert_eq!(marked["event"], "onToggleMark");
            assert_eq!(marked["props"]["action-mark-label"], "Unmark");

            let bury = cstr("bury-card");
            let buried = take(eg_handle_engram_app_event(
                session,
                bury.as_ptr(),
                deck_id.as_ptr(),
                NOW + 4,
            ));
            let buried: Value = serde_json::from_str(&buried).unwrap();
            assert_eq!(buried["ok"], true);
            assert_eq!(buried["event"], "onBuryCard");
            assert_eq!(buried["props"]["prompt"], "letter-aa");

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_review_history_returns_json() {
        unsafe {
            let session = eg_session_new();
            let snapshot = cstr(
                r#"{
                    "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                    "noteTypes": [],
                    "notes": [],
                    "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
                    "cardProgress": [],
                    "sessions": [],
                    "reviews": [],
                    "activeSession": null
                }"#,
            );
            take(eg_load_snapshot(session, snapshot.as_ptr()));

            let start_session = cstr(
                r#"{
                    "type": "startSession",
                    "sessionId": "session",
                    "deckId": "deck",
                    "queue": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
                    "startedAt": 1700000000000
                }"#,
            );
            take(eg_dispatch(session, start_session.as_ptr()));

            let command = cstr(
                r#"{
                    "type": "rateCard",
                    "reviewId": "review",
                    "sessionId": "session",
                    "cardId": "card",
                    "rating": "easy",
                    "reviewedAt": 1700000000010
                }"#,
            );
            take(eg_dispatch(session, command.as_ptr()));

            let deck_id = cstr("deck");
            let history = take(eg_review_history(session, deck_id.as_ptr(), NOW, NOW + 100));
            assert!(history.contains(r#""history":{"#));
            assert!(history.contains(r#""totalReviews":1"#));
            assert!(history.contains(r#""easy":1"#));

            eg_session_free(session);
        }
    }

    #[test]
    fn c_abi_daily_limits_return_json() {
        unsafe {
            let session = eg_session_new();
            let snapshot = cstr(
                r#"{
                    "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
                    "noteTypes": [],
                    "notes": [],
                    "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
                    "cardProgress": [],
                    "sessions": [],
                    "reviews": [],
                    "activeSession": null
                }"#,
            );
            take(eg_load_snapshot(session, snapshot.as_ptr()));

            let start_session = cstr(
                r#"{
                    "type": "startSession",
                    "sessionId": "session",
                    "deckId": "deck",
                    "queue": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
                    "startedAt": 1700000000000
                }"#,
            );
            take(eg_dispatch(session, start_session.as_ptr()));

            let command = cstr(
                r#"{
                    "type": "rateCard",
                    "reviewId": "review",
                    "sessionId": "session",
                    "cardId": "card",
                    "rating": "good",
                    "reviewedAt": 1700000000010
                }"#,
            );
            take(eg_dispatch(session, command.as_ptr()));

            let deck_id = cstr("deck");
            let options = cstr(r#"{"newCardsPerDay":1,"reviewsPerDay":1}"#);
            let usage = take(eg_daily_limit_usage(
                session,
                deck_id.as_ptr(),
                NOW,
                NOW + 100,
                options.as_ptr(),
            ));
            assert!(usage.contains(r#""usage":{"#));
            assert!(usage.contains(r#""newCardsSeen":1"#));
            assert!(usage.contains(r#""remainingNewCards":0"#));

            let queue = take(eg_build_queue_with_daily_limits(
                session,
                deck_id.as_ptr(),
                NOW,
                NOW,
                NOW + 100,
                options.as_ptr(),
            ));
            assert!(queue.contains(r#""queue":[]"#));

            eg_session_free(session);
        }
    }

    #[test]
    fn null_handle_returns_null_and_free_is_safe() {
        unsafe {
            let deck_id = cstr("deck");
            assert!(eg_build_queue(ptr::null_mut(), deck_id.as_ptr(), NOW).is_null());
            eg_session_free(ptr::null_mut());
            eg_string_free(ptr::null_mut());
        }
    }
}
