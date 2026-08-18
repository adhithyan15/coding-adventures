//! D18D vault tool handlers bound to the D18 vault runtime.
//!
//! # What this crate is for
//!
//! Three pieces of the Chief of Staff stack already existed and did not touch
//! each other:
//!
//! ```text
//!   chief-of-staff-tool-api        declares  vault.request_lease
//!                                            vault.request_direct
//!                                            ... and dispatches to handlers
//!
//!   chief-of-staff-vault-runtime   owns      the secret material, the lease
//!                                            manager, and the trusted
//!                                            direct-delivery boundary
//!
//!   chief-of-staff-host-runtime    enforces  which tools a host may register,
//!                                            the privilege ceiling, and the
//!                                            declared capabilities
//! ```
//!
//! The tool declarations were catalogued but nothing implemented them, so an
//! agent that called `vault.request_lease` got `ToolNotFound` — the vault was
//! reachable in principle and unreachable in practice. This crate is the
//! missing edge: it implements the two declared tools over the vault runtime
//! and registers them through the host's checked registration path.
//!
//! # The shape of the two tools, and why they differ
//!
//! The two vault tools answer two genuinely different questions, and the
//! difference is visible in their return types:
//!
//! ```text
//!   vault.request_lease  ->  { vault_ref, expires_at_ms }
//!                            "here is a handle; a trusted host tool can
//!                             redeem it later"
//!
//!   vault.request_direct ->  null
//!                            "the secret has been handed to the consumer you
//!                             named; you are not the consumer"
//! ```
//!
//! `request_lease` returns a *bearer capability*, not a secret: a `VaultRef` is
//! an opaque string that only the trusted host broker can redeem, and redeeming
//! it consumes it. The caller learns a handle and an expiry, never bytes.
//!
//! `request_direct` returns *nothing at all*. This is the interesting one. A
//! naive design would return the secret so the caller could forward it, which
//! makes the caller a secret-handling component. Instead the vault runtime
//! moves the payload into a trusted delivery adapter and hands the caller back
//! unit. There is no return path a secret could travel along, because the
//! return type has no room for one.
//!
//! # The four invariants (D18D section 7.1)
//!
//! The spec states these normatively; they are restated here because this is
//! the file that has to obey them.
//!
//! **V1 — no secret return channel.** Nothing derived from secret bytes may
//! reach [`ToolHandlerOutput`] or [`ToolCallError`].
//!
//! For `request_direct` the `output` field specifically is more than handler
//! discipline: the declared `output_schema` is JSON `null`, and the tool
//! runtime validates handler output against that schema *after* the handler
//! returns, so an edit that smuggled bytes into `output` would be rejected even
//! if it got past review.
//!
//! Be precise about how far that reaches, because a guarantee believed to be
//! wider than it is, is worse than none. The runtime validates `output` and
//! **only** `output`. `artifact_refs` and `memory_refs` are copied to the
//! result unchecked; `events` are assembled *before* validation runs and are
//! published even on the rejection path; and the check is `if let Some(schema)`
//! against the definition passed to `register_handler`, so it is a property of
//! that definition rather than of the tool id — another crate registering
//! `vault.request_direct` with `output_schema: None` would get no validation at
//! all. Both handlers here therefore return empty `artifact_refs`,
//! `memory_refs`, and `events`, and there are tests pinning that, because for
//! those three fields handler discipline is genuinely all there is.
//!
//! **V2 — bounded, secret-free errors.** Every error this crate produces
//! carries one of a closed set of `&'static str` messages, listed in
//! [`errors`]. The one apparent exception is the lease layer's
//! `LeaseError::InvalidParameter`, which we do surface — but its payload is
//! typed `&'static str`, so it structurally cannot carry runtime data, let
//! alone secret data.
//!
//! **V3 — validate arguments independently of the registry.** The tool runtime
//! validates arguments against `input_schema` before dispatching, but a handler
//! is a plain trait object that anything can call directly. Every argument is
//! therefore re-checked here. There is no `unwrap`, no indexing, and no
//! arithmetic on caller-controlled values.
//!
//! **V4 — the registration path is the tier gate.**
//! [`VaultToolBridge::register_into_host`] goes through the host runtime's
//! `register_handler`, which checks `allows_tool`, the `required_tier` ceiling,
//! and `required_capabilities`. Registration failure is returned to the caller
//! rather than swallowed, because a silently unregistered vault tool is
//! indistinguishable — from the agent's side — from a denied one, and the
//! operator deserves to learn about it at startup instead of at first use. The
//! pair is registered all-or-nothing, via a pre-flight that is co-total with
//! the registration path — profile checks *and* the registry's own.
//!
//! Note what this gate is not. It runs once, at registration, per host, and is
//! a statement about a *tool*, not a *secret* — it cannot express "this host
//! may lease the weather key but not the bank password". The per-secret half of
//! that question is answered below the handlers, by the vault runtime's
//! admission policy (VLT06): each secret carries `allowed_agents` and
//! `allowed_mode`, and both handlers forward the attested `agent_id` so the
//! vault can apply them. What remains absent is a *per-call* policy engine —
//! the default runtime policy still admits everything.
//!
//! # Example
//!
//! ```
//! use std::sync::Arc;
//!
//! use chief_of_staff_vault_dispatch::VaultToolBridge;
//! use chief_of_staff_vault_runtime::{
//!     ChiefVaultRuntime, SecretPolicy, VaultDirectDelivery, VaultDirectDeliveryError,
//!     VaultDirectRequest,
//! };
//! use coding_adventures_vault_leases::LeasePayload;
//!
//! // A trusted adapter that accepts deliveries and never returns bytes.
//! // A real one would decide using `request` — which secret, asked for by
//! // whom, bound for where — rather than accepting everything.
//! struct Sink;
//! impl VaultDirectDelivery for Sink {
//!     fn deliver(&self, request: VaultDirectRequest<'_>, _payload: LeasePayload)
//!         -> Result<(), VaultDirectDeliveryError> {
//!         if request.consumer_agent_id == "agent:printer" {
//!             Ok(())
//!         } else {
//!             Err(VaultDirectDeliveryError::Rejected)
//!         }
//!     }
//! }
//!
//! let vault = Arc::new(ChiefVaultRuntime::new());
//! vault.register_secret(
//!     "weather-api-key",
//!     LeasePayload::new(b"s3cret".to_vec()),
//!     SecretPolicy::unrestricted(0),
//! );
//!
//! let bridge = VaultToolBridge::new(vault, Arc::new(Sink));
//! let mut tools = chief_of_staff_tool_api::InMemoryToolRuntime::new();
//! bridge.register_all(&mut tools).expect("vault tools should register");
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::sync::Arc;

