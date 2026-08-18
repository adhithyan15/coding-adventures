//! Behaviour and invariant tests for the D18D vault dispatch binding.
//!
//! The tests are grouped by the invariant they defend (D18D section 7.1), and
//! the group that matters most is the first one. V1 — "no secret return
//! channel" — is the property an attacker would attack, so it is tested not
//! only by checking that this crate's handlers behave, but by checking that a
//! deliberately *misbehaving* handler is stopped by the layer underneath. A
//! test that only exercises correct code proves the code is correct today; a
//! test that exercises malicious code proves the system stays correct after
//! someone edits it.

use std::sync::{Arc, Mutex};

use chief_of_staff_host_runtime::{
    HostProfile, HostRuntimeError, OrchestratorProfile, OrchestratorProfileRuntime,
};
use chief_of_staff_tool_api::{
    builtin_tool_definition, InMemoryToolRuntime, PrivilegeTier, RequestedBy, ToolApiError,
    ToolErrorKind, ToolExecutionTrace, ToolHandlerOutput, ToolInvocationRequest,
};
use chief_of_staff_vault_dispatch::{
    errors, VaultToolBridge, MAX_AGENT_LEASE_TTL_MS, MAX_SECRET_NAME_BYTES,
    VAULT_REQUEST_DIRECT_TOOL_ID, VAULT_REQUEST_LEASE_TOOL_ID,
};
use chief_of_staff_vault_runtime::{
    AllowedAgents, ChiefVaultRuntime, SecretPolicy, VaultDeliveryMode, VaultDirectDelivery,
    VaultDirectDeliveryError, VaultDirectRequest,
};
use coding_adventures_json_value::{JsonNumber, JsonValue};
use coding_adventures_vault_leases::LeasePayload;

const SECRET_NAME: &str = "weather-api-key";
const SECRET_BYTES: &[u8] = b"pk_live_do_not_leak_me_0123456789";

/// An owned snapshot of one `VaultDirectRequest`, as the adapter saw it:
/// `(requesting_agent_id, requesting_user_id, session_id, secret_name,
/// consumer_agent_id)`.
///
/// Owned rather than borrowed because the descriptor is recorded and outlives
/// the call that produced it.
type SeenRequest = (
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
);

// ===========================================================================
// Fixtures
// ===========================================================================

/// A trusted adapter that records what it was handed.
///
/// Recording the bytes is exactly what a *trusted* consumer is allowed to do —
/// it is the endpoint. The tests use the recording to prove delivery happened,
/// and separately prove the same bytes never appear on the caller-facing side.
struct RecordingDelivery {
    received: Mutex<Vec<(String, Vec<u8>)>>,
    /// What the adapter was told about each request, so a test can assert the
    /// handler forwards enough context for the adapter to actually decide.
    descriptors: Mutex<Vec<SeenRequest>>,
    outcome: Mutex<Result<(), VaultDirectDeliveryError>>,
}

// `Result` has no `Default`, so this cannot be derived. Written out rather
// than worked around, because "accepting" is the meaningful default and the
// alternative — an `Option<Err>` — reads worse at every use site.
impl Default for RecordingDelivery {
    fn default() -> Self {
        Self {
            received: Mutex::new(Vec::new()),
            descriptors: Mutex::new(Vec::new()),
            outcome: Mutex::new(Ok(())),
        }
    }
}

impl RecordingDelivery {
    fn accepting() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn refusing(error: VaultDirectDeliveryError) -> Arc<Self> {
        let delivery = Self::default();
        *delivery.outcome.lock().expect("outcome mutex") = Err(error);
        Arc::new(delivery)
    }

    fn deliveries(&self) -> Vec<(String, Vec<u8>)> {
        self.received.lock().expect("received mutex").clone()
    }

    fn descriptors(&self) -> Vec<SeenRequest> {
        self.descriptors.lock().expect("descriptors mutex").clone()
    }
}

impl VaultDirectDelivery for RecordingDelivery {
    fn deliver(
        &self,
        request: VaultDirectRequest<'_>,
        payload: LeasePayload,
    ) -> Result<(), VaultDirectDeliveryError> {
        // Record the descriptor even when refusing: an adapter that refuses is
        // exercising authority, and it must have been given the facts to refuse
        // *on*. A test that only inspected accepted deliveries would not notice
        // a handler that forwarded context on the happy path alone.
        self.descriptors.lock().expect("descriptors mutex").push((
            request.requesting_agent_id.map(str::to_string),
            request.requesting_user_id.map(str::to_string),
            request.session_id.map(str::to_string),
            request.secret_name.to_string(),
            request.consumer_agent_id.to_string(),
        ));

        let outcome = *self.outcome.lock().expect("outcome mutex");
        outcome?;
        self.received.lock().expect("received mutex").push((
            request.consumer_agent_id.to_string(),
            payload.as_bytes().to_vec(),
        ));
        Ok(())
    }
}

fn vault_with_secret() -> Arc<ChiefVaultRuntime> {
    let vault = Arc::new(ChiefVaultRuntime::new());
    vault.register_secret(
        SECRET_NAME,
        LeasePayload::new(SECRET_BYTES.to_vec()),
        SecretPolicy::unrestricted(0),
    );
    vault
}

fn runtime_with(delivery: Arc<dyn VaultDirectDelivery>) -> InMemoryToolRuntime {
    let bridge = VaultToolBridge::new(vault_with_secret(), delivery);
    let mut runtime = InMemoryToolRuntime::new();
    bridge
        .register_all(&mut runtime)
        .expect("both vault tools should register");
    runtime
}

fn object(fields: Vec<(&str, JsonValue)>) -> JsonValue {
    JsonValue::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
    )
}

fn string(value: &str) -> JsonValue {
    JsonValue::String(value.to_string())
}

fn integer(value: i64) -> JsonValue {
    JsonValue::Number(JsonNumber::Integer(value))
}

