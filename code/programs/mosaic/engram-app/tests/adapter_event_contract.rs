//! Every event `EngramApp.mil` declares must be one the adapter actually accepts.
//!
//! `EngramApp.mil` is the contract a generated host is built against: each `emit`
//! becomes a callback the host can raise. If the adapter rejects one, that button
//! is dead in every native shell — and nothing else in the tree would notice,
//! because the emitted source is only ever grepped for text, never run.
//!
//! This test lives here rather than in `engram-mosaic-app` deliberately. The
//! authoritative list of events is the compiled `.mil`, and the compiler lives on
//! this side. Re-deriving the list inside the adapter crate with a hand-rolled
//! parser would be a second reading of the same file that can drift from the
//! compiler's.

use std::collections::BTreeSet;

use engram_mosaic_app::EngramMosaicApp;
use mosaic_app_runtime::{Event, MosaicApp, Platform, StartContext};
use serde_json::Value;

fn read_source(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    })
}

fn declared_events() -> BTreeSet<String> {
    let mil = mosmodel_compiler::compile(&read_source("EngramApp.mil"))
        .expect("EngramApp.mil should compile");
    mil.component
        .emits
        .iter()
        .map(|emit| emit.name.clone())
        .collect()
}

/// A payload plausible enough that an event needing an argument can succeed.
///
/// Events differ in what they read — an index, a text value, a card id — and the
/// facade takes them from sibling keys, so supplying several at once lets one
/// generic payload serve every event. This is deliberately permissive: the
/// question here is whether the event is *recognised*, not whether one specific
/// argument shape is validated.
fn permissive_payload() -> Value {
    serde_json::json!({
        "value": 0,
        "index": 0,
        "checked": false,
        "cardId": "",
    })
}

/// The marker the facade uses for a name it does not recognise.
///
/// This is the distinction that makes the test below meaningful. Dispatching a
/// declared event against an empty collection legitimately fails — "cannot rate
/// without an active session", "cannot update deck options without a deck" — and
/// those are the domain refusing an action, not the adapter failing to route it.
/// Only this message means the event never reached Engram at all, which is the
/// failure that would leave a dead control in a generated host.
const UNKNOWN_EVENT_MARKER: &str = "unknown Engram app event";

#[test]
fn every_declared_event_is_routed_by_the_adapter() {
    let declared = declared_events();
    assert_eq!(
        declared.len(),
        88,
        "EngramApp.mil should declare 88 emits; if this changed, the adapter and \
         this test both need to keep up rather than the count being edited away"
    );

    let mut unrouted: Vec<(String, String)> = Vec::new();
    for (index, name) in declared.iter().enumerate() {
        // A fresh app per event: some events are only meaningful from a
        // particular screen, and a shared app would let an earlier event's state
        // change decide whether a later one is recognised.
        let mut app = EngramMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Linux))
            .expect("start must succeed");

        let event = Event::new(index as u64 + 1, name.clone(), permissive_payload());
        if let Err(error) = app.dispatch(event) {
            let message = error.to_string();
            if message.contains(UNKNOWN_EVENT_MARKER) {
                unrouted.push((name.clone(), message));
            }
            // Any other error is the domain declining the action on an empty
            // collection, which says nothing about routing.
        }
    }

    assert!(
        unrouted.is_empty(),
        "the adapter failed to route {} of {} declared events — a generated host \
         would have a dead control for each:\n{}",
        unrouted.len(),
        declared.len(),
        unrouted
            .iter()
            .map(|(name, error)| format!("  {name}: {error}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The test above must not pass by accepting everything.
///
/// If the adapter treated an unrecognised name as a no-op, every declared event
/// would "succeed" and the parity test would prove nothing. This pins the other
/// side: a name Engram does not declare has to be rejected.
#[test]
fn undeclared_events_are_still_rejected() {
    let mut app = EngramMosaicApp::default();
    app.start(StartContext::new("en-US", Platform::Linux))
        .expect("start must succeed");

    let error = app
        .dispatch(Event::new(1, "onThisIsNotAnEngramEvent", permissive_payload()))
        .expect_err("an undeclared event must be rejected");
    let message = error.to_string();
    assert!(
        message.contains(UNKNOWN_EVENT_MARKER),
        "an undeclared event must fail as unrouted, not as a domain error — \
         otherwise the parity test above cannot tell the two apart. Got: {message}"
    );
    assert!(
        message.contains("onThisIsNotAnEngramEvent"),
        "the error should name the offending event, got: {message}"
    );
}