use chief_of_staff_host_runtime::{HostRuntimeError, OrchestratorProfileRuntime};
use chief_of_staff_tool_api::{
    builtin_tool_definition, InMemoryToolRuntime, ToolApiError, ToolCallError, ToolDefinition,
    ToolErrorKind, ToolExecutionContext, ToolHandlerOutput,
};
use chief_of_staff_vault_runtime::{
    ChiefVaultRuntime, VaultDirectDelivery, VaultDirectDeliveryError, VaultDirectRequest,
    VaultLeaseRequest, VaultRuntimeError,
};
use coding_adventures_json_value::{JsonNumber, JsonValue};
use coding_adventures_vault_leases::LeaseError;

/// Canonical tool id for the lease-issuing vault tool.
pub const VAULT_REQUEST_LEASE_TOOL_ID: &str = "vault.request_lease";

/// Canonical tool id for the direct-delivery vault tool.
pub const VAULT_REQUEST_DIRECT_TOOL_ID: &str = "vault.request_direct";

/// Largest `secret_name` this crate will forward to the vault runtime.
///
/// The vault runtime looks names up in a `HashMap<String, _>`, so an unbounded
/// name is an unbounded hash of caller-controlled bytes on every call. The
/// bound is deliberately generous: real secret names are short, and anything
/// approaching this size is a probe rather than a lookup.
pub const MAX_SECRET_NAME_BYTES: usize = 512;

