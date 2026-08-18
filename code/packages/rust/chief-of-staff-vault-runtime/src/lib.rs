//! Host-side broker for opaque Chief of Staff secret leases and direct delivery.
//!
//! Model-facing tools receive only a [`VaultLeaseReceipt`] containing a
//! bearer-capability [`VaultRef`] and its authoritative expiry. They never
//! receive plaintext, ciphertext, or an unwrap key. Raw payload bytes remain
//! in zeroizing storage and can only be atomically consumed by a trusted host
//! handler or moved into a replaceable [`VaultDirectDelivery`] boundary.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::sync::Mutex;

use coding_adventures_vault_leases::{
    InMemoryLeaseManager, LeaseError, LeaseId, LeaseManager, LeasePayload,
};
use smart_home_core::VaultRef;

const VAULT_REF_PREFIX: &str = "vault-lease:";
const MAX_CONSUMER_AGENT_ID_BYTES: usize = 4 * 1024;

/// Failure reported by a trusted direct-delivery adapter.
///
/// Variants intentionally carry no free-form diagnostic text so adapters
/// cannot accidentally turn an error or log path into a secret channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultDirectDeliveryError {
    /// No trusted consumer is registered for the requested identifier.
    ConsumerNotFound,
    /// The trusted consumer refused the delivery.
    Rejected,
    /// The trusted transport is temporarily unavailable.
    Unavailable,
}

impl fmt::Display for VaultDirectDeliveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConsumerNotFound => f.write_str("direct-delivery consumer not found"),
            Self::Rejected => f.write_str("direct delivery rejected"),
            Self::Unavailable => f.write_str("direct-delivery transport unavailable"),
        }
    }
}

impl std::error::Error for VaultDirectDeliveryError {}

/// How a secret is permitted to leave the vault (VLT06 P1).
///
/// The distinction is not a preference. Direct delivery exists so that
/// plaintext never reaches the requesting agent — it is the mode for a bank
/// password. A lease hands back a redeemable reference. So allowing a
/// direct-only secret to be *leased* does not weaken the protection, it
/// inverts it: the caller obtains exactly the material direct mode exists to
/// withhold, simply by asking a different way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultDeliveryMode {
    /// Only direct delivery to a trusted consumer. Leases are refused.
    Direct,
    /// Only leased delivery. Direct delivery is refused.
    Leased,
    /// Either mode is admissible.
    Both,
}

impl VaultDeliveryMode {
    fn admits_lease(self) -> bool {
        matches!(self, Self::Leased | Self::Both)
    }

    fn admits_direct(self) -> bool {
        matches!(self, Self::Direct | Self::Both)
    }
}

/// Which agents may request a secret (VLT06 P2, P3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AllowedAgents {
    /// No agent constraint. Every caller is admissible.
    Any,
    /// Only these attested agent identities are admissible.
    Only(BTreeSet<String>),
}

impl AllowedAgents {
    /// Build an `Only` set from any iterator of identities.
    pub fn only<I, S>(identities: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Only(identities.into_iter().map(Into::into).collect())
    }

    /// Decide admissibility for a possibly-absent requesting identity.
    ///
    /// **An absent identity is refused under `Only`, never treated as
    /// unconstrained** (VLT06 P3). This is the rule that is easiest to get
    /// wrong, because the natural way to write the comparison — match the
    /// request's identity against the policy's — succeeds vacuously when both
    /// sides are absent, and a check that passes when it had nothing to check
    /// is worse than no check at all, since it reads as enforcement.
    ///
    /// The hazard is live, not theoretical: in the D18 tool stack only
    /// `agent_id` is host-attested, while `user_id` and `session_id` are
    /// unconditionally `None` outside tests. A rule written over one of those
    /// would compare absent to absent and admit everyone.
    fn admits(&self, requesting_agent_id: Option<&str>) -> bool {
        match self {
            Self::Any => true,
            Self::Only(allowed) => {
                requesting_agent_id.is_some_and(|identity| allowed.contains(identity))
            }
        }
    }
}

/// The admission policy a secret carries (VLT06, "Per-secret admission policy").
///
/// There is deliberately no `Default`. A permissive default would make a secret
/// unguarded by omission and say nothing when it happened; a restrictive
/// default would be discovered only when a legitimate caller was refused.
/// Neither is safe-by-default, so [`ChiefVaultRuntime::register_secret`]
/// requires the policy at the call site (VLT06 P5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretPolicy {
    /// Minimum approval tier, 0–3.
    ///
    /// **Nothing reads this yet.** It is recorded so the value has somewhere to
    /// live, and named here so the gap is visible rather than implied — a `u8`
    /// called `privilege_tier` on a policy struct reads as a control, and this
    /// one enforces nothing. Enforcing it needs the caller tier threaded down
    /// from the tool boundary, which no path does today.
    pub privilege_tier: u8,
    /// Which agents may request this secret.
    pub allowed_agents: AllowedAgents,
    /// Which delivery modes are admissible.
    pub allowed_mode: VaultDeliveryMode,
    /// When the secret was last changed, in milliseconds since Unix epoch.
    pub rotated_at_ms: u64,
}

impl SecretPolicy {
    /// The most permissive policy: any agent, either mode, tier 0.
    ///
    /// Named rather than derived so that "this secret is unguarded" is a thing
    /// someone had to type, and greppable when it turns out to be wrong.
    pub fn unrestricted(rotated_at_ms: u64) -> Self {
        Self {
            privilege_tier: 0,
            allowed_agents: AllowedAgents::Any,
            allowed_mode: VaultDeliveryMode::Both,
            rotated_at_ms,
        }
    }
}

