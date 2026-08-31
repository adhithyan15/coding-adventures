//! # The standard Mosaic application adapter for Engram
//!
//! Mosaic's generated native hosts — Qt, SwiftUI, XAML, Flutter, Compose — all
//! speak one small C ABI: create an app, dispatch events at it, read back props,
//! snapshot and restore. A crate that implements [`MosaicApp`] and invokes
//! `export_mosaic_app!` becomes the `libmosaic_app` those hosts load.
//!
//! Engram did not have one. It exposed `engram-capi` instead — a bespoke ABI of
//! roughly forty `eg_*` symbols — and each generated host bound to it through a
//! hand-written `MosaicHost` adapter shipped as a package asset. That works, but
//! it routes *around* `mosaic-app-capi` and `mosaic-app-runtime`, so the only
//! thing exercising the standard substrate end to end was a three-slot counter
//! fixture. Engram drives roughly 254 slots and 88 events across ten component
//! packages, two layout variants and two themes. This crate is what puts that
//! surface on the standard path — and what lets Engram enter the Mosaic runtime
//! lanes in CI, which bundle and byte-compare exactly this library.
//!
//! ## Why this is a thin wrapper
//!
//! Almost nothing here is new logic. [`EngramSession`] already exposes the two
//! calls the trait needs — `engram_app_props` and `handle_engram_app_event` —
//! and they already produce and accept precisely the slots and events that
//! `EngramApp.mil` declares. The Engram package's own test suite asserts that
//! bijection (`shared_engram_app_props_match_mosaic_slots`), so the contract is
//! pinned independently of this crate.
//!
//! What the adapter genuinely adds is the two things the Mosaic envelope does not
//! carry: a **selected-deck cursor** and a **clock**. Both facade calls take a
//! `deck_id` and a `now`, and an [`Event`] has neither.
//!
//! ## Native only, and why that matters here
//!
//! This crate is the artifact *native* hosts load. Browsers use `engram-wasm`,
//! which speaks its own linear-memory ABI over the same facade. That is why
//! reading the clock from [`std::time`] is fine: the one target where it would
//! be unavailable never loads this library.
//!
//! ## What this does not do
//!
//! It does not replace `engram-capi` or the hand-written host adapters, and it
//! cannot at protocol v1. Engram's Anki import and export return `hostIntent`
//! payloads so a host can open a file picker; the standard ABI's [`Effect`] is
//! serialised onto the wire but no generated host reads it, and the C header has
//! no effect-completion entry point, so an effect could never be answered. The
//! two mechanisms do not meet. This crate therefore sits alongside the existing
//! adapters rather than retiring them.
//!
//! [`Effect`]: mosaic_app_runtime::Effect

use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use engram_core_wasm::EngramSession;
use mosaic_app_runtime::{AppUpdate, Event, MosaicApp, Snapshot, StartContext};
use serde_json::{Map, Value};

/// Identifies the shape of [`EngramMosaicApp`]'s snapshot bytes.
///
/// The runtime rejects a snapshot whose schema or version does not match, so a
/// stored snapshot from an incompatible build is refused rather than silently
/// misread.
const SNAPSHOT_SCHEMA: &str = "engram-mosaic-app";

/// Bump when the snapshot payload's meaning changes.
const SNAPSHOT_VERSION: u32 = 1;

/// Errors this adapter can report to the Mosaic runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngramAppError {
    /// The core rejected an event, or the event was not one Engram declares.
    Event { event: String, message: String },
    /// The core could not produce props.
    Props(String),
    /// Snapshot bytes were not valid UTF-8 JSON, or the core rejected them.
    InvalidSnapshot(String),
}

impl fmt::Display for EngramAppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Event { event, message } => {
                write!(formatter, "Engram rejected event `{event}`: {message}")
            }
            Self::Props(message) => write!(formatter, "Engram could not build props: {message}"),
            Self::InvalidSnapshot(message) => write!(formatter, "invalid Engram snapshot: {message}"),
        }
    }
}

impl Error for EngramAppError {}

/// Engram behind the standard Mosaic application ABI.
pub struct EngramMosaicApp {
    session: EngramSession,
    /// Which deck the facade should treat as current.
    ///
    /// The Mosaic event envelope has no notion of a selected deck, and the facade
    /// takes one on every call, so the adapter has to remember it. Empty means
    /// "no explicit selection" — the facade then falls back to its own internal
    /// selection, which is what deck-selection events update.
    selected_deck_id: String,
}

impl Default for EngramMosaicApp {
    fn default() -> Self {
        Self {
            session: EngramSession::new(),
            selected_deck_id: String::new(),
        }
    }
}

