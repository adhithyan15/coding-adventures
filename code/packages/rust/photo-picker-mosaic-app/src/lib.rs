//! `photo-picker-mosaic-app` — the reference application for `UI59`'s
//! `files.open` effect.
//!
//! One button, one status line: click "Pick a Photo", the host opens its
//! native file/gallery picker, and the result (name, size, or why it
//! didn't happen) renders back. That's the whole app -- see
//! `code/specs/UI59-files-open-effect.md` for the effect contract this
//! exercises, and `code/programs/mosaic/photo-picker-app/` for the `.mil`/
//! `.mll`/`.msl` UI sources and the XAML handler that answers the effect.
//!
//! # Why a whole application for one effect
//!
//! `[host_effects]` (`UI47`) only wires a *declared* handler into a
//! *generated* entry point -- there is nothing to generate an entry point
//! for without a real Mosaic package. This crate is the minimum viable
//! one: a single event (`pickPhoto`), a single effect (`files.open`), and
//! nothing else, so the round trip end to end is provable without
//! Engram-scale complexity in the way.

use std::convert::Infallible;

use mosaic_app_runtime::{
    AppUpdate, Delivery, Effect, EffectCompletionError, EffectId, EffectResult, Event,
    MosaicApp, Snapshot, StartContext, MAX_EFFECT_ID,
};
use serde_json::{json, Value};

/// The MIME types this app's picker request filters to -- an ordinary
/// photo, matching what a QR-scanner caller (the motivating consumer,
/// `VIS00-vision-roadmap.md`'s L4/L5 phases) would actually want. Not a
/// claim that `files.open` itself is image-specific -- see UI59 §3.
const ACCEPT_IMAGE_TYPES: &[&str] = &["image/jpeg", "image/png", "image/webp"];

/// Application state: just enough to render a status line and know
/// whether a pick is in flight.
pub struct PhotoPickerApp {
    status: String,
    picking: bool,
    next_effect_id: EffectId,
}

impl Default for PhotoPickerApp {
    fn default() -> Self {
        Self {
            status: "No photo picked yet.".to_string(),
            picking: false,
            next_effect_id: 1,
        }
    }
}

impl PhotoPickerApp {
    fn render(&self) -> AppUpdate {
        AppUpdate::new(json!({
            "status": self.status,
            "picking": self.picking,
        }))
    }

    /// Mint the next effect id, bounded per `mosaic_app_runtime::
    /// MAX_EFFECT_ID`'s own documented contract ("applications must not
    /// mint past it"). A demo app clicking one button will never
    /// realistically exhaust `2^53 - 1` ids, but the guard costs nothing
    /// and keeps this correct in principle rather than by luck --
    /// returns `None` (never panics) if the space is ever exhausted, and
    /// the caller falls back to rendering without minting a new effect.
    fn mint_effect_id(&mut self) -> Option<EffectId> {
        if self.next_effect_id > MAX_EFFECT_ID {
            return None;
        }
        let id = self.next_effect_id;
        self.next_effect_id += 1;
        Some(id)
    }
}

impl MosaicApp for PhotoPickerApp {
    type Error = Infallible;

    fn start(&mut self, _context: StartContext) -> Result<AppUpdate, Self::Error> {
        Ok(self.render())
    }

    fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error> {
        // "onPickPhoto", not "pickPhoto" -- the wire event name is the RAW
        // emit name from `PhotoPickerApp.mil`'s `emit onPickPhoto ;`,
        // unstripped. Every backend emitter deliberately keeps the "on"
        // prefix on the wire value and only strips it for the generated
        // *class/case* name (confirmed directly against
        // `mosaic-emit-compose`'s own codegen and its own test asserting
        // `mosaicName: String = "onCommit"`, and against `mosaic-emit-xaml`'s
        // `event_name = escape_csharp_string(&emit.name)`, which uses the
        // raw name, not `strip_on_prefix`'s output). A real, live click on
        // the built XAML app proved this the hard way: this app's own
        // button did nothing on any of the four already-shipped backends
        // (XAML #15218, Qt #15252, Compose #15329, Flutter #15340) because
        // dispatch was checking for a wire name none of them ever send.
        if event.name != "onPickPhoto" {
            // Unknown event: render current state unchanged rather than
            // erroring -- a forward-compatible host sending an event this
            // version doesn't know about shouldn't break the app.
            return Ok(self.render());
        }

        let Some(id) = self.mint_effect_id() else {
            self.status = "Too many picks this session -- restart the app.".to_string();
            return Ok(self.render());
        };

        self.picking = true;
        self.status = "Opening the photo picker...".to_string();
        let mut update = self.render();
        update.effects.push(Effect {
            id,
            delivery: Delivery::Await,
            kind: "files.open".to_string(),
            payload: json!({ "accept": ACCEPT_IMAGE_TYPES }),
        });
        Ok(update)
    }

    fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error> {
        // No persisted state worth snapshotting -- the picked photo isn't
        // kept in memory past rendering its name/size (see
        // `complete_effect`), and the status line is trivially
        // reconstructible. Matches `UI47` §8.1: `Notify`/no-pending-effect
        // apps are always snapshot-compatible; returning `None` here says
        // "nothing to save," not "refuse."
        Ok(None)
    }