/// Longest lease an *agent* may request, in milliseconds (15 minutes).
///
/// The lease layer permits up to 90 days and bounds the table at
/// `MAX_OUTSTANDING_LEASES`. Those two facts together are a denial-of-service
/// waiting to happen on the agent-facing path: an agent that asks for the
/// maximum TTL in a loop fills the shared table with rows that cannot be swept
/// for three months, and the trusted host paths that share the same manager are
/// locked out for the duration. The memory bound is still worth having — it is
/// the squat's *persistence* that turns it from noisy into effective.
///
/// Capping the agent-facing TTL well below the sweep horizon makes the attack
/// self-healing: the rows expire, `issue`'s inline sweep reclaims them, and the
/// worst an agent achieves is a fifteen-minute nuisance rather than a quarter.
///
/// 15 minutes is chosen against what a lease is *for*: a handle a trusted host
/// tool redeems shortly after it is minted. A workflow that genuinely needs
/// longer should be re-requesting, not holding.
pub const MAX_AGENT_LEASE_TTL_MS: u64 = 15 * 60 * 1_000;

/// Every static error message this crate can produce.
///
/// Collected in one module so that V2 — "bounded, secret-free errors" — is
/// checkable by reading a single screen rather than by auditing every `Err`
/// site. A test asserts that each handler's errors come from this set.
pub mod errors {
    /// `secret_name` was absent from the arguments object.
    pub const SECRET_NAME_REQUIRED: &str = "secret_name is required";
    /// `secret_name` was present but not a JSON string.
    pub const SECRET_NAME_TYPE: &str = "secret_name must be a string";
    /// `secret_name` was empty or longer than [`super::MAX_SECRET_NAME_BYTES`].
    pub const SECRET_NAME_LENGTH: &str = "secret_name length is out of range";
    /// `ttl_ms` was absent from the arguments object.
    pub const TTL_REQUIRED: &str = "ttl_ms is required";
    /// `ttl_ms` was present but not a JSON integer.
    pub const TTL_TYPE: &str = "ttl_ms must be an integer";
    /// `ttl_ms` was negative, which cannot denote a duration.
    pub const TTL_NEGATIVE: &str = "ttl_ms must not be negative";
    /// `ttl_ms` exceeded [`super::MAX_AGENT_LEASE_TTL_MS`].
    pub const TTL_TOO_LONG: &str = "ttl_ms exceeds the agent lease ceiling";
    /// `consumer_agent_id` was absent from the arguments object.
    pub const CONSUMER_REQUIRED: &str = "consumer_agent_id is required";
    /// `consumer_agent_id` was present but not a JSON string.
    pub const CONSUMER_TYPE: &str = "consumer_agent_id must be a string";
    /// The arguments value was not a JSON object.
    pub const ARGUMENTS_NOT_OBJECT: &str = "arguments must be an object";
    /// The arguments object repeated a key this handler reads.
    pub const DUPLICATE_ARGUMENT: &str = "arguments must not repeat a key";
    /// The vault runtime rejected the consumer identifier's shape.
    pub const CONSUMER_INVALID: &str = "consumer_agent_id is not a valid identifier";
    /// No secret is registered under the requested name.
    pub const SECRET_NOT_FOUND: &str = "vault secret is not registered";
    /// The secret already has as many tracked leases as the vault will revoke
    /// on rotation. Transient: slots free as leases are redeemed or expire.
    pub const TOO_MANY_LEASES: &str = "secret has too many outstanding leases";
    /// The secret forbids the requested delivery mode (VLT06 P1).
    pub const MODE_NOT_PERMITTED: &str = "secret does not permit this delivery mode";
    /// The requesting agent is not on the secret's allow-list, or brought no
    /// attested identity at all (VLT06 P2, P3).
    pub const AGENT_NOT_PERMITTED: &str = "agent may not request this secret";
    /// The trusted adapter has no route to the named consumer.
    pub const DELIVERY_CONSUMER_NOT_FOUND: &str = "direct-delivery consumer not found";
    /// The trusted adapter refused to accept the delivery.
    pub const DELIVERY_REJECTED: &str = "direct delivery was refused";
    /// The trusted adapter's transport is temporarily unavailable.
    pub const DELIVERY_UNAVAILABLE: &str = "direct-delivery transport unavailable";
    /// The lease layer failed for a reason the caller cannot act on.
    pub const LEASE_UNAVAILABLE: &str = "vault lease could not be issued";
    /// A reference-shaped error surfaced on a path that mints no references.
    pub const UNEXPECTED_REFERENCE: &str = "vault reference error on a non-reference path";
}