/// Who is asking for a lease, and for what.
///
/// Mirrors [`VaultDirectRequest`]. The lease path needs the requester for the
/// same reason the direct path does — without it the vault cannot apply
/// [`AllowedAgents`] — and it is the weaker of the two paths to leave
/// unguarded, because a lease hands back a redeemable capability with no
/// trusted adapter anywhere in the loop.
///
/// `requesting_agent_id` is trustworthy only if the host attests it; see the
/// note on [`VaultDirectRequest`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultLeaseRequest<'a> {
    /// Identity of the agent that invoked the tool, if the host attested one.
    pub requesting_agent_id: Option<&'a str>,
    /// Name of the secret to lease.
    pub secret_name: &'a str,
    /// Requested lease lifetime in milliseconds.
    pub ttl_ms: u64,
}

/// Who asked for a direct delivery, what they asked for, and where it goes.
///
/// This descriptor exists because a delivery adapter that is told only the
/// destination cannot actually authorize anything. Given `(consumer, payload)`
/// alone, the strongest rule an adapter can express is a global destination
/// allowlist — and under that rule a caller permitted to send *one* secret to a
/// consumer is equally permitted to send *every* secret to it, because no
/// component in the chain can tell the two requests apart. That is a confused
/// deputy: the adapter holds the authority but not the facts.
///
/// Naming the requester and the secret does not by itself authorize anything.
/// It is the precondition for an adapter that wants to.
///
/// **All three identity fields are only as trustworthy as whatever populated
/// them** — `requesting_agent_id`, `requesting_user_id`, and `session_id`
/// alike. They are read from the tool invocation request. If a host lets a
/// caller assert its own identity, an adapter that authorizes on any of them is
/// authorizing on the attacker's own claim. Establish that your host attests a
/// field before relying on it; see D18D section 7.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultDirectRequest<'a> {
    /// Identity of the agent that invoked the tool. Unattested unless the host
    /// says otherwise — see the type-level note.
    pub requesting_agent_id: Option<&'a str>,
    /// User on whose behalf the request was made. Unattested unless the host
    /// says otherwise — see the type-level note.
    ///
    /// Carried separately from the agent because one vault instance may serve
    /// several users; without it an adapter cannot tell *whose* session asked
    /// for a delivery, only which agent process did.
    pub requesting_user_id: Option<&'a str>,
    /// Session the request arrived on. Unattested unless the host says
    /// otherwise — see the type-level note.
    pub session_id: Option<&'a str>,
    /// Name of the secret being moved.
    pub secret_name: &'a str,
    /// Already-authorized consumer the payload is destined for.
    pub consumer_agent_id: &'a str,
}

/// Replaceable trusted boundary that accepts direct secret deliveries.
///
/// Implementations may route the owned, zeroizing payload over an authenticated
/// browser, agent, or host channel. They must not return the bytes to the
/// requesting agent or include them in diagnostics.
pub trait VaultDirectDelivery: Send + Sync {
    /// Deliver one owned payload, given the full request context.
    ///
    /// The adapter is the component entitled to accept or refuse. Refusing is a
    /// first-class outcome, not an error condition — see
    /// [`VaultDirectDeliveryError::Rejected`].
    fn deliver(
        &self,
        request: VaultDirectRequest<'_>,
        payload: LeasePayload,
    ) -> Result<(), VaultDirectDeliveryError>;
}

/// A newly issued lease receipt safe to return across the tool boundary.
#[derive(Clone, PartialEq, Eq)]
pub struct VaultLeaseReceipt {
    /// Opaque handle understood only by the trusted host broker.
    pub vault_ref: VaultRef,
    /// Authoritative lease expiry in milliseconds since Unix epoch.
    pub expires_at_ms: u64,
}

impl fmt::Debug for VaultLeaseReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultLeaseReceipt")
            .field("vault_ref", &"<redacted>")
            .field("expires_at_ms", &self.expires_at_ms)
            .finish()
    }
}

/// Errors produced by the Chief vault lease boundary.
#[derive(Debug)]
pub enum VaultRuntimeError {
    /// The requested secret name is not registered with this broker.
    SecretNotFound,
    /// This secret already has as many tracked leases as the vault is willing
    /// to revoke on rotation. Failing closed is deliberate: an unrevocable
    /// capability is worse than a refused request.
    TooManyOutstandingLeases,
    /// The secret exists but forbids the requested delivery mode (VLT06 P1).
    DeliveryModeNotPermitted,
    /// The secret exists but the requesting agent is not on its allow-list,
    /// or no attested identity accompanied the request (VLT06 P2, P3).
    AgentNotPermitted,
    /// The consumer identifier is empty or exceeds the protocol bound.
    InvalidConsumerAgentId,
    /// The supplied reference was not minted by this broker format.
    InvalidVaultRef,
    /// The trusted direct-delivery adapter rejected the operation.
    DirectDelivery(VaultDirectDeliveryError),
    /// The underlying lease manager rejected the operation.
    Lease(LeaseError),
}

impl fmt::Display for VaultRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SecretNotFound => f.write_str("secret not found"),
            Self::TooManyOutstandingLeases => {
                f.write_str("too many outstanding leases for this secret")
            }
            Self::DeliveryModeNotPermitted => {
                f.write_str("secret does not permit the requested delivery mode")
            }
            Self::AgentNotPermitted => f.write_str("agent is not permitted to request this secret"),
            Self::InvalidConsumerAgentId => f.write_str("invalid consumer agent identifier"),
            Self::InvalidVaultRef => f.write_str("invalid VaultRef"),
            Self::DirectDelivery(error) => write!(f, "{error}"),
            Self::Lease(error) => write!(f, "lease operation failed: {error}"),
        }
    }
}

impl std::error::Error for VaultRuntimeError {}

