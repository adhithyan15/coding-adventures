//! One Rust application contract for every Mosaic backend.
//!
//! [`MosaicApp`] contains application state and operations. [`MosaicRuntime`]
//! guards the host boundary: it validates the protocol version and event order,
//! invokes the app serially, and assigns render revisions. Native FFI and
//! WebAssembly bridges can therefore share one set of wire types and invariants.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;

/// The JSON protocol version understood by this release.
pub const PROTOCOL_VERSION: u32 = 1;

/// Application behavior implemented once in Rust.
///
/// A method that returns `Err` must leave the app's observable state unchanged.
/// [`MosaicRuntime`] deliberately does not consume protocol sequence/revision state
/// on an application error so the host can safely retry the same request.
pub trait MosaicApp {
    type Error: Error + Send + Sync + 'static;

    /// Produce the initial view model for a newly created application.
    fn start(&mut self, context: StartContext) -> Result<AppUpdate, Self::Error>;

    /// Apply one semantic UI or host-effect event.
    fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error>;

    /// Return an opaque, versioned application snapshot when supported.
    fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error>;

    /// Replace application state from an opaque snapshot and render it.
    fn restore(&mut self, snapshot: Snapshot) -> Result<AppUpdate, Self::Error>;
}

/// Host information supplied at application startup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartContext {
    pub protocol_version: u32,
    pub locale: String,
    pub color_scheme: ColorScheme,
    pub text_scale: f32,
    pub platform: Platform,
    pub restored_snapshot: Option<Snapshot>,
}

impl StartContext {
    /// Build a startup context using system appearance and the standard text scale.
    pub fn new(locale: impl Into<String>, platform: Platform) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            locale: locale.into(),
            color_scheme: ColorScheme::System,
            text_scale: 1.0,
            platform,
            restored_snapshot: None,
        }
    }
}

/// The host's active color scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorScheme {
    System,
    Light,
    Dark,
}

/// The platform family hosting the generated application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
    Apple,
    Windows,
    Linux,
    Android,
    Web,
}

/// A semantic UI event or a completed host effect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub protocol_version: u32,
    pub sequence: u64,
    pub name: String,
    pub payload: Value,
}

impl Event {
    /// Build an event for the current protocol version.
    pub fn new(sequence: u64, name: impl Into<String>, payload: Value) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            sequence,
            name: name.into(),
            payload,
        }
    }
}

/// Opaque persisted application state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub schema: String,
    pub version: u32,
    pub bytes: Vec<u8>,
}

/// A capability request for the generated host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    pub id: u64,
    pub kind: String,
    pub payload: Value,
}

/// A screen-reader announcement requested by the application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Announcement {
    pub politeness: Politeness,
    pub message: String,
}

/// How urgently assistive technology should announce a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Politeness {
    Polite,
    Assertive,
}

/// Revision-free application output.
///
/// The runtime assigns revisions after the application call succeeds, so an app
/// cannot accidentally desynchronize itself from a host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdate {
    pub props: Value,
    pub effects: Vec<Effect>,
    pub announcements: Vec<Announcement>,
}

impl AppUpdate {
    pub fn new(props: Value) -> Self {
        Self {
            props,
            effects: Vec::new(),
            announcements: Vec::new(),
        }
    }
}

/// A complete update sent to a generated host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub protocol_version: u32,
    pub revision: u64,
    pub props: Value,
    pub effects: Vec<Effect>,
    pub announcements: Vec<Announcement>,
}

impl Update {
    fn from_app(revision: u64, app: AppUpdate) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            revision,
            props: app.props,
            effects: app.effects,
            announcements: app.announcements,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeState {
    Created,
    Running { last_sequence: u64, revision: u64 },
}

/// Serializes access to a [`MosaicApp`] and enforces the host protocol.
pub struct MosaicRuntime<A> {
    app: A,
    state: RuntimeState,
}

impl<A: MosaicApp> MosaicRuntime<A> {
    pub fn new(app: A) -> Self {
        Self {
            app,
            state: RuntimeState::Created,
        }
    }

