//! # `coding_adventures_vault_engine_kv2` — VLT08 KV-v2 engine
//!
//! ## What this crate is
//!
//! The **KV-v2 secret engine**: a versioned static key/value
//! store that satisfies the
//! [`SecretEngine`](coding_adventures_vault_engine_core::SecretEngine)
//! contract. It is the simplest dynamic-secret engine — "dynamic"
//! in the trait sense (mint/revoke/rotate API), even though the
//! data is static (the bytes you write are exactly the bytes you
//! read back).
//!
//! KV-v2 is the workhorse engine of every Vault deployment that
//! holds long-lived shared credentials, configuration secrets, or
//! anything else that doesn't have a natural mint-on-demand
//! source. It's also the on-ramp for password-manager-class
//! products: a Bitwarden / 1Password vault is structurally a
//! single-tenant KV-v2 mount with a typed-record codec on top
//! (VLT02).
//!
//! ## Semantics — same as HashiCorp Vault's KV-v2
//!
//! * **Versioned**: every write at a path produces a new monotonic
//!   integer version. The previous version is retained (capped by
//!   `max_versions` per role).
//! * **CAS on write**: callers can pass `expected_version` to
//!   `mint`; a mismatch returns
//!   [`EngineError::Conflict`].
//! * **Soft delete** (`revoke`): marks a specific version as
//!   destroyed. Reads of a destroyed version return
//!   [`EngineError::UnknownSecret`]; reads of a still-live
//!   version succeed.
//! * **Hard rotate** (`rotate_root`): bumps the engine-level
//!   "version-key" generation counter. In a sealed-store-backed
//!   build this would re-wrap every row under a new DEK; in this
//!   in-memory reference implementation we just bump the counter
//!   so audit-log consumers see a rotation event.
//!
//! ## Mint payload shape
//!
//! `mint` reads the row body and CAS token directly from
//! [`MintContext`]. The trait carries per-engine input as
//! optional fields on the context so every call is
//! self-contained — there is no shared "staged write" slot that
//! could be clobbered by another concurrent caller (a
//! confused-deputy hazard).
//!
//! ```ignore
//! use coding_adventures_zeroize::Zeroizing;
//! let ctx = MintContext {
//!     principal: "alice".into(),
//!     now_ms: 0,
//!     requested_ttl_ms: 60_000,
//!     path: Some("shared/db-password".into()),
//!     input: Some(Zeroizing::new(b"hunter2".to_vec())),
//!     cas_token: None,
//! };
//! let minted = engine.mint(&Role::new("shared"), &ctx)?;
//! // hand `minted.body` (the same bytes) to a lease manager
//! ```
//!
//! ## What this crate does *not* do
//!
//! * **Encryption at rest**: a real KV-v2 engine in production
//!   stores its rows through VLT01 sealed-store. The reference
//!   implementation here keeps rows in memory in plaintext but
//!   under [`Zeroizing`] so process-lifetime scrubbing is intact.
//!   A storage-backed sibling crate is future work.
//! * **Multi-tenant isolation**: a single `KvV2Engine` answers to
//!   one mount path. Multiple mounts → multiple engine instances.
//! * **Patch operations**: HashiCorp's KV-v2 has a JSON-merge
//!   patch op; we keep the surface to whole-row writes and let
//!   callers compose if they want patch semantics.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_vault_engine_core::{
    EngineError, MintContext, MintedSecret, Role, SecretEngine, SecretRef,
};
use coding_adventures_zeroize::Zeroizing;
use std::collections::HashMap;
use std::sync::Mutex;

// === Section 1. Internal data shape =========================================

/// One stored version of one KV path. The entry knows its own
/// version number, the bytes (held under `Zeroizing`), and a
/// destroyed flag — soft-deleted versions stay in the table so
/// the audit log sees both writes and revocations.
struct Version {
    version: u32,
    body: Zeroizing<Vec<u8>>,
    destroyed: bool,
}