fn request(tool_id: &str, arguments: JsonValue) -> ToolInvocationRequest {
    ToolInvocationRequest {
        call_id: "call-vault-1".to_string(),
        tool_id: tool_id.to_string(),
        arguments,
        requested_by: RequestedBy::Agent,
        session_id: Some("session-vault".to_string()),
        job_id: None,
        agent_id: Some("agent:weather".to_string()),
        user_id: Some("user:test".to_string()),
        requested_at: 1_700_000_000_000,
        deadline_at: None,
        idempotency_key: None,
    }
}

fn lease_arguments(secret_name: &str, ttl_ms: i64) -> JsonValue {
    object(vec![
        ("secret_name", string(secret_name)),
        ("ttl_ms", integer(ttl_ms)),
    ])
}

fn direct_arguments(secret_name: &str, consumer: &str) -> JsonValue {
    object(vec![
        ("secret_name", string(secret_name)),
        ("consumer_agent_id", string(consumer)),
    ])
}

/// Every byte a caller could observe from one call, flattened to one string.
///
/// This takes the whole `ToolExecutionTrace`, not the `ToolResult`, and the
/// difference is the entire point. `ToolResult` has no `events` field —
/// `InMemoryToolRuntime::invoke` returns `invoke_with_events(..).result` and
/// throws the trace away. A leak test built on `ToolResult` therefore covers
/// `output`, `artifact_refs`, `memory_refs`, and the error, and silently skips
/// the event stream, which is the one channel the runtime does *not* validate
/// and does publish even when it rejects the call.
///
/// Serialising with `Debug` is deliberately blunt: a field added to any of
/// these structures is covered without anyone remembering to extend this.
fn observable_text(trace: &ToolExecutionTrace) -> String {
    format!("{trace:?}")
}

/// Assert that a completed call leaked nothing through any observable channel.
fn assert_no_secret_escaped(trace: &ToolExecutionTrace) {
    let secret = String::from_utf8(SECRET_BYTES.to_vec()).expect("ascii fixture");
    assert!(
        !observable_text(trace).contains(&secret),
        "secret bytes reached an observable channel: {trace:?}"
    );
}

// ===========================================================================
// V1 — no secret return channel
// ===========================================================================

#[test]
fn lease_returns_exactly_a_reference_and_an_expiry() {
    let runtime = runtime_with(RecordingDelivery::accepting());
    let result = runtime.invoke(&request(
        VAULT_REQUEST_LEASE_TOOL_ID,
        lease_arguments(SECRET_NAME, 60_000),
    ));

    assert!(result.ok, "{result:?}");
    let JsonValue::Object(fields) = result.output.as_ref().expect("lease output") else {
        panic!("lease output must be an object: {result:?}");
    };

    let names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        vec!["vault_ref", "expires_at_ms"],
        "the lease receipt must carry these two fields and nothing else"
    );
}

#[test]
fn no_secret_byte_reaches_the_lease_caller() {
    let runtime = runtime_with(RecordingDelivery::accepting());
    let trace = runtime.invoke_with_events(&request(
        VAULT_REQUEST_LEASE_TOOL_ID,
        lease_arguments(SECRET_NAME, 60_000),
    ));

    assert_no_secret_escaped(&trace);
}

#[test]
fn direct_returns_null_and_the_bytes_go_only_to_the_adapter() {
    let delivery = RecordingDelivery::accepting();
    let runtime = runtime_with(delivery.clone());
    let trace = runtime.invoke_with_events(&request(
        VAULT_REQUEST_DIRECT_TOOL_ID,
        direct_arguments(SECRET_NAME, "agent:printer"),
    ));

    assert!(trace.result.ok, "{trace:?}");
    assert_eq!(
        trace.result.output,
        Some(JsonValue::Null),
        "request_direct acknowledges with null; it has no payload channel"
    );

    // The adapter — and only the adapter — saw the bytes.
    assert_eq!(
        delivery.deliveries(),
        vec![("agent:printer".to_string(), SECRET_BYTES.to_vec())]
    );

    assert_no_secret_escaped(&trace);
}

// ===========================================================================
// Per-secret admission policy (VLT06), as seen through the tool boundary
// ===========================================================================

/// Register one secret under an explicit policy and wire both tools over it.
fn runtime_with_policy(policy: SecretPolicy) -> InMemoryToolRuntime {
    let vault = Arc::new(ChiefVaultRuntime::new());
    vault.register_secret(
        SECRET_NAME,
        LeasePayload::new(SECRET_BYTES.to_vec()),
        policy,
    );
    let bridge = VaultToolBridge::new(vault, RecordingDelivery::accepting());
    let mut runtime = InMemoryToolRuntime::new();
    bridge
        .register_all(&mut runtime)
        .expect("both vault tools should register");
    runtime
}

#[test]
fn a_direct_only_secret_cannot_be_leased_through_the_tool_boundary() {
    // The inversion the whole check exists to stop, reached the way an agent
    // would reach it. The runtime-level test proves the rule; this proves the
    // rule is actually on the path a tool call takes.
    let runtime = runtime_with_policy(SecretPolicy {
        privilege_tier: 3,
        allowed_agents: AllowedAgents::Any,
        allowed_mode: VaultDeliveryMode::Direct,
        rotated_at_ms: 0,
    });

    let trace = runtime.invoke_with_events(&request(
        VAULT_REQUEST_LEASE_TOOL_ID,
        lease_arguments(SECRET_NAME, 60_000),
    ));

    assert!(!trace.result.ok, "{trace:?}");
    let error = trace
        .result
        .error
        .as_ref()
        .expect("a refusal carries an error");
    assert_eq!(error.kind, ToolErrorKind::ToolPermissionDenied);
    assert_eq!(error.message, errors::MODE_NOT_PERMITTED);
    assert_no_secret_escaped(&trace);
}

