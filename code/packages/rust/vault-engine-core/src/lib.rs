//! # `coding_adventures_vault_engine_core` — VLT08 trait
//!
//! ## What this crate is
//!
//! The **`SecretEngine` trait** plus the small vocabulary every
//! engine speaks (`Role`, `MintContext`, `MintedSecret`,
//! `SecretRef`, `EngineError`). Every concrete engine in the Vault
//! stack — KV-v2, Database, PKI, AWS, GCP, Azure, SSH, Transit,
//! TOTP, Kubernetes — lives in its own sibling crate and
//! implements this one trait, so:
//!
//!   * the **transport layer** (VLT11) routes mount paths to
//!     engines without knowing their type,
//!   * the **policy engine** (VLT06) authorises calls to
//!     engine-mediated paths uniformly,
//!   * the **lease manager** (VLT07) wraps every minted secret in
//!     a TTL'd, revocable envelope with no special-case code per
//!     engine,
//!   * the **audit log** (VLT09) records mint / revoke / rotate as
//!     three structured events instead of N-engine-specific
//!     events.
//!
//! ## Why a separate crate
//!
//! Concrete engines pull in heavy, distinct dependency trees
//! (database client SDKs for `vault-engine-database`, AWS SDK for
//! `vault-engine-aws`, an X.509 codec for `vault-engine-pki`).
//! Putting the trait in its own dependency-light crate means
//! every consumer (transports, policy, audit) imports only the
//! trait, not the union of every engine's deps.
//!
//! It also means a workspace can compile against a *subset* of
//! engines (e.g. an embedded password manager that only needs
//! KV-v2 + TOTP) without dragging in the AWS SDK.
//!
//! ## Where it fits
//!
//! ```text
//!          ┌────────────────────────────────────────┐
//!          │  VLT11 transports (CLI / HTTP / gRPC)  │
//!          └──────────────────┬─────────────────────┘
//!                             │ dispatch on mount_path
//!          ┌──────────────────▼─────────────────────┐
//!          │  Box<dyn SecretEngine>     ◄── HERE    │
//!          │   ├─ KvV2Engine   (VLT08-KV2)          │
//!          │   ├─ DatabaseEngine (future)           │
//!          │   ├─ PkiEngine     (future)            │
//!          │   ├─ AwsEngine     (future)            │
//!          │   └─ TransitEngine (future)            │
//!          └──────────────────┬─────────────────────┘
//!                             │ MintedSecret
//!          ┌──────────────────▼─────────────────────┐
//!          │  VLT07 LeaseManager                    │
//!          │  wraps the bytes in a TTL'd lease,     │
//!          │  hands the LeaseId back to the caller  │
//!          └────────────────────────────────────────┘
//! ```
//!
//! ## Threat model (engine-tier)
//!
//! * **Backend never sees plaintext**: each engine is responsible
//!   for *not* persisting its minted bytes outside a sealed-store
//!   path. Storage-backed engines route through VLT01.
//! * **Replay**: minted secrets are addressed by `SecretRef` (an
//!   opaque identifier) rather than by content; the audit log
//!   records the mint event so duplicate mints under the same
//!   role are visible.
//! * **Caller error**: the trait's `MintContext` carries the
//!   policy decision plus the auth principal so engines can
//!   defence-in-depth check (e.g. AWS engine refusing to mint
//!   for a principal not on the allow-list, even if the policy
//!   already said yes).
//!
//! ## What this crate does *not* do
//!
//! * Not a registry: routing mount paths to engines is VLT11's
//!   job. We give the engine a `mount_path()` method and stop.
//! * Not a policy decision: by the time an engine's `mint` is
//!   called, VLT06 has already approved the call.
//! * Not a lease implementer: `MintedSecret` carries raw bytes;
//!   the caller passes them to the lease manager.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_zeroize::Zeroizing;

// === Section 1. Vocabulary types ============================================
//
// These are what every engine consumes/produces. Keeping them
// here (instead of redefining per-engine) is the entire point of
// the trait crate: a downstream tool that aggregates audit events
// from many engines can pattern-match on a single set of types.

