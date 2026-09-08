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
///
/// - **1** — the collection alone.
/// - **2** — the collection *and* the presentation cursor: which deck is
///   selected, which screen is showing, what is typed in the browser's search
///   box, how far into a review you are. Reopening Engram now puts the reader
///   back where they were instead of at the deck list.
const SNAPSHOT_VERSION: u32 = 2;

/// The oldest payload [`EngramMosaicApp::restore`] still understands.
///
/// A stored version-1 snapshot is a collection with no cursor, which is exactly
/// what a first launch after this change will find on disk. Refusing it would
/// throw away the reader's collection to avoid restoring their scroll position,
/// which is the wrong trade by a wide margin.
const OLDEST_SUPPORTED_SNAPSHOT_VERSION: u32 = 1;

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
            Self::InvalidSnapshot(message) => {
                write!(formatter, "invalid Engram snapshot: {message}")
            }
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
        let reply = self.session.session_snapshot();
        if let Some(message) = facade_error(&reply) {
            return Err(EngramAppError::InvalidSnapshot(message));
        }
        // The facade wraps the document as `{"ok": true, "session": {...}}`,
        // while `load_session_snapshot` expects the bare `{state, cursor}`
        // object. Unwrap here so the two halves of the round trip agree.
        let value: Value = serde_json::from_str(&reply)
            .map_err(|error| EngramAppError::InvalidSnapshot(error.to_string()))?;
        let mut session = value.get("session").cloned().ok_or_else(|| {
            EngramAppError::InvalidSnapshot("snapshot reply carried no `session`".to_string())
        })?;
        // The adapter's own deck selection is presentation state too, and it
        // lives here rather than in the facade -- so the facade's cursor cannot
        // carry it and this is the only place that can persist it. Leaving it
        // out would restore the screen and the search box but silently drop
        // which deck the reader was looking at, which is the most visible half.
        session
            .as_object_mut()
            .ok_or_else(|| {
                EngramAppError::InvalidSnapshot("`session` was not an object".to_string())
            })?
            .insert(
                "adapterSelectedDeckId".to_string(),
                Value::String(self.selected_deck_id.clone()),
            );
        Ok(Some(Snapshot {
            schema: SNAPSHOT_SCHEMA.to_string(),
            version: SNAPSHOT_VERSION,
            bytes: serde_json::to_vec(&session)
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
        if !(OLDEST_SUPPORTED_SNAPSHOT_VERSION..=SNAPSHOT_VERSION).contains(&snapshot.version) {
            return Err(EngramAppError::InvalidSnapshot(format!(
                "expected version {OLDEST_SUPPORTED_SNAPSHOT_VERSION}..={SNAPSHOT_VERSION}, got {}",
                snapshot.version
            )));
        }
        let version = snapshot.version;
        let json = String::from_utf8(snapshot.bytes)
            .map_err(|error| EngramAppError::InvalidSnapshot(error.to_string()))?;

        if version == 1 {
            // Version 1 bytes are the bare collection, with no cursor to
            // restore. `load_snapshot` resets the facade's cursor, so the
            // adapter's must not outlive it and point at a deck the restored
            // collection may not contain.
            let reply = self.session.load_snapshot(&json);
            if let Some(message) = facade_error(&reply) {
                return Err(EngramAppError::InvalidSnapshot(message));
            }
            self.selected_deck_id.clear();
            return self.update();
        }

        let reply = self.session.load_session_snapshot(&json);
        if let Some(message) = facade_error(&reply) {
            return Err(EngramAppError::InvalidSnapshot(message));
        }
        // Read the adapter's own half back. A document that omits it restores
        // an empty selection, which is the same "no explicit selection" the
        // adapter starts life with -- not a stale deck from before the restore.
        //
        // Only this one field is deserialised, NOT the whole document into a
        // `Value`. The document contains the entire collection -- media blobs
        // included -- and parsing it a second time here just to read one string
        // would materialise all of it again, on top of the copy
        // `load_session_snapshot` already built. `AdapterHalf` ignores every
        // other key, so the cost is the one string.
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct AdapterHalf {
            #[serde(default)]
            adapter_selected_deck_id: String,
        }

        let half: AdapterHalf = serde_json::from_str(&json)
            .map_err(|error| EngramAppError::InvalidSnapshot(error.to_string()))?;
        let candidate = half.adapter_selected_deck_id.as_str();
        // Check the id against the collection we just restored, because this
        // field does NOT get the check the facade's own cursor gets.
        //
        // Both end up in `selected_deck_id_with_override`, but at different
        // argument positions with different rules: the facade's cursor arrives
        // as the *override* and is filtered against `state.decks`, while this
        // one arrives as the explicit `deck_id` and is returned verbatim when
        // non-empty. Until this commit that asymmetry was unreachable — the
        // adapter's field was only ever `String::new()` or cleared, so the
        // unchecked path never saw a value. Restoring from a snapshot makes
        // these bytes its only writer, so the check has to happen here.
        //
        // An id no deck carries is not a selection; falling back to empty hands
        // resolution to the facade's checked path rather than letting a phantom
        // deck become the target of a subsequent write.
        self.selected_deck_id = if self
            .session
            .state()
            .decks
            .iter()
            .any(|deck| deck.id == candidate)
        {
            candidate.to_string()
        } else {
            String::new()
        };
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
        assert_eq!(
            json.get("event").and_then(Value::as_str),
            Some("showBrowserScreen")
        );
        assert_eq!(json.get("value").and_then(Value::as_i64), Some(3));
    }

    /// A non-object payload is read as the event's value rather than discarded.
    #[test]
    fn scalar_payload_becomes_a_value_field() {
        let event = Event::new(1, "selectDeck", serde_json::json!(2));
        let json = EngramMosaicApp::event_json(&event);
        assert_eq!(
            json.get("event").and_then(Value::as_str),
            Some("selectDeck")
        );
        assert_eq!(json.get("value").and_then(Value::as_i64), Some(2));
    }

    #[test]
    fn null_payload_yields_only_the_event_name() {
        let event = Event::new(1, "showDeckScreen", Value::Null);
        let json = EngramMosaicApp::event_json(&event);
        assert_eq!(json.as_object().map(Map::len), Some(1));
        assert_eq!(
            json.get("event").and_then(Value::as_str),
            Some("showDeckScreen")
        );
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

    /// Reopening Engram puts the reader back on the screen they left.
    ///
    /// The assertion is on a rendered prop rather than on internal fields,
    /// because that is the thing a person actually sees. Restoring a cursor the
    /// props then ignore would be no fix at all.
    #[test]
    fn restore_puts_the_reader_back_on_the_screen_they_left() {
        let mut app = EngramMosaicApp::default();
        app.start(start_context()).unwrap();
        app.dispatch(Event::new(1, "onShowBrowse", Value::Null))
            .expect("browse is a declared event");

        let left_on = app.update().expect("props must build");
        assert_eq!(
            left_on.props["show-browse-screen"], true,
            "the fixture must actually leave the deck list, or the test is vacuous"
        );

        let snapshot = app.snapshot().unwrap().expect("Engram supports snapshots");
        assert_eq!(snapshot.version, SNAPSHOT_VERSION);

        let mut restored = EngramMosaicApp::default();
        let update = restored.restore(snapshot).expect("restore must succeed");
        assert_eq!(
            update.props["show-browse-screen"], true,
            "restore dropped the screen the reader was on"
        );
        assert_eq!(update.props["show-decks-screen"], false);
    }

    /// A snapshot stored by the previous build still opens.
    ///
    /// Version 1 bytes are the bare collection. This is what a first launch
    /// after the upgrade finds on disk, so refusing it would mean the reader's
    /// collection fails to load — a far worse outcome than losing a cursor that
    /// version 1 never stored.
    #[test]
    fn a_version_one_snapshot_still_restores() {
        let mut source = EngramMosaicApp::default();
        source.start(start_context()).unwrap();
        // Version 1 payloads were exactly the facade's `state` object.
        let reply: Value = serde_json::from_str(&source.session.snapshot()).unwrap();
        let legacy_bytes = serde_json::to_vec(&reply["state"]).unwrap();

        let mut restored = EngramMosaicApp::default();
        let update = restored
            .restore(Snapshot {
                schema: SNAPSHOT_SCHEMA.to_string(),
                version: 1,
                bytes: legacy_bytes,
            })
            .expect("a version 1 snapshot must still restore");
        assert!(update.props.is_object());
        // No cursor was stored, so the reader lands on the deck list.
        assert_eq!(update.props["show-decks-screen"], true);
    }

    /// A snapshot naming a deck the collection does not contain is not trusted.
    ///
    /// This field takes the one path through `selected_deck_id_with_override`
    /// that does *not* check the id against `state.decks` — it is the explicit
    /// `deck_id` argument, returned verbatim when non-empty, where the facade's
    /// own cursor is the filtered override. Restoring makes snapshot bytes this
    /// field's only writer, so an unchecked value here would let a phantom deck
    /// become the target of a later write.
    #[test]
    fn a_restored_deck_id_the_collection_lacks_is_not_selected() {
        let mut source = EngramMosaicApp::default();
        source.start(start_context()).unwrap();
        let mut snapshot = source.snapshot().unwrap().unwrap();

        let mut document: Value = serde_json::from_slice(&snapshot.bytes).unwrap();
        document["adapterSelectedDeckId"] = Value::String("deck-that-does-not-exist".to_string());
        snapshot.bytes = serde_json::to_vec(&document).unwrap();

        let mut restored = EngramMosaicApp::default();
        restored.restore(snapshot).expect("restore must succeed");
        assert_eq!(
            restored.selected_deck_id, "",
            "a deck id no deck carries must not survive restore"
        );
    }

    /// A version this build predates is still refused.
    ///
    /// Widening the accepted range to take version 1 must not turn into
    /// accepting anything at all — bytes from a *newer* build would be misread.
    #[test]
    fn a_newer_snapshot_version_is_still_refused() {
        let mut app = EngramMosaicApp::default();
        app.start(start_context()).unwrap();
        let mut snapshot = app.snapshot().unwrap().unwrap();
        snapshot.version = SNAPSHOT_VERSION + 1;

        let mut restored = EngramMosaicApp::default();
        restored
            .restore(snapshot)
            .expect_err("a newer snapshot version must be refused");
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