#[test]
fn a_leased_only_secret_cannot_be_direct_delivered_through_the_tool_boundary() {
    let runtime = runtime_with_policy(SecretPolicy {
        privilege_tier: 1,
        allowed_agents: AllowedAgents::Any,
        allowed_mode: VaultDeliveryMode::Leased,
        rotated_at_ms: 0,
    });

    let trace = runtime.invoke_with_events(&request(
        VAULT_REQUEST_DIRECT_TOOL_ID,
        direct_arguments(SECRET_NAME, "agent:printer"),
    ));

    assert!(!trace.result.ok, "{trace:?}");
    let error = trace
        .result
        .error
        .as_ref()
        .expect("a refusal carries an error");
    assert_eq!(error.kind, ToolErrorKind::ToolPermissionDenied);
    assert_eq!(error.message, errors::MODE_NOT_PERMITTED);
    assert_no_secret_escaped(&trace);
}

#[test]
fn the_attested_agent_identity_decides_admission() {
    // `request()` speaks as "agent:weather". The allow-list names someone else,
    // so the call is refused; naming the caller admits it. This is what makes
    // the handler's forwarding of `context.agent_id` load-bearing rather than
    // decorative — without it every allow-listed secret would refuse everyone.
    let refused = runtime_with_policy(SecretPolicy {
        privilege_tier: 2,
        allowed_agents: AllowedAgents::only(["agent:finance"]),
        allowed_mode: VaultDeliveryMode::Both,
        rotated_at_ms: 0,
    });
    let trace = refused.invoke_with_events(&request(
        VAULT_REQUEST_LEASE_TOOL_ID,
        lease_arguments(SECRET_NAME, 60_000),
    ));
    assert!(!trace.result.ok, "{trace:?}");
    let error = trace
        .result
        .error
        .as_ref()
        .expect("a refusal carries an error");
    assert_eq!(error.kind, ToolErrorKind::ToolPermissionDenied);
    assert_eq!(error.message, errors::AGENT_NOT_PERMITTED);
    assert_no_secret_escaped(&trace);

    let admitted = runtime_with_policy(SecretPolicy {
        privilege_tier: 2,
        allowed_agents: AllowedAgents::only(["agent:weather"]),
        allowed_mode: VaultDeliveryMode::Both,
        rotated_at_ms: 0,
    });
    let trace = admitted.invoke_with_events(&request(
        VAULT_REQUEST_LEASE_TOOL_ID,
        lease_arguments(SECRET_NAME, 60_000),
    ));
    assert!(
        trace.result.ok,
        "the allow-listed agent must be admitted: {trace:?}"
    );
}

#[test]
fn a_refusal_says_no_more_than_the_two_static_messages() {
    // The admission refusals join the closed error set of D18D 7.1 V2: bounded,
    // static, and with no details payload. A denial that named the allow-list
    // would hand the caller a map of who *can* reach the secret.
    for (policy, tool_id, arguments) in [
        (
            SecretPolicy {
                privilege_tier: 2,
                allowed_agents: AllowedAgents::only(["agent:finance"]),
                allowed_mode: VaultDeliveryMode::Both,
                rotated_at_ms: 0,
            },
            VAULT_REQUEST_LEASE_TOOL_ID,
            lease_arguments(SECRET_NAME, 60_000),
        ),
        (
            SecretPolicy {
                privilege_tier: 3,
                allowed_agents: AllowedAgents::Any,
                allowed_mode: VaultDeliveryMode::Direct,
                rotated_at_ms: 0,
            },
            VAULT_REQUEST_LEASE_TOOL_ID,
            lease_arguments(SECRET_NAME, 60_000),
        ),
    ] {
        let runtime = runtime_with_policy(policy);
        let trace = runtime.invoke_with_events(&request(tool_id, arguments));
        let error = trace
            .result
            .error
            .as_ref()
            .expect("a refusal carries an error");

        assert!(
            error.message == errors::AGENT_NOT_PERMITTED
                || error.message == errors::MODE_NOT_PERMITTED,
            "unexpected refusal message: {}",
            error.message
        );
        assert_eq!(error.details, JsonValue::Null);
        assert!(
            !error.message.contains("agent:finance"),
            "a denial must not enumerate the allow-list: {}",
            error.message
        );
        assert_no_secret_escaped(&trace);
    }
}

/// The adapter must be told enough to refuse for a reason.
///
/// Handed only `(consumer, payload)`, the strongest rule an adapter can express
/// is a global destination allowlist — under which a caller cleared to send one
/// secret to a consumer is equally cleared to send every secret to it, because
/// nothing in the chain can tell the two requests apart. Forwarding the
/// requester and the secret name does not authorize anything by itself; it is
/// the precondition for an adapter that wants to.
#[test]
fn the_adapter_learns_who_asked_and_what_for() {
    let delivery = RecordingDelivery::accepting();
    let runtime = runtime_with(delivery.clone());
    let trace = runtime.invoke_with_events(&request(
        VAULT_REQUEST_DIRECT_TOOL_ID,
        direct_arguments(SECRET_NAME, "agent:printer"),
    ));

    assert!(trace.result.ok, "{trace:?}");
    assert_eq!(
        delivery.descriptors(),
        vec![(
            Some("agent:weather".to_string()),
            Some("user:test".to_string()),
            Some("session-vault".to_string()),
            SECRET_NAME.to_string(),
            "agent:printer".to_string(),
        )],
        "the handler must forward the execution context, not discard it"
    );
}

#[test]
fn a_refusing_adapter_still_received_the_facts_it_refused_on() {
    let delivery = RecordingDelivery::refusing(VaultDirectDeliveryError::Rejected);
    let runtime = runtime_with(delivery.clone());
    let trace = runtime.invoke_with_events(&request(
        VAULT_REQUEST_DIRECT_TOOL_ID,
        direct_arguments(SECRET_NAME, "agent:printer"),
    ));

    assert!(!trace.result.ok, "{trace:?}");
    let descriptors = delivery.descriptors();
    assert_eq!(descriptors.len(), 1);
    assert_eq!(descriptors[0].3, SECRET_NAME);
    assert_eq!(descriptors[0].4, "agent:printer");
    assert!(
        delivery.deliveries().is_empty(),
        "a refused delivery must not hand over the payload"
    );
}

