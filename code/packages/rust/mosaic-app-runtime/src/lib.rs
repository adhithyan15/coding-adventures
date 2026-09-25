//! One Rust application contract for every Mosaic backend.
//!
//! [`MosaicApp`] contains application state and operations. [`MosaicRuntime`]
//! guards the host boundary: it validates the protocol version and event order,
//! invokes the app serially, and assigns render revisions. Native FFI and
//! WebAssembly bridges can therefore share one set of wire types and invariants.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// Default JSON protocol version, retained for existing hosts.
pub const PROTOCOL_VERSION: u32 = 1;
/// Opt-in protocol with dedicated awaited-effect completion.
pub const EFFECT_PROTOCOL_VERSION: u32 = 2;
pub type EffectId = u64;
/// The largest effect id that survives a JSON double.
///
/// Public because applications must not mint past it: an id that arrives
/// rounded at a host whose only integer is a double answers a DIFFERENT effect,
/// and [`MosaicRuntime`] poisons the instance for one out of range. An adapter
/// restating the literal would silently diverge if this ever moved.
pub const MAX_EFFECT_ID: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Delivery {
    #[default]
    Notify,
    Await,
}

/// Exactly one externally tagged outcome. Cancellation is not a failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EffectResult {
    Ok(Value),
    Cancelled(EmptyOutcome),
    Failed(EffectFailure),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyOutcome {}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectFailure {
    pub message: String,
}
#[derive(Debug)]
pub enum EffectCompletionError<E> {
    Unsupported,
    Application(E),
}
impl<E> From<E> for EffectCompletionError<E> {
    fn from(error: E) -> Self {
        Self::Application(error)
    }
}

/// Application behavior implemented once in Rust.
///
/// A method that returns `Err` must leave the app's observable state unchanged.
/// [`MosaicRuntime`] deliberately does not consume protocol sequence/revision state
/// on an application error so the host can safely retry the same request.
pub trait MosaicApp {
    type Error: Error + Send + Sync + 'static;

    /// Produce the initial view model for a newly created application.
    fn start(&mut self, context: StartContext) -> Result<AppUpdate, Self::Error>;

    /// Apply one semantic UI event.
    fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error>;

    /// Return an opaque, versioned application snapshot when supported.
    fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error>;

    /// Replace application state from an opaque snapshot and render it.
    fn restore(&mut self, snapshot: Snapshot) -> Result<AppUpdate, Self::Error>;

    /// Complete an awaited capability without consuming a UI event sequence.
    /// As with dispatch, an error must leave application state unchanged.
    fn complete_effect(
        &mut self,
        _id: EffectId,
        _result: EffectResult,
    ) -> Result<AppUpdate, EffectCompletionError<Self::Error>> {
        Err(EffectCompletionError::Unsupported)
    }

    /// React to a change in the host environment (UI48): a new size class,
    /// orientation, pointer, color scheme or motion preference, delivered as
    /// one coalesced value. Return `Some` to re-render, `None` to ignore it.
    /// The default ignores it, so an app that never looks at its environment
    /// keeps working when a host starts reporting one. As with dispatch, an
    /// error must leave application state unchanged.
    fn environment_changed(
        &mut self,
        _environment: Environment,
    ) -> Result<Option<AppUpdate>, Self::Error> {
        Ok(None)
    }
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
    /// The host's UTC offset at startup, in minutes east of UTC (`-300` in
    /// New York in winter, `330` in India). `None` when the host does not
    /// say: optional on the wire, so earlier hosts decode unchanged, and an
    /// app told nothing falls back to UTC. Validated at start to
    /// [`MIN_UTC_OFFSET_MINUTES`]`..=`[`MAX_UTC_OFFSET_MINUTES`]. It is a
    /// snapshot, not a clock: a daylight-saving change takes effect at the
    /// next start (UI38 §4, "Local time").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub utc_offset_minutes: Option<i32>,
    /// The rest of the host environment (UI48 §4): size class, pointer,
    /// hover, orientation and motion preference, flat on the wire beside
    /// `colorScheme`. Every axis defaults, so earlier hosts decode unchanged.
    #[serde(flatten)]
    pub environment: EnvironmentAxes,
}

