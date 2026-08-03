//! Authenticated WebSocket control API for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_orchestrator_core::{
    ChannelWiringAuthorizer, HostHealth, OrchestratorCore, OrchestratorCoreError,
};
use chief_of_staff_service_reconciler::{
    HostSupervisor, ReconcileAction, ReconcileReport, SupervisorObservation, SupervisorPhase,
};
use chief_of_staff_service_registry::{
    DesiredState, HostName, HostRegistration, HostStatus, LoadedHost, PackagePath, RegistryError,
    RestartPolicy,
};
use coding_adventures_json_parser::try_parse_json;
use coding_adventures_json_serializer::serialize;
use coding_adventures_json_value::{from_ast, JsonNumber, JsonValue};
use core::fmt::{self, Display, Formatter};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use transport_platform::TransportPlatform;
use websocket_core::{Frame, MessageEvent};
use websocket_runtime::{
    WebSocketConnectionInfo, WebSocketHandlerResult, WebSocketRuntime, WebSocketRuntimeError,
    WebSocketServerOptions,
};

pub use tcp_runtime::BindAddress;

const PROTOCOL_VERSION: i64 = 1;
const MAX_REQUEST_BYTES: usize = 64 * 1024;
const MAX_REQUEST_ID_BYTES: usize = 64;
const MAX_CREDENTIAL_BYTES: usize = 4096;

/// One operation considered by the injected session authorization policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Register immutable host package identity and initial intent.
    RegisterHost,
    /// List durable host records.
    ListHosts,
    /// Change durable desired lifecycle state.
    SetDesiredState,
    /// Run one bounded reconciliation tick.
    ReconcileOnce,
    /// Inspect durable and authoritative host health.
    HealthCheck,
    /// Delete stopped, inactive host intent.
    DeregisterHost,
}

/// Authentication and per-operation authorization boundary.
pub trait SessionAuthorizer {
    /// Opaque connection-local session authority.
    type Session;
    /// Adapter-specific failure that is never returned over the wire.
    type Error;

    /// Exchange one bounded opaque credential for a connection-local session.
    fn authenticate(&self, credential: &str) -> Result<Self::Session, Self::Error>;

    /// Decide whether this authenticated session may perform one operation.
    fn authorize(&self, session: &Self::Session, operation: Operation)
        -> Result<bool, Self::Error>;
}

/// Stable payload-blind failure from the injected control plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlPlaneError {
    /// Caller input violated a control-plane invariant.
    InvalidInput,
    /// The named durable object does not exist.
    NotFound,
    /// Current durable or authoritative state conflicts with the request.
    Conflict,
    /// A trust boundary denied the operation.
    Forbidden,
    /// An adapter, storage backend, or process authority failed.
    Internal,
}

/// Host lifecycle surface consumed by the daemon protocol.
pub trait ChiefControlPlane {
    /// Register immutable package identity and initial desired state.
    fn register_host(
        &mut self,
        registration: HostRegistration,
        desired_state: DesiredState,
    ) -> Result<LoadedHost, ControlPlaneError>;

    /// Return durable hosts in stable host-name order.
    fn list_hosts(&mut self) -> Result<Vec<LoadedHost>, ControlPlaneError>;

    /// CAS-update one host's desired lifecycle state.
    fn set_desired_state(
        &mut self,
        host_name: &HostName,
        desired_state: DesiredState,
    ) -> Result<LoadedHost, ControlPlaneError>;

    /// Run one deterministic bounded reconciliation tick.
    fn reconcile_once(&mut self) -> Result<ReconcileReport, ControlPlaneError>;

    /// Return durable intent and fresh supervisor authority separately.
    fn health_check(&mut self, host_name: &HostName) -> Result<HostHealth, ControlPlaneError>;

    /// Safely delete stopped intent for an absent or exited host.
    fn deregister_host(&mut self, host_name: &HostName) -> Result<(), ControlPlaneError>;
}

impl<S, A> ChiefControlPlane for OrchestratorCore<'_, S, A>
where
    S: HostSupervisor,
    A: ChannelWiringAuthorizer,
{
    fn register_host(
        &mut self,
        registration: HostRegistration,
        desired_state: DesiredState,
    ) -> Result<LoadedHost, ControlPlaneError> {
        OrchestratorCore::register_host(self, registration, desired_state).map_err(map_core_error)
    }

    fn list_hosts(&mut self) -> Result<Vec<LoadedHost>, ControlPlaneError> {
        OrchestratorCore::list_hosts(self).map_err(map_core_error)
    }

    fn set_desired_state(
        &mut self,
        host_name: &HostName,
        desired_state: DesiredState,
    ) -> Result<LoadedHost, ControlPlaneError> {
        OrchestratorCore::set_desired_state(self, host_name, desired_state).map_err(map_core_error)
    }

    fn reconcile_once(&mut self) -> Result<ReconcileReport, ControlPlaneError> {
        OrchestratorCore::reconcile_once(self).map_err(map_core_error)
    }

    fn health_check(&mut self, host_name: &HostName) -> Result<HostHealth, ControlPlaneError> {
        OrchestratorCore::health_check(self, host_name).map_err(map_core_error)
    }

    fn deregister_host(&mut self, host_name: &HostName) -> Result<(), ControlPlaneError> {
        OrchestratorCore::deregister_host(self, host_name).map_err(map_core_error)
    }
}

fn map_core_error<S, A>(error: OrchestratorCoreError<S, A>) -> ControlPlaneError {
    match error {
        OrchestratorCoreError::Registry(error) => map_registry_error(error),
        OrchestratorCoreError::Authorization(_) => ControlPlaneError::Forbidden,
        OrchestratorCoreError::HostDesiredRunning(_)
        | OrchestratorCoreError::HostStillActive(_)
        | OrchestratorCoreError::ClockRegressed => ControlPlaneError::Conflict,
        OrchestratorCoreError::Reconciliation(_)
        | OrchestratorCoreError::Supervisor { .. }
        | OrchestratorCoreError::Channel(_) => ControlPlaneError::Internal,
    }
}