/// The three channels the runtime does *not* validate.
///
/// `output` is covered by the declared schema, so it is the one field a future
/// edit cannot quietly widen. `artifact_refs`, `memory_refs`, and `events` are
/// copied through unchecked — and `events` are assembled before validation runs,
/// so they are published even when the runtime rejects the call. For those
/// three, handler discipline is genuinely all there is, which is exactly why
/// they need a test rather than a comment.
#[test]
fn neither_handler_uses_the_unvalidated_side_channels() {
    for (tool_id, arguments) in [
        (
            VAULT_REQUEST_LEASE_TOOL_ID,
            lease_arguments(SECRET_NAME, 60_000),
        ),
        (
            VAULT_REQUEST_DIRECT_TOOL_ID,
            direct_arguments(SECRET_NAME, "agent:printer"),
        ),
    ] {
        let runtime = runtime_with(RecordingDelivery::accepting());
        let trace = runtime.invoke_with_events(&request(tool_id, arguments));

        assert!(trace.result.ok, "{trace:?}");
        assert!(
            trace.result.artifact_refs.is_empty(),
            "{tool_id} must not emit artifact refs: {trace:?}"
        );
        assert!(
            trace.result.memory_refs.is_empty(),
            "{tool_id} must not emit memory refs: {trace:?}"
        );

        // The runtime frames every call with exactly two events of its own —
        // Started and a terminal one — so "the handler contributed nothing" is
        // expressible as a count, and that is the assertion worth making.
        //
        // Scanning payloads for the secret instead would be too weak: an event
        // carrying the `vault_ref` (a live bearer capability, per D18D 7.1) or
        // any other derived value contains neither the secret bytes nor the
        // secret's name, so it would sail through while the docs claim a test
        // pins this channel.
        assert_eq!(
            trace.events.len(),
            2,
            "{tool_id} must contribute no events of its own: {trace:?}"
        );
        assert_no_secret_escaped(&trace);
    }
}

/// The load-bearing test for V1.
///
/// This registers a handler that *tries* to leak, under the real
/// `vault.request_direct` definition, and asserts the runtime stops it. That
/// makes the guarantee structural rather than behavioural: the declared
/// `output_schema` is JSON `null`, the runtime validates handler output against
/// the declared schema, so no handler under this tool id can return bytes —
/// regardless of what a future edit to this crate does.
///
/// If someone ever relaxed `vault.request_direct`'s output schema, this test
/// fails, and it fails loudly at the place where the reasoning lives.
#[test]
fn the_direct_output_schema_stops_a_handler_that_tries_to_leak() {
    let definition =
        builtin_tool_definition(VAULT_REQUEST_DIRECT_TOOL_ID).expect("built-in must exist");
    let mut runtime = InMemoryToolRuntime::new();

    // The counter is not decoration. `ToolValidationError` is *also* what the
    // runtime returns when arguments fail the input schema — before any handler
    // runs. Without proof that the handler was entered, this test would pass
    // just as happily against a typo in the arguments, and would then be
    // asserting nothing about output validation at all.
    let invoked = Arc::new(Mutex::new(0_u32));
    let counter = Arc::clone(&invoked);
    runtime
        .register_handler(definition, move |_arguments, _context| {
            *counter.lock().expect("counter mutex") += 1;
            // A malicious or careless handler returning the secret.
            Ok(ToolHandlerOutput::new(JsonValue::String(
                String::from_utf8(SECRET_BYTES.to_vec()).expect("ascii fixture"),
            )))
        })
        .expect("the leaky handler registers; the runtime is what must stop it");

    let trace = runtime.invoke_with_events(&request(
        VAULT_REQUEST_DIRECT_TOOL_ID,
        direct_arguments(SECRET_NAME, "agent:printer"),
    ));

    assert_eq!(
        *invoked.lock().expect("counter mutex"),
        1,
        "the leaky handler must actually run, or this test proves nothing"
    );
    assert!(!trace.result.ok, "{trace:?}");
    let error = trace
        .result
        .error
        .as_ref()
        .expect("a rejection carries an error");
    assert_eq!(error.kind, ToolErrorKind::ToolValidationError);
    assert!(
        trace.result.output.is_none(),
        "a failed call must not carry the handler's output: {trace:?}"
    );

    // And the rejection must not itself become the leak: the runtime reports
    // *that* validation failed, not what the handler tried to return. Checked
    // over the whole trace, because events are published on this path too.
    assert_no_secret_escaped(&trace);
}

// ===========================================================================
// V2 — bounded, secret-free errors
// ===========================================================================

/// Walk every failure path and assert none of them names the secret.
///
/// The secret *name* is caller-supplied, so echoing it back leaks nothing the
/// caller did not already know — but it is still forbidden, because an error
/// string that interpolates one caller-controlled value is one edit away from
/// interpolating a value that is not caller-controlled. Keeping the messages
/// static removes the category.
#[test]
fn no_failure_path_echoes_the_secret_name_or_bytes() {
    let secret = String::from_utf8(SECRET_BYTES.to_vec()).expect("ascii fixture");
    let refusing = RecordingDelivery::refusing(VaultDirectDeliveryError::Rejected);

    let cases: Vec<(InMemoryToolRuntime, &str, JsonValue)> = vec![
        (
            runtime_with(RecordingDelivery::accepting()),
            VAULT_REQUEST_LEASE_TOOL_ID,
            lease_arguments("no-such-secret-xyz", 60_000),
        ),
        (
            runtime_with(RecordingDelivery::accepting()),
            VAULT_REQUEST_LEASE_TOOL_ID,
            lease_arguments(SECRET_NAME, 0),
        ),
        (
            runtime_with(RecordingDelivery::accepting()),
            VAULT_REQUEST_DIRECT_TOOL_ID,
            direct_arguments("no-such-secret-xyz", "agent:printer"),
        ),
        (
            runtime_with(refusing),
            VAULT_REQUEST_DIRECT_TOOL_ID,
            direct_arguments(SECRET_NAME, "agent:printer"),
        ),
    ];

    for (runtime, tool_id, arguments) in cases {
        let result = runtime.invoke(&request(tool_id, arguments.clone()));
        assert!(!result.ok, "expected {tool_id} to fail for {arguments:?}");
        let error = result.error.as_ref().expect("failure carries an error");
        assert!(
            !error.message.contains(&secret),
            "error message leaked secret bytes: {}",
            error.message
        );
        assert!(
            !error.message.contains(SECRET_NAME) && !error.message.contains("no-such-secret-xyz"),
            "error message interpolated a secret name: {}",
            error.message
        );
        assert_eq!(
            error.details,
            JsonValue::Null,
            "handler errors carry no details payload: {error:?}"
        );
    }
}