/// All versions of one path. The `Vec` is append-only; revocation
/// flips a `destroyed` flag rather than removing.
struct PathHistory {
    versions: Vec<Version>,
}

impl PathHistory {
    fn new() -> Self {
        Self { versions: Vec::new() }
    }

    /// Latest live version, or `None` if every version is
    /// destroyed (or the path has none yet).
    fn latest_live(&self) -> Option<&Version> {
        self.versions.iter().rev().find(|v| !v.destroyed)
    }

    /// Highest version number ever issued at this path,
    /// regardless of destroyed-state. Used to compute the next
    /// version number on write.
    fn high_water_mark(&self) -> u32 {
        self.versions.last().map(|v| v.version).unwrap_or(0)
    }

    /// Cap *live* versions at `max_versions`; destroyed
    /// tombstones are kept indefinitely (they cost ~24B each and
    /// are how `revoke` exposes "this version existed and was
    /// destroyed at time T" to the audit log). We evict the
    /// oldest *live* row whenever the live count exceeds the
    /// cap.
    fn enforce_max_versions(&mut self, max_versions: usize) {
        loop {
            let live_count = self.versions.iter().filter(|v| !v.destroyed).count();
            if live_count <= max_versions {
                break;
            }
            // Find the index of the oldest live row.
            let idx = match self.versions.iter().position(|v| !v.destroyed) {
                Some(i) => i,
                None => break, // shouldn't happen given the count check
            };
            self.versions.remove(idx);
        }
    }
}

// === Section 2. The engine ==================================================

/// Engine configuration. Per-role caps live on the role
/// (`Role::default_ttl_ms`, `Role::max_ttl_ms`); engine-wide caps
/// (max-versions-per-path, engine-level max-TTL) live here.
#[derive(Clone, Debug)]
pub struct KvV2Config {
    /// Mount path, including trailing slash. Convention from the
    /// trait: empty string is reserved.
    pub mount_path: String,
    /// Maximum versions retained per path. Older versions are
    /// dropped (and their `Zeroizing` bytes scrubbed) once this
    /// cap is exceeded.
    pub max_versions: usize,
    /// Engine-wide max TTL (ms). Caller-requested TTLs are
    /// clamped to `min(requested, role.max, this)`.
    pub max_ttl_ms: u64,
    /// Engine-wide default TTL when both the caller and the role
    /// leave it unspecified. Applied as a floor in case
    /// `requested_ttl_ms == 0`.
    pub default_ttl_ms: u64,
}

impl Default for KvV2Config {
    fn default() -> Self {
        Self {
            mount_path: "kv/".into(),
            max_versions: 16,
            max_ttl_ms: 24 * 60 * 60 * 1_000, // 24 h
            default_ttl_ms: 60 * 60 * 1_000,  // 1 h
        }
    }
}

/// The KV-v2 engine itself.
struct Inner {
    table: HashMap<String, PathHistory>,
    /// Bumped by `rotate_root`. Visible in the audit-log payload
    /// of the next mint so consumers can correlate before/after.
    root_generation: u32,
}

/// KV-v2 engine. Instantiate one per mount path.
///
/// Threadsafe via an internal mutex. The intended pattern is
/// `Arc::new(KvV2Engine::new(cfg))` at startup, then hand the
/// `Arc` to the transport layer.
pub struct KvV2Engine {
    cfg: KvV2Config,
    inner: Mutex<Inner>,
}

impl KvV2Engine {
    /// Construct a fresh, empty engine.
    ///
    /// Returns `EngineError::InvalidParameter` if the configured
    /// mount path is empty (the trait reserves `""`). Using a
    /// `Result` here means request-handling code that constructs
    /// engines from untrusted input cannot be made to panic.
    pub fn new(cfg: KvV2Config) -> Result<Self, EngineError> {
        if cfg.mount_path.is_empty() {
            return Err(EngineError::InvalidParameter(
                "mount_path must not be empty (trait reserves '')",
            ));
        }
        Ok(Self {
            cfg,
            inner: Mutex::new(Inner {
                table: HashMap::new(),
                root_generation: 0,
            }),
        })
    }