fn map_registry_error(error: RegistryError) -> ControlPlaneError {
    match error {
        RegistryError::InvalidField { .. } => ControlPlaneError::InvalidInput,
        RegistryError::HostNotFound(_) => ControlPlaneError::NotFound,
        RegistryError::ConflictingRegistration(_) | RegistryError::ConcurrentUpdate(_) => {
            ControlPlaneError::Conflict
        }
        RegistryError::Storage(_)
        | RegistryError::CorruptRecord(_)
        | RegistryError::TooManyHosts => ControlPlaneError::Internal,
    }
}

/// Connection-local application state. It never retains the presented credential.
pub struct DaemonSession<S> {
    authority: Option<S>,
}

impl<S> Default for DaemonSession<S> {
    fn default() -> Self {
        Self { authority: None }
    }
}

impl<S> DaemonSession<S> {
    /// Whether this connection currently has authenticated authority.
    pub fn is_authenticated(&self) -> bool {
        self.authority.is_some()
    }
}

/// Thread-safe protocol dispatcher over one mutable orchestrator control plane.
pub struct DaemonApi<C, A> {
    control_plane: Mutex<C>,
    authorizer: A,
}

impl<C, A> DaemonApi<C, A> {
    /// Bind a control plane and authentication policy without opening a socket.
    pub fn new(control_plane: C, authorizer: A) -> Self {
        Self {
            control_plane: Mutex::new(control_plane),
            authorizer,
        }
    }

    /// Recover the owned adapters when no WebSocket handlers retain this API.
    pub fn into_parts(self) -> Result<(C, A), DaemonApiError> {
        self.control_plane
            .into_inner()
            .map(|control_plane| (control_plane, self.authorizer))
            .map_err(|_| DaemonApiError::Poisoned)
    }
}

/// Local API ownership failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonApiError {
    /// A control-plane operation panicked while holding the synchronization boundary.
    Poisoned,
}

impl Display for DaemonApiError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("chief daemon API control plane is unavailable")
    }
}

impl std::error::Error for DaemonApiError {}

impl<C, A> DaemonApi<C, A>
where
    C: ChiefControlPlane,
    A: SessionAuthorizer,
{
    /// Handle one complete WebSocket event for one connection-local session.
    pub fn handle(
        &self,
        session: &mut DaemonSession<A::Session>,
        event: MessageEvent,
    ) -> WebSocketHandlerResult {
        match event {
            MessageEvent::Text(text) => {
                WebSocketHandlerResult::send(Frame::text(self.handle_text(session, &text)))
            }
            MessageEvent::Binary(_) => WebSocketHandlerResult::close(Some(1003), "text required")
                .unwrap_or_else(|_| WebSocketHandlerResult {
                    frames: Vec::new(),
                    close: true,
                }),
            MessageEvent::Ping(_) | MessageEvent::Pong(_) | MessageEvent::Close(_) => {
                WebSocketHandlerResult::default()
            }
        }
    }

    fn handle_text(&self, session: &mut DaemonSession<A::Session>, text: &str) -> String {
        if text.len() > MAX_REQUEST_BYTES {
            return error_response("", PublicError::InvalidRequest);
        }
        let request = match decode_request(text) {
            Ok(request) => request,
            Err(error) => return error_response(&error.id, error.error),
        };
        if request.method == Method::Authenticate {
            return self.authenticate(session, request);
        }
        let Some(authority) = session.authority.as_ref() else {
            return error_response(&request.id, PublicError::Unauthenticated);
        };
        let operation = request
            .method
            .operation()
            .expect("non-auth method has operation");
        match self.authorizer.authorize(authority, operation) {
            Ok(true) => {}
            Ok(false) => return error_response(&request.id, PublicError::Forbidden),
            Err(_) => return error_response(&request.id, PublicError::Internal),
        }
        let mut control_plane = match self.control_plane.lock() {
            Ok(control_plane) => control_plane,
            Err(_) => return error_response(&request.id, PublicError::Internal),
        };
        match dispatch(&mut *control_plane, request.method, &request.params) {
            Ok(result) => success_response(&request.id, result),
            Err(error) => error_response(&request.id, error),
        }
    }

    fn authenticate(&self, session: &mut DaemonSession<A::Session>, request: Request) -> String {
        if session.authority.is_some() {
            return error_response(&request.id, PublicError::AlreadyAuthenticated);
        }
        let credential = match authenticate_credential(&request.params) {
            Ok(credential) => credential,
            Err(error) => return error_response(&request.id, error),
        };
        match self.authorizer.authenticate(credential) {
            Ok(authority) => {
                session.authority = Some(authority);
                success_response(
                    &request.id,
                    object(vec![("authenticated", JsonValue::Bool(true))]),
                )
            }
            Err(_) => error_response(&request.id, PublicError::AuthenticationFailed),
        }
    }
}

/// Bind an authenticated Chief API to the repository WebSocket reactor runtime.
pub fn bind_daemon<P, C, A>(
    platform: P,
    address: BindAddress,
    mut options: WebSocketServerOptions,
    api: Arc<DaemonApi<C, A>>,
) -> Result<WebSocketRuntime<P, DaemonSession<A::Session>>, WebSocketRuntimeError>
where
    P: TransportPlatform,
    C: ChiefControlPlane + Send + 'static,
    A: SessionAuthorizer + Send + Sync + 'static,
    A::Session: Send + 'static,
{
    options.max_frame_payload = options.max_frame_payload.min(MAX_REQUEST_BYTES);
    options.max_message_payload = options.max_message_payload.min(MAX_REQUEST_BYTES);
    let handler_api = Arc::clone(&api);
    WebSocketRuntime::bind_with_state(
        platform,
        address,
        options,
        |_| DaemonSession::default(),
        move |_info: WebSocketConnectionInfo, session, event| handler_api.handle(session, event),
        |_info, _session| {},
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Method {
    Authenticate,
    RegisterHost,
    ListHosts,
    SetDesiredState,
    ReconcileOnce,
    HealthCheck,
    DeregisterHost,
}

impl Method {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "authenticate" => Some(Self::Authenticate),
            "register_host" => Some(Self::RegisterHost),
            "list_hosts" => Some(Self::ListHosts),
            "set_desired_state" => Some(Self::SetDesiredState),
            "reconcile_once" => Some(Self::ReconcileOnce),
            "health_check" => Some(Self::HealthCheck),
            "deregister_host" => Some(Self::DeregisterHost),
            _ => None,
        }
    }

    fn operation(self) -> Option<Operation> {
        match self {
            Self::Authenticate => None,
            Self::RegisterHost => Some(Operation::RegisterHost),
            Self::ListHosts => Some(Operation::ListHosts),
            Self::SetDesiredState => Some(Operation::SetDesiredState),
            Self::ReconcileOnce => Some(Operation::ReconcileOnce),
            Self::HealthCheck => Some(Operation::HealthCheck),
            Self::DeregisterHost => Some(Operation::DeregisterHost),
        }
    }
}

