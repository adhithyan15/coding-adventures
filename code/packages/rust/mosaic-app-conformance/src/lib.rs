//! Real Rust application fixture shared by Mosaic native binding acceptance.

use mosaic_app_runtime::{
    AppUpdate, Delivery, Effect, EffectCompletionError, EffectId, EffectResult, Event, MosaicApp,
    Platform, Snapshot, StartContext, EFFECT_PROTOCOL_VERSION,
};
use serde_json::{json, Value};
use std::error::Error;
use std::fmt;

const SNAPSHOT_SCHEMA: &str = "mosaic-app-conformance/counter";
const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Default)]
pub struct ConformanceApp {
    count: i64,
    platform: Option<Platform>,
    protocol_version: u32,
    next_effect_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConformanceError {
    UnknownEvent(String),
    InvalidAmount,
    InvalidSnapshot,
    EffectsUnavailable,
}

impl fmt::Display for ConformanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEvent(name) => write!(formatter, "unknown conformance event `{name}`"),
            Self::InvalidAmount => formatter.write_str("increment amount must be an integer"),
            Self::InvalidSnapshot => formatter.write_str("invalid conformance snapshot"),
            Self::EffectsUnavailable => {
                formatter.write_str("effect completion requires protocol 2")
            }
        }
    }
}

impl Error for ConformanceError {}

impl ConformanceApp {
    fn request_effect(&mut self, notify: bool) -> AppUpdate {
        self.next_effect_id += 1;
        let mut update = self.update("requested");
        update.effects.push(Effect {
            id: self.next_effect_id,
            kind: "conformance.counter".into(),
            payload: json!({}),
            delivery: if notify {
                Delivery::Notify
            } else {
                Delivery::Await
            },
        });
        update
    }
    fn update(&self, status: &str) -> AppUpdate {
        AppUpdate::new(json!({
            "count": self.count,
            "platform": self.platform.map(platform_name).unwrap_or("unknown"),
            "status": status,
        }))
    }
}

impl MosaicApp for ConformanceApp {
    type Error = ConformanceError;

    fn start(&mut self, context: StartContext) -> Result<AppUpdate, Self::Error> {
        self.platform = Some(context.platform);
        self.protocol_version = context.protocol_version;
        if let Some(snapshot) = context.restored_snapshot {
            self.restore(snapshot)
        } else {
            Ok(self.update("started"))
        }
    }

    fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error> {
        if event.name == "requestEffect" {
            if self.protocol_version != EFFECT_PROTOCOL_VERSION {
                return Err(ConformanceError::EffectsUnavailable);
            }
            return Ok(self.request_effect(
                event
                    .payload
                    .get("notify")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ));
        }
        if event.name != "increment" {
            return Err(ConformanceError::UnknownEvent(event.name));
        }
        let amount = event
            .payload
            .get("amount")
            .and_then(Value::as_i64)
            .ok_or(ConformanceError::InvalidAmount)?;
        self.count = self.count.saturating_add(amount);
        Ok(self.update("dispatched"))
    }

    fn complete_effect(
        &mut self,
        _id: EffectId,
        result: EffectResult,
    ) -> Result<AppUpdate, EffectCompletionError<Self::Error>> {
        match result {
            EffectResult::Ok(payload) => {
                let amount = payload
                    .get("amount")
                    .and_then(Value::as_i64)
                    .ok_or(ConformanceError::InvalidAmount)?;
                self.count = self.count.saturating_add(amount);
                if payload.get("chain").and_then(Value::as_bool) == Some(true) {
                    Ok(self.request_effect(false))
                } else {
                    Ok(self.update("completed"))
                }
            }
            EffectResult::Cancelled(_) => Ok(self.update("cancelled")),
            EffectResult::Failed(failure) => {
                Ok(self.update(&format!("failed: {}", failure.message)))
            }
        }
    }

    fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error> {
        Ok(Some(Snapshot {
            schema: SNAPSHOT_SCHEMA.to_string(),
            version: SNAPSHOT_VERSION,
            bytes: self.count.to_le_bytes().to_vec(),
        }))
    }

    fn restore(&mut self, snapshot: Snapshot) -> Result<AppUpdate, Self::Error> {
        if snapshot.schema != SNAPSHOT_SCHEMA || snapshot.version != SNAPSHOT_VERSION {
            return Err(ConformanceError::InvalidSnapshot);
        }
        let bytes: [u8; 8] = snapshot
            .bytes
            .try_into()
            .map_err(|_| ConformanceError::InvalidSnapshot)?;
        self.count = i64::from_le_bytes(bytes);
        Ok(self.update("restored"))
    }
}