    /// Read the latest live version at `path`, returning a fresh
    /// `Zeroizing<Vec<u8>>` clone. Read does not bump any
    /// version number.
    pub fn read_latest(&self, path: &str) -> Result<Zeroizing<Vec<u8>>, EngineError> {
        let g = self.inner.lock().expect("kv2 mutex poisoned");
        let history = g.table.get(path).ok_or(EngineError::UnknownSecret)?;
        let v = history.latest_live().ok_or(EngineError::UnknownSecret)?;
        Ok(Zeroizing::new(v.body.to_vec()))
    }

    /// Read a specific version at `path`. Returns
    /// `EngineError::UnknownSecret` if the version was destroyed
    /// or never existed.
    pub fn read_version(
        &self,
        path: &str,
        version: u32,
    ) -> Result<Zeroizing<Vec<u8>>, EngineError> {
        let g = self.inner.lock().expect("kv2 mutex poisoned");
        let history = g.table.get(path).ok_or(EngineError::UnknownSecret)?;
        let v = history
            .versions
            .iter()
            .find(|v| v.version == version)
            .ok_or(EngineError::UnknownSecret)?;
        if v.destroyed {
            return Err(EngineError::UnknownSecret);
        }
        Ok(Zeroizing::new(v.body.to_vec()))
    }

    /// Effective TTL for this call, clamped:
    ///
    /// `granted = min(requested_or_default, role.max_ttl_ms, engine.max_ttl_ms)`
    fn clamp_ttl(&self, role: &Role, requested: u64) -> Result<u64, EngineError> {
        let mut t = if requested == 0 {
            // 0 → use role default → engine default.
            role.default_ttl_ms.unwrap_or(self.cfg.default_ttl_ms)
        } else {
            requested
        };
        if let Some(rmax) = role.max_ttl_ms {
            t = t.min(rmax);
        }
        t = t.min(self.cfg.max_ttl_ms);
        if t == 0 {
            return Err(EngineError::InvalidParameter("clamped TTL is zero"));
        }
        Ok(t)
    }
}

// === Section 3. SecretEngine implementation =================================

impl SecretEngine for KvV2Engine {
    fn mount_path(&self) -> &str {
        &self.cfg.mount_path
    }

    fn mint(&self, role: &Role, ctx: &MintContext) -> Result<MintedSecret, EngineError> {
        // === Pull engine-specific input from MintContext ===
        // The trait's signature carries this on `ctx` rather than
        // a separate stage_write call so each `mint` is
        // self-contained — there is no shared per-engine "staged"
        // slot that a concurrent caller could clobber.
        let path = ctx
            .path
            .as_ref()
            .ok_or(EngineError::InvalidParameter("ctx.path is required"))?;
        if path.is_empty() {
            return Err(EngineError::InvalidParameter("ctx.path must not be empty"));
        }
        let input = ctx
            .input
            .as_ref()
            .ok_or(EngineError::InvalidParameter("ctx.input is required"))?;
        if input.is_empty() {
            return Err(EngineError::InvalidParameter(
                "ctx.input must not be empty",
            ));
        }
        // KV-v2's CAS token is the expected version (u32-shaped;
        // we accept u64 from the trait and validate on the way
        // in).
        let expected_version: Option<u32> = match ctx.cas_token {
            None => None,
            Some(n) => Some(u32::try_from(n).map_err(|_| {
                EngineError::InvalidParameter("cas_token does not fit in u32")
            })?),
        };
        let granted_ttl_ms = self.clamp_ttl(role, ctx.requested_ttl_ms)?;

        // === Atomic mutation under a single lock ===
        let mut g = self.inner.lock().expect("kv2 mutex poisoned");
        let history = g
            .table
            .entry(path.clone())
            .or_insert_with(PathHistory::new);
        // CAS check.
        let live = history.latest_live().map(|v| v.version);
        match expected_version {
            None => {}
            Some(0) => {
                if live.is_some() {
                    return Err(EngineError::Conflict);
                }
            }
            Some(n) => {
                if live != Some(n) {
                    return Err(EngineError::Conflict);
                }
            }
        }
        // Allocate next version.
        let next = history
            .high_water_mark()
            .checked_add(1)
            .ok_or(EngineError::InvalidParameter("version counter overflow"))?;
        // Build both copies into `Zeroizing` directly. The bytes
        // never sit in a bare `Vec<u8>` longer than the single
        // expression that wraps them, so an unwind cannot leave
        // unscrubbed plaintext on the heap.
        let body_for_store: Zeroizing<Vec<u8>> = Zeroizing::new(input.to_vec());
        let body_for_payload: Zeroizing<Vec<u8>> = Zeroizing::new(input.to_vec());
        history.versions.push(Version {
            version: next,
            body: body_for_store,
            destroyed: false,
        });
        history.enforce_max_versions(self.cfg.max_versions);
        Ok(MintedSecret {
            body: body_for_payload,
            secret_ref: SecretRef::KvV2 {
                path: path.clone(),
                version: next,
            },
            granted_ttl_ms,
        })
    }