/// The westernmost UTC offset in use (UTC−14:00 bounds it with room to spare;
/// the real minimum is UTC−12:00), in minutes east of UTC.
pub const MIN_UTC_OFFSET_MINUTES: i32 = -840;
/// The easternmost UTC offset in use: UTC+14:00 (Line Islands).
pub const MAX_UTC_OFFSET_MINUTES: i32 = 840;

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
            utc_offset_minutes: None,
            environment: EnvironmentAxes::default(),
        }
    }

    /// The complete environment the app starts in: the color scheme and the
    /// other axes together, the same shape `environmentChanged` delivers.
    pub fn full_environment(&self) -> Environment {
        Environment {
            color_scheme: self.color_scheme,
            axes: self.environment,
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

/// The reserved event a host dispatches when its environment changes
/// (UI48 §5.2). The runtime intercepts it; the app sees
/// [`MosaicApp::environment_changed`], never a `dispatch` of this name.
pub const ENVIRONMENT_CHANGED: &str = "environmentChanged";

/// Available width, as a bucket rather than pixels (UI48 §4). Each backend
/// maps its native notion (SwiftUI `horizontalSizeClass`, Compose
/// `WindowSizeClass`, a width observer on the web) onto these three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SizeClass {
    Compact,
    #[default]
    Regular,
    Expanded,
}

/// Precision of the primary pointing device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pointer {
    Coarse,
    #[default]
    Fine,
    None,
}

/// Whether hover affordances are reachable. Separate from [`Pointer`]: a
/// stylus is fine but cannot hover, and a TV remote is neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Hover {
    #[default]
    Hover,
    None,
}

/// The window's orientation (not the device's).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Orientation {
    Portrait,
    #[default]
    Landscape,
}

/// The accessibility motion preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReducedMotion {
    Reduce,
    #[default]
    NoPreference,
}

/// The UI48 axes other than the color scheme, which [`StartContext`] already
/// carried. In a start context each is optional and defaults; in an
/// [`Environment`] each is required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentAxes {
    #[serde(default)]
    pub size_class: SizeClass,
    #[serde(default)]
    pub pointer: Pointer,
    #[serde(default)]
    pub hover: Hover,
    #[serde(default)]
    pub orientation: Orientation,
    #[serde(default)]
    pub reduced_motion: ReducedMotion,
}

/// The whole host environment (UI48 §4), as `environmentChanged` delivers it:
/// one coalesced value, because rotating a device changes orientation and
/// size class together and an app should never see the state in between.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub color_scheme: ColorScheme,
    #[serde(flatten)]
    pub axes: EnvironmentAxes,
}

impl Environment {
    /// Decode an `environmentChanged` payload. Every axis is required here —
    /// a change event that leaves one out is a host bug, not a default — and
    /// unknown keys are ignored, so a newer host may add an axis.
    pub fn from_payload(payload: &Value) -> Result<Self, String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire {
            color_scheme: ColorScheme,
            size_class: SizeClass,
            pointer: Pointer,
            hover: Hover,
            orientation: Orientation,
            reduced_motion: ReducedMotion,
        }
        let wire = Wire::deserialize(payload).map_err(|error| error.to_string())?;
        Ok(Self {
            color_scheme: wire.color_scheme,
            axes: EnvironmentAxes {
                size_class: wire.size_class,
                pointer: wire.pointer,
                hover: wire.hover,
                orientation: wire.orientation,
                reduced_motion: wire.reduced_motion,
            },
        })
    }

    /// The event a host sends to report this environment.
    pub fn into_event(self, sequence: u64) -> Event {
        Event::new(
            sequence,
            ENVIRONMENT_CHANGED,
            serde_json::to_value(self).expect("an environment always serializes"),
        )
    }
}

/// A semantic UI event. Awaited results use the dedicated completion method.
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
    pub id: EffectId,
    #[serde(default)]
    pub delivery: Delivery,
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
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub protocol_version: u32,
    pub revision: u64,
    pub props: Value,
    pub effects: Vec<Effect>,
    pub announcements: Vec<Announcement>,
}