impl From<LeaseError> for VaultRuntimeError {
    fn from(value: LeaseError) -> Self {
        Self::Lease(value)
    }
}

impl From<VaultDirectDeliveryError> for VaultRuntimeError {
    fn from(value: VaultDirectDeliveryError) -> Self {
        Self::DirectDelivery(value)
    }
}

/// Decide whether a request is admissible, given only the policy.
///
/// Deliberately a free function over `&SecretPolicy` rather than a method that
/// can reach a payload: it *cannot* leak a secret, because it is never handed
/// one. That is what makes VLT06 P4 checkable — the ordering "decide, then
/// clone" is enforced by which function has access to what, not by the order of
/// two statements inside one body.
///
/// Agent admissibility is checked **before** delivery mode. Both orders refuse
/// the same requests, but the reverse order tells a caller who is not permitted
/// at all what the secret's delivery mode is, which is a fact about the policy
/// they have no business learning. The cheaper denial is the one that reveals
/// less.
fn check_admission(
    policy: &SecretPolicy,
    requesting_agent_id: Option<&str>,
    wants_lease: bool,
) -> Result<Admitted, VaultRuntimeError> {
    if !policy.allowed_agents.admits(requesting_agent_id) {
        return Err(VaultRuntimeError::AgentNotPermitted);
    }
    let admitted = if wants_lease {
        policy.allowed_mode.admits_lease()
    } else {
        policy.allowed_mode.admits_direct()
    };
    if !admitted {
        return Err(VaultRuntimeError::DeliveryModeNotPermitted);
    }
    Ok(Admitted(()))
}

/// Proof that [`check_admission`] ran and admitted the request.
///
/// Its field is private, so no *other crate* can forge one. Within this crate
/// it is decorative, and saying otherwise would be the same over-claim this
/// type exists to prevent: the wall that actually matters is `custody`'s
/// module privacy on the payload field, and `StoredSecret::admit` performs the
/// check itself rather than trusting a token a caller supplies.
#[must_use]
pub struct Admitted(());

/// The payload and its policy, behind a wall the rest of this file cannot climb.
///
/// This is a module rather than a plain struct because in Rust a private field
/// is visible to the whole *module*, and this file is one module. Two rounds of
/// review found the same defect: the "decide, then clone" ordering was plain
/// statement order inside one function, so moving the clone above the checks
/// compiled and left the entire suite green. Rewriting the comment to insist
/// harder would not have helped; a reviewer had already read it.
///
/// Here the field is unreachable from outside `custody`, and the only accessor
/// demands an [`Admitted`] that only [`check_admission`] can produce. Cloning
/// the payload before deciding is now a compile error rather than a review
/// finding — which is the difference between an invariant and a wish.
mod custody {
    use super::{check_admission, Admitted, SecretPolicy, VaultRuntimeError};
    use coding_adventures_vault_leases::LeasePayload;

    /// One registered secret: the bytes, and the policy governing who gets them.
    pub struct StoredSecret {
        payload: LeasePayload,
        policy: SecretPolicy,
    }

    impl StoredSecret {
        pub fn new(payload: LeasePayload, policy: SecretPolicy) -> Self {
            Self { payload, policy }
        }

        /// The policy is freely readable — it is not the secret.
        pub fn policy(&self) -> &SecretPolicy {
            &self.policy
        }

        /// Decide, and hand back the payload only on an affirmative answer.
        ///
        /// The single door to the bytes. It performs the check itself rather
        /// than taking a token from the caller, so there is no ordering left
        /// for a caller to get wrong.
        pub fn admit(
            &self,
            requesting_agent_id: Option<&str>,
            wants_lease: bool,
        ) -> Result<LeasePayload, VaultRuntimeError> {
            let proof = check_admission(&self.policy, requesting_agent_id, wants_lease)?;
            Ok(self.payload_with(&proof))
        }

        /// Clone the payload, given proof of admission.
        fn payload_with(&self, _proof: &Admitted) -> LeasePayload {
            self.payload.clone()
        }
    }
}

use custody::StoredSecret;

/// In-process vault actor boundary used by Chief host runtimes.
pub struct ChiefVaultRuntime {
    secrets: Mutex<HashMap<String, StoredSecret>>,
    leases: InMemoryLeaseManager,
    /// Lease ids issued per secret name, so rotation can revoke them.
    ///
    /// The lease manager is keyed by lease id and holds no back-reference to
    /// the secret a lease came from, so without this index there is no way to
    /// answer "which capabilities are outstanding over *this* secret" — and
    /// therefore no way to make rotation revoke them.
    ///
    /// **Lock order is `secrets` → `issued` → `leases`** for every path that
    /// holds more than one at a time. `consume` and `revoke` touch `leases` and
    /// then `issued`, and must not acquire `issued` while a lease-table guard is
    /// live. (Holding `issued` *across* `leases.consume` would in fact be the
    /// same `issued` → `leases` order `request_lease` already uses, so it would
    /// not deadlock — it is simply unnecessary, because a stale id left in the
    /// index is harmless: rotation's `revoke` of an already-dead lease is a
    /// no-op.) `request_lease` holds
    /// `secrets` across the whole mint, which is what stops a rotation from
    /// slipping between the admission decision and the lease being indexed;
    /// see [`ChiefVaultRuntime::request_lease`].
    issued: Mutex<IssuedIndex>,
}

/// Largest number of tracked leases a single secret may accumulate.
///
/// The lease table below has its own cap. This index is a *second* table over
/// the same agent-driven path, so leaving it unbounded would reintroduce
/// exactly the exhaustion the lower layer was hardened against — and the
/// entries here are worse than wasted memory, because each one is a capability
/// that rotation is supposed to be able to revoke.
///
/// Failing closed is right when the bound is reached: an unrevocable capability
/// is worse than a refused request.
const MAX_TRACKED_LEASES_PER_SECRET: usize = 1024;