    /// Start the app exactly once and assign revision 1.
    pub fn start(&mut self, context: StartContext) -> Result<Update, RuntimeError<A::Error>> {
        if self.state != RuntimeState::Created {
            return Err(RuntimeError::AlreadyStarted);
        }
        validate_protocol(context.protocol_version)?;
        if !context.text_scale.is_finite() || context.text_scale <= 0.0 {
            return Err(RuntimeError::InvalidTextScale);
        }

        let app_update = self.app.start(context).map_err(RuntimeError::Application)?;
        self.state = RuntimeState::Running {
            last_sequence: 0,
            revision: 1,
        };
        Ok(Update::from_app(1, app_update))
    }

    /// Dispatch the next event in sequence and assign the next revision.
    pub fn dispatch(&mut self, event: Event) -> Result<Update, RuntimeError<A::Error>> {
        validate_protocol(event.protocol_version)?;
        let RuntimeState::Running {
            last_sequence,
            revision,
        } = self.state
        else {
            return Err(RuntimeError::NotStarted);
        };

        let expected = last_sequence
            .checked_add(1)
            .ok_or(RuntimeError::SequenceOverflow)?;
        if event.sequence != expected {
            return Err(RuntimeError::UnexpectedSequence {
                expected,
                received: event.sequence,
            });
        }
        let next_revision = revision
            .checked_add(1)
            .ok_or(RuntimeError::RevisionOverflow)?;

        let app_update = self
            .app
            .dispatch(event)
            .map_err(RuntimeError::Application)?;
        self.state = RuntimeState::Running {
            last_sequence: expected,
            revision: next_revision,
        };
        Ok(Update::from_app(next_revision, app_update))
    }

    /// Snapshot a running app without changing its sequence or revision.
    pub fn snapshot(&self) -> Result<Option<Snapshot>, RuntimeError<A::Error>> {
        if self.state == RuntimeState::Created {
            return Err(RuntimeError::NotStarted);
        }
        self.app.snapshot().map_err(RuntimeError::Application)
    }

    /// Restore a running app and assign a new revision without consuming an event.
    pub fn restore(&mut self, snapshot: Snapshot) -> Result<Update, RuntimeError<A::Error>> {
        let RuntimeState::Running {
            last_sequence,
            revision,
        } = self.state
        else {
            return Err(RuntimeError::NotStarted);
        };
        let next_revision = revision
            .checked_add(1)
            .ok_or(RuntimeError::RevisionOverflow)?;

        let app_update = self
            .app
            .restore(snapshot)
            .map_err(RuntimeError::Application)?;
        self.state = RuntimeState::Running {
            last_sequence,
            revision: next_revision,
        };
        Ok(Update::from_app(next_revision, app_update))
    }

    pub fn current_revision(&self) -> Option<u64> {
        match self.state {
            RuntimeState::Created => None,
            RuntimeState::Running { revision, .. } => Some(revision),
        }
    }

    pub fn next_sequence(&self) -> Option<u64> {
        match self.state {
            RuntimeState::Created => None,
            RuntimeState::Running { last_sequence, .. } => last_sequence.checked_add(1),
        }
    }

    pub fn app(&self) -> &A {
        &self.app
    }

    pub fn into_inner(self) -> A {
        self.app
    }
}

fn validate_protocol<E>(received: u32) -> Result<(), RuntimeError<E>> {
    if received == PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(RuntimeError::ProtocolVersionMismatch {
            expected: PROTOCOL_VERSION,
            received,
        })
    }
}

/// A host-protocol or application error.
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeError<E> {
    ProtocolVersionMismatch { expected: u32, received: u32 },
    InvalidTextScale,
    AlreadyStarted,
    NotStarted,
    UnexpectedSequence { expected: u64, received: u64 },
    SequenceOverflow,
    RevisionOverflow,
    Application(E),
}

