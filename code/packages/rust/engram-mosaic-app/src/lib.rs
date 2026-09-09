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
//! ## Anki import and export, as effects
//!
//! Engram's import and export need a file dialog, which only the host can open.
//! The facade reports that need as a `hostIntent`, and this adapter turns it
//! into a standard [`Effect`]: `importAnki` and `exportAnki` as
//! [`Delivery::Await`], because neither can proceed without the host and the
//! app has to know whether it happened; `openCard` as [`Delivery::Notify`],
//! because opening a card elsewhere is fire-and-forget and waiting on an answer
//! could only invent a way to wedge.
//!
//! This became possible when the fifth generated host learned to answer effects
//! (UI47 §5.4 step 4). Before that the two mechanisms did not meet: `Effect` was
//! serialised onto the wire, no generated host read it, and the C header had no
//! completion entry point — so an `Await` could never be answered, and emitting
//! one would have left the app waiting forever.
//!
//! **The bytes travel, not the path.** An export builds the package here and
//! sends it out in the payload for the host to write; an import comes back with
//! the package the host read. Every native target can be sandboxed — macOS most
//! strictly — and there a process may open only what the user picked in the
//! host's own dialog, so keeping all filesystem access on the host side is the
//! one arrangement that works on all five.
//!
//! ## What this does not do
//!
//! It does not replace `engram-capi` or the hand-written host adapters. Intents
//! other than those three still ride `hostIntent` for them, and are deliberately
//! *not* minted as effects: an `Await` nothing answers wedges snapshot and
//! restore for the life of the process, which is the exact failure the hosts'
//! sweeps exist to prevent.
//!
//! [`Effect`]: mosaic_app_runtime::Effect
//! [`Delivery::Await`]: mosaic_app_runtime::Delivery::Await
//! [`Delivery::Notify`]: mosaic_app_runtime::Delivery::Notify

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use engram_core_wasm::EngramSession;
use mosaic_app_runtime::{
    AppUpdate, Delivery, Effect, EffectCompletionError, EffectId, EffectResult, Event, MosaicApp,
    Snapshot, StartContext, MAX_EFFECT_ID,
};
use serde_json::{Map, Value};

/// The largest base64 payload this adapter will decode.
///
/// Sized against the package layer's own expansion ceiling, plus the 4/3 base64
/// overhead and room for the archive around it. A package past this is refused
/// with a sentence rather than by running out of memory.
const MAX_IMPORT_BASE64_LEN: usize = 512 * 1024 * 1024;

/// Trim text from outside this process before it reaches a reader.
///
/// Package-layer errors interpolate names lifted out of the archive -- a zip
/// entry name is up to 65535 arbitrary bytes -- and host failure messages are
/// whatever the host wrote. Both land in a prop rendered by five native
/// toolkits, and at least one of them (Qt's `QLabel`, on `Qt::AutoText`)
/// detects and interprets markup. Control characters go, and the length is cut
/// to something a person would read, so the boundary is enforced HERE rather
/// than trusted to five renderers.
fn reader_safe(message: &str) -> String {
    let trimmed: String = message
        .chars()
        .filter(|c| {
            // Control characters, and the markup delimiters the doc comment
            // above is actually about: `is_control` alone left `<`, `>` and `&`
            // untouched, so a zip entry name of `<img src=...>` would reach the
            // prop intact and be RENDERED by a toolkit that auto-detects rich
            // text. A filter whose comment claims more than it does is worse
            // than no filter.
            //
            // Format characters go too. `is_control` matches category Cc only,
            // so U+202E RLO and the directional isolates survived it -- enough
            // to make the status line appear to say something it does not,
            // within the 200 characters below.
            !c.is_control()
                && !matches!(c, '<' | '>' | '&')
                && !matches!(
                    u32::from(*c),
                    0x200B..=0x200F | 0x202A..=0x202E | 0x2066..=0x2069
                )
        })
        // AFTER the filter, so padding cannot push markup past the window. If
        // this ever becomes entity escaping rather than dropping, escape after
        // truncating -- cutting `&amp;` in half produces new nonsense.
        .take(200)
        .collect();
    if trimmed.trim().is_empty() {
        "no details given".to_string()
    } else {
        trimmed
    }
}

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