/// Which leases are outstanding over which secret.
///
/// Two maps rather than one, so a redemption can prune in constant time. With
/// only `by_secret`, `consume` would have to scan every secret's set to find
/// the id it just spent, and the natural response to that cost is to skip
/// pruning — which is how the index became unbounded in the first place.
#[derive(Default)]
struct IssuedIndex {
    by_secret: HashMap<String, HashSet<LeaseId>>,
    by_lease: HashMap<LeaseId, String>,
}

impl IssuedIndex {
    fn record(&mut self, secret_name: &str, lease_id: LeaseId) {
        self.by_lease
            .insert(lease_id.clone(), secret_name.to_string());
        self.by_secret
            .entry(secret_name.to_string())
            .or_default()
            .insert(lease_id);
    }

    /// Forget one lease, whichever secret it belonged to.
    ///
    /// Called on redemption and revocation: both make the capability dead, and
    /// a dead capability is not something rotation needs to revoke.
    fn forget(&mut self, lease_id: &LeaseId) {
        if let Some(secret_name) = self.by_lease.remove(lease_id) {
            if let Some(set) = self.by_secret.get_mut(&secret_name) {
                set.remove(lease_id);
                if set.is_empty() {
                    self.by_secret.remove(&secret_name);
                }
            }
        }
    }

    /// Drop tracked ids for leases that are no longer usable.
    ///
    /// Covers the case neither `consume` nor `revoke` can: a lease that simply
    /// expired. Nothing calls back into this runtime when a TTL elapses, so
    /// without this sweep an index over short-lived leases fills up and stays
    /// full, with no attacker involved.
    ///
    /// The predicate is *usability*, not presence, and the difference is the
    /// whole bug this replaced. `LeaseManager::lookup` deliberately returns
    /// `Ok` for expired and revoked leases -- it reports status rather than
    /// withholding it -- so an `is_err()` test reclaimed nothing. And because
    /// the per-secret cap here refuses before `issue` is ever reached, the
    /// lease layer's own reaper never ran either. The result was that 1024
    /// one-millisecond leases wedged a secret permanently: every later request
    /// refused, with no way back except operator rotation.
    fn sweep(&mut self, secret_name: &str, leases: &InMemoryLeaseManager, now_ms: u64) {
        let Some(set) = self.by_secret.get_mut(secret_name) else {
            return;
        };
        let dead: Vec<LeaseId> = set
            .iter()
            .filter(|id| match leases.lookup(id) {
                // Still in the table: keep it only while it is actually usable.
                Ok(info) => !info.is_active_at(now_ms),
                // Already reaped out of the table.
                Err(_) => true,
            })
            .cloned()
            .collect();
        for id in dead {
            set.remove(&id);
            self.by_lease.remove(&id);
        }
        if set.is_empty() {
            self.by_secret.remove(secret_name);
        }
    }

    fn take(&mut self, secret_name: &str) -> Vec<LeaseId> {
        let ids: Vec<LeaseId> = self
            .by_secret
            .remove(secret_name)
            .map(|set| set.into_iter().collect())
            .unwrap_or_default();
        for id in &ids {
            self.by_lease.remove(id);
        }
        ids
    }

    fn count(&self, secret_name: &str) -> usize {
        self.by_secret.get(secret_name).map_or(0, HashSet::len)
    }
}

impl Default for ChiefVaultRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ChiefVaultRuntime {
    /// Construct an empty vault runtime.
    pub fn new() -> Self {
        Self {
            secrets: Mutex::new(HashMap::new()),
            leases: InMemoryLeaseManager::new(),
            issued: Mutex::new(IssuedIndex::default()),
        }
    }

    /// Register or rotate a named secret while retaining it in zeroizing memory.
    ///
    /// The policy is required rather than defaulted (VLT06 P5): a permissive
    /// default would leave a secret unguarded silently, and a restrictive one
    /// would surface only as a refused legitimate caller. Use
    /// [`SecretPolicy::unrestricted`] when a secret genuinely has no
    /// constraints, so that fact is written down and greppable.
    /// Rotating or re-registering a secret **revokes every outstanding lease
    /// over the previous value**, which is the only behaviour that makes
    /// rotation mean anything.
    ///
    /// A lease holds its own copy of the payload, taken when it was issued.
    /// Overwriting the map entry alone would leave that copy redeemable — so a
    /// secret rotated *because it was compromised* would keep handing out the
    /// compromised value for as long as the lease lived, which the lease layer
    /// allows to be ninety days. Tightening a policy has the same shape: new
    /// requests get refused while an already-minted reference sails past.
    ///
    /// Revocation is best-effort by design: a lease already consumed or expired
    /// is simply gone, and its error is discarded. What matters is that nothing
    /// *live* survives the rotation.
    pub fn register_secret(
        &self,
        name: impl Into<String>,
        payload: LeasePayload,
        policy: SecretPolicy,
    ) {
        let name = name.into();

        // `secrets` is taken FIRST and held across the whole rotation. That is
        // the part that matters: `request_lease` also holds it from the
        // admission decision through to indexing the new lease, so a mint
        // cannot slip in between this drain and this insert. Revoking without
        // that exclusion left a real race -- a lease admitted against the old
        // value, issued after the drain, and so never revoked.
        let mut secrets = self.secrets.lock().expect("vault secret mutex poisoned");
        let stale = self
            .issued
            .lock()
            .expect("vault issued-lease mutex poisoned")
            .take(&name);
        for lease_id in stale {
            let _ = self.leases.revoke(&lease_id);
        }
        secrets.insert(name, StoredSecret::new(payload, policy));
    }