/// Identifies a *role* — a named bundle of constraints attached
/// to a mount. Examples (engine-specific):
///
/// - KV-v2: `Role { name: "shared-team", .. }` (no constraints,
///   pure namespacing).
/// - Database: `Role { name: "readonly-prod", .. }` ties to a
///   particular SQL grant template.
/// - AWS: `Role { name: "deploy-bot", .. }` ties to an IAM
///   policy ARN.
///
/// The vocabulary stays neutral — the engine interprets `name`
/// against its own role table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Role {
    /// Human-readable role name.
    pub name: String,
    /// Engine-specific role-level TTL (None → engine default).
    pub default_ttl_ms: Option<u64>,
    /// Engine-specific role-level max-TTL (None → engine default).
    pub max_ttl_ms: Option<u64>,
}

impl Role {
    /// Constructor with name only; TTLs default to engine-supplied.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            default_ttl_ms: None,
            max_ttl_ms: None,
        }
    }
}

/// Per-call context handed to `mint`. Carries the *who* (auth
/// principal), the *when* (a caller-supplied wall-clock `now_ms`
/// for deterministic testing and to avoid each engine re-reading
/// the system clock), and the *engine-specific input* (the
/// fields below).
///
/// The engine-specific fields exist because the trait's
/// `mint(role, ctx)` signature cannot grow per-engine arguments
/// without breaking object-safety. Carrying them on `MintContext`
/// keeps every call self-contained — no shared "staged write"
/// slot the engine has to multiplex across concurrent callers
/// (which would be a confused-deputy hazard).
///
/// Engines ignore fields they don't need:
///
/// | Engine     | `path`             | `input`                   | `cas_token`        |
/// |------------|--------------------|---------------------------|--------------------|
/// | KV-v2      | row path (req'd)   | bytes to store (req'd)    | `expected_version` |
/// | Database   | unused             | unused                    | unused             |
/// | PKI        | role-specific path | CSR DER (req'd)           | unused             |
/// | AWS / GCP  | unused             | unused                    | unused             |
/// | Transit    | key name           | plaintext (encrypt) / ct  | unused             |
/// | TOTP       | account label      | seed (provision-only)     | unused             |
///
/// `requested_ttl_ms` is the ceiling the *caller* is asking for;
/// the engine clamps to `min(requested, role.max_ttl_ms,
/// engine_max)`.
///
/// `Clone` and `Debug` are intentionally *not* derived: the
/// `input` field carries plaintext under `Zeroizing<Vec<u8>>`
/// which deliberately doesn't implement either trait. Cloning
/// would silently duplicate plaintext into a non-zeroizing
/// intermediate; `Debug` would let `dbg!(ctx)` leak the bytes.
/// Callers that need a hash-friendly structured snapshot of a
/// context should serialize only the non-secret fields by hand.
pub struct MintContext {
    /// Opaque principal identifier from VLT05 (e.g. user ID,
    /// service account, AWS-STS ARN). The engine treats it as an
    /// audit-log breadcrumb, *not* as a privilege check.
    pub principal: String,
    /// Wall-clock time at the moment of the call, in ms since
    /// UNIX epoch. Caller-supplied for determinism.
    pub now_ms: u64,
    /// Caller's requested TTL in ms. The engine may clamp.
    pub requested_ttl_ms: u64,
    /// Engine-specific path within the mount (KV-v2: row path,
    /// PKI: role path). `None` when the engine has no notion of
    /// path (Database, AWS, GCP).
    pub path: Option<String>,
    /// Engine-specific input bytes (KV-v2: row body, PKI: CSR,
    /// Transit: plaintext or ciphertext, TOTP: seed). Held under
    /// `Zeroizing` so a stray drop scrubs them. `None` when the
    /// engine generates the secret internally (Database, AWS).
    pub input: Option<Zeroizing<Vec<u8>>>,
    /// Engine-specific compare-and-swap token (KV-v2: expected
    /// version; PKI: expected serial range; Transit: key
    /// generation). `None` skips CAS.
    pub cas_token: Option<u64>,
}

impl MintContext {
    /// Convenience constructor for the common "no engine-specific
    /// input" case (Database, AWS, GCP).
    pub fn simple(principal: impl Into<String>, now_ms: u64, requested_ttl_ms: u64) -> Self {
        Self {
            principal: principal.into(),
            now_ms,
            requested_ttl_ms,
            path: None,
            input: None,
            cas_token: None,
        }
    }
}