/// What the host is being asked to do, for an effect it has not answered yet.
///
/// Kept because the answer arrives by id alone: `complete_effect` is handed an
/// [`EffectId`] and an [`EffectResult`], with nothing to say which request it
/// belongs to. Reading the kind back off the id is the only way to know whether
/// a returned package is one to merge or one that has just been written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingEffect {
    /// The host is choosing a package to import; its answer carries the bytes.
    Import,
    /// The host is writing a package this app already produced; the answer only
    /// says whether it landed.
    Export,
}

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
    /// Next id to mint. Monotonic, and never reused.
    ///
    /// Bounded by [`MAX_EFFECT_ID`] rather than `u64::MAX` because the id rides
    /// a JSON number all the way to hosts whose only integer is a double: JNA,
    /// Dart and JavaScript all read it as one. An id past 2^53-1 would arrive
    /// rounded, and answering a rounded id answers a *different* effect. Every
    /// host refuses such an id on the way in; minting one would just guarantee
    /// the refusal.
    next_effect_id: EffectId,
    /// Effects the host owes an answer for, by id.
    pending_effects: BTreeMap<EffectId, PendingEffect>,
    /// The outcome of the last import or export, for the UI to show.
    ///
    /// The facade owns Engram's state and knows nothing about host dialogs, so
    /// it cannot report "the file you chose was not a package" — that sentence
    /// only exists here. Carried as a prop so a cancelled or failed import says
    /// something rather than leaving the screen unchanged and the reader
    /// guessing whether anything happened.
    last_transfer: Option<String>,
    /// The protocol version the host started this app with.
    ///
    /// Effects are minted only at [`EFFECT_PROTOCOL_VERSION`] or above. The
    /// runtime does not merely ignore an `Await` from a v1 host -- it fails the
    /// call with `EffectsRequireV2` and **poisons the instance**, so Engram
    /// would be bricked by its first import rather than degraded. Below v2 the
    /// intents ride `hostIntent` to the hand-written adapters exactly as they
    /// did before this change, which is a working import, not a broken one.
    protocol_version: u32,
}