struct Request {
    id: String,
    method: Method,
    params: Vec<(String, JsonValue)>,
}

struct DecodeError {
    id: String,
    error: PublicError,
}

fn decode_request(text: &str) -> Result<Request, DecodeError> {
    let ast = try_parse_json(text).map_err(|_| DecodeError {
        id: String::new(),
        error: PublicError::InvalidRequest,
    })?;
    let value = from_ast(&ast).map_err(|_| DecodeError {
        id: String::new(),
        error: PublicError::InvalidRequest,
    })?;
    if has_duplicate_object_keys(&value) {
        return Err(DecodeError {
            id: String::new(),
            error: PublicError::InvalidRequest,
        });
    }
    let fields = match &value {
        JsonValue::Object(fields) => fields,
        _ => {
            return Err(DecodeError {
                id: String::new(),
                error: PublicError::InvalidRequest,
            })
        }
    };
    let id = valid_request_id(field(fields, "id")).unwrap_or_default();
    if !has_exact_fields(fields, &["version", "id", "method", "params"]) {
        return Err(DecodeError {
            id,
            error: PublicError::InvalidRequest,
        });
    }
    if !matches!(
        field(fields, "version"),
        Some(JsonValue::Number(JsonNumber::Integer(PROTOCOL_VERSION)))
    ) {
        return Err(DecodeError {
            id,
            error: PublicError::InvalidRequest,
        });
    }
    let Some(valid_id) = valid_request_id(field(fields, "id")) else {
        return Err(DecodeError {
            id: String::new(),
            error: PublicError::InvalidRequest,
        });
    };
    let method = match field(fields, "method") {
        Some(JsonValue::String(value)) => Method::parse(value),
        _ => None,
    }
    .ok_or_else(|| DecodeError {
        id: valid_id.clone(),
        error: PublicError::InvalidRequest,
    })?;
    let params = match field(fields, "params") {
        Some(JsonValue::Object(params)) => params.clone(),
        _ => {
            return Err(DecodeError {
                id: valid_id,
                error: PublicError::InvalidRequest,
            })
        }
    };
    Ok(Request {
        id: valid_id,
        method,
        params,
    })
}

fn valid_request_id(value: Option<&JsonValue>) -> Option<String> {
    let JsonValue::String(value) = value? else {
        return None;
    };
    if value.is_empty()
        || value.len() > MAX_REQUEST_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
    {
        None
    } else {
        Some(value.clone())
    }
}

fn has_duplicate_object_keys(value: &JsonValue) -> bool {
    match value {
        JsonValue::Object(fields) => {
            let mut seen = HashSet::with_capacity(fields.len());
            fields.iter().any(|(name, _)| !seen.insert(name.as_str()))
                || fields
                    .iter()
                    .any(|(_, child)| has_duplicate_object_keys(child))
        }
        JsonValue::Array(values) => values.iter().any(has_duplicate_object_keys),
        JsonValue::String(_) | JsonValue::Number(_) | JsonValue::Bool(_) | JsonValue::Null => false,
    }
}

fn field<'a>(fields: &'a [(String, JsonValue)], name: &str) -> Option<&'a JsonValue> {
    fields
        .iter()
        .find_map(|(candidate, value)| (candidate == name).then_some(value))
}

fn has_exact_fields(fields: &[(String, JsonValue)], expected: &[&str]) -> bool {
    fields.len() == expected.len()
        && fields
            .iter()
            .all(|(name, _)| expected.iter().any(|candidate| name == candidate))
}

fn authenticate_credential(params: &[(String, JsonValue)]) -> Result<&str, PublicError> {
    if !has_exact_fields(params, &["credential"]) {
        return Err(PublicError::InvalidParams);
    }
    let Some(JsonValue::String(credential)) = field(params, "credential") else {
        return Err(PublicError::InvalidParams);
    };
    if credential.is_empty() || credential.len() > MAX_CREDENTIAL_BYTES {
        Err(PublicError::InvalidParams)
    } else {
        Ok(credential)
    }
}

fn dispatch<C: ChiefControlPlane>(
    control_plane: &mut C,
    method: Method,
    params: &[(String, JsonValue)],
) -> Result<JsonValue, PublicError> {
    match method {
        Method::Authenticate => unreachable!("authentication dispatch is connection-local"),
        Method::RegisterHost => {
            let (registration, desired_state) = parse_registration(params)?;
            control_plane
                .register_host(registration, desired_state)
                .map(|host| loaded_host_json(&host))
                .map_err(PublicError::from)
        }
        Method::ListHosts => {
            require_empty(params)?;
            control_plane
                .list_hosts()
                .map(|hosts| JsonValue::Array(hosts.iter().map(loaded_host_json).collect()))
                .map_err(PublicError::from)
        }
        Method::SetDesiredState => {
            if !has_exact_fields(params, &["host_name", "desired_state"]) {
                return Err(PublicError::InvalidParams);
            }
            let host_name = parse_host_name(field(params, "host_name"))?;
            let desired_state = parse_desired_state(field(params, "desired_state"))?;
            control_plane
                .set_desired_state(&host_name, desired_state)
                .map(|host| loaded_host_json(&host))
                .map_err(PublicError::from)
        }
        Method::ReconcileOnce => {
            require_empty(params)?;
            control_plane
                .reconcile_once()
                .map(|report| reconcile_report_json(&report))
                .map_err(PublicError::from)
        }
        Method::HealthCheck => {
            let host_name = parse_name_only(params)?;
            control_plane
                .health_check(&host_name)
                .map(|health| health_json(&health))
                .map_err(PublicError::from)
        }
        Method::DeregisterHost => {
            let host_name = parse_name_only(params)?;
            control_plane
                .deregister_host(&host_name)
                .map(|()| object(vec![("deregistered", JsonValue::Bool(true))]))
                .map_err(PublicError::from)
        }
    }
}

