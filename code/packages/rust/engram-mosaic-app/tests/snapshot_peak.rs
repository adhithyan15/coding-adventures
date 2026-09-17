//! `snapshot()` costs a copy of the collection, not a parse of it (#14523).
//!
//! Version 2 of the payload unwrapped the facade's reply into a
//! `serde_json::Value`, inserted the adapter's one field into that object, and
//! serialised it again. That materialised the whole collection — decks, notes,
//! cards, the review log, media blobs — as a tree of `Value` nodes to add one
//! string, on the success path of an ordinary save. The same amplification cost
//! ~300 MB of peak RSS on a 41.6 MB collection when `ok_with` had it (#13671).
//!
//! Version 3 keeps the facade's document as an unparsed fragment, so the
//! collection is copied twice (the facade's reply, and the snapshot bytes) and
//! parsed zero times.
//!
//! The issue asks for this to be measured rather than asserted, because the
//! failure mode is a regression that still passes every functional test — which
//! is exactly how it came back the first time. So a counting global allocator
//! records the peak live heap while `snapshot()` runs, and the bound is stated
//! as a multiple of the payload.
//!
//! One file, one `#[test]`: a `#[global_allocator]` is process-wide and the
//! harness runs tests in parallel threads, so anything else running alongside
//! would be counted too.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use engram_mosaic_app::EngramMosaicApp;
use mosaic_app_runtime::{MosaicApp, Platform, Snapshot, StartContext};
use serde_json::Value;

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn record(size: usize) {
    let live = LIVE.fetch_add(size, Ordering::SeqCst) + size;
    PEAK.fetch_max(live, Ordering::SeqCst);
}

// SAFETY: every method forwards to `System` with the caller's own arguments;
// the counters are bookkeeping and never change what is returned.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            record(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
    }

    /// Counted as "new block allocated, then old block freed": the worst case a
    /// moving `realloc` reaches. An in-place grow is over-counted, which only
    /// makes the bound stricter.
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            record(new_size);
            LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn peak_during<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let base = LIVE.load(Ordering::SeqCst);
    PEAK.store(base, Ordering::SeqCst);
    let out = f();
    (out, PEAK.load(Ordering::SeqCst).saturating_sub(base))
}

/// A collection big enough for a per-node cost to show: the demo collection
/// with its notes, cards and review log repeated, and each copy given fresh
/// ids so nothing collides.
fn inflated_session_json(copies: usize) -> String {
    let demo = engram_core_wasm::EngramSession::new_demo();
    let reply: Value = serde_json::from_str(&demo.session_snapshot()).expect("a demo snapshot");
    let mut session = reply["session"].clone();
    let state = session["state"].as_object_mut().expect("a state object");

    for key in ["notes", "cards", "cardProgress", "reviews"] {
        let Some(rows) = state.get(key).and_then(Value::as_array).cloned() else {
            continue;
        };
        if rows.is_empty() {
            continue;
        }
        let mut grown = Vec::with_capacity(rows.len() * copies);
        for copy in 0..copies {
            for row in &rows {
                let mut row = row.clone();
                if let Some(object) = row.as_object_mut() {
                    for id_key in ["noteId", "cardId", "reviewId", "id"] {
                        if let Some(Value::String(id)) = object.get(id_key) {
                            let fresh = format!("{id}-{copy}");
                            object.insert(id_key.to_string(), Value::String(fresh));
                        }
                    }
                }
                grown.push(row);
            }
        }
        state.insert(key.to_string(), Value::Array(grown));
    }
    serde_json::to_string(&session).expect("a session document")
}

#[test]
fn snapshot_does_not_materialise_the_collection() {
    let session_json = inflated_session_json(400);
    assert!(
        session_json.len() > 1_000_000,
        "the fixture must be big enough for a per-node cost to show, got {} bytes",
        session_json.len()
    );

    // Restore it as a version-2 payload: the merged shape, which is also the
    // compatibility case this change has to keep working.
    let mut app = EngramMosaicApp::default();
    app.start(StartContext::new("en-US", Platform::Linux))
        .expect("start");
    app.restore(Snapshot {
        schema: "engram-mosaic-app".to_string(),
        version: 2,
        bytes: session_json.clone().into_bytes(),
    })
    .expect("a version-2 payload must still restore");

    let (snapshot, peak) = peak_during(|| app.snapshot().expect("snapshot").expect("supported"));
    let bytes = snapshot.bytes.len();
    assert_eq!(snapshot.version, 3);

    // What a correct `snapshot()` holds at peak: the facade's reply string, the
    // snapshot bytes it writes, and the growth of each while it is built — a
    // small constant times the payload, not a multiple that grows with how many
    // JSON nodes the collection happens to have.
    //
    // Measured on this fixture (1.4 MB): **5.0x** the payload with version 3,
    // and **18.8x** with the version-2 `Value` round trip restored in its
    // place, which is the regression this bounds.
    let bound = bytes * 8;
    eprintln!("payload {bytes} bytes, peak heap growth {peak} bytes ({:.1}x)", peak as f64 / bytes as f64);
    assert!(
        peak < bound,
        "snapshot() held {peak} bytes for a {bytes}-byte payload; a version that does \
         not parse the collection stays under {bound}"
    );
}