    /// Count the leases this runtime still tracks for a secret.
    ///
    /// Tracking is pruned on rotation, on redemption, and on revocation, and
    /// ids whose leases are no longer usable are swept when the same secret is
    /// next leased. So this counts the references rotation would still have to
    /// revoke -- close to, but not exactly, the number of live leases, since an
    /// expired id stays counted until the next sweep touches it.
    pub fn tracked_lease_count(&self, secret_name: &str) -> usize {
        self.issued
            .lock()
            .expect("vault issued-lease mutex poisoned")
            .count(secret_name)
    }

    /// Read the policy recorded for a secret, without touching the payload.
    pub fn secret_policy(&self, secret_name: &str) -> Option<SecretPolicy> {
        self.secrets
            .lock()
            .expect("vault secret mutex poisoned")
            .get(secret_name)
            .map(|stored| stored.policy().clone())
    }

    /// Issue a short-lived opaque reference for a named secret.
    ///
    /// The receipt is the canonical agent-facing `{ vault_ref,
    /// expires_at_ms }` shape. Secret bytes and decryption material never cross
    /// this boundary.
    pub fn request_lease(
        &self,
        request: VaultLeaseRequest<'_>,
    ) -> Result<VaultLeaseReceipt, VaultRuntimeError> {
        // Hold `secrets` from the admission decision through to indexing the
        // lease. Releasing it earlier -- as an earlier version did -- lets a
        // concurrent rotation drain the index between the two, leaving a live
        // capability over the pre-rotation bytes that nothing can revoke.
        // Lock order is secrets -> issued -> leases throughout.
        let secrets = self.secrets.lock().expect("vault secret mutex poisoned");
        let stored = secrets
            .get(request.secret_name)
            .ok_or(VaultRuntimeError::SecretNotFound)?;
        let mut issued = self
            .issued
            .lock()
            .expect("vault issued-lease mutex poisoned");

        // Reclaim dead ids before testing the bound, so expiry alone can never
        // exhaust a secret's budget. This has to run before the capacity check,
        // not after: the check is what refuses, and refusing on the strength of
        // leases that expired long ago is the wedge described on `sweep`.
        issued.sweep(request.secret_name, &self.leases, now_ms());
        if issued.count(request.secret_name) >= MAX_TRACKED_LEASES_PER_SECRET {
            return Err(VaultRuntimeError::TooManyOutstandingLeases);
        }

        // Admission last, so the payload is cloned only once every refusal is
        // behind us. VLT06 P4 is about the admission checks specifically, but
        // the capacity refusal is a refusal too, and there is no reason to
        // materialize a secret for a request that is about to be turned away.
        let payload = stored.admit(request.requesting_agent_id, true)?;

        let lease_id = self.leases.issue(payload, request.ttl_ms)?;
        let info = self.leases.lookup(&lease_id)?;
        issued.record(request.secret_name, lease_id.clone());

        Ok(VaultLeaseReceipt {
            vault_ref: VaultRef::trusted(format!("{VAULT_REF_PREFIX}{}", lease_id.as_hex())),
            expires_at_ms: info.expires_at_ms,
        })
    }

    /// Deliver a named secret to a trusted consumer without returning it.
    ///
    /// The registered secret is cloned directly into zeroizing payload storage
    /// and ownership is transferred to `delivery`. The caller receives only
    /// success or a bounded, secret-free error.
    pub fn request_direct(
        &self,
        request: VaultDirectRequest<'_>,
        delivery: &dyn VaultDirectDelivery,
    ) -> Result<(), VaultRuntimeError> {
        if request.consumer_agent_id.is_empty()
            || request.consumer_agent_id.len() > MAX_CONSUMER_AGENT_ID_BYTES
        {
            return Err(VaultRuntimeError::InvalidConsumerAgentId);
        }

        let payload = {
            let secrets = self.secrets.lock().expect("vault secret mutex poisoned");
            let stored = secrets
                .get(request.secret_name)
                .ok_or(VaultRuntimeError::SecretNotFound)?;
            stored.admit(request.requesting_agent_id, false)?
            // The guard drops here. The adapter is replaceable and arbitrary,
            // so it must not run while a vault lock is held.
        };
        delivery.deliver(request, payload)?;
        Ok(())
    }

    /// Atomically resolve and revoke a lease inside a trusted host handler.
    ///
    /// Agent and model code must not call this boundary directly. Successful
    /// resolution returns zeroizing payload storage and makes the reference
    /// unusable for every subsequent request.
    pub fn consume(&self, vault_ref: &VaultRef) -> Result<LeasePayload, VaultRuntimeError> {
        let lease_id = lease_id(vault_ref)?;
        let payload = self.leases.consume(&lease_id)?;
        // A redeemed lease is dead, so rotation no longer needs to revoke it.
        // Pruning here is what keeps the index bounded in ordinary use.
        self.issued
            .lock()
            .expect("vault issued-lease mutex poisoned")
            .forget(&lease_id);
        Ok(payload)
    }

    /// Revoke an outstanding lease before its TTL elapses.
    pub fn revoke(&self, vault_ref: &VaultRef) -> Result<(), VaultRuntimeError> {
        let lease_id = lease_id(vault_ref)?;
        self.leases.revoke(&lease_id)?;
        self.issued
            .lock()
            .expect("vault issued-lease mutex poisoned")
            .forget(&lease_id);
        Ok(())
    }
}