fn require_empty(params: &[(String, JsonValue)]) -> Result<(), PublicError> {
    if params.is_empty() {
        Ok(())
    } else {
        Err(PublicError::InvalidParams)
    }
}

fn parse_name_only(params: &[(String, JsonValue)]) -> Result<HostName, PublicError> {
    if !has_exact_fields(params, &["host_name"]) {
        return Err(PublicError::InvalidParams);
    }
    parse_host_name(field(params, "host_name"))
}

fn parse_registration(
    params: &[(String, JsonValue)],
) -> Result<(HostRegistration, DesiredState), PublicError> {
    if !has_exact_fields(
        params,
        &[
            "host_name",
            "package_path",
            "package_hash",
            "restart_policy",
            "desired_state",
        ],
    ) {
        return Err(PublicError::InvalidParams);
    }
    let host_name = parse_host_name(field(params, "host_name"))?;
    let package_path = match field(params, "package_path") {
        Some(JsonValue::String(value)) => {
            PackagePath::new(value.clone()).map_err(|_| PublicError::InvalidParams)?
        }
        _ => return Err(PublicError::InvalidParams),
    };
    let package_hash = match field(params, "package_hash") {
        Some(JsonValue::String(value)) => decode_hash(value)?,
        _ => return Err(PublicError::InvalidParams),
    };
    let restart_policy = parse_restart_policy(field(params, "restart_policy"))?;
    let desired_state = parse_desired_state(field(params, "desired_state"))?;
    Ok((
        HostRegistration::new(host_name, package_path, package_hash, restart_policy),
        desired_state,
    ))
}

fn parse_host_name(value: Option<&JsonValue>) -> Result<HostName, PublicError> {
    match value {
        Some(JsonValue::String(value)) => {
            HostName::new(value.clone()).map_err(|_| PublicError::InvalidParams)
        }
        _ => Err(PublicError::InvalidParams),
    }
}

fn parse_restart_policy(value: Option<&JsonValue>) -> Result<RestartPolicy, PublicError> {
    match value {
        Some(JsonValue::String(value)) if value == "always" => Ok(RestartPolicy::Always),
        Some(JsonValue::String(value)) if value == "on_failure" => Ok(RestartPolicy::OnFailure),
        Some(JsonValue::String(value)) if value == "never" => Ok(RestartPolicy::Never),
        _ => Err(PublicError::InvalidParams),
    }
}

fn parse_desired_state(value: Option<&JsonValue>) -> Result<DesiredState, PublicError> {
    match value {
        Some(JsonValue::String(value)) if value == "running" => Ok(DesiredState::Running),
        Some(JsonValue::String(value)) if value == "stopped" => Ok(DesiredState::Stopped),
        _ => Err(PublicError::InvalidParams),
    }
}

fn decode_hash(value: &str) -> Result<[u8; 32], PublicError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PublicError::InvalidParams);
    }
    let mut output = [0_u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        let offset = index * 2;
        *byte =
            (hex_nibble(value.as_bytes()[offset]) << 4) | hex_nibble(value.as_bytes()[offset + 1]);
    }
    Ok(output)
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => 0,
    }
}

#[derive(Clone, Copy)]
enum PublicError {
    InvalidRequest,
    InvalidParams,
    Unauthenticated,
    AuthenticationFailed,
    AlreadyAuthenticated,
    Forbidden,
    NotFound,
    Conflict,
    Internal,
}

impl From<ControlPlaneError> for PublicError {
    fn from(error: ControlPlaneError) -> Self {
        match error {
            ControlPlaneError::InvalidInput => Self::InvalidParams,
            ControlPlaneError::NotFound => Self::NotFound,
            ControlPlaneError::Conflict => Self::Conflict,
            ControlPlaneError::Forbidden => Self::Forbidden,
            ControlPlaneError::Internal => Self::Internal,
        }
    }
}

impl PublicError {
    fn code(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::InvalidParams => "invalid_params",
            Self::Unauthenticated => "unauthenticated",
            Self::AuthenticationFailed => "authentication_failed",
            Self::AlreadyAuthenticated => "already_authenticated",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::Internal => "internal",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::InvalidRequest => "request envelope is invalid",
            Self::InvalidParams => "request parameters are invalid",
            Self::Unauthenticated => "connection is not authenticated",
            Self::AuthenticationFailed => "authentication failed",
            Self::AlreadyAuthenticated => "connection is already authenticated",
            Self::Forbidden => "operation is not authorized",
            Self::NotFound => "requested host was not found",
            Self::Conflict => "request conflicts with current state",
            Self::Internal => "operation failed",
        }
    }
}

fn success_response(id: &str, result: JsonValue) -> String {
    serialize(&object(vec![
        (
            "version",
            JsonValue::Number(JsonNumber::Integer(PROTOCOL_VERSION)),
        ),
        ("id", JsonValue::String(id.to_string())),
        ("ok", JsonValue::Bool(true)),
        ("result", result),
    ]))
    .expect("response values are finite JSON")
}

fn error_response(id: &str, error: PublicError) -> String {
    serialize(&object(vec![
        (
            "version",
            JsonValue::Number(JsonNumber::Integer(PROTOCOL_VERSION)),
        ),
        ("id", JsonValue::String(id.to_string())),
        ("ok", JsonValue::Bool(false)),
        (
            "error",
            object(vec![
                ("code", JsonValue::String(error.code().to_string())),
                ("message", JsonValue::String(error.message().to_string())),
            ]),
        ),
    ]))
    .expect("response values are finite JSON")
}

fn object(fields: Vec<(&str, JsonValue)>) -> JsonValue {
    JsonValue::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
    )
}

fn loaded_host_json(host: &LoadedHost) -> JsonValue {
    let entry = host.entry();
    let registration = entry.registration();
    object(vec![
        (
            "host_name",
            JsonValue::String(registration.host_name().as_str().to_string()),
        ),
        (
            "package_path",
            JsonValue::String(registration.package_path().as_str().to_string()),
        ),
        (
            "package_hash",
            JsonValue::String(hex_bytes(registration.package_hash())),
        ),
        (
            "restart_policy",
            JsonValue::String(restart_policy_name(registration.restart_policy()).to_string()),
        ),
        (
            "desired_state",
            JsonValue::String(desired_state_name(entry.desired_state()).to_string()),
        ),
        ("observation", observation_json(entry.observation())),
        (
            "revision",
            JsonValue::String(host.revision().as_str().to_string()),
        ),
    ])
}

