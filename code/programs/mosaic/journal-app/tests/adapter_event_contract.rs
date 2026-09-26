//! Every event `JournalApp.mil` declares must be one the Rust app accepts — a
//! rejected one is a dead control in every native host, and nothing else would
//! notice because generated source is only ever compiled, not run. The list
//! comes from the compiled `.mil`, not a second hand-written copy.

use journal_mosaic_app::JournalMosaicApp;
use mosaic_app_runtime::{Event, MosaicApp, Platform, StartContext};
use serde_json::json;

/// The adapter's marker for a name it does not recognise. Any other error is
/// the domain declining an action (e.g. deleting while nothing is selected).
const UNKNOWN_EVENT_MARKER: &str = "unknown Journal event";

fn declared_events() -> Vec<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/JournalApp.mil");
    let mil = mosmodel_compiler::compile(&std::fs::read_to_string(path).unwrap()).unwrap();
    mil.component.emits.iter().map(|e| e.name.clone()).collect()
}

fn started() -> JournalMosaicApp {
    let mut app = JournalMosaicApp::default();
    app.start(StartContext::new("en-US", Platform::Linux)).unwrap();
    app
}

#[test]
fn every_declared_event_is_routed_by_the_adapter() {
    let declared = declared_events();
    assert_eq!(declared.len(), 22, "if this changed, the adapter must keep up too");
    for (i, name) in declared.iter().enumerate() {
        let mut app = started();
        let payload = json!({ "value": "x", "index": 0 });
        if let Err(error) = app.dispatch(Event::new(i as u64 + 1, name.clone(), payload)) {
            assert!(
                !error.to_string().contains(UNKNOWN_EVENT_MARKER),
                "{name} is declared but the adapter does not route it: {error}"
            );
        }
    }
}

#[test]
fn undeclared_events_are_still_rejected() {
    let error = started()
        .dispatch(Event::new(1, "onNotAJournalEvent", json!({})))
        .expect_err("an undeclared event must be rejected");
    assert!(error.to_string().contains(UNKNOWN_EVENT_MARKER));
    assert!(error.to_string().contains("onNotAJournalEvent"));
}