/// Binds the two declared vault tools to a vault runtime and a trusted adapter.
///
/// Cheap to clone: both fields are `Arc`, so a bridge can be handed to several
/// host runtimes without duplicating the secret store.
#[derive(Clone)]
pub struct VaultToolBridge {
    vault: Arc<ChiefVaultRuntime>,
    delivery: Arc<dyn VaultDirectDelivery>,
}

impl VaultToolBridge {
    /// Construct a bridge over an existing vault runtime and delivery adapter.
    ///
    /// The adapter is the component entitled to accept or refuse a delivery to
    /// a named consumer. This crate deliberately does not authorize consumers
    /// itself: a `consumer_agent_id` arriving in a tool call is a *name*, not
    /// evidence, and treating it as evidence would make any caller able to
    /// redirect a secret by renaming its destination.
    pub fn new(vault: Arc<ChiefVaultRuntime>, delivery: Arc<dyn VaultDirectDelivery>) -> Self {
        Self { vault, delivery }
    }

    /// The two canonical definitions this bridge implements.
    ///
    /// Read from the built-in catalog rather than reconstructed here, so the
    /// schemas, tier, and capabilities this crate registers under are the same
    /// values the rest of the system validates against. If a definition ever
    /// disappeared from the catalog this returns an error instead of silently
    /// registering a locally invented one.
    pub fn definitions() -> Result<Vec<ToolDefinition>, ToolApiError> {
        [VAULT_REQUEST_LEASE_TOOL_ID, VAULT_REQUEST_DIRECT_TOOL_ID]
            .into_iter()
            .map(|tool_id| {
                builtin_tool_definition(tool_id)
                    .ok_or_else(|| ToolApiError::UnknownTool(tool_id.to_string()))
            })
            .collect()
    }

    /// Register both tools with a bare tool runtime.
    ///
    /// This path applies schema validation and handler dispatch but **not** the
    /// host's tool-allowed, tier, and capability checks — the tool registry has
    /// no host profile to check against. Use it for local dispatch and tests;
    /// use [`Self::register_into_host`] at a real host boundary.
    pub fn register_all(&self, runtime: &mut InMemoryToolRuntime) -> Result<(), ToolApiError> {
        for definition in Self::definitions()? {
            match definition.tool_id.as_str() {
                VAULT_REQUEST_LEASE_TOOL_ID => {
                    runtime.register_handler(definition, self.lease_handler())?
                }
                VAULT_REQUEST_DIRECT_TOOL_ID => {
                    runtime.register_handler(definition, self.direct_handler())?
                }
                // `definitions()` builds this list from two known ids, so this
                // arm is unreachable today. It is a hard error rather than a
                // skip because a third vault tool appearing here should stop
                // the host, not register with no handler behind it.
                other => return Err(ToolApiError::UnknownTool(other.to_string())),
            }
        }
        Ok(())
    }