// Protocol 1 keeps its original effect shape; v2 always includes Delivery.
impl Serialize for Update {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let effects: Vec<Value> = self.effects.iter().map(|effect| {
            if self.protocol_version == PROTOCOL_VERSION {
                serde_json::json!({"id": effect.id, "kind": effect.kind, "payload": effect.payload})
            } else { serde_json::json!(effect) }
        }).collect();
        let mut wire = serializer.serialize_struct("Update", 5)?;
        wire.serialize_field("protocolVersion", &self.protocol_version)?;
        wire.serialize_field("revision", &self.revision)?;
        wire.serialize_field("props", &self.props)?;
        wire.serialize_field("effects", &effects)?;
        wire.serialize_field("announcements", &self.announcements)?;
        wire.end()
    }
}

impl Update {
    fn from_app(protocol_version: u32, revision: u64, app: AppUpdate) -> Self {
        Self {
            protocol_version,
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
    protocol_version: u32,
    pending: BTreeSet<EffectId>,
    last_effect_id: EffectId,
    poisoned: bool,
    environment: Environment,
}

impl<A: MosaicApp> MosaicRuntime<A> {
    pub fn new(app: A) -> Self {
        Self {
            app,
            state: RuntimeState::Created,
            protocol_version: PROTOCOL_VERSION,
            pending: BTreeSet::new(),
            last_effect_id: 0,
            poisoned: false,
            environment: StartContext::new("", Platform::Web).full_environment(),
        }
    }

    /// The host environment as last reported: by the start context, then by
    /// each `environmentChanged` (UI48).
    pub fn environment(&self) -> Environment {
        self.environment
    }

    fn healthy(&self) -> Result<(), RuntimeError<A::Error>> {
        if self.poisoned {
            Err(RuntimeError::Poisoned)
        } else {
            Ok(())
        }
    }

    // Bad application output cannot be rolled back generically. Refuse further
    // use of that instance instead of retrying against potentially changed state.
    fn accept_effects(&mut self, update: &AppUpdate) -> Result<(), RuntimeError<A::Error>> {
        let mut batch = BTreeSet::new();
        for effect in &update.effects {
            let unsupported = self.protocol_version < EFFECT_PROTOCOL_VERSION
                && effect.delivery == Delivery::Await;
            let invalid = self.protocol_version == EFFECT_PROTOCOL_VERSION
                && (effect.id <= self.last_effect_id
                    || effect.id > MAX_EFFECT_ID
                    || !batch.insert(effect.id));
            if unsupported || invalid {
                self.poisoned = true;
                return Err(if unsupported {
                    RuntimeError::EffectsRequireV2
                } else {
                    RuntimeError::InvalidEffectId(effect.id)
                });
            }
        }
        if self.protocol_version == EFFECT_PROTOCOL_VERSION {
            if let Some(id) = batch.last() {
                self.last_effect_id = *id;
            }
            self.pending.extend(
                update
                    .effects
                    .iter()
                    .filter(|effect| effect.delivery == Delivery::Await)
                    .map(|effect| effect.id),
            );
        }
        Ok(())
    }

    /// Start the app exactly once and assign revision 1.
    pub fn start(&mut self, context: StartContext) -> Result<Update, RuntimeError<A::Error>> {
        self.healthy()?;
        if self.state != RuntimeState::Created {
            return Err(RuntimeError::AlreadyStarted);
        }
        validate_protocol(context.protocol_version)?;
        if !context.text_scale.is_finite() || context.text_scale <= 0.0 {
            return Err(RuntimeError::InvalidTextScale);
        }
        if let Some(offset) = context.utc_offset_minutes {
            if !(MIN_UTC_OFFSET_MINUTES..=MAX_UTC_OFFSET_MINUTES).contains(&offset) {
                return Err(RuntimeError::InvalidUtcOffset);
            }
        }
        let version = context.protocol_version;
        let environment = context.full_environment();
        let app_update = self.app.start(context).map_err(RuntimeError::Application)?;
        self.environment = environment;
        self.protocol_version = version;
        self.accept_effects(&app_update)?;
        self.state = RuntimeState::Running {
            last_sequence: 0,
            revision: 1,
        };
        Ok(Update::from_app(version, 1, app_update))
    }

    /// Dispatch the next event in sequence and assign the next revision.
    pub fn dispatch(&mut self, event: Event) -> Result<Update, RuntimeError<A::Error>> {
        self.healthy()?;
        validate_protocol(event.protocol_version)?;
        let RuntimeState::Running {
            last_sequence,
            revision,
        } = self.state
        else {
            return Err(RuntimeError::NotStarted);
        };
        if event.protocol_version != self.protocol_version {
            return Err(RuntimeError::ProtocolVersionMismatch {
                expected: self.protocol_version,
                received: event.protocol_version,
            });
        }
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
        if event.name == ENVIRONMENT_CHANGED {
            return self.environment_changed(&event.payload, expected, revision, next_revision);
        }
        let app_update = self
            .app
            .dispatch(event)
            .map_err(RuntimeError::Application)?;
        self.accept_effects(&app_update)?;
        self.state = RuntimeState::Running {
            last_sequence: expected,
            revision: next_revision,
        };
        Ok(Update::from_app(
            self.protocol_version,
            next_revision,
            app_update,
        ))
    }

    /// The `environmentChanged` branch of [`Self::dispatch`] (UI48 ENV1). An
    /// invalid payload is refused before the app sees it and consumes no
    /// sequence. An app that does not react still consumes the sequence, but
    /// the update keeps the current revision and carries no props: a host
    /// renders nothing for an update that is not newer than the last.
    fn environment_changed(
        &mut self,
        payload: &Value,
        sequence: u64,
        revision: u64,
        next_revision: u64,
    ) -> Result<Update, RuntimeError<A::Error>> {
        let environment =
            Environment::from_payload(payload).map_err(RuntimeError::InvalidEnvironment)?;
        let reaction = self
            .app
            .environment_changed(environment)
            .map_err(RuntimeError::Application)?;
        self.environment = environment;
        match reaction {
            Some(app_update) => {
                self.accept_effects(&app_update)?;
                self.state = RuntimeState::Running {
                    last_sequence: sequence,
                    revision: next_revision,
                };
                Ok(Update::from_app(self.protocol_version, next_revision, app_update))
            }
            None => {
                self.state = RuntimeState::Running {
                    last_sequence: sequence,
                    revision,
                };
                Ok(Update {
                    protocol_version: self.protocol_version,
                    revision,
                    props: Value::Null,
                    effects: Vec::new(),
                    announcements: Vec::new(),
                })
            }
        }
    }

    fn settled(&self) -> Result<(), RuntimeError<A::Error>> {
        self.healthy()?;
        if self.state == RuntimeState::Created {
            return Err(RuntimeError::NotStarted);
        }
        if !self.pending.is_empty() {
            return Err(RuntimeError::PendingEffects(self.pending_effects()));
        }
        Ok(())
    }

    /// Snapshot a settled app without changing its sequence or revision.
    pub fn snapshot(&self) -> Result<Option<Snapshot>, RuntimeError<A::Error>> {
        self.settled()?;
        self.app.snapshot().map_err(RuntimeError::Application)
    }

    /// Restore a settled app without consuming an event or recycling effect IDs.
    pub fn restore(&mut self, snapshot: Snapshot) -> Result<Update, RuntimeError<A::Error>> {
        self.settled()?;
        let RuntimeState::Running {
            last_sequence,
            revision,
        } = self.state
        else {
            unreachable!()
        };
        let next_revision = revision
            .checked_add(1)
            .ok_or(RuntimeError::RevisionOverflow)?;
        let app_update = self
            .app
            .restore(snapshot)
            .map_err(RuntimeError::Application)?;
        self.accept_effects(&app_update)?;
        self.state = RuntimeState::Running {
            last_sequence,
            revision: next_revision,
        };
        Ok(Update::from_app(
            self.protocol_version,
            next_revision,
            app_update,
        ))
    }

    /// Accept a pending result and advance revision, preserving UI event sequence.
    pub fn complete_effect(
        &mut self,
        id: EffectId,
        result: EffectResult,
    ) -> Result<Update, RuntimeError<A::Error>> {
        self.healthy()?;
        let RuntimeState::Running {
            last_sequence,
            revision,
        } = self.state
        else {
            return Err(RuntimeError::NotStarted);
        };
        if self.protocol_version < EFFECT_PROTOCOL_VERSION {
            return Err(RuntimeError::EffectsRequireV2);
        }
        if !self.pending.contains(&id) {
            return Err(RuntimeError::UnknownEffect(id));
        }
        let next_revision = revision
            .checked_add(1)
            .ok_or(RuntimeError::RevisionOverflow)?;
        let app_update = self
            .app
            .complete_effect(id, result)
            .map_err(|error| match error {
                EffectCompletionError::Unsupported => RuntimeError::CompletionUnsupported,
                EffectCompletionError::Application(error) => RuntimeError::Application(error),
            })?;
        self.accept_effects(&app_update)?;
        self.pending.remove(&id);
        self.state = RuntimeState::Running {
            last_sequence,
            revision: next_revision,
        };
        Ok(Update::from_app(
            self.protocol_version,
            next_revision,
            app_update,
        ))
    }

    /// Outstanding Await IDs, in ascending order, for host diagnostics.
    pub fn pending_effects(&self) -> Vec<EffectId> {
        self.pending.iter().copied().collect()
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
    if matches!(received, PROTOCOL_VERSION | EFFECT_PROTOCOL_VERSION) {
        Ok(())
    } else {
        Err(RuntimeError::ProtocolVersionMismatch {
            expected: EFFECT_PROTOCOL_VERSION,
            received,
        })
    }
}

/// A host-protocol or application error.
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeError<E> {
    ProtocolVersionMismatch { expected: u32, received: u32 },
    InvalidTextScale,
    InvalidUtcOffset,
    AlreadyStarted,
    NotStarted,
    UnexpectedSequence { expected: u64, received: u64 },
    SequenceOverflow,
    RevisionOverflow,
    Application(E),
    PendingEffects(Vec<EffectId>),
    UnknownEffect(EffectId),
    InvalidEffectId(EffectId),
    EffectsRequireV2,
    CompletionUnsupported,
    Poisoned,
    /// An `environmentChanged` payload that is not a whole environment.
    InvalidEnvironment(String),
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
            Self::InvalidUtcOffset => f.write_str(
                "Mosaic UTC offset must be between -840 and 840 minutes (UTC-14:00 to UTC+14:00)",
            ),
            Self::AlreadyStarted => f.write_str("Mosaic application has already started"),
            Self::NotStarted => f.write_str("Mosaic application has not started"),
            Self::UnexpectedSequence { expected, received } => write!(
                f,
                "unexpected Mosaic event sequence: expected {expected}, received {received}"
            ),
            Self::SequenceOverflow => f.write_str("Mosaic event sequence overflow"),
            Self::RevisionOverflow => f.write_str("Mosaic update revision overflow"),
            Self::Application(error) => write!(f, "Mosaic application error: {error}"),
            Self::PendingEffects(ids) => write!(f, "Mosaic pending effects: {ids:?}"),
            Self::UnknownEffect(id) => write!(f, "unknown or completed Mosaic effect: {id}"),
            Self::InvalidEffectId(id) => write!(f, "invalid or reused Mosaic effect id: {id}"),
            Self::EffectsRequireV2 => f.write_str("awaited Mosaic effects require protocol 2"),
            Self::CompletionUnsupported => {
                f.write_str("application does not implement effect completion")
            }
            Self::Poisoned => f.write_str("Mosaic instance produced invalid effects; recreate it"),
            Self::InvalidEnvironment(detail) => {
                write!(f, "invalid Mosaic environmentChanged payload: {detail}")
            }
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
        effect: Option<(EffectId, Delivery)>,
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
                    id: self.effect.unwrap_or((7, Delivery::Notify)).0,
                    delivery: self.effect.unwrap_or((7, Delivery::Notify)).1,
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

    // ------------------------------------------------------------------
    // UI48 ENV1: the environment
    // ------------------------------------------------------------------

    /// An app that reacts to its environment, and remembers what it saw.
    #[derive(Default)]
    struct AdaptiveApp {
        started_in: Option<Environment>,
        seen: Vec<Environment>,
        fail_next_change: bool,
    }

    impl MosaicApp for AdaptiveApp {
        type Error = TestError;

        fn start(&mut self, context: StartContext) -> Result<AppUpdate, Self::Error> {
            self.started_in = Some(context.full_environment());
            Ok(AppUpdate::new(json!({ "layout": "regular" })))
        }

        fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error> {
            Ok(AppUpdate::new(json!({ "event": event.name })))
        }

        fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error> {
            Ok(None)
        }

        fn restore(&mut self, _snapshot: Snapshot) -> Result<AppUpdate, Self::Error> {
            Ok(AppUpdate::new(json!({})))
        }

        fn environment_changed(
            &mut self,
            environment: Environment,
        ) -> Result<Option<AppUpdate>, Self::Error> {
            if std::mem::take(&mut self.fail_next_change) {
                return Err(TestError);
            }
            self.seen.push(environment);
            let layout = match environment.axes.size_class {
                SizeClass::Compact => "compact",
                _ => "regular",
            };
            Ok(Some(AppUpdate::new(json!({ "layout": layout }))))
        }
    }

    fn phone() -> Environment {
        Environment {
            color_scheme: ColorScheme::Dark,
            axes: EnvironmentAxes {
                size_class: SizeClass::Compact,
                pointer: Pointer::Coarse,
                hover: Hover::None,
                orientation: Orientation::Portrait,
                reduced_motion: ReducedMotion::Reduce,
            },
        }
    }

    #[test]
    fn start_contexts_without_the_new_axes_decode_with_the_defaults() {
        let context: StartContext = serde_json::from_value(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "locale": "en-US",
            "colorScheme": "light",
            "textScale": 1.0,
            "platform": "apple",
            "restoredSnapshot": null
        }))
        .unwrap();
        assert_eq!(context.environment, EnvironmentAxes::default());
        let environment = context.full_environment();
        assert_eq!(environment.color_scheme, ColorScheme::Light);
        assert_eq!(environment.axes.size_class, SizeClass::Regular);
        assert_eq!(environment.axes.pointer, Pointer::Fine);
        assert_eq!(environment.axes.hover, Hover::Hover);
        assert_eq!(environment.axes.orientation, Orientation::Landscape);
        assert_eq!(environment.axes.reduced_motion, ReducedMotion::NoPreference);
    }

    #[test]
    fn start_contexts_carry_the_axes_flat_beside_the_color_scheme() {
        let mut context = start_context();
        context.environment = phone().axes;
        let wire = serde_json::to_value(&context).unwrap();
        assert_eq!(wire["sizeClass"], "compact");
        assert_eq!(wire["pointer"], "coarse");
        assert_eq!(wire["hover"], "none");
        assert_eq!(wire["orientation"], "portrait");
        assert_eq!(wire["reducedMotion"], "reduce");
        let decoded: StartContext = serde_json::from_value(wire).unwrap();
        assert_eq!(decoded.environment, phone().axes);

        let mut runtime = MosaicRuntime::new(AdaptiveApp::default());
        runtime.start(context).unwrap();
        assert_eq!(runtime.environment().axes, phone().axes);
    }

    #[test]
    fn environment_payloads_are_whole_and_tolerate_new_axes() {
        let mut payload = serde_json::to_value(phone()).unwrap();
        assert_eq!(payload["colorScheme"], "dark");
        assert_eq!(payload["reducedMotion"], "reduce");
        assert_eq!(Environment::from_payload(&payload).unwrap(), phone());
        payload["futureAxis"] = json!("anything");
        assert_eq!(Environment::from_payload(&payload).unwrap(), phone());
        payload.as_object_mut().unwrap().remove("orientation");
        assert!(Environment::from_payload(&payload).is_err());
        assert!(Environment::from_payload(&json!({"colorScheme": "purple"})).is_err());
        assert!(Environment::from_payload(&json!(null)).is_err());
    }

    #[test]
    fn an_app_that_reacts_rerenders_at_the_next_revision() {
        let mut runtime = MosaicRuntime::new(AdaptiveApp::default());
        runtime.start(start_context()).unwrap();
        let update = runtime.dispatch(phone().into_event(1)).unwrap();
        assert_eq!(update.revision, 2);
        assert_eq!(update.props, json!({ "layout": "compact" }));
        assert_eq!(runtime.environment(), phone());
        // The change consumed sequence 1; an ordinary event follows at 2.
        let next = runtime.dispatch(Event::new(2, "onTap", json!({}))).unwrap();
        assert_eq!(next.revision, 3);
    }

    #[test]
    fn an_app_that_ignores_it_consumes_the_sequence_and_keeps_its_revision() {
        let mut runtime = MosaicRuntime::new(TestApp::default());
        runtime.start(start_context()).unwrap();
        let update = runtime.dispatch(phone().into_event(1)).unwrap();
        assert_eq!(update.revision, 1, "nothing new to render");
        assert_eq!(update.props, Value::Null);
        assert!(update.effects.is_empty() && update.announcements.is_empty());
        assert_eq!(runtime.environment(), phone());
        // TestApp::dispatch was never called with the reserved name.
        assert_eq!(runtime.app.dispatches, 0);
        let next = runtime.dispatch(Event::new(2, "onTap", json!({}))).unwrap();
        assert_eq!(next.revision, 2);
    }

    #[test]
    fn an_invalid_environment_is_refused_before_the_app_and_consumes_nothing() {
        let mut runtime = MosaicRuntime::new(AdaptiveApp::default());
        runtime.start(start_context()).unwrap();
        let before = runtime.environment();
        let error = runtime
            .dispatch(Event::new(1, ENVIRONMENT_CHANGED, json!({ "sizeClass": "compact" })))
            .unwrap_err();
        assert!(matches!(error, RuntimeError::InvalidEnvironment(_)), "{error}");
        assert!(runtime.app.seen.is_empty());
        assert_eq!(runtime.environment(), before);
        // Sequence 1 is still next.
        assert_eq!(runtime.dispatch(phone().into_event(1)).unwrap().revision, 2);
    }

    #[test]
    fn an_app_error_on_a_change_leaves_the_environment_and_sequence_unchanged() {
        let mut runtime = MosaicRuntime::new(AdaptiveApp {
            fail_next_change: true,
            ..AdaptiveApp::default()
        });
        runtime.start(start_context()).unwrap();
        let before = runtime.environment();
        assert!(matches!(
            runtime.dispatch(phone().into_event(1)),
            Err(RuntimeError::Application(TestError))
        ));
        assert_eq!(runtime.environment(), before);
        assert_eq!(runtime.dispatch(phone().into_event(1)).unwrap().revision, 2);
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
    fn version_one_wire_shape_is_unchanged_and_version_two_is_explicit() {
        for version in [PROTOCOL_VERSION, EFFECT_PROTOCOL_VERSION] {
            let mut runtime = MosaicRuntime::new(TestApp::default());
            runtime
                .start(StartContext {
                    protocol_version: version,
                    ..start_context()
                })
                .unwrap();
            let mut event = Event::new(1, "increment", json!({}));
            event.protocol_version = version;
            let update = runtime.dispatch(event).unwrap();
            let wire = serde_json::to_value(&update).unwrap();
            if version == PROTOCOL_VERSION {
                assert_eq!(
                    wire["effects"][0],
                    json!({"id": 7, "kind": "storage.set", "payload": {"key": "counter", "value": 1}})
                );
            } else {
                assert_eq!(wire["effects"][0]["delivery"], "notify");
            }
            assert_eq!(serde_json::from_value::<Update>(wire).unwrap(), update);
        }
    }

    #[test]
    fn invalid_output_poisoning_prevents_retry_against_changed_app_state() {
        for (version, id, delivery) in [
            (1, 1, Delivery::Await),
            (2, 0, Delivery::Notify),
            (2, MAX_EFFECT_ID + 1, Delivery::Await),
        ] {
            let mut runtime = MosaicRuntime::new(TestApp {
                effect: Some((id, delivery)),
                ..TestApp::default()
            });
            runtime
                .start(StartContext {
                    protocol_version: version,
                    ..start_context()
                })
                .unwrap();
            let mut event = Event::new(1, "increment", json!({}));
            event.protocol_version = version;
            assert!(runtime.dispatch(event.clone()).is_err());
            assert!(matches!(
                runtime.dispatch(event),
                Err(RuntimeError::Poisoned)
            ));
            assert!(matches!(runtime.snapshot(), Err(RuntimeError::Poisoned)));
            assert_eq!(runtime.app().dispatches, 1);
        }
        let mut runtime = MosaicRuntime::new(TestApp::default());
        runtime
            .start(StartContext {
                protocol_version: 2,
                ..start_context()
            })
            .unwrap();
        for sequence in [1, 2] {
            let mut event = Event::new(sequence, "increment", json!({}));
            event.protocol_version = 2;
            let result = runtime.dispatch(event);
            if sequence == 1 {
                assert!(result.is_ok());
            } else {
                assert!(matches!(result, Err(RuntimeError::InvalidEffectId(7))));
            }
        }
        assert!(matches!(
            runtime.restore(snapshot()),
            Err(RuntimeError::Poisoned)
        ));
        assert_eq!(runtime.app().restores, 0);
    }

    #[test]
    fn default_completion_and_pending_restore_never_silently_drop_work() {
        let mut runtime = MosaicRuntime::new(TestApp {
            effect: Some((1, Delivery::Await)),
            ..TestApp::default()
        });
        runtime
            .start(StartContext {
                protocol_version: 2,
                ..start_context()
            })
            .unwrap();
        let mut event = Event::new(1, "increment", json!({}));
        event.protocol_version = 2;
        runtime.dispatch(event).unwrap();
        assert!(matches!(
            runtime.restore(snapshot()),
            Err(RuntimeError::PendingEffects(_))
        ));
        assert_eq!(runtime.app().restores, 0);
        assert!(matches!(
            runtime.complete_effect(1, EffectResult::Cancelled(EmptyOutcome {})),
            Err(RuntimeError::CompletionUnsupported)
        ));
        assert_eq!(runtime.pending_effects(), vec![1]);
        assert_eq!(runtime.current_revision(), Some(2));
        assert_eq!(runtime.next_sequence(), Some(2));
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
            protocol_version: EFFECT_PROTOCOL_VERSION + 1,
            ..start_context()
        };
        assert!(matches!(
            runtime.start(context),
            Err(RuntimeError::ProtocolVersionMismatch {
                expected: EFFECT_PROTOCOL_VERSION,
                received
            }) if received == EFFECT_PROTOCOL_VERSION + 1
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
    fn rejects_an_implausible_utc_offset_before_calling_the_app() {
        for offset in [-841, 841, i32::MIN, i32::MAX] {
            let mut runtime = MosaicRuntime::new(TestApp::default());
            let mut context = start_context();
            context.utc_offset_minutes = Some(offset);
            assert!(matches!(
                runtime.start(context),
                Err(RuntimeError::InvalidUtcOffset)
            ));
            assert_eq!(runtime.app().starts, 0);
        }
        for offset in [
            MIN_UTC_OFFSET_MINUTES,
            -300,
            0,
            330,
            345,
            MAX_UTC_OFFSET_MINUTES,
        ] {
            let mut runtime = MosaicRuntime::new(TestApp::default());
            let mut context = start_context();
            context.utc_offset_minutes = Some(offset);
            assert!(runtime.start(context).is_ok(), "{offset}");
        }
    }

    /// Optional on the wire both ways: an earlier host's context (no key)
    /// decodes to `None`, and `None` is not written, so an earlier app that
    /// round-trips the context never sees an unknown key.
    #[test]
    fn the_utc_offset_is_optional_on_the_wire() {
        let without = serde_json::to_value(start_context()).unwrap();
        assert!(without.get("utcOffsetMinutes").is_none());
        let decoded: StartContext = serde_json::from_value(without).unwrap();
        assert_eq!(decoded.utc_offset_minutes, None);

        let mut context = start_context();
        context.utc_offset_minutes = Some(-300);
        let with = serde_json::to_value(&context).unwrap();
        assert_eq!(with["utcOffsetMinutes"], -300);
        let decoded: StartContext = serde_json::from_value(with).unwrap();
        assert_eq!(decoded.utc_offset_minutes, Some(-300));
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