/// Milliseconds since the Unix epoch.
///
/// Scheduling is time-dependent, so this cannot be a fixed constant: a card's due
/// state is computed against it. A clock that ran backwards would be worse than
/// one that is merely coarse, so a `SystemTime` before the epoch saturates to 0
/// rather than wrapping.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// Pull `{"ok": false, "error": "..."}` out of a facade reply.
///
/// The facade is string-in / JSON-out and reports failure in the payload rather
/// than by a Rust `Err`, so every call has to be inspected. A reply that is not
/// even parseable JSON is itself a failure worth surfacing, not something to
/// treat as success.
fn facade_error(reply: &str) -> Option<String> {
    let value: Value = match serde_json::from_str(reply) {
        Ok(value) => value,
        Err(error) => return Some(format!("unparseable reply: {error}")),
    };
    if value.get("ok").and_then(Value::as_bool) == Some(false) {
        return Some(
            value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_string(),
        );
    }
    None
}

/// Take the `props` object out of a facade reply.
fn facade_props(reply: &str) -> Result<Value, EngramAppError> {
    if let Some(message) = facade_error(reply) {
        return Err(EngramAppError::Props(message));
    }
    let value: Value = serde_json::from_str(reply)
        .map_err(|error| EngramAppError::Props(format!("unparseable reply: {error}")))?;
    value
        .get("props")
        .cloned()
        .ok_or_else(|| EngramAppError::Props("reply carried no `props`".to_string()))
}

impl EngramMosaicApp {
    /// Current props for the selected deck.
    fn props(&self) -> Result<Value, EngramAppError> {
        facade_props(
            &self
                .session
                .engram_app_props(&self.selected_deck_id, now_millis()),
        )
    }

    fn update(&self) -> Result<AppUpdate, EngramAppError> {
        Ok(AppUpdate::new(self.props()?))
    }

    /// Fold a Mosaic [`Event`] into the JSON object the facade parses.
    ///
    /// The facade reads the event name from `event` / `name` / `type` and takes
    /// its arguments from sibling keys (`value`, `index`, `cardId`, …). A Mosaic
    /// event carries the name separately from an arbitrary payload object, so the
    /// two are merged: payload fields first, then the name, which therefore wins
    /// if a payload ever carries a conflicting `event` key. That precedence is
    /// deliberate — the envelope's name is authoritative, and a payload must not
    /// be able to redirect dispatch to a different event.
    fn event_json(event: &Event) -> Value {
        let mut object = match &event.payload {
            Value::Object(fields) => fields.clone(),
            Value::Null => Map::new(),
            // A non-object payload still has a meaningful reading: it is the
            // event's value. Dropping it would silently lose the argument.
            other => {
                let mut fields = Map::new();
                fields.insert("value".to_string(), other.clone());
                fields
            }
        };
        object.insert("event".to_string(), Value::String(event.name.clone()));
        Value::Object(object)
    }
}

impl MosaicApp for EngramMosaicApp {
    type Error = EngramAppError;

    fn start(&mut self, context: StartContext) -> Result<AppUpdate, Self::Error> {
        if let Some(snapshot) = context.restored_snapshot {
            return self.restore(snapshot);
        }
        self.update()
    }

    fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error> {
        let payload = Self::event_json(&event);
        let reply = self.session.handle_engram_app_event(
            &payload.to_string(),
            &self.selected_deck_id,
            now_millis(),
        );

        if let Some(message) = facade_error(&reply) {
            // The facade applies an event atomically or not at all, so there is
            // no partial state to unwind here — unlike an adapter that mutates
            // its own fields step by step.
            return Err(EngramAppError::Event {
                event: event.name,
                message,
            });
        }

        // A reply may or may not carry props; ask for them explicitly rather
        // than depending on which events happen to include them.
        self.update()
    }

    fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error> {
        let reply = self.session.snapshot();
        if let Some(message) = facade_error(&reply) {
            return Err(EngramAppError::InvalidSnapshot(message));
        }
        // The facade wraps state as `{"ok": true, "state": {...}}`, while
        // `load_snapshot` expects the bare state object. Unwrap here so the two
        // halves of the round trip agree.
        let value: Value = serde_json::from_str(&reply)
            .map_err(|error| EngramAppError::InvalidSnapshot(error.to_string()))?;
        let state = value.get("state").cloned().ok_or_else(|| {
            EngramAppError::InvalidSnapshot("snapshot reply carried no `state`".to_string())
        })?;
        Ok(Some(Snapshot {
            schema: SNAPSHOT_SCHEMA.to_string(),
            version: SNAPSHOT_VERSION,
            bytes: serde_json::to_vec(&state)
                .map_err(|error| EngramAppError::InvalidSnapshot(error.to_string()))?,
        }))
    }

    fn restore(&mut self, snapshot: Snapshot) -> Result<AppUpdate, Self::Error> {
        if snapshot.schema != SNAPSHOT_SCHEMA {
            return Err(EngramAppError::InvalidSnapshot(format!(
                "expected schema `{SNAPSHOT_SCHEMA}`, got `{}`",
                snapshot.schema
            )));
        }
        if snapshot.version != SNAPSHOT_VERSION {
            return Err(EngramAppError::InvalidSnapshot(format!(
                "expected version {SNAPSHOT_VERSION}, got {}",
                snapshot.version
            )));
        }
        let json = String::from_utf8(snapshot.bytes)
            .map_err(|error| EngramAppError::InvalidSnapshot(error.to_string()))?;
        let reply = self.session.load_snapshot(&json);
        if let Some(message) = facade_error(&reply) {
            return Err(EngramAppError::InvalidSnapshot(message));
        }
        // `load_snapshot` resets the facade's own presentation cursor, so the
        // adapter's must not outlive it and point at a deck the restored
        // collection may not contain.
        self.selected_deck_id.clear();
        self.update()
    }
}

