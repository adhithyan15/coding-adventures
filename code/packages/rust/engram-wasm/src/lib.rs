//! Linear-memory WASM ABI over `engram-core-wasm`.
//!
//! This crate is the browser/Electron boundary for Engram. It deliberately
//! mirrors the repo's `spreadsheet-wasm` convention: all app behavior stays in
//! `engram-core-wasm` and below; this layer only owns `extern "C"` exports,
//! UTF-8 string marshalling, and one global session suitable for
//! `wasm32-unknown-unknown`.

use std::alloc::{alloc as raw_alloc, dealloc as raw_dealloc, Layout};
use std::cell::RefCell;

use engram_core_wasm::EngramSession;

thread_local! {
    static SESSION: RefCell<EngramSession> = RefCell::new(EngramSession::new());
}

#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return std::ptr::null_mut();
    }
    let layout = match Layout::from_size_align(len, 1) {
        Ok(layout) => layout,
        Err(_) => return std::ptr::null_mut(),
    };
    unsafe { raw_alloc(layout) }
}

/// # Safety
/// `ptr` and `len` must exactly match a live allocation made by this module.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let layout = match Layout::from_size_align(len, 1) {
        Ok(layout) => layout,
        Err(_) => return,
    };
    unsafe { raw_dealloc(ptr, layout) };
}

unsafe fn read_input(ptr: *const u8, len: usize) -> String {
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).into_owned()
}

fn pack(value: String) -> *mut u8 {
    let bytes = value.into_bytes();
    let payload_len = bytes.len();
    let total = match payload_len.checked_add(4) {
        Some(total) => total,
        None => return std::ptr::null_mut(),
    };
    let layout = match Layout::from_size_align(total, 1) {
        Ok(layout) => layout,
        Err(_) => return std::ptr::null_mut(),
    };

    unsafe {
        let ptr = raw_alloc(layout);
        if ptr.is_null() {
            return ptr;
        }
        let len_prefix = (payload_len as u32).to_le_bytes();
        std::ptr::copy_nonoverlapping(len_prefix.as_ptr(), ptr, 4);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), payload_len);
        ptr
    }
}

#[no_mangle]
pub extern "C" fn reset() {
    SESSION.with(|session| *session.borrow_mut() = EngramSession::new());
}

#[no_mangle]
pub extern "C" fn snapshot() -> *mut u8 {
    pack(SESSION.with(|session| session.borrow().snapshot()))
}

#[no_mangle]
pub extern "C" fn get_state() -> *mut u8 {
    snapshot()
}

/// # Safety
/// `snapshot_ptr` must point to `snapshot_len` readable bytes, or be null with
/// a zero length.
#[no_mangle]
pub unsafe extern "C" fn load_snapshot(snapshot_ptr: *const u8, snapshot_len: usize) -> *mut u8 {
    let snapshot_json = unsafe { read_input(snapshot_ptr, snapshot_len) };
    pack(SESSION.with(|session| session.borrow_mut().load_snapshot(&snapshot_json)))
}

/// # Safety
/// `command_ptr` must point to `command_len` readable bytes, or be null with a
/// zero length.
#[no_mangle]
pub unsafe extern "C" fn dispatch(command_ptr: *const u8, command_len: usize) -> *mut u8 {
    let command_json = unsafe { read_input(command_ptr, command_len) };
    pack(SESSION.with(|session| session.borrow_mut().dispatch(&command_json)))
}

/// # Safety
/// `deck_ptr` must point to `deck_len` readable bytes, or be null with a zero
/// length.
#[no_mangle]
pub unsafe extern "C" fn build_queue(deck_ptr: *const u8, deck_len: usize, now: u64) -> *mut u8 {
    let deck_id = unsafe { read_input(deck_ptr, deck_len) };
    pack(SESSION.with(|session| session.borrow().build_queue(&deck_id, now)))
}

/// # Safety
/// `deck_ptr` must point to `deck_len` readable bytes, or be null with a zero
/// length.
#[no_mangle]
pub unsafe extern "C" fn get_deck_stats(deck_ptr: *const u8, deck_len: usize, now: u64) -> *mut u8 {
    let deck_id = unsafe { read_input(deck_ptr, deck_len) };
    pack(SESSION.with(|session| session.borrow().deck_stats(&deck_id, now)))
}

/// # Safety
/// `deck_ptr` must point to `deck_len` readable bytes, or be null with a zero
/// length.
#[no_mangle]
pub unsafe extern "C" fn empty_filtered_deck(deck_ptr: *const u8, deck_len: usize) -> *mut u8 {
    let deck_id = unsafe { read_input(deck_ptr, deck_len) };
    pack(SESSION.with(|session| session.borrow_mut().empty_filtered_deck(&deck_id)))
}