#[test]
fn an_unregistered_secret_is_an_execution_error_with_the_static_message() {
    let runtime = runtime_with(RecordingDelivery::accepting());
    let result = runtime.invoke(&request(
        VAULT_REQUEST_LEASE_TOOL_ID,
        lease_arguments("absent", 60_000),
    ));

    let error = result.error.expect("failure carries an error");
    assert_eq!(error.kind, ToolErrorKind::ToolExecutionError);
    assert_eq!(error.message, errors::SECRET_NOT_FOUND);
}

#[test]
fn a_refused_delivery_is_a_permission_denial_not_an_execution_fault() {
    // The adapter refusing is the adapter exercising authority the caller does
    // not have. Reporting that as an execution error would tell the caller to
    // retry, which is exactly wrong.
    let runtime = runtime_with(RecordingDelivery::refusing(
        VaultDirectDeliveryError::Rejected,
    ));
    let result = runtime.invoke(&request(
        VAULT_REQUEST_DIRECT_TOOL_ID,
        direct_arguments(SECRET_NAME, "agent:printer"),
    ));

    let error = result.error.expect("failure carries an error");
    assert_eq!(error.kind, ToolErrorKind::ToolPermissionDenied);
    assert_eq!(error.message, errors::DELIVERY_REJECTED);
}

#[test]
fn the_three_delivery_failures_stay_distinguishable() {
    let cases = [
        (
            VaultDirectDeliveryError::ConsumerNotFound,
            errors::DELIVERY_CONSUMER_NOT_FOUND,
            ToolErrorKind::ToolExecutionError,
        ),
        (
            VaultDirectDeliveryError::Rejected,
            errors::DELIVERY_REJECTED,
            ToolErrorKind::ToolPermissionDenied,
        ),
        (
            VaultDirectDeliveryError::Unavailable,
            errors::DELIVERY_UNAVAILABLE,
            ToolErrorKind::ToolExecutionError,
        ),
    ];

    for (failure, expected_message, expected_kind) in cases {
        let runtime = runtime_with(RecordingDelivery::refusing(failure));
        let result = runtime.invoke(&request(
            VAULT_REQUEST_DIRECT_TOOL_ID,
            direct_arguments(SECRET_NAME, "agent:printer"),
        ));
        let error = result.error.expect("failure carries an error");
        assert_eq!(error.message, expected_message, "for {failure:?}");
        assert_eq!(error.kind, expected_kind, "for {failure:?}");
    }
}

#[test]
fn a_zero_ttl_is_rejected_with_the_lease_layers_static_reason() {
    // The lease layer's InvalidParameter payload is `&'static str`, which is
    // why surfacing it cannot leak runtime data by construction.
    let runtime = runtime_with(RecordingDelivery::accepting());
    let result = runtime.invoke(&request(
        VAULT_REQUEST_LEASE_TOOL_ID,
        lease_arguments(SECRET_NAME, 0),
    ));

    let error = result.error.expect("failure carries an error");
    assert_eq!(error.kind, ToolErrorKind::ToolValidationError);
    assert!(
        error.message.contains("ttl_ms"),
        "expected the lease layer's reason, got {}",
        error.message
    );
}

// ===========================================================================
// V3 — handlers validate their own arguments
// ===========================================================================
//
// These call the handler closures directly, bypassing the registry, because
// that is the situation the invariant exists for. Going through `invoke` would
// test the registry's schema validation instead of the handler's.

fn lease_direct_call(
    arguments: JsonValue,
) -> Result<ToolHandlerOutput, chief_of_staff_tool_api::ToolCallError> {
    let bridge = VaultToolBridge::new(vault_with_secret(), RecordingDelivery::accepting());
    let handler = bridge.lease_handler();
    handler(arguments, execution_context(VAULT_REQUEST_LEASE_TOOL_ID))
}

fn direct_direct_call(
    arguments: JsonValue,
) -> Result<ToolHandlerOutput, chief_of_staff_tool_api::ToolCallError> {
    let bridge = VaultToolBridge::new(vault_with_secret(), RecordingDelivery::accepting());
    let handler = bridge.direct_handler();
    handler(arguments, execution_context(VAULT_REQUEST_DIRECT_TOOL_ID))
}

fn execution_context(tool_id: &str) -> chief_of_staff_tool_api::ToolExecutionContext {
    chief_of_staff_tool_api::ToolExecutionContext::from_request(&request(tool_id, JsonValue::Null))
}