    fn revoke(&self, secret_ref: &SecretRef) -> Result<(), EngineError> {
        let (path, version) = match secret_ref {
            SecretRef::KvV2 { path, version } => (path, *version),
            // Hand back UnknownSecret for non-KV refs rather than
            // a confusing variant-mismatch error: the engine
            // simply does not own that ref.
            _ => return Err(EngineError::UnknownSecret),
        };
        let mut g = self.inner.lock().expect("kv2 mutex poisoned");
        let history = g.table.get_mut(path).ok_or(EngineError::UnknownSecret)?;
        let v = history
            .versions
            .iter_mut()
            .find(|v| v.version == version)
            .ok_or(EngineError::UnknownSecret)?;
        // Idempotent: re-revoking a destroyed version is fine.
        if v.destroyed {
            return Ok(());
        }
        v.destroyed = true;
        // Drop the bytes — `Zeroizing` runs its scrub.
        v.body = Zeroizing::new(Vec::new());
        Ok(())
    }

    fn rotate_root(&self) -> Result<(), EngineError> {
        let mut g = self.inner.lock().expect("kv2 mutex poisoned");
        g.root_generation = g
            .root_generation
            .checked_add(1)
            .ok_or(EngineError::InvalidParameter("root_generation overflow"))?;
        Ok(())
    }
}