fn observation_json(observation: &chief_of_staff_service_registry::HostObservation) -> JsonValue {
    object(vec![
        ("status", status_json(observation.status())),
        ("process_id", optional_u32(observation.process_id())),
        ("started_at_ns", optional_u64(observation.started_at_ns())),
        (
            "last_heartbeat_ns",
            optional_u64(observation.last_heartbeat_ns()),
        ),
        (
            "control_channel_id",
            optional_channel(observation.control_channel_id()),
        ),
        (
            "restart_count",
            JsonValue::Number(JsonNumber::Integer(i64::from(observation.restart_count()))),
        ),
        (
            "last_restart_ns",
            optional_u64(observation.last_restart_ns()),
        ),
    ])
}

fn status_json(status: &HostStatus) -> JsonValue {
    match status {
        HostStatus::Starting => object(vec![("kind", JsonValue::String("starting".to_string()))]),
        HostStatus::Running => object(vec![("kind", JsonValue::String("running".to_string()))]),
        HostStatus::Restarting => {
            object(vec![("kind", JsonValue::String("restarting".to_string()))])
        }
        HostStatus::Stopping => object(vec![("kind", JsonValue::String("stopping".to_string()))]),
        HostStatus::Stopped => object(vec![("kind", JsonValue::String("stopped".to_string()))]),
        HostStatus::Crashed { exit_code } => object(vec![
            ("kind", JsonValue::String("crashed".to_string())),
            ("exit_code", optional_i32(*exit_code)),
        ]),
        HostStatus::Quarantined { until_ns, reason } => object(vec![
            ("kind", JsonValue::String("quarantined".to_string())),
            ("until_ns", JsonValue::String(until_ns.to_string())),
            ("reason", JsonValue::String(reason.clone())),
        ]),
    }
}

fn reconcile_report_json(report: &ReconcileReport) -> JsonValue {
    JsonValue::Array(
        report
            .outcomes()
            .iter()
            .map(|outcome| {
                object(vec![
                    (
                        "host_name",
                        JsonValue::String(outcome.host_name().as_str().to_string()),
                    ),
                    (
                        "action",
                        JsonValue::String(reconcile_action_name(outcome.action()).to_string()),
                    ),
                    ("status", status_json(outcome.status())),
                ])
            })
            .collect(),
    )
}

fn health_json(health: &HostHealth) -> JsonValue {
    object(vec![
        ("durable", loaded_host_json(health.durable())),
        ("authoritative", authority_json(health.authoritative())),
    ])
}

fn authority_json(authority: &SupervisorObservation) -> JsonValue {
    match authority {
        SupervisorObservation::Absent => {
            object(vec![("kind", JsonValue::String("absent".to_string()))])
        }
        SupervisorObservation::Instance(instance) => object(vec![
            ("kind", JsonValue::String("instance".to_string())),
            (
                "package_hash",
                JsonValue::String(hex_bytes(instance.package_hash())),
            ),
            ("phase", supervisor_phase_json(instance.phase())),
            ("process_id", optional_u32(instance.process_id())),
            ("started_at_ns", optional_u64(instance.started_at_ns())),
            (
                "last_heartbeat_ns",
                optional_u64(instance.last_heartbeat_ns()),
            ),
            (
                "control_channel_id",
                optional_channel(instance.control_channel_id()),
            ),
        ]),
    }
}

fn supervisor_phase_json(phase: SupervisorPhase) -> JsonValue {
    match phase {
        SupervisorPhase::Starting => {
            object(vec![("kind", JsonValue::String("starting".to_string()))])
        }
        SupervisorPhase::Running => {
            object(vec![("kind", JsonValue::String("running".to_string()))])
        }
        SupervisorPhase::Stopping => {
            object(vec![("kind", JsonValue::String("stopping".to_string()))])
        }
        SupervisorPhase::Exited { exit_code } => object(vec![
            ("kind", JsonValue::String("exited".to_string())),
            ("exit_code", optional_i32(exit_code)),
        ]),
    }
}

fn optional_u64(value: Option<u64>) -> JsonValue {
    value.map_or(JsonValue::Null, |value| {
        JsonValue::String(value.to_string())
    })
}

fn optional_u32(value: Option<u32>) -> JsonValue {
    value.map_or(JsonValue::Null, |value| {
        JsonValue::Number(JsonNumber::Integer(i64::from(value)))
    })
}

fn optional_i32(value: Option<i32>) -> JsonValue {
    value.map_or(JsonValue::Null, |value| {
        JsonValue::Number(JsonNumber::Integer(i64::from(value)))
    })
}

fn optional_channel(value: Option<chief_of_staff_channel_crypto::ChannelId>) -> JsonValue {
    value.map_or(JsonValue::Null, |value| {
        JsonValue::String(hex_bytes(&value.0))
    })
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn restart_policy_name(policy: RestartPolicy) -> &'static str {
    match policy {
        RestartPolicy::Always => "always",
        RestartPolicy::OnFailure => "on_failure",
        RestartPolicy::Never => "never",
    }
}

fn desired_state_name(state: DesiredState) -> &'static str {
    match state {
        DesiredState::Running => "running",
        DesiredState::Stopped => "stopped",
    }
}