#[test]
fn the_lease_handler_rejects_every_malformed_argument_shape() {
    let cases: Vec<(JsonValue, &str)> = vec![
        (JsonValue::Null, errors::ARGUMENTS_NOT_OBJECT),
        (JsonValue::Array(vec![]), errors::ARGUMENTS_NOT_OBJECT),
        (string("not an object"), errors::ARGUMENTS_NOT_OBJECT),
        (
            object(vec![("ttl_ms", integer(1_000))]),
            errors::SECRET_NAME_REQUIRED,
        ),
        (
            object(vec![
                ("secret_name", JsonValue::Null),
                ("ttl_ms", integer(1_000)),
            ]),
            errors::SECRET_NAME_REQUIRED,
        ),
        (
            object(vec![
                ("secret_name", integer(7)),
                ("ttl_ms", integer(1_000)),
            ]),
            errors::SECRET_NAME_TYPE,
        ),
        (
            object(vec![
                ("secret_name", string("")),
                ("ttl_ms", integer(1_000)),
            ]),
            errors::SECRET_NAME_LENGTH,
        ),
        (
            object(vec![
                (
                    "secret_name",
                    string(&"a".repeat(MAX_SECRET_NAME_BYTES + 1)),
                ),
                ("ttl_ms", integer(1_000)),
            ]),
            errors::SECRET_NAME_LENGTH,
        ),
        (
            object(vec![("secret_name", string(SECRET_NAME))]),
            errors::TTL_REQUIRED,
        ),
        (
            object(vec![
                ("secret_name", string(SECRET_NAME)),
                ("ttl_ms", JsonValue::Null),
            ]),
            errors::TTL_REQUIRED,
        ),
        (
            object(vec![
                ("secret_name", string(SECRET_NAME)),
                ("ttl_ms", string("60000")),
            ]),
            errors::TTL_TYPE,
        ),
        (
            object(vec![
                ("secret_name", string(SECRET_NAME)),
                ("ttl_ms", JsonValue::Number(JsonNumber::Float(1.5))),
            ]),
            errors::TTL_TYPE,
        ),
        (
            object(vec![
                ("secret_name", string(SECRET_NAME)),
                ("ttl_ms", integer(-1)),
            ]),
            errors::TTL_NEGATIVE,
        ),
    ];

    for (arguments, expected) in cases {
        let error = lease_direct_call(arguments.clone())
            .expect_err(&format!("expected rejection for {arguments:?}"));
        assert_eq!(error.kind, ToolErrorKind::ToolValidationError);
        assert_eq!(error.message, expected, "for {arguments:?}");
    }
}

#[test]
fn the_direct_handler_rejects_every_malformed_argument_shape() {
    let cases: Vec<(JsonValue, &str)> = vec![
        (JsonValue::Null, errors::ARGUMENTS_NOT_OBJECT),
        (
            object(vec![("consumer_agent_id", string("agent:printer"))]),
            errors::SECRET_NAME_REQUIRED,
        ),
        (
            object(vec![("secret_name", string(SECRET_NAME))]),
            errors::CONSUMER_REQUIRED,
        ),
        (
            object(vec![
                ("secret_name", string(SECRET_NAME)),
                ("consumer_agent_id", JsonValue::Null),
            ]),
            errors::CONSUMER_REQUIRED,
        ),
        (
            object(vec![
                ("secret_name", string(SECRET_NAME)),
                ("consumer_agent_id", integer(7)),
            ]),
            errors::CONSUMER_TYPE,
        ),
        (
            object(vec![
                ("secret_name", string(SECRET_NAME)),
                ("consumer_agent_id", string("")),
            ]),
            errors::CONSUMER_INVALID,
        ),
    ];

    for (arguments, expected) in cases {
        let error = direct_direct_call(arguments.clone())
            .expect_err(&format!("expected rejection for {arguments:?}"));
        assert_eq!(error.kind, ToolErrorKind::ToolValidationError);
        assert_eq!(error.message, expected, "for {arguments:?}");
    }
}

#[test]
fn a_duplicated_argument_key_is_rejected_rather_than_resolved() {
    // `JsonValue::Object` preserves duplicates, and the schema validator checks
    // *every* occurrence against the property schema — so both names below are
    // individually valid and nothing type-invalid slips past.
    //
    // The hazard is disagreement, not validation. D18D V4 puts per-call
    // authorization in the policy engine; a policy parsing these arguments with
    // map or last-wins semantics would authorize one name while this handler
    // fetched the other, with both components individually correct. Rejecting
    // removes the category instead of betting on two parsers agreeing forever.
    for arguments in [
        object(vec![
            ("secret_name", string(SECRET_NAME)),
            ("secret_name", string("absent")),
            ("ttl_ms", integer(60_000)),
        ]),
        object(vec![
            ("secret_name", string(SECRET_NAME)),
            ("ttl_ms", integer(60_000)),
            ("ttl_ms", integer(1)),
        ]),
    ] {
        let error = lease_direct_call(arguments.clone())
            .expect_err(&format!("a repeated key must be refused: {arguments:?}"));
        assert_eq!(error.kind, ToolErrorKind::ToolValidationError);
        assert_eq!(error.message, errors::DUPLICATE_ARGUMENT);
    }

    // The direct handler is the one where this matters most: a repeated
    // `consumer_agent_id` is a redirected destination, so covering only the
    // lease handler would leave the more interesting attack untested. The
    // mechanism is the shared `field()` helper, but a shared mechanism is a
    // reason the test is cheap, not a reason to skip it.
    for arguments in [
        object(vec![
            ("secret_name", string(SECRET_NAME)),
            ("consumer_agent_id", string("agent:printer")),
            ("consumer_agent_id", string("agent:attacker")),
        ]),
        object(vec![
            ("secret_name", string(SECRET_NAME)),
            ("secret_name", string("other")),
            ("consumer_agent_id", string("agent:printer")),
        ]),
    ] {
        let error = direct_direct_call(arguments.clone())
            .expect_err(&format!("a repeated key must be refused: {arguments:?}"));
        assert_eq!(error.kind, ToolErrorKind::ToolValidationError);
        assert_eq!(error.message, errors::DUPLICATE_ARGUMENT);
    }
}

#[test]
fn an_agent_cannot_mint_a_lease_that_outlives_the_sweep_horizon() {
    // The lease layer permits 90 days and bounds its table. Together those are
    // a squat: fill the shared table at the maximum TTL and every other
    // consumer — including trusted host paths — is locked out for a quarter.
    // Capping the agent-facing TTL bounds how long a squat can hold slots in
    // the shared lease table; the runtime index reclaims them via its own
    // usability sweep.
    let over = i64::try_from(MAX_AGENT_LEASE_TTL_MS).expect("ceiling fits in i64") + 1;
    let error = lease_direct_call(lease_arguments(SECRET_NAME, over))
        .expect_err("a TTL past the agent ceiling must be refused");
    assert_eq!(error.kind, ToolErrorKind::ToolValidationError);
    assert_eq!(error.message, errors::TTL_TOO_LONG);

    // And the boundary itself is allowed, so the check is a ceiling and not an
    // off-by-one that quietly shortens every legitimate lease.
    let at = i64::try_from(MAX_AGENT_LEASE_TTL_MS).expect("ceiling fits in i64");
    lease_direct_call(lease_arguments(SECRET_NAME, at))
        .expect("a TTL exactly at the ceiling is in range");
}

