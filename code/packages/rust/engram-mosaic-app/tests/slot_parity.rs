//! The adapter must be a faithful pass-through of the Engram facade.
//!
//! Two assertions elsewhere already pin the other halves of this contract:
//! `code/programs/mosaic/engram-app/tests/package_compiles.rs`'s
//! `shared_engram_app_props_match_mosaic_slots` asserts that the facade's prop
//! keys equal `EngramApp.mil`'s declared slots exactly, and the `.mil` is the
//! Mosaic UI's own source of truth.
//!
//! So this file deliberately does **not** re-parse the `.mil`. Re-deriving the
//! slot list with a second, hand-rolled parser would be a copy that can drift
//! from the compiler's reading of the same file, and it would pass or fail for
//! reasons unrelated to this crate. What is worth pinning here is narrower and
//! entirely this crate's responsibility: that wrapping the facade in the standard
//! Mosaic ABI neither drops, renames, nor invents a slot.
//!
//! Chain the three together and the guarantee is end to end: MIL slots == facade
//! props == what a generated native host receives.

use engram_core_wasm::EngramSession;
use engram_mosaic_app::EngramMosaicApp;
use mosaic_app_runtime::{MosaicApp, Platform, StartContext};
use serde_json::Value;

/// Prop keys the facade produces when called directly.
fn facade_prop_keys() -> Vec<String> {
    let session = EngramSession::new();
    // The `now` value only affects scheduling-derived *values*, never the key
    // set, so a fixed timestamp keeps this deterministic.
    let reply = session.engram_app_props("", 0);
    let value: Value = serde_json::from_str(&reply).expect("facade reply must be JSON");
    let props = value.get("props").expect("facade reply must carry props");
    let mut keys: Vec<String> = props
        .as_object()
        .expect("props must be an object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    keys
}

/// Prop keys a generated host receives through the standard ABI.
fn adapter_prop_keys() -> Vec<String> {
    let mut app = EngramMosaicApp::default();
    let update = app
        .start(StartContext::new("en-US", Platform::Linux))
        .expect("start must succeed");
    let mut keys: Vec<String> = update
        .props
        .as_object()
        .expect("props must be an object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    keys
}

#[test]
fn adapter_props_match_the_facade_exactly() {
    let facade = facade_prop_keys();
    let adapter = adapter_prop_keys();

    // Report the difference rather than just "not equal" — a bare inequality on
    // 254 keys is unreadable when it fails.
    let missing: Vec<_> = facade.iter().filter(|key| !adapter.contains(key)).collect();
    let extra: Vec<_> = adapter.iter().filter(|key| !facade.contains(key)).collect();
    assert!(
        missing.is_empty(),
        "the adapter dropped slots the facade produces: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "the adapter invented slots the facade does not produce: {extra:?}"
    );
    assert_eq!(facade, adapter);
}

/// A guard against the comparison above passing vacuously.
///
/// If both sides ever returned an empty object — a facade that failed, say, or a
/// props path that silently produced `{}` — the set comparison would be trivially
/// true and would report nothing wrong. Engram declares a large slot surface, so
/// requiring a substantial count makes the emptiness case fail loudly.
#[test]
fn the_slot_surface_is_substantial() {
    let keys = adapter_prop_keys();
    assert!(
        keys.len() > 200,
        "expected Engram's full slot surface, got only {} keys: {keys:?}",
        keys.len()
    );
}