/// Wall-clock milliseconds, for deciding whether a tracked lease is still live.
///
/// Falls back to 0 if the clock is before the epoch. That is not "sweep
/// everything" — at `now_ms == 0` a freshly issued lease still reads as active,
/// so the sweep reclaims nothing. The reason the fallback is nonetheless right
/// is consistency: `InMemoryLeaseManager::now_ms` uses the identical
/// `unwrap_or(0)`, so under a pre-epoch clock both layers agree that leases are
/// live, and the index never disagrees with the table it indexes.
///
/// Which direction is "safe" also runs the other way from the obvious guess.
/// Sweeping too early drops a *live* id, and rotation-revokes (VLT06 P6) is
/// exactly what depends on those ids being present; keeping one too long only
/// costs a slot.
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

fn lease_id(vault_ref: &VaultRef) -> Result<LeaseId, VaultRuntimeError> {
    vault_ref
        .as_str()
        .strip_prefix(VAULT_REF_PREFIX)
        .and_then(LeaseId::from_hex)
        .ok_or(VaultRuntimeError::InvalidVaultRef)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"chief-vault-runtime-secret";

    #[derive(Default)]
    struct RecordingDelivery {
        deliveries: Mutex<Vec<(String, Vec<u8>)>>,
        result: Mutex<Option<VaultDirectDeliveryError>>,
    }

    impl RecordingDelivery {
        fn rejecting(error: VaultDirectDeliveryError) -> Self {
            Self {
                deliveries: Mutex::new(Vec::new()),
                result: Mutex::new(Some(error)),
            }
        }
    }

    impl VaultDirectDelivery for RecordingDelivery {
        fn deliver(
            &self,
            request: VaultDirectRequest<'_>,
            payload: LeasePayload,
        ) -> Result<(), VaultDirectDeliveryError> {
            self.deliveries.lock().unwrap().push((
                request.consumer_agent_id.to_string(),
                payload.as_bytes().to_vec(),
            ));
            match *self.result.lock().unwrap() {
                Some(error) => Err(error),
                None => Ok(()),
            }
        }
    }

    /// Build a descriptor for the common test case: an agent asking for one
    /// secret on behalf of one consumer.
    fn direct_request<'a>(
        secret_name: &'a str,
        consumer_agent_id: &'a str,
    ) -> VaultDirectRequest<'a> {
        VaultDirectRequest {
            requesting_agent_id: Some("agent:test"),
            requesting_user_id: Some("user:test"),
            session_id: Some("session:test"),
            secret_name,
            consumer_agent_id,
        }
    }

    /// A lease request from the standard test agent.
    fn lease_request(secret_name: &str, ttl_ms: u64) -> VaultLeaseRequest<'_> {
        VaultLeaseRequest {
            requesting_agent_id: Some("agent:test"),
            secret_name,
            ttl_ms,
        }
    }

    #[test]
    fn a_direct_only_secret_cannot_be_leased_instead() {
        // The inversion this whole check exists to stop: a secret configured so
        // plaintext never reaches the agent, obtained as a redeemable reference
        // by asking the other way.
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "bank-password",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy {
                privilege_tier: 3,
                allowed_agents: AllowedAgents::Any,
                allowed_mode: VaultDeliveryMode::Direct,
                rotated_at_ms: 0,
            },
        );

        assert!(matches!(
            vault.request_lease(lease_request("bank-password", 30_000)),
            Err(VaultRuntimeError::DeliveryModeNotPermitted)
        ));

        // ... while the mode it *is* configured for still works.
        let delivery = RecordingDelivery::default();
        vault
            .request_direct(direct_request("bank-password", "browser-agent"), &delivery)
            .expect("direct delivery is the permitted mode");
    }

    #[test]
    fn a_leased_only_secret_cannot_be_delivered_directly() {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "weather-api-key",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy {
                privilege_tier: 1,
                allowed_agents: AllowedAgents::Any,
                allowed_mode: VaultDeliveryMode::Leased,
                rotated_at_ms: 0,
            },
        );
        let delivery = RecordingDelivery::default();

        assert!(matches!(
            vault.request_direct(
                direct_request("weather-api-key", "browser-agent"),
                &delivery
            ),
            Err(VaultRuntimeError::DeliveryModeNotPermitted)
        ));
        assert!(
            delivery.deliveries.lock().unwrap().is_empty(),
            "a refused mode must not reach the adapter at all"
        );

        vault
            .request_lease(lease_request("weather-api-key", 30_000))
            .expect("leasing is the permitted mode");
    }

    #[test]
    fn an_agent_outside_the_allow_list_is_refused_on_both_paths() {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "finance-key",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy {
                privilege_tier: 2,
                allowed_agents: AllowedAgents::only(["agent:finance"]),
                allowed_mode: VaultDeliveryMode::Both,
                rotated_at_ms: 0,
            },
        );
        let delivery = RecordingDelivery::default();

        // `lease_request` / `direct_request` both speak as "agent:test".
        assert!(matches!(
            vault.request_lease(lease_request("finance-key", 30_000)),
            Err(VaultRuntimeError::AgentNotPermitted)
        ));
        assert!(matches!(
            vault.request_direct(direct_request("finance-key", "browser-agent"), &delivery),
            Err(VaultRuntimeError::AgentNotPermitted)
        ));
        assert!(delivery.deliveries.lock().unwrap().is_empty());

        // The named agent gets through.
        vault
            .request_lease(VaultLeaseRequest {
                requesting_agent_id: Some("agent:finance"),
                secret_name: "finance-key",
                ttl_ms: 30_000,
            })
            .expect("the allow-listed agent is admitted");
    }

    #[test]
    fn an_absent_identity_is_refused_rather_than_treated_as_unconstrained() {
        // VLT06 P3, and the reason it is a numbered rule. Writing the check the
        // natural way — compare the request's identity to the policy's — passes
        // vacuously when both are absent. In this stack `user_id` and
        // `session_id` are unconditionally None outside tests, so a rule
        // written over either would admit everyone while reading as
        // enforcement.
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "finance-key",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy {
                privilege_tier: 2,
                allowed_agents: AllowedAgents::only(["agent:finance"]),
                allowed_mode: VaultDeliveryMode::Both,
                rotated_at_ms: 0,
            },
        );
        let delivery = RecordingDelivery::default();

        assert!(matches!(
            vault.request_lease(VaultLeaseRequest {
                requesting_agent_id: None,
                secret_name: "finance-key",
                ttl_ms: 30_000,
            }),
            Err(VaultRuntimeError::AgentNotPermitted)
        ));
        assert!(matches!(
            vault.request_direct(
                VaultDirectRequest {
                    requesting_agent_id: None,
                    requesting_user_id: None,
                    session_id: None,
                    secret_name: "finance-key",
                    consumer_agent_id: "browser-agent",
                },
                &delivery
            ),
            Err(VaultRuntimeError::AgentNotPermitted)
        ));

        // `Any` still admits an anonymous caller — absence is only fatal where
        // the policy actually names who is allowed.
        vault.register_secret(
            "public-key",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy::unrestricted(0),
        );
        vault
            .request_lease(VaultLeaseRequest {
                requesting_agent_id: None,
                secret_name: "public-key",
                ttl_ms: 30_000,
            })
            .expect("Any admits a caller with no attested identity");
    }

    #[test]
    fn rotating_a_secret_revokes_every_outstanding_lease_over_the_old_value() {
        // Rotation is the compromise response. If the pre-rotation bytes stay
        // redeemable, rotating a leaked secret does not un-leak it — it just
        // adds a second value alongside the one the attacker already holds.
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "weather-api-key",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy::unrestricted(0),
        );
        let receipt = vault
            .request_lease(lease_request("weather-api-key", 600_000))
            .expect("lease should issue");

        vault.register_secret(
            "weather-api-key",
            LeasePayload::new(b"rotated-value".to_vec()),
            SecretPolicy::unrestricted(1),
        );

        assert!(
            vault.consume(&receipt.vault_ref).is_err(),
            "a lease over the pre-rotation value must not survive rotation"
        );
        assert_eq!(vault.tracked_lease_count("weather-api-key"), 0);
    }

    #[test]
    fn tightening_a_policy_revokes_references_minted_under_the_looser_one() {
        // Same mechanism, different motivation: new requests being refused is
        // worth little while an already-minted reference sails past the new
        // rule.
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "finance-key",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy::unrestricted(0),
        );
        let receipt = vault
            .request_lease(lease_request("finance-key", 600_000))
            .expect("lease should issue under the loose policy");

        vault.register_secret(
            "finance-key",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy {
                privilege_tier: 3,
                allowed_agents: AllowedAgents::only(["agent:finance"]),
                allowed_mode: VaultDeliveryMode::Direct,
                rotated_at_ms: 1,
            },
        );

        assert!(vault.consume(&receipt.vault_ref).is_err());
        assert!(matches!(
            vault.request_lease(lease_request("finance-key", 600_000)),
            Err(VaultRuntimeError::AgentNotPermitted)
        ));
    }

    #[test]
    fn rotation_leaves_other_secrets_leases_alone() {
        let vault = ChiefVaultRuntime::new();
        for name in ["first", "second"] {
            vault.register_secret(
                name,
                LeasePayload::new(SECRET.to_vec()),
                SecretPolicy::unrestricted(0),
            );
        }
        let untouched = vault
            .request_lease(lease_request("second", 600_000))
            .expect("lease should issue");

        vault.register_secret(
            "first",
            LeasePayload::new(b"rotated".to_vec()),
            SecretPolicy::unrestricted(1),
        );

        vault
            .consume(&untouched.vault_ref)
            .expect("rotating one secret must not revoke another's leases");
    }

    #[test]
    fn the_admission_decision_refuses_before_it_can_see_a_payload() {
        // VLT06 P4 as a property of visibility rather than of statement order.
        // `check_admission` takes a `&SecretPolicy` and never receives a
        // payload, and the payload itself lives behind `custody`'s module wall
        // — cloning it before deciding does not compile from out here.
        //
        // The previous version of this test asserted only that a refused
        // request never reached the delivery adapter, which is a consequence of
        // refusal and not of ordering: moving the clone above the checks left
        // it green.
        let restrictive = SecretPolicy {
            privilege_tier: 3,
            allowed_agents: AllowedAgents::only(["agent:finance"]),
            allowed_mode: VaultDeliveryMode::Direct,
            rotated_at_ms: 0,
        };

        assert!(matches!(
            check_admission(&restrictive, Some("agent:other"), false),
            Err(VaultRuntimeError::AgentNotPermitted)
        ));
        assert!(matches!(
            check_admission(&restrictive, None, false),
            Err(VaultRuntimeError::AgentNotPermitted)
        ));
        assert!(matches!(
            check_admission(&restrictive, Some("agent:finance"), true),
            Err(VaultRuntimeError::DeliveryModeNotPermitted)
        ));
        assert!(check_admission(&restrictive, Some("agent:finance"), false).is_ok());
    }

    #[test]
    fn a_non_admitted_caller_is_not_told_the_delivery_mode() {
        // Agent admissibility is checked first on purpose. Both orders refuse
        // the same requests, but checking mode first hands someone with no
        // access at all a fact about the policy — cheapest possible denial is
        // the one that reveals least.
        let policy = SecretPolicy {
            privilege_tier: 3,
            allowed_agents: AllowedAgents::only(["agent:finance"]),
            allowed_mode: VaultDeliveryMode::Direct,
            rotated_at_ms: 0,
        };

        assert!(
            matches!(
                check_admission(&policy, Some("agent:other"), true),
                Err(VaultRuntimeError::AgentNotPermitted)
            ),
            "an outsider must not learn that the mode would also have refused"
        );
    }

    #[test]
    fn an_empty_allow_list_refuses_everyone() {
        // `Only(empty)` is the natural way to express "nobody, for now". It
        // must fail closed rather than degenerate into `Any`.
        let policy = SecretPolicy {
            privilege_tier: 0,
            allowed_agents: AllowedAgents::only(Vec::<String>::new()),
            allowed_mode: VaultDeliveryMode::Both,
            rotated_at_ms: 0,
        };

        assert!(check_admission(&policy, Some("agent:anyone"), true).is_err());
        assert!(check_admission(&policy, None, true).is_err());
    }

    #[test]
    fn a_refused_request_never_reaches_the_delivery_adapter() {
        // Narrower than its old name suggested: this shows a refused request
        // does not reach the adapter, which is a consequence of refusal rather
        // than proof of clone ordering. P4 itself is pinned by
        // `the_admission_decision_refuses_before_it_can_see_a_payload`.
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "bank-password",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy {
                privilege_tier: 3,
                allowed_agents: AllowedAgents::only(["agent:vault"]),
                allowed_mode: VaultDeliveryMode::Direct,
                rotated_at_ms: 0,
            },
        );
        let delivery = RecordingDelivery::default();

        assert!(vault
            .request_direct(direct_request("bank-password", "browser-agent"), &delivery)
            .is_err());
        assert!(
            delivery.deliveries.lock().unwrap().is_empty(),
            "a refused request must not reach the delivery boundary"
        );
    }

    #[test]
    fn the_recorded_policy_is_readable_without_touching_the_payload() {
        let vault = ChiefVaultRuntime::new();
        let policy = SecretPolicy {
            privilege_tier: 2,
            allowed_agents: AllowedAgents::only(["agent:finance", "agent:audit"]),
            allowed_mode: VaultDeliveryMode::Leased,
            rotated_at_ms: 1_700_000_000_000,
        };
        vault.register_secret(
            "finance-key",
            LeasePayload::new(SECRET.to_vec()),
            policy.clone(),
        );

        assert_eq!(vault.secret_policy("finance-key"), Some(policy));
        assert_eq!(vault.secret_policy("absent"), None);
    }

    #[test]
    fn opaque_reference_resolves_once_inside_host_boundary() {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "weather-api-key",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy::unrestricted(0),
        );

        let receipt = vault
            .request_lease(lease_request("weather-api-key", 30_000))
            .expect("lease should be issued");
        assert!(receipt.vault_ref.as_str().starts_with(VAULT_REF_PREFIX));
        assert!(!receipt.vault_ref.as_str().contains("runtime-secret"));

        let payload = vault
            .consume(&receipt.vault_ref)
            .expect("trusted host should consume lease");
        assert_eq!(payload.as_bytes(), SECRET);
        assert!(vault.consume(&receipt.vault_ref).is_err());
    }

    #[test]
    fn lease_receipt_debug_redacts_bearer_capability() {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "weather-api-key",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy::unrestricted(0),
        );

        let receipt = vault
            .request_lease(lease_request("weather-api-key", 30_000))
            .expect("lease should be issued");
        let debug = format!("{receipt:?}");

        assert!(debug.contains("vault_ref: \"<redacted>\""));
        assert!(debug.contains("expires_at_ms"));
        assert!(!debug.contains(receipt.vault_ref.as_str()));
    }

    #[test]
    fn revoked_and_unknown_references_fail_closed() {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "weather-api-key",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy::unrestricted(0),
        );
        let receipt = vault
            .request_lease(lease_request("weather-api-key", 30_000))
            .expect("lease should be issued");
        vault
            .revoke(&receipt.vault_ref)
            .expect("revoke should work");

        assert!(vault.consume(&receipt.vault_ref).is_err());
        assert!(vault
            .consume(&VaultRef::trusted("raw-secret-or-random-handle"))
            .is_err());
        assert!(vault
            .request_lease(lease_request("missing", 30_000))
            .is_err());
    }

    #[test]
    fn direct_delivery_moves_secret_only_into_trusted_adapter() {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "browser-session",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy::unrestricted(0),
        );
        let delivery = RecordingDelivery::default();

        vault
            .request_direct(
                direct_request("browser-session", "browser-agent"),
                &delivery,
            )
            .expect("trusted delivery should succeed");

        let deliveries = delivery.deliveries.lock().unwrap();
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].0, "browser-agent");
        assert_eq!(deliveries[0].1, SECRET);
    }

    #[test]
    fn direct_delivery_fails_closed_before_or_at_adapter_boundary() {
        let vault = ChiefVaultRuntime::new();
        vault.register_secret(
            "browser-session",
            LeasePayload::new(SECRET.to_vec()),
            SecretPolicy::unrestricted(0),
        );
        let delivery = RecordingDelivery::default();

        assert!(matches!(
            vault.request_direct(direct_request("browser-session", ""), &delivery),
            Err(VaultRuntimeError::InvalidConsumerAgentId)
        ));
        assert!(matches!(
            vault.request_direct(direct_request("missing", "browser-agent"), &delivery),
            Err(VaultRuntimeError::SecretNotFound)
        ));
        assert!(delivery.deliveries.lock().unwrap().is_empty());

        let rejecting = RecordingDelivery::rejecting(VaultDirectDeliveryError::Rejected);
        let error = vault
            .request_direct(
                direct_request("browser-session", "browser-agent"),
                &rejecting,
            )
            .expect_err("adapter rejection must reach the host");
        assert!(matches!(
            error,
            VaultRuntimeError::DirectDelivery(VaultDirectDeliveryError::Rejected)
        ));
        assert!(!format!("{error:?}").contains("runtime-secret"));
        assert!(!error.to_string().contains("runtime-secret"));
    }
}