fn platform_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Apple => "apple",
        Platform::Windows => "windows",
        Platform::Linux => "linux",
        Platform::Android => "android",
        Platform::Web => "web",
    }
}

mosaic_app_capi::export_mosaic_app!(ConformanceApp, ConformanceApp::default());

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_app_runtime::{ColorScheme, MosaicRuntime, PROTOCOL_VERSION};

    fn context() -> StartContext {
        StartContext {
            protocol_version: PROTOCOL_VERSION,
            locale: "en-US".to_string(),
            color_scheme: ColorScheme::System,
            text_scale: 1.0,
            platform: Platform::Windows,
            restored_snapshot: None,
        }
    }

    fn request(
        runtime: &mut MosaicRuntime<ConformanceApp>,
        notify: bool,
    ) -> mosaic_app_runtime::Update {
        let mut event = Event::new(
            runtime.next_sequence().unwrap(),
            "requestEffect",
            json!({"notify": notify}),
        );
        event.protocol_version = EFFECT_PROTOCOL_VERSION;
        runtime.dispatch(event).unwrap()
    }

    #[test]
    fn awaited_results_are_atomic_and_checkpoint_only_when_settled() {
        use mosaic_app_runtime::{EffectFailure, EmptyOutcome, RuntimeError};
        let mut runtime = MosaicRuntime::new(ConformanceApp::default());
        runtime
            .start(StartContext {
                protocol_version: EFFECT_PROTOCOL_VERSION,
                ..context()
            })
            .unwrap();
        let saved = runtime.snapshot().unwrap().unwrap();
        let update = request(&mut runtime, false);
        let id = update.effects[0].id;
        assert_eq!(
            serde_json::to_value(&update).unwrap()["effects"][0]["delivery"],
            "await"
        );
        assert!(
            matches!(runtime.snapshot(), Err(RuntimeError::PendingEffects(ids)) if ids == vec![id])
        );
        assert!(matches!(
            runtime.restore(saved.clone()),
            Err(RuntimeError::PendingEffects(_))
        ));
        assert!(matches!(
            runtime.complete_effect(id + 1, EffectResult::Ok(json!({"amount": 99}))),
            Err(RuntimeError::UnknownEffect(_))
        ));
        assert!(matches!(
            runtime.complete_effect(id, EffectResult::Ok(json!({"amount": "bad"}))),
            Err(RuntimeError::Application(_))
        ));
        assert_eq!(runtime.app().count, 0);
        assert_eq!(runtime.current_revision(), Some(2));
        assert_eq!(runtime.next_sequence(), Some(2));
        assert_eq!(runtime.pending_effects(), vec![id]);
        let chained = runtime
            .complete_effect(id, EffectResult::Ok(json!({"amount": 7, "chain": true})))
            .unwrap();
        let next = chained.effects[0].id;
        assert!(next > id);
        assert_eq!(chained.props["count"], 7);
        assert_eq!(runtime.next_sequence(), Some(2));
        assert_eq!(runtime.pending_effects(), vec![next]);
        assert!(matches!(
            runtime.complete_effect(id, EffectResult::Cancelled(EmptyOutcome {})),
            Err(RuntimeError::UnknownEffect(_))
        ));
        assert!(matches!(
            runtime.snapshot(),
            Err(RuntimeError::PendingEffects(_))
        ));
        runtime
            .complete_effect(next, EffectResult::Cancelled(EmptyOutcome {}))
            .unwrap();
        assert_eq!(runtime.app().count, 7);
        runtime.restore(saved.clone()).unwrap();
        let failed = request(&mut runtime, false).effects[0].id;
        assert!(failed > next, "restore must not recycle an effect ID");
        runtime
            .complete_effect(
                failed,
                EffectResult::Failed(EffectFailure {
                    message: "disk full".into(),
                }),
            )
            .unwrap();
        assert!(runtime.pending_effects().is_empty());
        assert_eq!(runtime.app().count, 0);
        let notify = request(&mut runtime, true).effects[0].id;
        assert!(runtime.snapshot().is_ok());
        assert!(matches!(
            runtime.complete_effect(notify, EffectResult::Ok(json!({"amount": 1}))),
            Err(RuntimeError::UnknownEffect(_))
        ));
        let mut fresh = MosaicRuntime::new(ConformanceApp::default());
        let initial = fresh
            .start(StartContext {
                protocol_version: EFFECT_PROTOCOL_VERSION,
                restored_snapshot: Some(saved),
                ..context()
            })
            .unwrap();
        assert!(initial.effects.is_empty());
        assert!(fresh.pending_effects().is_empty());
    }

    #[test]
    fn native_completion_symbol_returns_owned_diagnostics_and_updates() {
        use mosaic_app_capi::{MosaicBuffer, MosaicBytes, MosaicStatus};
        // Every returned buffer (including errors) is consumed and freed once.
        unsafe fn take(buffer: MosaicBuffer) -> String {
            let value =
                String::from_utf8(std::slice::from_raw_parts(buffer.ptr, buffer.len).to_vec())
                    .unwrap();
            mosaic_buffer_free(buffer);
            value
        }
        unsafe {
            let mut handle = std::ptr::null_mut();
            let mut out = MosaicBuffer::empty();
            let start = serde_json::to_vec(&StartContext {
                protocol_version: EFFECT_PROTOCOL_VERSION,
                ..context()
            })
            .unwrap();
            assert_eq!(
                mosaic_app_create(MosaicBytes::new(&start), &mut handle, &mut out),
                MosaicStatus::Ok
            );
            take(out);
            assert_eq!(mosaic_app_snapshot(handle, &mut out), MosaicStatus::Ok);
            let saved = take(out);
            let event =
                br#"{"protocolVersion":2,"sequence":1,"name":"requestEffect","payload":{}}"#;
            assert_eq!(
                mosaic_app_dispatch(handle, MosaicBytes::new(event), &mut out),
                MosaicStatus::Ok
            );
            assert_eq!(
                serde_json::from_str::<Value>(&take(out)).unwrap()["effects"][0]["id"],
                1
            );
            assert_eq!(
                mosaic_app_snapshot(handle, &mut out),
                MosaicStatus::PendingEffects
            );
            assert!(take(out).contains('1'));
            assert_eq!(
                mosaic_app_restore(handle, MosaicBytes::new(saved.as_bytes()), &mut out),
                MosaicStatus::PendingEffects
            );
            take(out);
            assert_eq!(
                mosaic_app_complete_effect(
                    handle,
                    MosaicBytes::new(b"1"),
                    MosaicBytes::new(br#"{"ok":{"amount":"bad"}}"#),
                    &mut out
                ),
                MosaicStatus::ApplicationError
            );
            take(out);
            assert_eq!(
                mosaic_app_complete_effect(
                    handle,
                    MosaicBytes::new(b"1"),
                    MosaicBytes::new(br#"{"ok":{"amount":5}}"#),
                    &mut out
                ),
                MosaicStatus::Ok
            );
            let completed: Value = serde_json::from_str(&take(out)).unwrap();
            assert_eq!(completed["props"]["count"], 5);
            assert_eq!(completed["revision"], 3);
            assert_eq!(
                mosaic_app_complete_effect(
                    handle,
                    MosaicBytes::new(b"1"),
                    MosaicBytes::new(br#"{"cancelled":{}}"#),
                    &mut out
                ),
                MosaicStatus::ProtocolError
            );
            take(out);
            assert_eq!(
                mosaic_app_restore(handle, MosaicBytes::new(saved.as_bytes()), &mut out),
                MosaicStatus::Ok
            );
            assert_eq!(
                serde_json::from_str::<Value>(&take(out)).unwrap()["props"]["count"],
                0
            );
            mosaic_app_destroy(handle);
        }
    }

    #[test]
    fn starts_dispatches_snapshots_and_restores() {
        let mut runtime = MosaicRuntime::new(ConformanceApp::default());
        let started = runtime.start(context()).unwrap();
        assert_eq!(started.revision, 1);
        assert_eq!(started.props["count"], 0);
        assert_eq!(started.props["platform"], "windows");

        let dispatched = runtime
            .dispatch(Event::new(1, "increment", json!({ "amount": 3 })))
            .unwrap();
        assert_eq!(dispatched.revision, 2);
        assert_eq!(dispatched.props["count"], 3);

        let snapshot = runtime.snapshot().unwrap().unwrap();
        let mut restored = MosaicRuntime::new(ConformanceApp::default());
        restored.start(context()).unwrap();
        let update = restored.restore(snapshot).unwrap();
        assert_eq!(update.props["count"], 3);
    }

    #[test]
    fn rejects_unknown_events_without_advancing_revision() {
        let mut runtime = MosaicRuntime::new(ConformanceApp::default());
        runtime.start(context()).unwrap();
        assert!(runtime
            .dispatch(Event::new(1, "unknown", json!({})))
            .is_err());
        assert_eq!(runtime.current_revision(), Some(1));
    }
}

mosaic_app_wasm::export_mosaic_wasm!(ConformanceApp, ConformanceApp::default());