impl Default for EngramMosaicApp {
    fn default() -> Self {
        Self {
            session: EngramSession::new(),
            selected_deck_id: String::new(),
            next_effect_id: 0,
            pending_effects: BTreeMap::new(),
            last_transfer: None,
            // Assume the worst until `start` says otherwise: a default-built
            // app that never saw a StartContext must not mint effects.
            protocol_version: 1,
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
    match serde_json::from_str::<FacadeAck>(reply) {
        Err(error) => Some(format!("unparseable reply: {error}")),
        Ok(ack) if !ack.ok => Some(ack.error.unwrap_or_else(|| "unknown error".to_string())),
        Ok(_) => None,
    }
}

/// Just the acknowledgement fields, so a reply is never retained whole.
///
/// `merge_anki_apkg` answers with the ENTIRE post-merge collection, media
/// included as base64. Parsing that into a `Value` to read one boolean
/// materialises a tree with a node per note field, per card and per tag --
/// hundreds of megabytes of `Value` for a reply that is being consulted for a
/// single bit, and driven by a file the reader was handed. serde walks the
/// document and keeps only these two fields.
///
/// The same reasoning as [`EngramMosaicApp::restore`]'s `AdapterHalf`, applied
/// to the reply side.
#[derive(serde::Deserialize)]
struct FacadeAck {
    #[serde(default = "yes")]
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

/// A reply with no `ok` key is a success; only an explicit `false` is failure.
fn yes() -> bool {
    true
}

/// Take one named field out of a successful facade reply.
///
/// Same contract as [`facade_props`]: the facade reports failure in the payload
/// rather than by a Rust `Err`, so a reply has to be inspected before its
/// contents are trusted.
fn facade_package(reply: &str) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct PackageReply {
        #[serde(default = "yes")]
        ok: bool,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        apkg: Option<String>,
    }
    // One pass, and the package string is MOVED out rather than cloned: it is
    // the whole exported collection, so parsing twice and then copying it would
    // hold three of them at once.
    let reply: PackageReply =
        serde_json::from_str(reply).map_err(|error| format!("unparseable reply: {error}"))?;
    if !reply.ok {
        return Err(reply.error.unwrap_or_else(|| "unknown error".to_string()));
    }
    reply
        .apkg
        .ok_or_else(|| "reply carried no `apkg`".to_string())
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

    /// Props, plus the transfer outcome the facade cannot know about.
    fn props_with_transfer(&self) -> Result<Value, EngramAppError> {
        let mut props = self.props()?;
        let Some(message) = &self.last_transfer else {
            return Ok(props);
        };
        // Only if props are an object; a non-object would mean the facade
        // changed shape underneath us, and silently reshaping it here would
        // hide that rather than let the runtime's own validation see it.
        if let Value::Object(fields) = &mut props {
            fields.insert(
                "anki-transfer-status".to_string(),
                Value::String(message.clone()),
            );
        }
        Ok(props)
    }

    fn update(&self) -> Result<AppUpdate, EngramAppError> {
        Ok(AppUpdate::new(self.props_with_transfer()?))
    }

    /// Mint the next effect id.
    ///
    /// Saturating rather than wrapping: reuse would let a late answer to a
    /// long-dead effect land on a live one. An app that somehow minted 2^53
    /// effects in one session stops being able to request them, which is the
    /// safe direction to fail.
    fn mint_effect_id(&mut self) -> Option<EffectId> {
        if self.next_effect_id >= MAX_EFFECT_ID {
            self.last_transfer =
                Some("Too many file requests this session; restart Engram.".to_string());
            return None;
        }
        self.next_effect_id += 1;
        Some(self.next_effect_id)
    }

    /// Merge the package a host handed back, reporting either way.
    ///
    /// Both arms return a sentence for the reader rather than an `Err`: a file
    /// that turned out not to be a package is an ordinary thing for a person to
    /// do, not a fault in the application. Returning `Err` here would surface as
    /// a runtime error and, worse, would leave the effect answered but the
    /// reader with no idea why nothing changed.
    fn merge_package(&mut self, value: &Value) -> Result<String, String> {
        let Some(encoded) = value.get("apkg").and_then(Value::as_str) else {
            return Err("Import failed: the file request came back with no package.".to_string());
        };
        // Capped on the ENCODED length, before decoding, so the gate is a
        // string comparison rather than the allocation it prevents. The encoded
        // string, the JSON value holding it and the decoded bytes all coexist
        // at roughly three times the file's size, and this is the trust
        // boundary: the bytes came from a file the reader was handed.
        if encoded.len() > MAX_IMPORT_BASE64_LEN {
            return Err("Import failed: that package is too large to open.".to_string());
        }
        let bytes = coding_adventures_base64::decode(encoded, &coding_adventures_base64::STANDARD)
            .map_err(|_| "Import failed: the package was not valid base64.".to_string())?;
        if bytes.is_empty() {
            return Err("Import failed: the package was empty.".to_string());
        }
        let reply = self.session.merge_anki_apkg(&bytes);
        match facade_error(&reply) {
            Some(message) => Err(format!("Import failed: {}", reader_safe(&message))),
            None => Ok("Deck imported.".to_string()),
        }
    }

    /// Turn the facade's `hostIntent` into the effect that carries it.
    ///
    /// `importAnki` and `exportAnki` are [`Delivery::Await`]: neither can
    /// proceed without the host, and the app has to know whether it happened.
    /// `openCard` is [`Delivery::Notify`] -- opening a card in an external
    /// viewer is fire-and-forget, and nothing here changes based on the answer,
    /// so making the app wait for one would only invent a way to wedge it.
    ///
    /// The export package is built HERE and rides out in the payload, rather
    /// than the host handing back a path for this app to write. Every native
    /// target that matters can be sandboxed -- macOS most strictly -- and there
    /// the process may open only what the user picked in the host's own dialog.
    /// Keeping all filesystem access on the host side is the only arrangement
    /// that works on all five. The cost is that a cancelled export did work
    /// nobody used, which is cheap and recoverable; the alternative fails
    /// outright on the platform Engram most needs to ship to.
    fn effect_for_intent(&mut self, intent: &Value) -> Option<Effect> {
        if self.protocol_version < mosaic_app_runtime::EFFECT_PROTOCOL_VERSION {
            return None;
        }
        let kind = intent.get("type").and_then(Value::as_str)?;
        let (delivery, pending) = match kind {
            "importAnki" => (Delivery::Await, Some(PendingEffect::Import)),
            "exportAnki" => (Delivery::Await, Some(PendingEffect::Export)),
            "openCard" => (Delivery::Notify, None),
            // Every other intent still rides `hostIntent` for the hand-written
            // adapters. Minting an `Await` for one nothing answers would wedge
            // persistence for the session, which is exactly the failure the
            // host sweeps exist to prevent -- so an unknown intent produces no
            // effect at all rather than an unanswerable one.
            _ => return None,
        };
        let id = self.mint_effect_id()?;
        let mut payload = intent.clone();
        if pending == Some(PendingEffect::Export) {
            let reply = self.session.export_anki_apkg();
            match facade_package(&reply) {
                Ok(apkg) => {
                    if let Value::Object(fields) = &mut payload {
                        fields.insert("apkg".to_string(), Value::String(apkg));
                    }
                }
                Err(message) => {
                    // No package, nothing for the host to write. Report it here
                    // rather than sending an effect whose answer could only be
                    // "there was nothing to save".
                    self.last_transfer = Some(format!("Export failed: {}", reader_safe(&message)));
                    return None;
                }
            }
        }
        if let Some(pending) = pending {
            self.pending_effects.insert(id, pending);
        }
        Some(Effect {
            id,
            delivery,
            kind: kind.to_string(),
            payload,
        })
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
        self.protocol_version = context.protocol_version;
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

        // A new request supersedes whatever the last one said. Leaving the old
        // message up would caption a fresh dialog with a stale outcome.
        // Narrow, because the dispatch reply carries the whole collection under
        // `state` and the whole prop set under `props`, on EVERY event. Parsing
        // it into a `Value` to reach one optional field would rebuild all of
        // that a second time per keystroke.
        #[derive(serde::Deserialize)]
        struct IntentOnly {
            #[serde(default, rename = "hostIntent")]
            host_intent: Option<Value>,
        }
        let intent: Option<Value> = serde_json::from_str::<IntentOnly>(&reply)
            .ok()
            .and_then(|reply| reply.host_intent)
            .filter(|intent| !intent.is_null());
        if intent.is_some() {
            self.last_transfer = None;
        }

        // A reply may or may not carry props; ask for them explicitly rather
        // than depending on which events happen to include them.
        let mut update = self.update()?;
        if let Some(intent) = intent {
            if let Some(effect) = self.effect_for_intent(&intent) {
                // Props are rebuilt AFTER the effect, because building an export
                // effect can set the transfer message and the props above were
                // taken before that happened.
                update.props = self.props_with_transfer()?;
                update.effects.push(effect);
            } else {
                // No effect. Either the host is below protocol 2, where the
                // intent rides `hostIntent` as before and there is nothing to
                // say, or building one recorded why -- and `props_with_transfer`
                // carries that.
                update.props = self.props_with_transfer()?;
            }
        }
        Ok(update)
    }

    fn complete_effect(
        &mut self,
        id: EffectId,
        result: EffectResult,
    ) -> Result<AppUpdate, EffectCompletionError<Self::Error>> {
        // Removed on the way in, whatever the outcome: an answered effect is
        // answered, and leaving the id here would let a second answer act on it
        // a second time -- merging the same package twice.
        let Some(pending) = self.pending_effects.remove(&id) else {
            // The runtime already refuses an id it is not awaiting, so reaching
            // here means the id was real but this app has no record of it.
            // Reporting `Unsupported` would tell the host to stop sending
            // completions altogether, which is far worse than ignoring one.
            self.last_transfer = Some("An unrecognised file request was answered.".to_string());
            return Ok(self.update()?);
        };

        self.last_transfer = Some(match (pending, result) {
            (PendingEffect::Import, EffectResult::Ok(value)) => match self.merge_package(&value) {
                Ok(message) => message,
                Err(message) => message,
            },
            (PendingEffect::Export, EffectResult::Ok(_)) => "Deck exported.".to_string(),
            // Cancellation is an ordinary user action, not a failure: Escape in
            // a file dialog means "never mind", and saying so plainly is the
            // whole reason `Cancelled` is a separate arm from `Failed`.
            (PendingEffect::Import, EffectResult::Cancelled(_)) => "Import cancelled.".to_string(),
            (PendingEffect::Export, EffectResult::Cancelled(_)) => "Export cancelled.".to_string(),
            (PendingEffect::Import, EffectResult::Failed(failure)) => {
                format!("Import failed: {}", reader_safe(&failure.message))
            }
            (PendingEffect::Export, EffectResult::Failed(failure)) => {
                format!("Export failed: {}", reader_safe(&failure.message))
            }
        });
        Ok(self.update()?)
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

#[cfg(test)]
mod anki_effect_tests {
    use super::*;
    use mosaic_app_runtime::{EmptyOutcome, Platform};

    fn started() -> EngramMosaicApp {
        // Explicitly v2, because `StartContext::new` fills in `PROTOCOL_VERSION`
        // -- which is still 1 -- and effects are gated on v2. A generated host
        // sends 2 in its start envelope; this mimics that rather than the
        // default, which would silently make every assertion below vacuous.
        let mut context = StartContext::new("en-US", Platform::Linux);
        context.protocol_version = mosaic_app_runtime::EFFECT_PROTOCOL_VERSION;
        let mut app = EngramMosaicApp::default();
        app.start(context).expect("start must succeed");
        app
    }

    fn dispatch(app: &mut EngramMosaicApp, name: &str) -> AppUpdate {
        // The sequence is the runtime's business, not this adapter's; any
        // monotonic value does here.
        app.dispatch(Event::new(1, name, Value::Null))
            .unwrap_or_else(|error| panic!("`{name}` must dispatch: {error}"))
    }

    fn only_effect(update: &AppUpdate, context: &str) -> Effect {
        assert_eq!(
            update.effects.len(),
            1,
            "{context} must carry exactly one effect, got {:?}",
            update.effects
        );
        update.effects[0].clone()
    }

    fn transfer_status(update: &AppUpdate) -> String {
        update
            .props
            .get("anki-transfer-status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    }

    #[test]
    fn import_is_an_awaited_effect() {
        let mut app = started();
        let effect = only_effect(&dispatch(&mut app, "importAnki"), "importAnki");
        assert_eq!(effect.kind, "importAnki");
        // Await, not Notify: an import that the app does not wait for cannot
        // report whether the collection changed.
        assert_eq!(effect.delivery, Delivery::Await);
        assert!(effect.id > 0, "an effect id must be mintable");
        assert!(
            effect.id <= MAX_EFFECT_ID,
            "an id past 2^53-1 arrives rounded at a JSON-double host"
        );
        // The dialog needs to know what to accept; losing this makes the picker
        // show every file on the disk.
        assert!(
            effect.payload.get("accept").is_some(),
            "the import effect must carry the accepted extensions: {:?}",
            effect.payload
        );
    }

    #[test]
    fn export_carries_the_package_for_the_host_to_write() {
        let mut app = started();
        let effect = only_effect(&dispatch(&mut app, "exportAnki"), "exportAnki");
        assert_eq!(effect.kind, "exportAnki");
        assert_eq!(effect.delivery, Delivery::Await);
        // The bytes travel, not a path: a sandboxed host may write only where
        // the user pointed its own dialog, so this app never touches the disk.
        let apkg = effect
            .payload
            .get("apkg")
            .and_then(Value::as_str)
            .expect("the export effect must carry the package");
        let bytes = coding_adventures_base64::decode(apkg, &coding_adventures_base64::STANDARD)
            .expect("the package must be valid base64");
        assert!(!bytes.is_empty(), "an empty package is nothing to write");
        // Not merely non-empty: a real zip, which is what an .apkg is.
        assert_eq!(&bytes[..2], b"PK", "an .apkg is a zip archive");
    }

    #[test]
    fn open_card_is_notify_so_nothing_waits_on_it() {
        let mut app = started();
        // `openCard` needs a selected card, so this drives the browser first.
        // If a future change stops producing the intent here, the assertion
        // below fails rather than passing vacuously on an empty effect list.
        let update = dispatch(&mut app, "browserOpenSelected");
        // Asserted to EXIST, not merely filtered for: a `for` loop over an
        // empty list would pass this test while proving nothing, which is how
        // the delivery of an intent that stopped being produced would go
        // unnoticed.
        let effect = only_effect(&update, "browserOpenSelected");
        assert_eq!(effect.kind, "openCard");
        assert_eq!(
            effect.delivery,
            Delivery::Notify,
            "opening a card must not make the app wait for an answer"
        );
        // And whatever it emitted, nothing awaited is left outstanding: an
        // unanswered await here would switch persistence off for the session.
        assert!(
            app.pending_effects.is_empty(),
            "openCard must leave nothing pending: {:?}",
            app.pending_effects
        );
    }

    #[test]
    fn an_exported_package_imports_back_through_the_effect_round_trip() {
        // The whole point of the arc, end to end: export produces bytes, the
        // host hands them back to import, and the collection survives.
        let mut app = started();
        let exported = only_effect(&dispatch(&mut app, "exportAnki"), "exportAnki");
        let apkg = exported
            .payload
            .get("apkg")
            .and_then(Value::as_str)
            .expect("export must carry a package")
            .to_string();
        app.complete_effect(exported.id, EffectResult::Ok(Value::Null))
            .expect("answering an export must succeed");

        let imported = only_effect(&dispatch(&mut app, "importAnki"), "importAnki");
        let update = app
            .complete_effect(
                imported.id,
                EffectResult::Ok(serde_json::json!({ "apkg": apkg })),
            )
            .expect("answering an import must succeed");
        assert_eq!(transfer_status(&update), "Deck imported.");
        assert!(
            app.pending_effects.is_empty(),
            "both effects were answered, so nothing may stay pending"
        );
    }

    #[test]
    fn a_cancelled_import_says_so_and_leaves_nothing_pending() {
        let mut app = started();
        let effect = only_effect(&dispatch(&mut app, "importAnki"), "importAnki");
        let update = app
            .complete_effect(effect.id, EffectResult::Cancelled(EmptyOutcome {}))
            .expect("a cancellation is an answer, not an error");
        // Escape in a file dialog is an ordinary action, so it reads as one --
        // not as a failure, and not as silence.
        assert_eq!(transfer_status(&update), "Import cancelled.");
        assert!(app.pending_effects.is_empty());
    }

    #[test]
    fn a_failed_import_reports_why() {
        let mut app = started();
        let effect = only_effect(&dispatch(&mut app, "importAnki"), "importAnki");
        let update = app
            .complete_effect(
                effect.id,
                EffectResult::Failed(mosaic_app_runtime::EffectFailure {
                    message: "the disk went away".to_string(),
                }),
            )
            .expect("a failure is an answer, not an error");
        assert!(
            transfer_status(&update).contains("the disk went away"),
            "the reader is told why: {}",
            transfer_status(&update)
        );
        assert!(app.pending_effects.is_empty());
    }

    #[test]
    fn rubbish_bytes_are_reported_rather_than_merged() {
        let mut app = started();
        let effect = only_effect(&dispatch(&mut app, "importAnki"), "importAnki");
        let update = app
            .complete_effect(
                effect.id,
                // A real thing for a person to do: pick the wrong file.
                EffectResult::Ok(serde_json::json!({ "apkg": "bm90IGFuIGFwa2c=" })),
            )
            .expect("a bad package is the reader's problem to see, not an Err");
        assert!(
            transfer_status(&update).starts_with("Import failed:"),
            "a file that is not a package must say so: {}",
            transfer_status(&update)
        );
        assert!(app.pending_effects.is_empty());
    }

    #[test]
    fn an_answer_cannot_be_applied_twice() {
        let mut app = started();
        let effect = only_effect(&dispatch(&mut app, "importAnki"), "importAnki");
        app.complete_effect(effect.id, EffectResult::Cancelled(EmptyOutcome {}))
            .expect("first answer");
        // The id is gone, so a repeat cannot merge the same package again.
        let update = app
            .complete_effect(effect.id, EffectResult::Cancelled(EmptyOutcome {}))
            .expect("a stray second answer must not be an error");
        assert!(
            transfer_status(&update).contains("unrecognised"),
            "a second answer is reported, not silently reapplied: {}",
            transfer_status(&update)
        );
    }

    #[test]
    fn ids_are_never_reused_across_requests() {
        // Reuse would let a late answer to a dead effect land on a live one.
        let mut app = started();
        let first = only_effect(&dispatch(&mut app, "importAnki"), "importAnki");
        app.complete_effect(first.id, EffectResult::Cancelled(EmptyOutcome {}))
            .expect("answer the first");
        let second = only_effect(&dispatch(&mut app, "importAnki"), "importAnki");
        assert_ne!(
            first.id, second.id,
            "a fresh request must not reuse a retired id"
        );
    }
}

#[cfg(test)]
mod protocol_gate_tests {
    use super::*;
    use mosaic_app_runtime::{Platform, EFFECT_PROTOCOL_VERSION};

    fn started_at(protocol_version: u32) -> EngramMosaicApp {
        let mut context = StartContext::new("en-US", Platform::Linux);
        context.protocol_version = protocol_version;
        let mut app = EngramMosaicApp::default();
        app.start(context).expect("start must succeed");
        app
    }

    #[test]
    fn a_v1_host_gets_no_effects_at_all() {
        // Not a silent degradation to guard against: the runtime FAILS an
        // `Await` from a v1 host with `EffectsRequireV2` and poisons the
        // instance, so minting one would brick Engram at the first import
        // rather than fall back. Below v2 the intent rides `hostIntent` to the
        // hand-written adapters, which is what shipped before this change.
        let mut app = started_at(1);
        let update = app
            .dispatch(Event::new(1, "importAnki", Value::Null))
            .expect("import must still dispatch on a v1 host");
        assert!(
            update.effects.is_empty(),
            "a v1 host must be sent no effects: {:?}",
            update.effects
        );
        assert!(
            app.pending_effects.is_empty(),
            "and nothing may be recorded as owed"
        );
    }

    #[test]
    fn a_v2_host_gets_the_effect() {
        // The other half of the gate: without this, the test above would pass
        // just as well if effects were never minted for anyone.
        let mut app = started_at(EFFECT_PROTOCOL_VERSION);
        let update = app
            .dispatch(Event::new(1, "importAnki", Value::Null))
            .expect("import must dispatch");
        assert_eq!(
            update.effects.len(),
            1,
            "a v2 host must receive the import effect"
        );
        assert_eq!(update.effects[0].delivery, Delivery::Await);
    }
}

#[cfg(test)]
mod untrusted_input_tests {
    use super::*;
    use mosaic_app_runtime::{EffectFailure, Platform, EFFECT_PROTOCOL_VERSION};

    fn started() -> EngramMosaicApp {
        let mut context = StartContext::new("en-US", Platform::Linux);
        context.protocol_version = EFFECT_PROTOCOL_VERSION;
        let mut app = EngramMosaicApp::default();
        app.start(context).expect("start must succeed");
        app
    }

    fn import_effect(app: &mut EngramMosaicApp) -> Effect {
        let update = app
            .dispatch(Event::new(1, "importAnki", Value::Null))
            .expect("import must dispatch");
        update.effects[0].clone()
    }

    fn status(update: &AppUpdate) -> String {
        update
            .props
            .get("anki-transfer-status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    }

    #[test]
    fn an_oversized_package_is_refused_without_decoding_it() {
        let mut app = started();
        let effect = import_effect(&mut app);
        // One byte past the cap. Built as a string rather than real base64
        // because the point is that the length is checked BEFORE the decoder
        // ever sees it -- if the order were wrong this would spend a second
        // decoding half a gigabyte of 'A's first.
        let oversized = "A".repeat(MAX_IMPORT_BASE64_LEN + 1);
        let update = app
            .complete_effect(
                effect.id,
                EffectResult::Ok(serde_json::json!({ "apkg": oversized })),
            )
            .expect("an oversized package is the reader's problem, not an Err");
        assert!(
            status(&update).contains("too large"),
            "the reader is told, rather than the process running out of memory: {}",
            status(&update)
        );
    }

    #[test]
    fn a_hostile_failure_message_cannot_carry_control_characters_to_the_ui() {
        let mut app = started();
        let effect = import_effect(&mut app);
        // A host -- or a package-layer error quoting a zip entry name -- can put
        // arbitrary bytes here. Five native toolkits render this prop, and at
        // least one interprets markup, so the boundary is enforced here.
        // Everything the doc comment claims to stop, in one string: control
        // characters, the markup a rich-text toolkit would render, a bidi
        // override that rewrites what the line appears to say, and length.
        let hostile = format!(
            "line one\nline two\r\0<img src=\"file:///etc/passwd\">&amp;\u{202E}\u{2066}{}",
            "x".repeat(5_000)
        );
        let update = app
            .complete_effect(
                effect.id,
                EffectResult::Failed(EffectFailure { message: hostile }),
            )
            .expect("a failure is an answer");
        let reported = status(&update);
        assert!(
            !reported.contains('\n') && !reported.contains('\r') && !reported.contains('\0'),
            "control characters must not reach the prop: {reported:?}"
        );
        assert!(
            !reported.contains('<') && !reported.contains('>') && !reported.contains('&'),
            "markup delimiters must not reach a toolkit that auto-detects rich text: {reported:?}"
        );
        assert!(
            !reported.contains('\u{202E}') && !reported.contains('\u{2066}'),
            "bidi overrides must not reach the prop: {reported:?}"
        );
        assert!(
            reported.chars().count() <= 220,
            "an unbounded message must be cut to something readable: {} chars",
            reported.chars().count()
        );
    }

    #[test]
    fn the_id_bound_is_the_runtimes_own() {
        // Not a restated literal: a divergence here would mint ids the runtime
        // rejects, and it poisons the instance for one out of range.
        assert_eq!(MAX_EFFECT_ID, mosaic_app_runtime::MAX_EFFECT_ID);
    }
}