    /// Register both tools at a host boundary, through the checked path.
    ///
    /// This is the D18D section 7.1 V4 path. The host runtime rejects the
    /// registration if the owning host profile does not allow the tool, if the
    /// profile's `max_tier` is below the definition's `required_tier` (`Tier2`
    /// for both vault tools), or if the profile does not carry the declared
    /// capabilities (`vault:lease` and `vault:direct`).
    ///
    /// The error is returned rather than logged: a host that cannot register
    /// its vault tools should fail to start, because an unregistered vault tool
    /// looks exactly like a denied one from the agent's side and the difference
    /// matters enormously to whoever is debugging it.
    pub fn register_into_host(
        &self,
        host: &mut OrchestratorProfileRuntime,
    ) -> Result<(), HostRuntimeError> {
        let definitions = Self::definitions().map_err(|_| {
            HostRuntimeError::ToolNotAllowed(VAULT_REQUEST_LEASE_TOOL_ID.to_string())
        })?;

        // Pre-flight every definition before registering any of them.
        //
        // Registering in a bare loop would leave a host that fails the second
        // check holding the first tool, and the caller only learns "something
        // failed". A half-wired vault is worse than an unwired one: the tools
        // that did register look healthy, so the failure surfaces later and
        // somewhere else. Either both go in or neither does.
        for definition in &definitions {
            host.check_registration(definition)?;
        }

        for definition in definitions {
            match definition.tool_id.as_str() {
                VAULT_REQUEST_LEASE_TOOL_ID => {
                    host.register_handler(definition, self.lease_handler())?
                }
                VAULT_REQUEST_DIRECT_TOOL_ID => {
                    host.register_handler(definition, self.direct_handler())?
                }
                other => return Err(HostRuntimeError::ToolNotAllowed(other.to_string())),
            }
        }
        Ok(())
    }

    /// Register **only** `vault.request_lease` at a host boundary.
    ///
    /// For deployments that have no trusted [`VaultDirectDelivery`]
    /// implementation. `request_direct` without one has nowhere to deliver, and
    /// registering it against a stub that accepts everything would be strictly
    /// worse than leaving it out: the agent would see a working direct-delivery
    /// tool while the secret went nowhere, or somewhere unaudited.
    ///
    /// This is not a hole in [`Self::register_into_host`]'s all-or-nothing
    /// rule. That rule is about *partial failure* — a host left holding one
    /// tool because the second was refused, where the half that registered
    /// looks healthy and the failure resurfaces somewhere else. This is a
    /// deliberate subset, chosen up front and complete in itself.
    ///
    /// The two are separate named operations precisely so a reader can tell
    /// which one a call site meant. Reaching the lease-only case by catching
    /// the pair's error would make an intended configuration indistinguishable
    /// from a misconfigured one.
    ///
    /// The host still needs `vault.request_lease` in `allowed_tools`, the
    /// `vault:lease` capability, and `max_tier >= Tier2`; it does **not** need
    /// anything for `request_direct`.
    pub fn register_lease_only_into_host(
        &self,
        host: &mut OrchestratorProfileRuntime,
    ) -> Result<(), HostRuntimeError> {
        let definition = builtin_tool_definition(VAULT_REQUEST_LEASE_TOOL_ID).ok_or_else(|| {
            HostRuntimeError::ToolNotAllowed(VAULT_REQUEST_LEASE_TOOL_ID.to_string())
        })?;
        host.check_registration(&definition)?;
        host.register_handler(definition, self.lease_handler())
    }

    /// Handler for `vault.request_lease`.
    ///
    /// Returns `{ vault_ref, expires_at_ms }` and nothing else. The `VaultRef`
    /// is intended to leave the boundary — that is the whole point of a lease —
    /// but it is a handle the trusted broker redeems, not secret material.
    pub fn lease_handler(
        &self,
    ) -> impl Fn(JsonValue, ToolExecutionContext) -> Result<ToolHandlerOutput, ToolCallError> + 'static
    {
        let vault = Arc::clone(&self.vault);
        move |arguments, context| {
            let secret_name = required_secret_name(&arguments)?;
            let ttl_ms = required_ttl_ms(&arguments)?;

            let receipt = vault
                .request_lease(VaultLeaseRequest {
                    requesting_agent_id: context.agent_id.as_deref(),
                    secret_name: &secret_name,
                    ttl_ms,
                })
                .map_err(lease_error)?;

            Ok(ToolHandlerOutput::new(JsonValue::Object(vec![
                (
                    "vault_ref".to_string(),
                    JsonValue::String(receipt.vault_ref.as_str().to_string()),
                ),
                (
                    "expires_at_ms".to_string(),
                    // `expires_at_ms` is a u64 from the lease layer and JSON
                    // integers are i64. A value past i64::MAX would be a clock
                    // roughly 292 million years out, but saturating is still
                    // cheaper than reasoning about it: an absurd-but-finite
                    // expiry is better than a wrapped one that reads as the
                    // distant past.
                    JsonValue::Number(JsonNumber::Integer(
                        i64::try_from(receipt.expires_at_ms).unwrap_or(i64::MAX),
                    )),
                ),
            ])))
        }
    }