/// What `mint` returns: the bytes that should be wrapped in a
/// lease and a `SecretRef` that can be passed back to `revoke`.
///
/// `body` is opaque to the trait; engines define the wire shape
/// (e.g. KV-v2 returns CBOR, Database returns
/// `{"username":..., "password":...}` JSON, PKI returns PEM).
pub struct MintedSecret {
    /// The actual secret bytes, ready to wrap in a
    /// [`coding_adventures_vault_leases::LeasePayload`]. Held
    /// under [`Zeroizing`] so a stray drop scrubs them.
    pub body: Zeroizing<Vec<u8>>,
    /// Engine-defined revocation handle. The audit log records
    /// it so an operator can later say "revoke that mint" by
    /// looking it up.
    pub secret_ref: SecretRef,
    /// Effective TTL the engine applied (after clamping).
    pub granted_ttl_ms: u64,
}

impl core::fmt::Debug for MintedSecret {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MintedSecret")
            .field("body", &format_args!("<{} bytes redacted>", self.body.len()))
            .field("secret_ref", &self.secret_ref)
            .field("granted_ttl_ms", &self.granted_ttl_ms)
            .finish()
    }
}

impl MintedSecret {
    /// Convert the body into a [`LeasePayload`] suitable for
    /// handing to a [`coding_adventures_vault_leases::LeaseManager`].
    /// This is the canonical bridge between engines (VLT08) and
    /// leases (VLT07): every engine implementation mints bytes,
    /// every caller wraps them in a lease.
    ///
    /// `self.body` is moved into the payload (which is itself
    /// zeroizing). We deref-and-clone the inner `Vec` to feed
    /// `LeasePayload::new`; the intermediate `Vec` exists only
    /// for the single statement and is then owned by the
    /// `LeasePayload`'s `Zeroizing<Vec<u8>>`. After the move,
    /// `self.body`'s `Zeroizing` runs its scrub on drop.
    pub fn into_lease_payload(self) -> LeasePayload {
        let bytes: Vec<u8> = (*self.body).clone();
        LeasePayload::new(bytes)
    }
}

/// Engine-defined revocation handle. The trait treats it as
/// opaque; engines pattern-match on it inside `revoke`.
///
/// We use a small enum rather than `Vec<u8>` so each engine can
/// document the shape of its handle. Adding new variants is
/// non-breaking because the enum is `#[non_exhaustive]`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SecretRef {
    /// KV-v2: a `(path, version)` tuple naming the KV row.
    KvV2 {
        /// The mount-relative path of the KV row.
        path: String,
        /// The version that was returned by `mint`.
        version: u32,
    },
    /// Database: an opaque server-side username that was created
    /// during `mint` and should be `DROP USER`'d during `revoke`.
    DbUsername(String),
    /// PKI: an X.509 serial number to put on the next CRL update.
    PkiSerial(Vec<u8>),
    /// AWS: an STS session ID or IAM access-key-id.
    AwsSession(String),
    /// Catch-all for engines that haven't been added yet, so
    /// downstream code can still construct a `SecretRef`. The
    /// inner string is a debugging hint.
    Other(String),
}

/// Errors any engine implementation can return.
///
/// We keep variants narrow so audit consumers can handle them
/// uniformly; engine-specific reasons go in the
/// `InvalidParameter` payload as a `&'static str`.
#[derive(Debug)]
pub enum EngineError {
    /// The named role is not configured on this engine.
    UnknownRole(String),
    /// The caller asked for a TTL above the engine/role max, or
    /// passed another out-of-bounds parameter.
    InvalidParameter(&'static str),
    /// An external dependency (DB driver, AWS API, OS keystore)
    /// returned a transient failure. The engine's caller may
    /// retry.
    Backend(String),
    /// Cryptographic failure (e.g. CSPRNG unavailable, KMS
    /// unwrap failed).
    Crypto(String),
    /// The principal in `MintContext` is not allowed on this
    /// engine even though policy approved the call. Engines
    /// surface this from their defence-in-depth checks.
    PrincipalDenied(String),
    /// `revoke` was called with a `SecretRef` the engine doesn't
    /// own (wrong variant, or a serial that was never issued).
    UnknownSecret,
    /// Engine-tier resource conflict, e.g. two writers raced on
    /// the same KV-v2 path with the same expected version.
    Conflict,
}

impl core::fmt::Display for EngineError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownRole(r) => write!(f, "unknown role: {}", r),
            Self::InvalidParameter(p) => write!(f, "invalid parameter: {}", p),
            Self::Backend(b) => write!(f, "backend error: {}", b),
            Self::Crypto(c) => write!(f, "crypto error: {}", c),
            Self::PrincipalDenied(p) => write!(f, "principal denied: {}", p),
            Self::UnknownSecret => write!(f, "unknown secret_ref"),
            Self::Conflict => write!(f, "engine-tier conflict"),
        }
    }
}