#[test]
fn a_secret_name_at_the_length_bound_is_accepted() {
    // Guards the boundary in the direction that matters: an off-by-one here
    // would reject legitimate names rather than accept oversized ones, which
    // is a quieter failure and therefore worth pinning.
    let name = "b".repeat(MAX_SECRET_NAME_BYTES);
    let vault = Arc::new(ChiefVaultRuntime::new());
    vault.register_secret(
        name.clone(),
        LeasePayload::new(SECRET_BYTES.to_vec()),
        SecretPolicy::unrestricted(0),
    );
    let bridge = VaultToolBridge::new(vault, RecordingDelivery::accepting());
    let handler = bridge.lease_handler();

    let output = handler(
        lease_arguments(&name, 60_000),
        execution_context(VAULT_REQUEST_LEASE_TOOL_ID),
    )
    .expect("a name of exactly MAX_SECRET_NAME_BYTES is in range");
    assert!(matches!(output.output, JsonValue::Object(_)));
}

// ===========================================================================
// V4 — registration goes through the host's checked path
// ===========================================================================

fn host_profile(max_tier: PrivilegeTier, allowed: &[&str], capabilities: &[&str]) -> HostProfile {
    HostProfile {
        profile_id: "profile-vault".to_string(),
        host_id: "host-vault".to_string(),
        max_tier,
        allowed_tools: allowed.iter().map(|tool| tool.to_string()).collect(),
        capabilities: capabilities.iter().map(|cap| cap.to_string()).collect(),
    }
}

fn orchestrator(host: HostProfile) -> OrchestratorProfileRuntime {
    OrchestratorProfileRuntime::new(OrchestratorProfile {
        profile_id: "profile-vault".to_string(),
        hosts: vec![host],
    })
    .expect("profile should be valid")
}

#[test]
fn a_conforming_host_registers_both_vault_tools() {
    let mut host = orchestrator(host_profile(
        PrivilegeTier::Tier2,
        &[VAULT_REQUEST_LEASE_TOOL_ID, VAULT_REQUEST_DIRECT_TOOL_ID],
        &["vault:lease", "vault:direct"],
    ));
    let bridge = VaultToolBridge::new(vault_with_secret(), RecordingDelivery::accepting());

    bridge
        .register_into_host(&mut host)
        .expect("a Tier2 host holding both capabilities may register both tools");
}

#[test]
fn a_host_below_tier2_cannot_register_the_vault_tools() {
    let mut host = orchestrator(host_profile(
        PrivilegeTier::Tier1,
        &[VAULT_REQUEST_LEASE_TOOL_ID, VAULT_REQUEST_DIRECT_TOOL_ID],
        &["vault:lease", "vault:direct"],
    ));
    let bridge = VaultToolBridge::new(vault_with_secret(), RecordingDelivery::accepting());

    let error = bridge
        .register_into_host(&mut host)
        .expect_err("the privilege ceiling must reject a Tier1 host");
    assert!(
        matches!(error, HostRuntimeError::PrivilegeCeilingExceeded { .. }),
        "expected a ceiling rejection, got {error:?}"
    );
}

#[test]
fn a_host_missing_the_declared_capability_cannot_register() {
    let mut host = orchestrator(host_profile(
        PrivilegeTier::Tier2,
        &[VAULT_REQUEST_LEASE_TOOL_ID, VAULT_REQUEST_DIRECT_TOOL_ID],
        &["vault:lease"],
    ));
    let bridge = VaultToolBridge::new(vault_with_secret(), RecordingDelivery::accepting());

    let error = bridge
        .register_into_host(&mut host)
        .expect_err("vault:direct is declared by the definition and missing from the host");
    assert!(
        matches!(
            error,
            HostRuntimeError::MissingCapability { ref capability, .. }
                if capability == "vault:direct"
        ),
        "expected the missing capability to be named, got {error:?}"
    );
}

#[test]
fn a_rejected_registration_leaves_nothing_registered() {
    // Registering in a plain loop would leave request_lease wired up and
    // request_direct refused, and the caller would only learn "something
    // failed". A half-wired vault is worse than an unwired one, because the
    // half that registered looks healthy.
    let mut host = orchestrator(host_profile(
        PrivilegeTier::Tier2,
        &[VAULT_REQUEST_LEASE_TOOL_ID, VAULT_REQUEST_DIRECT_TOOL_ID],
        &["vault:lease"],
    ));
    let bridge = VaultToolBridge::new(vault_with_secret(), RecordingDelivery::accepting());

    bridge
        .register_into_host(&mut host)
        .expect_err("vault:direct is missing, so the pair must be refused");
    assert_eq!(
        host.summary().registered_tool_count,
        0,
        "a refused pair must leave the host untouched, not half-wired"
    );
}

#[test]
fn the_preflight_covers_the_registry_checks_too_not_only_the_profile_ones() {
    // A pre-flight that checks fewer things than the real call is worse than
    // none: it turns "this will fail" into "this will succeed" immediately
    // before it fails anyway, which is exactly the half-wired state the
    // pre-flight exists to prevent. The profile checks are the obvious three;
    // the registry's duplicate-id check is the one a caller trips over.
    let mut host = orchestrator(host_profile(
        PrivilegeTier::Tier2,
        &[VAULT_REQUEST_LEASE_TOOL_ID, VAULT_REQUEST_DIRECT_TOOL_ID],
        &["vault:lease", "vault:direct"],
    ));

    // Something else already owns one of the two ids.
    let squatted =
        builtin_tool_definition(VAULT_REQUEST_DIRECT_TOOL_ID).expect("built-in must exist");
    host.register_handler(squatted, |_arguments, _context| {
        Ok(ToolHandlerOutput::new(JsonValue::Null))
    })
    .expect("the squatter registers first");

    let bridge = VaultToolBridge::new(vault_with_secret(), RecordingDelivery::accepting());
    let error = bridge
        .register_into_host(&mut host)
        .expect_err("the occupied id must be caught during pre-flight");
    assert!(
        matches!(
            error,
            HostRuntimeError::ToolApi(ToolApiError::DuplicateToolId(ref tool))
                if tool == VAULT_REQUEST_DIRECT_TOOL_ID
        ),
        "expected a duplicate-id rejection, got {error:?}"
    );

    // Only the squatter is registered: the bridge added neither of its own.
    assert_eq!(
        host.summary().registered_tool_count,
        1,
        "the refused pair must not have wired request_lease on the way past"
    );
}

