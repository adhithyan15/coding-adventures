//! Stable C ABI over `engram_core_wasm::EngramSession`.
//!
//! Native shells call this crate through an opaque session handle and
//! NUL-terminated UTF-8 strings. All string results are allocated by Rust and
//! must be released with `eg_string_free`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use engram_core_wasm::EngramSession;

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

unsafe fn read_cstr(value: *const c_char) -> String {
    if value.is_null() {
        return String::new();
    }
    CStr::from_ptr(value).to_string_lossy().into_owned()
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

fn into_cstr(value: String) -> *mut c_char {
    match CString::new(value) {
        Ok(value) => value.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_700_000_000_000;

    unsafe fn take(value: *mut c_char) -> String {
        let result = CStr::from_ptr(value).to_string_lossy().into_owned();
        eg_string_free(value);
        result
    }

    fn cstr(value: &str) -> CString {
        CString::new(value).unwrap()
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

            let basic = cstr("front,back\nletter-aa,aa\n");
            let imported = take(eg_parse_basic_cards_csv(
                session,
                basic.as_ptr(),
                deck_id.as_ptr(),
                cstr("import").as_ptr(),
                NOW,
            ));
            assert!(imported.contains(r#""id":"import-1""#));

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