/// # Safety
/// The deck and query `(ptr, len)` pairs must point to readable bytes, or be
/// null with zero length.
#[no_mangle]
pub unsafe extern "C" fn rebuild_filtered_deck(
    deck_ptr: *const u8,
    deck_len: usize,
    query_ptr: *const u8,
    query_len: usize,
    limit: usize,
    reschedule: bool,
    rebuilt_at: u64,
) -> *mut u8 {
    let deck_id = unsafe { read_input(deck_ptr, deck_len) };
    let query = unsafe { read_input(query_ptr, query_len) };
    pack(SESSION.with(|session| {
        session
            .borrow_mut()
            .rebuild_filtered_deck(&deck_id, &query, limit, reschedule, rebuilt_at)
    }))
}

#[no_mangle]
pub extern "C" fn session_progress() -> *mut u8 {
    pack(SESSION.with(|session| session.borrow().session_progress()))
}

/// # Safety
/// `deck_ptr` must point to `deck_len` readable bytes, or be null with a zero
/// length.
#[no_mangle]
pub unsafe extern "C" fn review_history(
    deck_ptr: *const u8,
    deck_len: usize,
    reviewed_after: u64,
    reviewed_before: u64,
) -> *mut u8 {
    let deck_id = unsafe { read_input(deck_ptr, deck_len) };
    pack(SESSION.with(|session| {
        session
            .borrow()
            .review_history(&deck_id, reviewed_after, reviewed_before)
    }))
}

/// # Safety
/// `query_ptr` must point to `query_len` readable bytes, or be null with a zero
/// length.
#[no_mangle]
pub unsafe extern "C" fn search_cards(query_ptr: *const u8, query_len: usize, now: u64) -> *mut u8 {
    let query = unsafe { read_input(query_ptr, query_len) };
    pack(SESSION.with(|session| session.borrow().search_cards(&query, now)))
}

/// # Safety
/// `deck_ptr` must point to `deck_len` readable bytes, or be null with a zero
/// length.
#[no_mangle]
pub unsafe extern "C" fn engram_app_props(
    deck_ptr: *const u8,
    deck_len: usize,
    now: u64,
) -> *mut u8 {
    let deck_id = unsafe { read_input(deck_ptr, deck_len) };
    pack(SESSION.with(|session| session.borrow().engram_app_props(&deck_id, now)))
}

/// # Safety
/// `query_ptr` must point to `query_len` readable bytes, or be null with a zero
/// length.
#[no_mangle]
pub unsafe extern "C" fn engram_browser_props(
    query_ptr: *const u8,
    query_len: usize,
    now: u64,
) -> *mut u8 {
    let query = unsafe { read_input(query_ptr, query_len) };
    pack(SESSION.with(|session| session.borrow().engram_browser_props(&query, now)))
}