#[test]
fn a_host_that_does_not_allow_the_tool_cannot_register_it() {
    let mut host = orchestrator(host_profile(
        PrivilegeTier::Tier2,
        &[VAULT_REQUEST_LEASE_TOOL_ID],
        &["vault:lease", "vault:direct"],
    ));
    let bridge = VaultToolBridge::new(vault_with_secret(), RecordingDelivery::accepting());

    let error = bridge
        .register_into_host(&mut host)
        .expect_err("request_direct is not in the host's allowed_tools");
    assert!(
        matches!(error, HostRuntimeError::ToolNotAllowed(ref tool) if tool == VAULT_REQUEST_DIRECT_TOOL_ID),
        "expected the disallowed tool to be named, got {error:?}"
    );
}

// ===========================================================================
// Catalog agreement
// ===========================================================================

#[test]
fn the_bridge_registers_under_the_catalogs_own_definitions() {
    // If this crate invented its own definitions, it could quietly register the
    // vault tools at a lower tier than the rest of the system validates against.
    let definitions = VaultToolBridge::definitions().expect("catalog must carry both tools");
    assert_eq!(definitions.len(), 2);

    for definition in definitions {
        assert_eq!(definition.required_tier, PrivilegeTier::Tier2);
        let catalog = builtin_tool_definition(&definition.tool_id).expect("built-in must exist");
        assert_eq!(definition, catalog);
    }
}

#[test]
fn a_lease_reference_is_redeemable_exactly_once_and_only_by_the_broker() {
    // The reference the caller receives is a bearer capability the *host* holds
    // the redemption path for. Redemption is not a tool, so an agent cannot
    // reach it; and it is one-shot, so a copied reference is worth nothing
    // after the trusted handler has used it.
    let vault = vault_with_secret();
    let bridge = VaultToolBridge::new(Arc::clone(&vault), RecordingDelivery::accepting());
    let handler = bridge.lease_handler();

    let output = handler(
        lease_arguments(SECRET_NAME, 60_000),
        execution_context(VAULT_REQUEST_LEASE_TOOL_ID),
    )
    .expect("lease should issue");
    let JsonValue::Object(fields) = output.output else {
        panic!("lease output must be an object");
    };
    let JsonValue::String(reference) = &fields[0].1 else {
        panic!("vault_ref must be a string");
    };

    let vault_ref = smart_home_core::VaultRef::trusted(reference.clone());
    let payload = vault
        .consume(&vault_ref)
        .expect("first redemption succeeds");
    assert_eq!(payload.as_bytes(), SECRET_BYTES);
    assert!(
        vault.consume(&vault_ref).is_err(),
        "a redeemed reference must not be redeemable again"
    );
}

// ===========================================================================
// Deliberate lease-only registration (D18D 7.1 V4)
// ===========================================================================

#[test]
fn a_lease_only_registration_wires_exactly_one_tool() {
    // A deployment with no trusted delivery adapter cannot honestly offer
    // request_direct. The profile below grants only the lease tool, which the
    // all-or-nothing pair registration would refuse outright.
    let mut host = orchestrator(host_profile(
        PrivilegeTier::Tier2,
        &[VAULT_REQUEST_LEASE_TOOL_ID],
        &["vault:lease"],
    ));
    let bridge = VaultToolBridge::new(vault_with_secret(), RecordingDelivery::accepting());

    bridge
        .register_lease_only_into_host(&mut host)
        .expect("a lease-only host should accept the lease tool");
    assert_eq!(host.summary().registered_tool_count, 1);
}

#[test]
fn the_pair_registration_still_refuses_that_same_host() {
    // The two operations must stay distinguishable. If `register_into_host`
    // quietly succeeded here, "deliberate subset" and "misconfigured host"
    // would look identical from the call site, which is the thing V4's
    // amendment exists to prevent.
    let mut host = orchestrator(host_profile(
        PrivilegeTier::Tier2,
        &[VAULT_REQUEST_LEASE_TOOL_ID],
        &["vault:lease"],
    ));
    let bridge = VaultToolBridge::new(vault_with_secret(), RecordingDelivery::accepting());

    bridge
        .register_into_host(&mut host)
        .expect_err("request_direct is not allowed on this host");
    assert_eq!(
        host.summary().registered_tool_count,
        0,
        "a refused pair must leave the host untouched"
    );
}

#[test]
fn a_lease_only_host_still_enforces_tier_and_capability() {
    // The subset is narrower, not laxer: it goes through the same checked path.
    let bridge = VaultToolBridge::new(vault_with_secret(), RecordingDelivery::accepting());

    let mut low_tier = orchestrator(host_profile(
        PrivilegeTier::Tier1,
        &[VAULT_REQUEST_LEASE_TOOL_ID],
        &["vault:lease"],
    ));
    assert!(matches!(
        bridge.register_lease_only_into_host(&mut low_tier),
        Err(HostRuntimeError::PrivilegeCeilingExceeded { .. })
    ));

    let mut no_capability = orchestrator(host_profile(
        PrivilegeTier::Tier2,
        &[VAULT_REQUEST_LEASE_TOOL_ID],
        &["vault:direct"],
    ));
    assert!(matches!(
        bridge.register_lease_only_into_host(&mut no_capability),
        Err(HostRuntimeError::MissingCapability { .. })
    ));
}