    /// Handler for `vault.request_direct`.
    ///
    /// Returns JSON `null` on success. The payload travels from the vault
    /// runtime into the trusted delivery adapter and is never observed here:
    /// `ChiefVaultRuntime::request_direct` takes the adapter and returns unit,
    /// so this handler has no binding to the bytes at any point.
    ///
    /// The execution context is forwarded, not discarded. An adapter told only
    /// the destination can express nothing stronger than a global allowlist,
    /// under which a caller cleared to send one secret to a consumer is equally
    /// cleared to send every secret to it. Passing the requester and the secret
    /// name is what makes a real decision possible; see [`VaultDirectRequest`].
    pub fn direct_handler(
        &self,
    ) -> impl Fn(JsonValue, ToolExecutionContext) -> Result<ToolHandlerOutput, ToolCallError> + 'static
    {
        let vault = Arc::clone(&self.vault);
        let delivery = Arc::clone(&self.delivery);
        move |arguments, context| {
            let secret_name = required_secret_name(&arguments)?;
            let consumer_agent_id = required_consumer_agent_id(&arguments)?;

            vault
                .request_direct(
                    VaultDirectRequest {
                        requesting_agent_id: context.agent_id.as_deref(),
                        requesting_user_id: context.user_id.as_deref(),
                        session_id: context.session_id.as_deref(),
                        secret_name: &secret_name,
                        consumer_agent_id: &consumer_agent_id,
                    },
                    delivery.as_ref(),
                )
                .map_err(direct_error)?;

            Ok(ToolHandlerOutput::new(JsonValue::Null))
        }
    }
}

// ===========================================================================
// Argument extraction (V3)
// ===========================================================================
//
// The tool runtime validates arguments against `input_schema` before it
// dispatches, so in the normal path these checks are redundant. They are here
// because a `ToolHandler` is a trait object: anything holding one can call
// `invoke` directly with arbitrary JSON, and "the layer above validated it"
// stops being true the moment someone wires the handler up differently.

fn object_fields(value: &JsonValue) -> Result<&[(String, JsonValue)], ToolCallError> {
    match value {
        JsonValue::Object(fields) => Ok(fields),
        _ => Err(validation_error(errors::ARGUMENTS_NOT_OBJECT)),
    }
}

/// The sole occurrence of `field`, or `None`. A repeated key is an error.
///
/// `JsonValue::Object` preserves duplicate keys, so `{"secret_name": "a",
/// "secret_name": "b"}` is representable, and every consumer of that object has
/// to pick one. `JsonSchema::validate_value_at` iterates *every* field and
/// checks each duplicate against the property schema, so nothing type-invalid
/// slips past — but the schema constrains types, not values, and both names
/// above are valid strings.
///
/// So the danger is not validation, it is *disagreement*. D18D V4 puts per-call
/// authorization in the policy engine. A policy that parsed these arguments
/// with map or last-wins semantics would authorize `"a"` while this handler
/// fetched `"b"`, and both components would be individually correct. No shipped
/// policy engine reads arguments today, which makes this the cheapest possible
/// moment to remove the category rather than bet on two parsers agreeing
/// forever.
///
/// Rejecting is strictly safer than resolving: a duplicate key has no
/// legitimate sender, and the caller learns immediately instead of receiving
/// whichever value this side happened to prefer.
fn field<'a>(value: &'a JsonValue, name: &str) -> Result<Option<&'a JsonValue>, ToolCallError> {
    let mut found = None;
    for (key, entry) in object_fields(value)? {
        if key != name {
            continue;
        }
        if found.is_some() {
            return Err(validation_error(errors::DUPLICATE_ARGUMENT));
        }
        found = Some(entry);
    }
    Ok(found)
}