impl std::error::Error for EngineError {}

// === Section 2. The trait ===================================================
//
// Four methods. Two of them — `mint` and `revoke` — are the
// money-makers; the other two are housekeeping.

/// Contract every secret engine implements.
///
/// All methods are `&self` (interior mutability) so an engine can
/// be shared by `Arc` across threads without macro-level wrapping.
/// Mutability is the engine's concern — the trait stays
/// signature-stable.
pub trait SecretEngine: Send + Sync {
    /// The mount path this engine answers to (e.g.
    /// `"kv/"`, `"database/postgres-prod/"`). Transports use
    /// this to dispatch incoming requests.
    ///
    /// Convention: trailing `/`. Empty string is reserved.
    fn mount_path(&self) -> &str;

    /// Mint a fresh secret under `role`. The engine is free to
    /// generate, fetch, or compute the bytes; the only invariant
    /// is that the returned `MintedSecret::granted_ttl_ms` is
    /// `<= ctx.requested_ttl_ms`.
    fn mint(&self, role: &Role, ctx: &MintContext) -> Result<MintedSecret, EngineError>;

    /// Revoke a previously-minted secret. For DB / AWS / PKI
    /// engines this calls out to the upstream system. For KV-v2
    /// it removes the version from the table.
    ///
    /// Idempotent: revoking the same `SecretRef` twice does not
    /// return an error.
    fn revoke(&self, secret_ref: &SecretRef) -> Result<(), EngineError>;

    /// Rotate the engine's *root* credential.
    ///
    /// What "root" means is engine-dependent: for the DB engine
    /// it's the privileged DB account the engine logs in as;
    /// for KV-v2 it's the encryption key that wraps the rows;
    /// for AWS it's the IAM user the engine assumes-role from.
    ///
    /// Engines that have no notion of "root" (e.g. a pure-compute
    /// engine) return `Ok(())` and document that.
    fn rotate_root(&self) -> Result<(), EngineError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time check that the trait is object-safe (we
    /// rely on `Box<dyn SecretEngine>` in the transport layer).
    #[test]
    fn trait_is_object_safe() {
        fn _accepts(_e: &dyn SecretEngine) {}
    }

    #[test]
    fn role_constructor_defaults_ttls_to_none() {
        let r = Role::new("admin");
        assert_eq!(r.name, "admin");
        assert_eq!(r.default_ttl_ms, None);
        assert_eq!(r.max_ttl_ms, None);
    }

    #[test]
    fn engine_error_display_includes_variant() {
        let e = EngineError::UnknownRole("missing".into());
        assert_eq!(format!("{}", e), "unknown role: missing");
        let e = EngineError::InvalidParameter("ttl_ms must be > 0");
        assert_eq!(format!("{}", e), "invalid parameter: ttl_ms must be > 0");
        let e = EngineError::UnknownSecret;
        assert_eq!(format!("{}", e), "unknown secret_ref");
    }

    #[test]
    fn minted_secret_debug_redacts_body() {
        let m = MintedSecret {
            body: Zeroizing::new(b"super-secret".to_vec()),
            secret_ref: SecretRef::Other("test".into()),
            granted_ttl_ms: 60_000,
        };
        let s = format!("{:?}", m);
        // The literal payload bytes must not appear in the Debug
        // output, but the byte length should.
        assert!(!s.contains("super-secret"));
        assert!(s.contains("12 bytes redacted"));
    }

    #[test]
    fn into_lease_payload_preserves_bytes() {
        let m = MintedSecret {
            body: Zeroizing::new(vec![1, 2, 3, 4, 5]),
            secret_ref: SecretRef::Other("t".into()),
            granted_ttl_ms: 60_000,
        };
        let lp = m.into_lease_payload();
        assert_eq!(lp.as_bytes(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn secret_ref_variants_are_distinct() {
        let a = SecretRef::KvV2 {
            path: "/foo".into(),
            version: 1,
        };
        let b = SecretRef::DbUsername("u1".into());
        assert_ne!(a, b);
        let c = a.clone();
        assert_eq!(a, c);
    }
}