    fn restore(&mut self, _snapshot: Snapshot) -> Result<AppUpdate, Self::Error> {
        // Never actually reachable by a host that respects `snapshot`
        // returning `None` above, but the trait requires an implementation
        // regardless. Re-render current state rather than trust an opaque
        // snapshot this app never produced.
        Ok(self.render())
    }

    fn complete_effect(
        &mut self,
        _id: EffectId,
        result: EffectResult,
    ) -> Result<AppUpdate, EffectCompletionError<Self::Error>> {
        self.picking = false;
        self.status = match result {
            EffectResult::Ok(value) => describe_picked_file(&value),
            EffectResult::Cancelled(_) => "Picker cancelled -- no photo selected.".to_string(),
            EffectResult::Failed(failure) => format!("Couldn't pick a photo: {}", failure.message),
        };
        Ok(self.render())
    }
}

/// Turn a `files.open` `ok` result (UI59 §3: `{ name, mimeType, bytes }`)
/// into a human-readable status line. Decodes `bytes` only to report an
/// accurate size -- this app never needs the decoded bytes for anything
/// else, so nothing is retained past this call.
fn describe_picked_file(value: &Value) -> String {
    let name = value.get("name").and_then(Value::as_str).unwrap_or("(unnamed)");
    let mime_type = value.get("mimeType").and_then(Value::as_str).unwrap_or("unknown type");
    let size_description = match value.get("bytes").and_then(Value::as_str) {
        Some(encoded) => match coding_adventures_base64::decode(encoded, &coding_adventures_base64::STANDARD) {
            Ok(bytes) => format!("{} bytes", bytes.len()),
            // A host that answers `ok` with unparseable base64 has broken
            // its own contract (UI59 §3) -- report that plainly rather
            // than silently treating it as zero bytes or panicking.
            Err(_) => "an unreadable size (bad base64 from the host)".to_string(),
        },
        None => "an unknown size (no bytes field)".to_string(),
    };
    format!("Picked \"{name}\" ({mime_type}, {size_description}).")
}

