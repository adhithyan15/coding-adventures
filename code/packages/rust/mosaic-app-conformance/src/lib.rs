//! Real Rust application fixture shared by Mosaic native binding acceptance.

use mosaic_app_runtime::{AppUpdate, Event, MosaicApp, Platform, Snapshot, StartContext};
use serde_json::{json, Value};
use std::error::Error;
use std::fmt;

const SNAPSHOT_SCHEMA: &str = "mosaic-app-conformance/counter";
const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Default)]
pub struct ConformanceApp {
    count: i64,
    platform: Option<Platform>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConformanceError {
    UnknownEvent(String),
    InvalidAmount,
    InvalidSnapshot,
}

impl fmt::Display for ConformanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEvent(name) => write!(formatter, "unknown conformance event `{name}`"),
            Self::InvalidAmount => formatter.write_str("increment amount must be an integer"),
            Self::InvalidSnapshot => formatter.write_str("invalid conformance snapshot"),
        }
    }
}

impl Error for ConformanceError {}

impl ConformanceApp {
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
        if let Some(snapshot) = context.restored_snapshot {
            self.restore(snapshot)
        } else {
            Ok(self.update("started"))
        }
    }

    fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error> {
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