// === Section 4. Tests =======================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper: build a `MintContext` for a KV-v2 write at
    /// `path` with `body` and optional CAS expectation.
    fn kv_ctx(path: &str, body: &[u8], cas: Option<u64>, ttl: u64) -> MintContext {
        MintContext {
            principal: "test-principal".into(),
            now_ms: 1_000_000,
            requested_ttl_ms: ttl,
            path: Some(path.into()),
            input: Some(Zeroizing::new(body.to_vec())),
            cas_token: cas,
        }
    }

    fn fresh() -> KvV2Engine {
        KvV2Engine::new(KvV2Config::default()).unwrap()
    }

    #[test]
    fn mount_path_round_trips() {
        let e = fresh();
        assert_eq!(e.mount_path(), "kv/");
    }

    #[test]
    fn empty_mount_path_returns_invalid_parameter() {
        let mut cfg = KvV2Config::default();
        cfg.mount_path = String::new();
        let r = KvV2Engine::new(cfg);
        assert!(matches!(r, Err(EngineError::InvalidParameter(_))));
    }

    #[test]
    fn mint_returns_minted_secret() {
        let e = fresh();
        let m = e
            .mint(&Role::new("r"), &kv_ctx("p1", b"first", None, 60_000))
            .unwrap();
        assert_eq!(m.body.as_slice(), b"first");
        match m.secret_ref {
            SecretRef::KvV2 { path, version } => {
                assert_eq!(path, "p1");
                assert_eq!(version, 1);
            }
            other => panic!("wrong variant: {:?}", other),
        }
        assert_eq!(m.granted_ttl_ms, 60_000);
    }

    #[test]
    fn second_write_increments_version() {
        let e = fresh();
        let m1 = e
            .mint(&Role::new("r"), &kv_ctx("p", b"v1", None, 60_000))
            .unwrap();
        let m2 = e
            .mint(&Role::new("r"), &kv_ctx("p", b"v2", None, 60_000))
            .unwrap();
        let v2_n = match m2.secret_ref {
            SecretRef::KvV2 { version, .. } => version,
            _ => panic!("wrong variant"),
        };
        let v1_n = match m1.secret_ref {
            SecretRef::KvV2 { version, .. } => version,
            _ => panic!("wrong variant"),
        };
        assert_eq!(v1_n, 1);
        assert_eq!(v2_n, 2);
    }

    #[test]
    fn read_latest_returns_most_recent_live_version() {
        let e = fresh();
        e.mint(&Role::new("r"), &kv_ctx("p", b"v1", None, 60_000))
            .unwrap();
        e.mint(&Role::new("r"), &kv_ctx("p", b"v2", None, 60_000))
            .unwrap();
        let bytes = e.read_latest("p").unwrap();
        assert_eq!(bytes.as_slice(), b"v2");
    }

    #[test]
    fn read_version_specific() {
        let e = fresh();
        e.mint(&Role::new("r"), &kv_ctx("p", b"v1", None, 60_000))
            .unwrap();
        e.mint(&Role::new("r"), &kv_ctx("p", b"v2", None, 60_000))
            .unwrap();
        let b1 = e.read_version("p", 1).unwrap();
        assert_eq!(b1.as_slice(), b"v1");
        let b2 = e.read_version("p", 2).unwrap();
        assert_eq!(b2.as_slice(), b"v2");
    }

    #[test]
    fn read_unknown_path_is_unknown_secret() {
        let e = fresh();
        assert!(matches!(
            e.read_latest("missing"),
            Err(EngineError::UnknownSecret)
        ));
        assert!(matches!(
            e.read_version("missing", 1),
            Err(EngineError::UnknownSecret)
        ));
    }

    #[test]
    fn cas_create_only_rejects_overwrite() {
        let e = fresh();
        e.mint(&Role::new("r"), &kv_ctx("p", b"v1", Some(0), 60_000))
            .unwrap();
        // Second create-only write should conflict.
        assert!(matches!(
            e.mint(&Role::new("r"), &kv_ctx("p", b"v2", Some(0), 60_000)),
            Err(EngineError::Conflict)
        ));
    }

    #[test]
    fn cas_update_from_wrong_version_rejected() {
        let e = fresh();
        e.mint(&Role::new("r"), &kv_ctx("p", b"v1", None, 60_000))
            .unwrap();
        // Latest is 1; ask to update from 7 → conflict.
        assert!(matches!(
            e.mint(&Role::new("r"), &kv_ctx("p", b"v2", Some(7), 60_000)),
            Err(EngineError::Conflict)
        ));
    }

    #[test]
    fn cas_update_from_correct_version_accepted() {
        let e = fresh();
        e.mint(&Role::new("r"), &kv_ctx("p", b"v1", None, 60_000))
            .unwrap();
        let m = e
            .mint(&Role::new("r"), &kv_ctx("p", b"v2", Some(1), 60_000))
            .unwrap();
        let v = match m.secret_ref {
            SecretRef::KvV2 { version, .. } => version,
            _ => panic!("wrong variant"),
        };
        assert_eq!(v, 2);
    }

    #[test]
    fn cas_token_overflow_u32_rejected() {
        let e = fresh();
        let mut ctx = kv_ctx("p", b"v", None, 60_000);
        ctx.cas_token = Some(u64::MAX);
        assert!(matches!(
            e.mint(&Role::new("r"), &ctx),
            Err(EngineError::InvalidParameter(_))
        ));
    }

    #[test]
    fn mint_without_path_errors() {
        let e = fresh();
        let mut ctx = kv_ctx("p", b"v", None, 60_000);
        ctx.path = None;
        assert!(matches!(
            e.mint(&Role::new("r"), &ctx),
            Err(EngineError::InvalidParameter(_))
        ));
    }

    #[test]
    fn mint_without_input_errors() {
        let e = fresh();
        let mut ctx = kv_ctx("p", b"v", None, 60_000);
        ctx.input = None;
        assert!(matches!(
            e.mint(&Role::new("r"), &ctx),
            Err(EngineError::InvalidParameter(_))
        ));
    }

    #[test]
    fn mint_with_empty_path_errors() {
        let e = fresh();
        let ctx = kv_ctx("", b"v", None, 60_000);
        assert!(matches!(
            e.mint(&Role::new("r"), &ctx),
            Err(EngineError::InvalidParameter(_))
        ));
    }

    #[test]
    fn mint_with_empty_input_errors() {
        let e = fresh();
        let ctx = kv_ctx("p", b"", None, 60_000);
        assert!(matches!(
            e.mint(&Role::new("r"), &ctx),
            Err(EngineError::InvalidParameter(_))
        ));
    }

    #[test]
    fn revoke_makes_version_unreadable() {
        let e = fresh();
        let m = e
            .mint(&Role::new("r"), &kv_ctx("p", b"v1", None, 60_000))
            .unwrap();
        e.revoke(&m.secret_ref).unwrap();
        assert!(matches!(
            e.read_version("p", 1),
            Err(EngineError::UnknownSecret)
        ));
        // Latest also gone since the only version is destroyed.
        assert!(matches!(
            e.read_latest("p"),
            Err(EngineError::UnknownSecret)
        ));
    }

    #[test]
    fn revoke_idempotent() {
        let e = fresh();
        let m = e
            .mint(&Role::new("r"), &kv_ctx("p", b"v1", None, 60_000))
            .unwrap();
        e.revoke(&m.secret_ref).unwrap();
        e.revoke(&m.secret_ref).unwrap();
    }

    #[test]
    fn revoke_unknown_ref_returns_unknown_secret() {
        let e = fresh();
        let bogus = SecretRef::KvV2 {
            path: "no-such".into(),
            version: 1,
        };
        assert!(matches!(e.revoke(&bogus), Err(EngineError::UnknownSecret)));
    }

    #[test]
    fn revoke_wrong_variant_returns_unknown_secret() {
        let e = fresh();
        let r = SecretRef::DbUsername("u".into());
        assert!(matches!(e.revoke(&r), Err(EngineError::UnknownSecret)));
    }

    #[test]
    fn revoke_then_latest_falls_back_to_prior_live() {
        let e = fresh();
        e.mint(&Role::new("r"), &kv_ctx("p", b"v1", None, 60_000))
            .unwrap();
        let m2 = e
            .mint(&Role::new("r"), &kv_ctx("p", b"v2", None, 60_000))
            .unwrap();
        e.revoke(&m2.secret_ref).unwrap();
        let bytes = e.read_latest("p").unwrap();
        assert_eq!(bytes.as_slice(), b"v1");
    }

    #[test]
    fn ttl_clamped_to_engine_max() {
        let mut cfg = KvV2Config::default();
        cfg.max_ttl_ms = 1_000;
        let e = KvV2Engine::new(cfg).unwrap();
        let m = e
            .mint(&Role::new("r"), &kv_ctx("p", b"v", None, 60_000))
            .unwrap();
        assert_eq!(m.granted_ttl_ms, 1_000);
    }

    #[test]
    fn ttl_clamped_to_role_max() {
        let e = fresh();
        let role = Role {
            name: "r".into(),
            default_ttl_ms: None,
            max_ttl_ms: Some(500),
        };
        let m = e
            .mint(&role, &kv_ctx("p", b"v", None, 60_000))
            .unwrap();
        assert_eq!(m.granted_ttl_ms, 500);
    }

    #[test]
    fn zero_request_uses_default_ttl() {
        let e = fresh();
        let m = e
            .mint(&Role::new("r"), &kv_ctx("p", b"v", None, 0))
            .unwrap();
        assert_eq!(m.granted_ttl_ms, 60 * 60 * 1_000);
    }

    #[test]
    fn rotate_root_succeeds_repeatedly() {
        let e = fresh();
        for _ in 0..5 {
            e.rotate_root().unwrap();
        }
    }

    #[test]
    fn max_versions_caps_live_history_and_keeps_tombstones() {
        let mut cfg = KvV2Config::default();
        cfg.max_versions = 3;
        let e = KvV2Engine::new(cfg).unwrap();
        // Write 5 live versions; only the last 3 live should
        // remain readable. Older live versions are evicted.
        for i in 0..5u8 {
            e.mint(
                &Role::new("r"),
                &kv_ctx("p", format!("v{}", i).as_bytes(), None, 60_000),
            )
            .unwrap();
        }
        // v0 (version 1) and v1 (version 2) — evicted (too old).
        assert!(matches!(
            e.read_version("p", 1),
            Err(EngineError::UnknownSecret)
        ));
        assert!(matches!(
            e.read_version("p", 2),
            Err(EngineError::UnknownSecret)
        ));
        // v2..v4 (versions 3,4,5) still readable.
        assert_eq!(e.read_version("p", 3).unwrap().as_slice(), b"v2");
        assert_eq!(e.read_version("p", 5).unwrap().as_slice(), b"v4");
        // Now soft-delete v3; live count drops to 2 (v4, v5).
        // Tombstones don't count against the cap, so the next
        // live write goes through without evicting anyone.
        let r3 = SecretRef::KvV2 {
            path: "p".into(),
            version: 3,
        };
        e.revoke(&r3).unwrap();
        e.mint(&Role::new("r"), &kv_ctx("p", b"v5b", None, 60_000))
            .unwrap();
        // v4 and v5 still live.
        assert_eq!(e.read_version("p", 4).unwrap().as_slice(), b"v3");
        assert_eq!(e.read_version("p", 5).unwrap().as_slice(), b"v4");
        // v3 is destroyed → unknown secret on read.
        assert!(matches!(
            e.read_version("p", 3),
            Err(EngineError::UnknownSecret)
        ));
    }

    #[test]
    fn engine_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<KvV2Engine>();
        assert_send_sync::<Box<dyn SecretEngine>>();
    }

    #[test]
    fn into_lease_payload_after_mint() {
        let e = fresh();
        let m = e
            .mint(&Role::new("r"), &kv_ctx("p", b"deadbeef", None, 60_000))
            .unwrap();
        let lp = m.into_lease_payload();
        assert_eq!(lp.as_bytes(), b"deadbeef");
    }

    #[test]
    fn concurrent_mints_do_not_cross_streams() {
        // Two threads minting to different paths concurrently
        // must end up with their own bytes at their own paths.
        // Before the fix this was a confused-deputy race via the
        // single staged-write slot; now each mint() carries its
        // own input on the MintContext so cross-talk is
        // structurally impossible.
        use std::sync::Arc;
        use std::thread;
        let e = Arc::new(fresh());
        let mut handles = Vec::new();
        for i in 0..16 {
            let e = e.clone();
            handles.push(thread::spawn(move || {
                let path = format!("p{}", i);
                let body = format!("body-{}", i);
                let ctx = kv_ctx(&path, body.as_bytes(), None, 60_000);
                let m = e.mint(&Role::new("r"), &ctx).unwrap();
                assert_eq!(m.body.as_slice(), body.as_bytes());
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        // Verify all paths landed correctly.
        for i in 0..16 {
            let path = format!("p{}", i);
            let body = format!("body-{}", i);
            assert_eq!(e.read_latest(&path).unwrap().as_slice(), body.as_bytes());
        }
    }
}