fn required_secret_name(arguments: &JsonValue) -> Result<String, ToolCallError> {
    let name = match field(arguments, "secret_name")? {
        Some(JsonValue::String(name)) => name.as_str(),
        Some(JsonValue::Null) | None => return Err(validation_error(errors::SECRET_NAME_REQUIRED)),
        Some(_) => return Err(validation_error(errors::SECRET_NAME_TYPE)),
    };
    // Bound before cloning, not after. The saving is only a transient copy of
    // bytes already resident in the request, but "check, then allocate" is the
    // habit that stays correct if the bound is ever tightened.
    if name.is_empty() || name.len() > MAX_SECRET_NAME_BYTES {
        return Err(validation_error(errors::SECRET_NAME_LENGTH));
    }
    Ok(name.to_string())
}

// The upper bound on this one lives in the vault runtime
// (MAX_CONSUMER_AGENT_ID_BYTES, 4 KiB), which rejects before touching the
// secret map, so there is nothing to re-check here.
fn required_consumer_agent_id(arguments: &JsonValue) -> Result<String, ToolCallError> {
    match field(arguments, "consumer_agent_id")? {
        Some(JsonValue::String(value)) => Ok(value.clone()),
        Some(JsonValue::Null) | None => Err(validation_error(errors::CONSUMER_REQUIRED)),
        Some(_) => Err(validation_error(errors::CONSUMER_TYPE)),
    }
}

fn required_ttl_ms(arguments: &JsonValue) -> Result<u64, ToolCallError> {
    match field(arguments, "ttl_ms")? {
        Some(JsonValue::Number(JsonNumber::Integer(value))) => {
            // `u64::try_from` on an i64 fails exactly for the negatives.
            let ttl_ms =
                u64::try_from(*value).map_err(|_| validation_error(errors::TTL_NEGATIVE))?;
            // The lease layer's own ceiling is 90 days, which is far too long
            // for an agent-issued handle to squat a bounded shared table with.
            if ttl_ms > MAX_AGENT_LEASE_TTL_MS {
                return Err(validation_error(errors::TTL_TOO_LONG));
            }
            Ok(ttl_ms)
        }
        // A float is a type error rather than something to round. `ttl_ms: 1.5`
        // has no defensible interpretation and the schema declares Integer.
        Some(JsonValue::Number(JsonNumber::Float(_))) => Err(validation_error(errors::TTL_TYPE)),
        Some(JsonValue::Null) | None => Err(validation_error(errors::TTL_REQUIRED)),
        Some(_) => Err(validation_error(errors::TTL_TYPE)),
    }
}

// ===========================================================================
// Error mapping (V2)
// ===========================================================================

fn validation_error(message: &'static str) -> ToolCallError {
    ToolCallError::new(ToolErrorKind::ToolValidationError, message)
}

fn execution_error(message: &'static str) -> ToolCallError {
    ToolCallError::new(ToolErrorKind::ToolExecutionError, message)
}