/// # Safety
/// The event and deck `(ptr, len)` pairs must point to readable bytes, or be
/// null with zero length.
#[no_mangle]
pub unsafe extern "C" fn handle_engram_app_event(
    event_ptr: *const u8,
    event_len: usize,
    deck_ptr: *const u8,
    deck_len: usize,
    now: u64,
) -> *mut u8 {
    let event = unsafe { read_input(event_ptr, event_len) };
    let deck_id = unsafe { read_input(deck_ptr, deck_len) };
    pack(SESSION.with(|session| {
        session
            .borrow_mut()
            .handle_engram_app_event(&event, &deck_id, now)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_700_000_000_000;

    fn put(value: &str) -> (*mut u8, usize) {
        let bytes = value.as_bytes();
        let ptr = alloc(bytes.len());
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
        (ptr, bytes.len())
    }

    fn take(ptr: *mut u8) -> String {
        unsafe {
            let len = u32::from_le_bytes([*ptr, *ptr.add(1), *ptr.add(2), *ptr.add(3)]) as usize;
            let bytes = std::slice::from_raw_parts(ptr.add(4), len).to_vec();
            dealloc(ptr, 4 + len);
            String::from_utf8(bytes).unwrap()
        }
    }

    fn call_str1(f: unsafe extern "C" fn(*const u8, usize) -> *mut u8, value: &str) -> String {
        let (ptr, len) = put(value);
        let out = unsafe { f(ptr, len) };
        unsafe { dealloc(ptr, len) };
        take(out)
    }

    fn call_deck_now(
        f: unsafe extern "C" fn(*const u8, usize, u64) -> *mut u8,
        deck_id: &str,
        now: u64,
    ) -> String {
        let (ptr, len) = put(deck_id);
        let out = unsafe { f(ptr, len, now) };
        unsafe { dealloc(ptr, len) };
        take(out)
    }

    fn call_rebuild_filtered_deck(
        deck_id: &str,
        query: &str,
        limit: usize,
        reschedule: bool,
        rebuilt_at: u64,
    ) -> String {
        let (deck_ptr, deck_len) = put(deck_id);
        let (query_ptr, query_len) = put(query);
        let out = unsafe {
            rebuild_filtered_deck(
                deck_ptr, deck_len, query_ptr, query_len, limit, reschedule, rebuilt_at,
            )
        };
        unsafe {
            dealloc(deck_ptr, deck_len);
            dealloc(query_ptr, query_len);
        }
        take(out)
    }

    fn load_fixture() {
        reset();
        let snapshot = r#"{
            "decks": [{"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000}],
            "noteTypes": [],
            "notes": [],
            "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;
        let loaded = call_str1(load_snapshot, snapshot);
        assert!(loaded.contains(r#""ok":true"#), "{loaded}");
    }

    #[test]
    fn abi_snapshot_load_and_dispatch_round_trip() {
        load_fixture();
        let queue = r#"{
            "type": "startSession",
            "sessionId": "session",
            "deckId": "deck",
            "queue": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
            "startedAt": 1700000000000
        }"#;
        let started = call_str1(dispatch, queue);
        assert!(started.contains(r#""ok":true"#), "{started}");
        assert!(started.contains(r#""activeSession""#), "{started}");

        let snap = take(snapshot());
        assert!(snap.contains(r#""state""#), "{snap}");
        assert!(snap.contains(r#""sessionId":"session""#), "{snap}");
    }

    #[test]
    fn abi_app_props_expose_mosaic_slot_names() {
        load_fixture();
        let props = call_deck_now(engram_app_props, "deck", NOW);

        assert!(props.contains(r#""ok":true"#), "{props}");
        assert!(props.contains(r#""app-title":"Engram""#), "{props}");
        assert!(props.contains(r#""deck-name":"Tamil""#), "{props}");
        assert!(
            props.contains(r#""deck-options-bury-new-siblings-value":true"#),
            "{props}"
        );
        assert!(
            props.contains(r#""browser-result-card-ids":["card"]"#),
            "{props}"
        );
    }

    #[test]
    fn abi_filtered_deck_exports_rebuild_and_empty() {
        reset();
        let snapshot = r#"{
            "decks": [
                {"id":"deck","name":"Tamil","description":"Script","createdAt":1700000000000},
                {"id":"filtered","name":"Filtered::Today","description":"Custom study","createdAt":1700000000000}
            ],
            "noteTypes": [],
            "notes": [],
            "cards": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
            "cardProgress": [],
            "sessions": [],
            "reviews": [],
            "activeSession": null
        }"#;
        let loaded = call_str1(load_snapshot, snapshot);
        assert!(loaded.contains(r#""ok":true"#), "{loaded}");

        let rebuilt = call_rebuild_filtered_deck("filtered", "deck:Tamil", 5, false, NOW);
        assert!(rebuilt.contains(r#""ok":true"#), "{rebuilt}");
        assert!(rebuilt.contains(r#""deckId":"filtered""#), "{rebuilt}");
        assert!(rebuilt.contains(r#""originalDeckId":"deck""#), "{rebuilt}");

        let emptied = call_str1(empty_filtered_deck, "filtered");
        assert!(emptied.contains(r#""ok":true"#), "{emptied}");
        assert!(emptied.contains(r#""deckId":"deck""#), "{emptied}");
    }

    #[test]
    fn abi_handle_app_event_returns_updated_props() {
        load_fixture();
        let queue = r#"{
            "type": "startSession",
            "sessionId": "session",
            "deckId": "deck",
            "queue": [{"id":"card","deckId":"deck","front":"letter-a","back":"a","createdAt":1700000000000}],
            "startedAt": 1700000000000
        }"#;
        call_str1(dispatch, queue);

        let (event_ptr, event_len) = put("onReveal");
        let (deck_ptr, deck_len) = put("deck");
        let revealed =
            unsafe { handle_engram_app_event(event_ptr, event_len, deck_ptr, deck_len, NOW) };
        unsafe {
            dealloc(event_ptr, event_len);
            dealloc(deck_ptr, deck_len);
        }
        let revealed = take(revealed);

        assert!(revealed.contains(r#""event":"onReveal""#), "{revealed}");
        assert!(revealed.contains(r#""answer-visible":true"#), "{revealed}");
    }

    #[test]
    fn abi_handle_app_event_returns_host_intents() {
        load_fixture();
        let (event_ptr, event_len) = put("onImportAnki");
        let (deck_ptr, deck_len) = put("deck");
        let imported =
            unsafe { handle_engram_app_event(event_ptr, event_len, deck_ptr, deck_len, NOW) };
        unsafe {
            dealloc(event_ptr, event_len);
            dealloc(deck_ptr, deck_len);
        }
        let imported = take(imported);

        assert!(imported.contains(r#""event":"onImportAnki""#), "{imported}");
        assert!(imported.contains(r#""hostIntent""#), "{imported}");
        assert!(
            imported.contains(r#""accept":[".apkg",".colpkg"]"#),
            "{imported}"
        );
        assert!(imported.contains(r#""type":"importAnki""#), "{imported}");
    }

    #[test]
    fn abi_empty_inputs_return_json_errors_instead_of_trapping() {
        reset();
        let loaded = unsafe { load_snapshot(std::ptr::null(), 0) };
        let loaded = take(loaded);
        assert!(loaded.contains(r#""ok":false"#), "{loaded}");

        unsafe { dealloc(std::ptr::null_mut(), 0) };
    }
}