mosaic_app_capi::export_mosaic_app!(EngramMosaicApp, EngramMosaicApp::default());

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_app_runtime::{ColorScheme, Platform};

    fn start_context() -> StartContext {
        StartContext::new("en-US", Platform::Linux)
    }

    #[test]
    fn start_produces_props() {
        let mut app = EngramMosaicApp::default();
        let update = app.start(start_context()).expect("start must succeed");
        assert!(
            update.props.is_object(),
            "props must be an object, got {:?}",
            update.props
        );
    }

    /// The envelope's event name wins over a payload that carries its own.
    ///
    /// Worth pinning: without it, a payload field could redirect dispatch to a
    /// different event than the one the host actually raised.
    #[test]
    fn envelope_event_name_overrides_a_payload_event_key() {
        let event = Event::new(
            1,
            "showBrowserScreen",
            serde_json::json!({ "event": "showReviewScreen", "value": 3 }),
        );
        let json = EngramMosaicApp::event_json(&event);
        assert_eq!(json.get("event").and_then(Value::as_str), Some("showBrowserScreen"));
        assert_eq!(json.get("value").and_then(Value::as_i64), Some(3));
    }

    /// A non-object payload is read as the event's value rather than discarded.
    #[test]
    fn scalar_payload_becomes_a_value_field() {
        let event = Event::new(1, "selectDeck", serde_json::json!(2));
        let json = EngramMosaicApp::event_json(&event);
        assert_eq!(json.get("event").and_then(Value::as_str), Some("selectDeck"));
        assert_eq!(json.get("value").and_then(Value::as_i64), Some(2));
    }

    #[test]
    fn null_payload_yields_only_the_event_name() {
        let event = Event::new(1, "showDeckScreen", Value::Null);
        let json = EngramMosaicApp::event_json(&event);
        assert_eq!(json.as_object().map(Map::len), Some(1));
        assert_eq!(json.get("event").and_then(Value::as_str), Some("showDeckScreen"));
    }

    /// An event Engram does not declare must be reported, not silently ignored.
    #[test]
    fn unknown_events_are_rejected() {
        let mut app = EngramMosaicApp::default();
        app.start(start_context()).unwrap();
        let error = app
            .dispatch(Event::new(1, "definitelyNotAnEngramEvent", Value::Null))
            .expect_err("an undeclared event must be rejected");
        match error {
            EngramAppError::Event { event, .. } => {
                assert_eq!(event, "definitelyNotAnEngramEvent");
            }
            other => panic!("expected an Event error, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_round_trips_through_restore() {
        let mut app = EngramMosaicApp::default();
        app.start(start_context()).unwrap();

        let snapshot = app
            .snapshot()
            .expect("snapshot must succeed")
            .expect("Engram supports snapshots");
        assert_eq!(snapshot.schema, SNAPSHOT_SCHEMA);
        assert_eq!(snapshot.version, SNAPSHOT_VERSION);

        let mut restored = EngramMosaicApp::default();
        let update = restored.restore(snapshot).expect("restore must succeed");
        assert!(update.props.is_object());
    }

    /// A snapshot from a different schema or version is refused rather than
    /// misread as Engram state.
    #[test]
    fn foreign_snapshots_are_refused() {
        let mut app = EngramMosaicApp::default();

        let wrong_schema = Snapshot {
            schema: "task-mosaic-app".to_string(),
            version: SNAPSHOT_VERSION,
            bytes: b"{}".to_vec(),
        };
        assert!(matches!(
            app.restore(wrong_schema),
            Err(EngramAppError::InvalidSnapshot(_))
        ));

        let wrong_version = Snapshot {
            schema: SNAPSHOT_SCHEMA.to_string(),
            version: SNAPSHOT_VERSION + 1,
            bytes: b"{}".to_vec(),
        };
        assert!(matches!(
            app.restore(wrong_version),
            Err(EngramAppError::InvalidSnapshot(_))
        ));
    }

    /// Malformed snapshot bytes are an error, not a panic.
    #[test]
    fn corrupt_snapshot_bytes_error_rather_than_panic() {
        let mut app = EngramMosaicApp::default();
        let corrupt = Snapshot {
            schema: SNAPSHOT_SCHEMA.to_string(),
            version: SNAPSHOT_VERSION,
            bytes: vec![0xff, 0xfe, 0xfd],
        };
        assert!(matches!(
            app.restore(corrupt),
            Err(EngramAppError::InvalidSnapshot(_))
        ));
    }

    #[test]
    fn dark_color_scheme_start_still_produces_props() {
        let mut app = EngramMosaicApp::default();
        let mut context = start_context();
        context.color_scheme = ColorScheme::Dark;
        let update = app.start(context).expect("start must succeed");
        assert!(update.props.is_object());
    }
}