fn reconcile_action_name(action: ReconcileAction) -> &'static str {
    match action {
        ReconcileAction::Observed => "observed",
        ReconcileAction::Started => "started",
        ReconcileAction::Stopped => "stopped",
        ReconcileAction::Deferred => "deferred",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_orchestrator_core::ChannelWiringRequest;
    use chief_of_staff_process_supervisor::MonotonicClock;
    use chief_of_staff_service_reconciler::{ReconcileConfig, SupervisorObservation};
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use storage_core::InMemoryStorageBackend;
    use websocket_runtime::{WebSocketClient, WebSocketClientOptions};

    #[derive(Default)]
    struct FakeSupervisor {
        observations: BTreeMap<String, SupervisorObservation>,
    }

    impl HostSupervisor for FakeSupervisor {
        type Error = ();

        fn inspect(
            &mut self,
            registration: &HostRegistration,
        ) -> Result<SupervisorObservation, Self::Error> {
            Ok(self
                .observations
                .get(registration.host_name().as_str())
                .cloned()
                .unwrap_or_else(SupervisorObservation::absent))
        }

        fn start(&mut self, _registration: &HostRegistration) -> Result<(), Self::Error> {
            Ok(())
        }

        fn stop(&mut self, _host_name: &HostName) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    struct TestClock(AtomicU64);

    impl TestClock {
        fn new() -> Self {
            Self(AtomicU64::new(100))
        }
    }

    impl MonotonicClock for TestClock {
        fn now_ns(&self) -> u64 {
            self.0.fetch_add(1, Ordering::SeqCst)
        }
    }

    struct NoopWiring;

    impl ChannelWiringAuthorizer for NoopWiring {
        type Error = ();

        fn authorize(&mut self, _request: ChannelWiringRequest<'_>) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct TestAuthorizer {
        accept_credential: bool,
        allow_operation: bool,
        fail_authorize: bool,
        seen_credentials: Arc<Mutex<Vec<String>>>,
        operations: Arc<Mutex<Vec<Operation>>>,
    }

    impl TestAuthorizer {
        fn allowing() -> Self {
            Self {
                accept_credential: true,
                allow_operation: true,
                fail_authorize: false,
                seen_credentials: Arc::new(Mutex::new(Vec::new())),
                operations: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl SessionAuthorizer for TestAuthorizer {
        type Session = String;
        type Error = ();

        fn authenticate(&self, credential: &str) -> Result<Self::Session, Self::Error> {
            self.seen_credentials
                .lock()
                .expect("credentials mutex poisoned")
                .push(credential.to_string());
            if self.accept_credential && credential == "secret" {
                Ok("operator".to_string())
            } else {
                Err(())
            }
        }

        fn authorize(
            &self,
            session: &Self::Session,
            operation: Operation,
        ) -> Result<bool, Self::Error> {
            assert_eq!(session, "operator");
            self.operations
                .lock()
                .expect("operations mutex poisoned")
                .push(operation);
            if self.fail_authorize {
                Err(())
            } else {
                Ok(self.allow_operation)
            }
        }
    }

    fn core<'a>(
        backend: &'a InMemoryStorageBackend,
    ) -> OrchestratorCore<'a, FakeSupervisor, NoopWiring> {
        OrchestratorCore::new(
            backend,
            FakeSupervisor::default(),
            NoopWiring,
            Arc::new(TestClock::new()),
            ReconcileConfig::new(50).expect("valid reconcile config"),
        )
    }

    fn request(id: &str, method: &str, params: &str) -> String {
        format!("{{\"version\":1,\"id\":\"{id}\",\"method\":\"{method}\",\"params\":{params}}}")
    }

    fn authenticate<C: ChiefControlPlane>(
        api: &DaemonApi<C, TestAuthorizer>,
        session: &mut DaemonSession<String>,
    ) -> String {
        api.handle_text(
            session,
            &request("auth", "authenticate", r#"{"credential":"secret"}"#),
        )
    }

    #[test]
    fn authenticated_host_lifecycle_round_trips_all_core_operations() {
        let backend = InMemoryStorageBackend::new();
        let authorizer = TestAuthorizer::allowing();
        let operations = Arc::clone(&authorizer.operations);
        let seen = Arc::clone(&authorizer.seen_credentials);
        let api = DaemonApi::new(core(&backend), authorizer);
        let mut session = DaemonSession::default();

        let authenticated = authenticate(&api, &mut session);
        assert!(authenticated.contains(r#""ok":true"#));
        assert!(!authenticated.contains("secret"));
        assert!(session.is_authenticated());
        assert_eq!(*seen.lock().unwrap(), ["secret"]);

        let hash = "11".repeat(32);
        let registered = api.handle_text(
            &mut session,
            &request(
                "register",
                "register_host",
                &format!(
                    r#"{{"host_name":"agent-one","package_path":"/agents/one","package_hash":"{hash}","restart_policy":"always","desired_state":"stopped"}}"#
                ),
            ),
        );
        assert!(registered.contains(r#""host_name":"agent-one""#));
        assert!(registered.contains(r#""revision":"r1""#));
        assert!(registered.contains(r#""started_at_ns":null"#));

        let listed = api.handle_text(&mut session, &request("list", "list_hosts", "{}"));
        assert!(listed.contains(r#""host_name":"agent-one""#));

        let running = api.handle_text(
            &mut session,
            &request(
                "state",
                "set_desired_state",
                r#"{"host_name":"agent-one","desired_state":"running"}"#,
            ),
        );
        assert!(running.contains(r#""desired_state":"running""#));

        let reconciled = api.handle_text(&mut session, &request("tick", "reconcile_once", "{}"));
        assert!(reconciled.contains(r#""action":"started""#));

        let health = api.handle_text(
            &mut session,
            &request("health", "health_check", r#"{"host_name":"agent-one"}"#),
        );
        assert!(health.contains(r#""durable""#));
        assert!(health.contains(r#""authoritative":{"kind":"absent"}"#));

        let stopped = api.handle_text(
            &mut session,
            &request(
                "stop",
                "set_desired_state",
                r#"{"host_name":"agent-one","desired_state":"stopped"}"#,
            ),
        );
        assert!(stopped.contains(r#""desired_state":"stopped""#));
        let deregistered = api.handle_text(
            &mut session,
            &request("remove", "deregister_host", r#"{"host_name":"agent-one"}"#),
        );
        assert!(deregistered.contains(r#""deregistered":true"#));
        assert_eq!(
            *operations.lock().unwrap(),
            [
                Operation::RegisterHost,
                Operation::ListHosts,
                Operation::SetDesiredState,
                Operation::ReconcileOnce,
                Operation::HealthCheck,
                Operation::SetDesiredState,
                Operation::DeregisterHost,
            ]
        );

        let (core, _authorizer) = api.into_parts().expect("unpoisoned API");
        assert!(core.list_hosts().unwrap().is_empty());
    }

    #[test]
    fn requests_require_authentication_and_each_authorization_decision() {
        let backend = InMemoryStorageBackend::new();
        let mut denied = TestAuthorizer::allowing();
        denied.allow_operation = false;
        let api = DaemonApi::new(core(&backend), denied);
        let mut session = DaemonSession::default();

        let unauthenticated = api.handle_text(&mut session, &request("1", "list_hosts", "{}"));
        assert!(unauthenticated.contains(r#""code":"unauthenticated""#));
        authenticate(&api, &mut session);
        let forbidden = api.handle_text(&mut session, &request("2", "list_hosts", "{}"));
        assert!(forbidden.contains(r#""code":"forbidden""#));

        let backend = InMemoryStorageBackend::new();
        let mut failed = TestAuthorizer::allowing();
        failed.fail_authorize = true;
        let api = DaemonApi::new(core(&backend), failed);
        let mut session = DaemonSession::default();
        authenticate(&api, &mut session);
        let internal = api.handle_text(&mut session, &request("3", "list_hosts", "{}"));
        assert!(internal.contains(r#""code":"internal""#));
    }

    #[test]
    fn authentication_is_bounded_connection_local_and_never_echoed() {
        let backend = InMemoryStorageBackend::new();
        let mut authorizer = TestAuthorizer::allowing();
        authorizer.accept_credential = false;
        let api = DaemonApi::new(core(&backend), authorizer);
        let mut first = DaemonSession::default();
        let failed = api.handle_text(
            &mut first,
            &request("1", "authenticate", r#"{"credential":"do-not-echo"}"#),
        );
        assert!(failed.contains(r#""code":"authentication_failed""#));
        assert!(!failed.contains("do-not-echo"));
        assert!(!first.is_authenticated());

        let backend = InMemoryStorageBackend::new();
        let api = DaemonApi::new(core(&backend), TestAuthorizer::allowing());
        let mut first = DaemonSession::default();
        authenticate(&api, &mut first);
        let repeated = authenticate(&api, &mut first);
        assert!(repeated.contains(r#""code":"already_authenticated""#));
        let second = DaemonSession::<String>::default();
        assert!(!second.is_authenticated());

        for params in [
            "{}".to_string(),
            r#"{"credential":1}"#.to_string(),
            r#"{"credential":""}"#.to_string(),
            format!(
                r#"{{"credential":"{}"}}"#,
                "x".repeat(MAX_CREDENTIAL_BYTES + 1)
            ),
            r#"{"credential":"secret","extra":true}"#.to_string(),
        ] {
            let mut session = DaemonSession::default();
            let response =
                api.handle_text(&mut session, &request("bound", "authenticate", &params));
            assert!(response.contains(r#""code":"invalid_params""#));
        }
    }

    #[test]
    fn untrusted_json_and_envelopes_are_strictly_bounded() {
        let backend = InMemoryStorageBackend::new();
        let api = DaemonApi::new(core(&backend), TestAuthorizer::allowing());
        let cases = [
            "not json".to_string(),
            "[]".to_string(),
            r#"{"version":1,"id":"x","id":"y","method":"list_hosts","params":{}}"#.to_string(),
            r#"{"version":1,"id":"x","method":"list_hosts","params":{"nested":{"a":1,"a":2}}}"#
                .to_string(),
            r#"{"version":1,"id":"x","method":"list_hosts","params":{},"extra":1}"#.to_string(),
            r#"{"version":1.0,"id":"x","method":"list_hosts","params":{}}"#.to_string(),
            r#"{"version":1,"id":"","method":"list_hosts","params":{}}"#.to_string(),
            r#"{"version":1,"id":"x","method":"unknown","params":{}}"#.to_string(),
            r#"{"version":1,"id":"x","method":"list_hosts","params":[]}"#.to_string(),
        ];
        for input in cases {
            let mut session = DaemonSession::default();
            let response = api.handle_text(&mut session, &input);
            assert!(response.contains(r#""code":"invalid_request""#), "{input}");
        }

        let mut deep = "0".to_string();
        for _ in 0..140 {
            deep = format!("[{deep}]");
        }
        let mut session = DaemonSession::default();
        assert!(api
            .handle_text(&mut session, &deep)
            .contains(r#""code":"invalid_request""#));
        assert!(api
            .handle_text(&mut session, &"x".repeat(MAX_REQUEST_BYTES + 1))
            .contains(r#""code":"invalid_request""#));
    }

    #[test]
    fn parameter_validation_and_control_plane_errors_are_stable() {
        let backend = InMemoryStorageBackend::new();
        let api = DaemonApi::new(core(&backend), TestAuthorizer::allowing());
        let mut session = DaemonSession::default();
        authenticate(&api, &mut session);

        for (method, params) in [
            ("list_hosts", r#"{"extra":true}"#),
            (
                "set_desired_state",
                r#"{"host_name":"A","desired_state":"running"}"#,
            ),
            (
                "set_desired_state",
                r#"{"host_name":"valid-host","desired_state":"paused"}"#,
            ),
            ("health_check", "{}"),
            ("deregister_host", r#"{"host_name":1}"#),
            ("register_host", r#"{"host_name":"valid-host"}"#),
        ] {
            let response = api.handle_text(&mut session, &request("bad", method, params));
            assert!(response.contains(r#""code":"invalid_params""#), "{method}");
        }

        for hash in ["0".repeat(63), "A".repeat(64), "g".repeat(64)] {
            let params = format!(
                r#"{{"host_name":"valid-host","package_path":"/x","package_hash":"{hash}","restart_policy":"never","desired_state":"stopped"}}"#
            );
            assert!(api
                .handle_text(&mut session, &request("hash", "register_host", &params))
                .contains(r#""code":"invalid_params""#));
        }

        let missing = api.handle_text(
            &mut session,
            &request("missing", "health_check", r#"{"host_name":"missing-host"}"#),
        );
        assert!(missing.contains(r#""code":"not_found""#));
    }

    #[test]
    fn websocket_events_are_text_only_and_control_events_are_runtime_owned() {
        let backend = InMemoryStorageBackend::new();
        let api = DaemonApi::new(core(&backend), TestAuthorizer::allowing());
        let mut session = DaemonSession::default();
        assert!(
            api.handle(&mut session, MessageEvent::Binary(vec![1, 2]))
                .close
        );
        assert!(!api.handle(&mut session, MessageEvent::Ping(vec![1])).close);
        assert!(!api.handle(&mut session, MessageEvent::Pong(vec![1])).close);
    }

    struct EmptyControlPlane;

    impl ChiefControlPlane for EmptyControlPlane {
        fn register_host(
            &mut self,
            _registration: HostRegistration,
            _desired_state: DesiredState,
        ) -> Result<LoadedHost, ControlPlaneError> {
            Err(ControlPlaneError::Internal)
        }

        fn list_hosts(&mut self) -> Result<Vec<LoadedHost>, ControlPlaneError> {
            Ok(Vec::new())
        }

        fn set_desired_state(
            &mut self,
            _host_name: &HostName,
            _desired_state: DesiredState,
        ) -> Result<LoadedHost, ControlPlaneError> {
            Err(ControlPlaneError::Internal)
        }

        fn reconcile_once(&mut self) -> Result<ReconcileReport, ControlPlaneError> {
            Err(ControlPlaneError::Internal)
        }

        fn health_check(&mut self, _host_name: &HostName) -> Result<HostHealth, ControlPlaneError> {
            Err(ControlPlaneError::NotFound)
        }

        fn deregister_host(&mut self, _host_name: &HostName) -> Result<(), ControlPlaneError> {
            Err(ControlPlaneError::Internal)
        }
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    type HostPlatform = transport_platform::bsd::KqueueTransportPlatform;
    #[cfg(target_os = "linux")]
    type HostPlatform = transport_platform::linux::EpollTransportPlatform;
    #[cfg(target_os = "windows")]
    type HostPlatform = transport_platform::windows::WindowsTransportPlatform;

    fn host_platform() -> HostPlatform {
        HostPlatform::new().expect("host platform")
    }

    #[test]
    fn real_loopback_client_authenticates_and_lists_over_websocket() {
        let api = Arc::new(DaemonApi::new(
            EmptyControlPlane,
            TestAuthorizer::allowing(),
        ));
        let mut runtime = bind_daemon(
            host_platform(),
            BindAddress::Ip("127.0.0.1:0".parse().unwrap()),
            WebSocketServerOptions::default(),
            api,
        )
        .expect("bind daemon");
        let address = runtime.local_addr();
        let stop = runtime.stop_handle();
        let server = thread::spawn(move || runtime.serve().expect("serve daemon"));
        let mut client = WebSocketClient::connect(
            "127.0.0.1",
            address.port(),
            "/chief",
            WebSocketClientOptions::default(),
        )
        .expect("connect client");

        client
            .send_text(request(
                "auth",
                "authenticate",
                r#"{"credential":"secret"}"#,
            ))
            .unwrap();
        let MessageEvent::Text(authenticated) = client.receive().unwrap() else {
            panic!("expected authentication text response")
        };
        assert!(authenticated.contains(r#""authenticated":true"#));
        client
            .send_text(request("list", "list_hosts", "{}"))
            .unwrap();
        let MessageEvent::Text(listed) = client.receive().unwrap() else {
            panic!("expected list text response")
        };
        assert!(listed.contains(r#""result":[]"#));
        client.close(Some(1000), "done").unwrap();
        let _ = client.receive();
        stop.stop();
        server.join().unwrap();
    }

    #[test]
    fn helpers_cover_precision_safe_variants_and_payload_blind_displays() {
        assert_eq!(hex_bytes(&[0, 15, 16, 255]), "000f10ff");
        assert_eq!(restart_policy_name(RestartPolicy::OnFailure), "on_failure");
        assert_eq!(desired_state_name(DesiredState::Stopped), "stopped");
        assert_eq!(reconcile_action_name(ReconcileAction::Deferred), "deferred");
        assert_eq!(
            DaemonApiError::Poisoned.to_string(),
            "chief daemon API control plane is unavailable"
        );
        assert_eq!(
            map_registry_error(RegistryError::TooManyHosts),
            ControlPlaneError::Internal
        );
        assert_eq!(
            PublicError::from(ControlPlaneError::Forbidden).code(),
            "forbidden"
        );
        let statuses = [
            HostStatus::Starting,
            HostStatus::Running,
            HostStatus::Restarting,
            HostStatus::Stopping,
            HostStatus::Stopped,
            HostStatus::Crashed { exit_code: Some(7) },
            HostStatus::Quarantined {
                until_ns: u64::MAX,
                reason: "policy".to_string(),
            },
        ];
        let text = statuses
            .iter()
            .map(status_json)
            .map(|value| serialize(&value).unwrap())
            .collect::<String>();
        assert!(text.contains(&u64::MAX.to_string()));
        assert!(text.contains("quarantined"));
        for phase in [
            SupervisorPhase::Starting,
            SupervisorPhase::Running,
            SupervisorPhase::Stopping,
            SupervisorPhase::Exited { exit_code: None },
        ] {
            assert!(matches!(supervisor_phase_json(phase), JsonValue::Object(_)));
        }
    }

    #[test]
    fn poisoned_control_plane_maps_to_stable_internal_failure() {
        struct PanickingControlPlane;
        impl ChiefControlPlane for PanickingControlPlane {
            fn register_host(
                &mut self,
                _: HostRegistration,
                _: DesiredState,
            ) -> Result<LoadedHost, ControlPlaneError> {
                unreachable!()
            }
            fn list_hosts(&mut self) -> Result<Vec<LoadedHost>, ControlPlaneError> {
                panic!("poison")
            }
            fn set_desired_state(
                &mut self,
                _: &HostName,
                _: DesiredState,
            ) -> Result<LoadedHost, ControlPlaneError> {
                unreachable!()
            }
            fn reconcile_once(&mut self) -> Result<ReconcileReport, ControlPlaneError> {
                unreachable!()
            }
            fn health_check(&mut self, _: &HostName) -> Result<HostHealth, ControlPlaneError> {
                unreachable!()
            }
            fn deregister_host(&mut self, _: &HostName) -> Result<(), ControlPlaneError> {
                unreachable!()
            }
        }
        let api = DaemonApi::new(PanickingControlPlane, TestAuthorizer::allowing());
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut session = DaemonSession::default();
            authenticate(&api, &mut session);
            let _ = api.handle_text(&mut session, &request("x", "list_hosts", "{}"));
        }));
        assert!(panic.is_err());
        assert!(matches!(api.into_parts(), Err(DaemonApiError::Poisoned)));
    }
}