impl<E: fmt::Display> fmt::Display for RuntimeError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtocolVersionMismatch { expected, received } => write!(
                f,
                "Mosaic protocol version mismatch: expected {expected}, received {received}"
            ),
            Self::InvalidTextScale => {
                f.write_str("Mosaic text scale must be finite and greater than zero")
            }
            Self::AlreadyStarted => f.write_str("Mosaic application has already started"),
            Self::NotStarted => f.write_str("Mosaic application has not started"),
            Self::UnexpectedSequence { expected, received } => write!(
                f,
                "unexpected Mosaic event sequence: expected {expected}, received {received}"
            ),
            Self::SequenceOverflow => f.write_str("Mosaic event sequence overflow"),
            Self::RevisionOverflow => f.write_str("Mosaic update revision overflow"),
            Self::Application(error) => write!(f, "Mosaic application error: {error}"),
        }
    }
}

impl<E: Error + 'static> Error for RuntimeError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Application(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestError;

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("test failure")
        }
    }

    impl Error for TestError {}

    #[derive(Default)]
    struct TestApp {
        starts: usize,
        dispatches: usize,
        restores: usize,
        fail_next_dispatch: bool,
    }

    impl MosaicApp for TestApp {
        type Error = TestError;

        fn start(&mut self, _context: StartContext) -> Result<AppUpdate, Self::Error> {
            self.starts += 1;
            Ok(AppUpdate::new(json!({ "count": 0 })))
        }

        fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error> {
            self.dispatches += 1;
            if self.fail_next_dispatch {
                self.fail_next_dispatch = false;
                return Err(TestError);
            }
            Ok(AppUpdate {
                props: json!({ "event": event.name }),
                effects: vec![Effect {
                    id: 7,
                    kind: "storage.set".to_string(),
                    payload: json!({ "key": "counter", "value": 1 }),
                }],
                announcements: vec![Announcement {
                    politeness: Politeness::Polite,
                    message: "Updated".to_string(),
                }],
            })
        }

        fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error> {
            Ok(Some(snapshot()))
        }

        fn restore(&mut self, _snapshot: Snapshot) -> Result<AppUpdate, Self::Error> {
            self.restores += 1;
            Ok(AppUpdate::new(json!({ "restored": true })))
        }
    }

    fn snapshot() -> Snapshot {
        Snapshot {
            schema: "counter".to_string(),
            version: 1,
            bytes: vec![1, 2, 3],
        }
    }

    fn start_context() -> StartContext {
        StartContext::new("en-US", Platform::Linux)
    }

    #[test]
    fn assigns_revisions_and_enforces_event_sequence() {
        let mut runtime = MosaicRuntime::new(TestApp::default());

        let started = runtime.start(start_context()).unwrap();
        assert_eq!(started.revision, 1);
        assert_eq!(runtime.next_sequence(), Some(1));

        let dispatched = runtime
            .dispatch(Event::new(1, "increment", json!({ "amount": 1 })))
            .unwrap();
        assert_eq!(dispatched.revision, 2);
        assert_eq!(dispatched.effects[0].kind, "storage.set");
        assert_eq!(dispatched.announcements[0].message, "Updated");
        assert_eq!(runtime.next_sequence(), Some(2));

        let restored = runtime.restore(snapshot()).unwrap();
        assert_eq!(restored.revision, 3);
        assert_eq!(runtime.next_sequence(), Some(2));

        let dispatched = runtime
            .dispatch(Event::new(2, "increment", json!({ "amount": 1 })))
            .unwrap();
        assert_eq!(dispatched.revision, 4);
        assert_eq!(runtime.app().dispatches, 2);
        assert_eq!(runtime.app().restores, 1);
    }

    #[test]
    fn rejects_calls_before_start_and_a_second_start() {
        let mut runtime = MosaicRuntime::new(TestApp::default());

        assert!(matches!(runtime.snapshot(), Err(RuntimeError::NotStarted)));
        assert!(matches!(
            runtime.restore(snapshot()),
            Err(RuntimeError::NotStarted)
        ));
        assert!(matches!(
            runtime.dispatch(Event::new(1, "increment", json!({}))),
            Err(RuntimeError::NotStarted)
        ));

        runtime.start(start_context()).unwrap();
        assert!(matches!(
            runtime.start(start_context()),
            Err(RuntimeError::AlreadyStarted)
        ));
        assert_eq!(runtime.app().starts, 1);
    }

    #[test]
    fn rejects_wrong_protocol_before_calling_the_app() {
        let mut runtime = MosaicRuntime::new(TestApp::default());
        let context = StartContext {
            protocol_version: PROTOCOL_VERSION + 1,
            ..start_context()
        };
        assert!(matches!(
            runtime.start(context),
            Err(RuntimeError::ProtocolVersionMismatch {
                expected: PROTOCOL_VERSION,
                received
            }) if received == PROTOCOL_VERSION + 1
        ));
        assert_eq!(runtime.app().starts, 0);

        runtime.start(start_context()).unwrap();
        let mut event = Event::new(1, "increment", json!({}));
        event.protocol_version += 1;
        assert!(matches!(
            runtime.dispatch(event),
            Err(RuntimeError::ProtocolVersionMismatch { .. })
        ));
        assert_eq!(runtime.app().dispatches, 0);
        assert_eq!(runtime.next_sequence(), Some(1));
    }

    #[test]
    fn rejects_invalid_text_scale_before_calling_the_app() {
        for text_scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let mut runtime = MosaicRuntime::new(TestApp::default());
            let mut context = start_context();
            context.text_scale = text_scale;
            assert!(matches!(
                runtime.start(context),
                Err(RuntimeError::InvalidTextScale)
            ));
            assert_eq!(runtime.app().starts, 0);
        }
    }

    #[test]
    fn rejects_stale_or_skipped_events_without_consuming_sequence() {
        let mut runtime = MosaicRuntime::new(TestApp::default());
        runtime.start(start_context()).unwrap();

        for received in [0, 2] {
            assert!(matches!(
                runtime.dispatch(Event::new(received, "increment", json!({}))),
                Err(RuntimeError::UnexpectedSequence {
                    expected: 1,
                    received: actual
                }) if actual == received
            ));
        }
        assert_eq!(runtime.app().dispatches, 0);
        assert_eq!(runtime.current_revision(), Some(1));
        assert_eq!(runtime.next_sequence(), Some(1));
    }

    #[test]
    fn application_error_does_not_advance_protocol_state() {
        let app = TestApp {
            fail_next_dispatch: true,
            ..TestApp::default()
        };
        let mut runtime = MosaicRuntime::new(app);
        runtime.start(start_context()).unwrap();

        assert!(matches!(
            runtime.dispatch(Event::new(1, "increment", json!({}))),
            Err(RuntimeError::Application(TestError))
        ));
        assert_eq!(runtime.current_revision(), Some(1));
        assert_eq!(runtime.next_sequence(), Some(1));

        let retried = runtime
            .dispatch(Event::new(1, "increment", json!({})))
            .unwrap();
        assert_eq!(retried.revision, 2);
        assert_eq!(runtime.app().dispatches, 2);
    }

    #[test]
    fn serializes_stable_camel_case_wire_envelopes() {
        let event = Event::new(9, "task.complete", json!({ "taskId": "t-1" }));
        let encoded = serde_json::to_value(&event).unwrap();
        assert_eq!(
            encoded,
            json!({
                "protocolVersion": 1,
                "sequence": 9,
                "name": "task.complete",
                "payload": { "taskId": "t-1" }
            })
        );
        assert_eq!(serde_json::from_value::<Event>(encoded).unwrap(), event);

        let context = serde_json::to_value(start_context()).unwrap();
        assert_eq!(context["colorScheme"], "system");
        assert_eq!(context["textScale"], 1.0);
        assert_eq!(context["platform"], "linux");
    }

    #[test]
    fn reports_overflow_before_calling_the_app() {
        let mut runtime = MosaicRuntime::new(TestApp::default());
        runtime.state = RuntimeState::Running {
            last_sequence: u64::MAX,
            revision: 4,
        };
        assert!(matches!(
            runtime.dispatch(Event::new(0, "increment", json!({}))),
            Err(RuntimeError::SequenceOverflow)
        ));
        assert_eq!(runtime.app().dispatches, 0);

        runtime.state = RuntimeState::Running {
            last_sequence: 0,
            revision: u64::MAX,
        };
        assert!(matches!(
            runtime.dispatch(Event::new(1, "increment", json!({}))),
            Err(RuntimeError::RevisionOverflow)
        ));
        assert_eq!(runtime.app().dispatches, 0);
    }
}