mosaic_app_capi::export_mosaic_app!(PhotoPickerApp, PhotoPickerApp::default());

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_app_runtime::{EmptyOutcome, MosaicRuntime, Platform, EFFECT_PROTOCOL_VERSION};

    /// This app's whole reason to exist is emitting an `Await` effect, which
    /// requires protocol v2 -- `StartContext::new` defaults to v1 (kept for
    /// existing v1-only hosts), so every test opts in explicitly, the same
    /// way `engram-mosaic-app`'s own tests do.
    fn start_context() -> StartContext {
        let mut context = StartContext::new("en-US", Platform::Windows);
        context.protocol_version = EFFECT_PROTOCOL_VERSION;
        context
    }

    /// Same reasoning as `start_context`: `Event::new` defaults to protocol
    /// v1, and `MosaicRuntime::dispatch` rejects an event whose
    /// `protocol_version` doesn't match the runtime's own (set to v2 by
    /// `start_context` above) -- so every dispatched event needs the same
    /// override, or every test would fail on a protocol mismatch before
    /// ever reaching this app's own dispatch logic.
    fn v2_event(sequence: u64, name: &str) -> Event {
        let mut event = Event::new(sequence, name, Value::Null);
        event.protocol_version = EFFECT_PROTOCOL_VERSION;
        event
    }

    #[test]
    fn start_renders_the_initial_status() {
        let mut runtime = MosaicRuntime::new(PhotoPickerApp::default());
        let update = runtime.start(start_context()).expect("start must succeed");
        assert_eq!(update.props["status"], json!("No photo picked yet."));
        assert_eq!(update.props["picking"], json!(false));
    }

    #[test]
    fn pick_photo_event_mints_an_await_files_open_effect() {
        let mut runtime = MosaicRuntime::new(PhotoPickerApp::default());
        runtime.start(start_context()).unwrap();
        let update = runtime
            .dispatch(v2_event(1, "onPickPhoto"))
            .expect("dispatch must succeed");

        assert_eq!(update.props["picking"], json!(true));
        assert_eq!(update.effects.len(), 1);
        let effect = &update.effects[0];
        assert_eq!(effect.kind, "files.open");
        assert_eq!(effect.delivery, Delivery::Await);
        assert_eq!(
            effect.payload["accept"],
            json!(["image/jpeg", "image/png", "image/webp"])
        );
    }

    #[test]
    fn unknown_event_renders_unchanged() {
        let mut runtime = MosaicRuntime::new(PhotoPickerApp::default());
        runtime.start(start_context()).unwrap();
        let update = runtime
            .dispatch(v2_event(1, "somethingElse"))
            .expect("dispatch must succeed");
        assert_eq!(update.props["status"], json!("No photo picked yet."));
        assert!(update.effects.is_empty());
    }

    /// Regression test for a real bug found by an actual live click on the
    /// built app: `dispatch` originally checked for `"pickPhoto"`, but every
    /// backend's generated client sends the RAW, unstripped emit name from
    /// `PhotoPickerApp.mil`'s `emit onPickPhoto ;` -- "onPickPhoto" -- as the
    /// wire event name (the "on" prefix is stripped only for the generated
    /// class/case name, never the wire value; confirmed directly against
    /// `mosaic-emit-compose`'s own test asserting `mosaicName: String =
    /// "onCommit"`). The old name is deliberately still checked here, and
    /// must keep behaving as an unrecognised event, not as `onPickPhoto`'s
    /// synonym -- a future "helpful" alias would silently mask this class of
    /// bug reappearing under a different event name.
    #[test]
    fn the_pre_fix_wire_name_is_still_just_an_unknown_event() {
        let mut runtime = MosaicRuntime::new(PhotoPickerApp::default());
        runtime.start(start_context()).unwrap();
        let update = runtime
            .dispatch(v2_event(1, "pickPhoto"))
            .expect("dispatch must succeed");
        assert_eq!(update.props["status"], json!("No photo picked yet."));
        assert_eq!(update.props["picking"], json!(false));
        assert!(update.effects.is_empty());
    }

    #[test]
    fn completing_with_ok_reports_name_type_and_exact_size() {
        let mut runtime = MosaicRuntime::new(PhotoPickerApp::default());
        runtime.start(start_context()).unwrap();
        let dispatch_update = runtime.dispatch(v2_event(1, "onPickPhoto")).unwrap();
        let effect_id = dispatch_update.effects[0].id;

        let bytes = b"not a real jpeg, just test bytes";
        let encoded = coding_adventures_base64::encode(bytes, &coding_adventures_base64::STANDARD);
        let update = runtime
            .complete_effect(
                effect_id,
                EffectResult::Ok(json!({
                    "name": "sunset.jpg",
                    "mimeType": "image/jpeg",
                    "bytes": encoded,
                })),
            )
            .expect("completion must succeed");

        assert_eq!(update.props["picking"], json!(false));
        let status = update.props["status"].as_str().unwrap();
        assert!(status.contains("sunset.jpg"), "status was: {status}");
        assert!(status.contains("image/jpeg"), "status was: {status}");
        assert!(status.contains(&bytes.len().to_string()), "status was: {status}");
    }

    #[test]
    fn completing_with_cancelled_is_not_reported_as_a_failure() {
        let mut runtime = MosaicRuntime::new(PhotoPickerApp::default());
        runtime.start(start_context()).unwrap();
        let dispatch_update = runtime.dispatch(v2_event(1, "onPickPhoto")).unwrap();
        let effect_id = dispatch_update.effects[0].id;

        let update = runtime
            .complete_effect(effect_id, EffectResult::Cancelled(EmptyOutcome {}))
            .expect("completion must succeed");

        let status = update.props["status"].as_str().unwrap();
        assert!(status.to_lowercase().contains("cancel"), "status was: {status}");
        assert!(!status.to_lowercase().contains("fail"), "status was: {status}");
    }

    #[test]
    fn completing_with_failed_surfaces_the_message() {
        let mut runtime = MosaicRuntime::new(PhotoPickerApp::default());
        runtime.start(start_context()).unwrap();
        let dispatch_update = runtime.dispatch(v2_event(1, "onPickPhoto")).unwrap();
        let effect_id = dispatch_update.effects[0].id;

        let update = runtime
            .complete_effect(
                effect_id,
                EffectResult::Failed(mosaic_app_runtime::EffectFailure {
                    message: "disk read error".to_string(),
                }),
            )
            .expect("completion must succeed");

        let status = update.props["status"].as_str().unwrap();
        assert!(status.contains("disk read error"), "status was: {status}");
    }

    #[test]
    fn malformed_base64_from_the_host_is_reported_not_panicked() {
        let mut runtime = MosaicRuntime::new(PhotoPickerApp::default());
        runtime.start(start_context()).unwrap();
        let dispatch_update = runtime.dispatch(v2_event(1, "onPickPhoto")).unwrap();
        let effect_id = dispatch_update.effects[0].id;

        let update = runtime
            .complete_effect(
                effect_id,
                EffectResult::Ok(json!({
                    "name": "broken.jpg",
                    "mimeType": "image/jpeg",
                    "bytes": "!!!not-valid-base64!!!",
                })),
            )
            .expect("completion must succeed -- a bad host payload is not an app error");

        let status = update.props["status"].as_str().unwrap();
        assert!(status.contains("unreadable"), "status was: {status}");
    }

    #[test]
    fn missing_fields_in_ok_result_do_not_panic() {
        let mut runtime = MosaicRuntime::new(PhotoPickerApp::default());
        runtime.start(start_context()).unwrap();
        let dispatch_update = runtime.dispatch(v2_event(1, "onPickPhoto")).unwrap();
        let effect_id = dispatch_update.effects[0].id;

        let update = runtime
            .complete_effect(effect_id, EffectResult::Ok(json!({})))
            .expect("completion must succeed even with a minimal/empty ok payload");

        let status = update.props["status"].as_str().unwrap();
        assert!(status.contains("(unnamed)"), "status was: {status}");
    }
}