/// Map a lease-path failure onto a bounded, secret-free tool error.
fn lease_error(error: VaultRuntimeError) -> ToolCallError {
    match error {
        VaultRuntimeError::SecretNotFound => execution_error(errors::SECRET_NOT_FOUND),
        // Unlike the admission refusals below, this one IS worth retrying —
        // slots free as leases are redeemed or expire — so it is a conflict
        // rather than a denial.
        VaultRuntimeError::TooManyOutstandingLeases => {
            ToolCallError::new(ToolErrorKind::ToolConflict, errors::TOO_MANY_LEASES)
        }
        // An admission refusal is the vault exercising authority the caller
        // does not have, so it is a permission denial rather than an execution
        // fault — the caller should not read it as "retry".
        VaultRuntimeError::DeliveryModeNotPermitted => ToolCallError::new(
            ToolErrorKind::ToolPermissionDenied,
            errors::MODE_NOT_PERMITTED,
        ),
        VaultRuntimeError::AgentNotPermitted => ToolCallError::new(
            ToolErrorKind::ToolPermissionDenied,
            errors::AGENT_NOT_PERMITTED,
        ),
        VaultRuntimeError::InvalidConsumerAgentId => {
            // Unreachable on this path: request_lease takes no consumer. Mapped
            // rather than panicked because a handler that aborts the process on
            // an unexpected enum variant is a denial-of-service switch.
            validation_error(errors::CONSUMER_INVALID)
        }
        VaultRuntimeError::InvalidVaultRef => execution_error(errors::UNEXPECTED_REFERENCE),
        VaultRuntimeError::DirectDelivery(error) => delivery_error(error),
        // `InvalidParameter` carries `&'static str`, so surfacing it cannot
        // leak runtime data by construction — the type has no room for any.
        // Every other lease failure collapses to one message: the distinctions
        // between NotFound, Expired, and Revoked are meaningful when redeeming
        // a reference the caller already holds, and meaningless here, where no
        // reference existed yet.
        VaultRuntimeError::Lease(LeaseError::InvalidParameter(reason)) => validation_error(reason),
        VaultRuntimeError::Lease(_) => execution_error(errors::LEASE_UNAVAILABLE),
    }
}

/// Map a direct-delivery failure onto a bounded, secret-free tool error.
fn direct_error(error: VaultRuntimeError) -> ToolCallError {
    match error {
        VaultRuntimeError::SecretNotFound => execution_error(errors::SECRET_NOT_FOUND),
        // Unlike the admission refusals below, this one IS worth retrying —
        // slots free as leases are redeemed or expire — so it is a conflict
        // rather than a denial.
        VaultRuntimeError::TooManyOutstandingLeases => {
            ToolCallError::new(ToolErrorKind::ToolConflict, errors::TOO_MANY_LEASES)
        }
        // An admission refusal is the vault exercising authority the caller
        // does not have, so it is a permission denial rather than an execution
        // fault — the caller should not read it as "retry".
        VaultRuntimeError::DeliveryModeNotPermitted => ToolCallError::new(
            ToolErrorKind::ToolPermissionDenied,
            errors::MODE_NOT_PERMITTED,
        ),
        VaultRuntimeError::AgentNotPermitted => ToolCallError::new(
            ToolErrorKind::ToolPermissionDenied,
            errors::AGENT_NOT_PERMITTED,
        ),
        VaultRuntimeError::InvalidConsumerAgentId => validation_error(errors::CONSUMER_INVALID),
        VaultRuntimeError::InvalidVaultRef => execution_error(errors::UNEXPECTED_REFERENCE),
        VaultRuntimeError::DirectDelivery(error) => delivery_error(error),
        VaultRuntimeError::Lease(LeaseError::InvalidParameter(reason)) => validation_error(reason),
        VaultRuntimeError::Lease(_) => execution_error(errors::LEASE_UNAVAILABLE),
    }
}

fn delivery_error(error: VaultDirectDeliveryError) -> ToolCallError {
    match error {
        VaultDirectDeliveryError::ConsumerNotFound => {
            execution_error(errors::DELIVERY_CONSUMER_NOT_FOUND)
        }
        // A refusal is the adapter exercising authority the caller does not
        // have, so it is a permission denial rather than an execution fault.
        VaultDirectDeliveryError::Rejected => ToolCallError::new(
            ToolErrorKind::ToolPermissionDenied,
            errors::DELIVERY_REJECTED,
        ),
        VaultDirectDeliveryError::Unavailable => execution_error(errors::DELIVERY_UNAVAILABLE),
    }
}
