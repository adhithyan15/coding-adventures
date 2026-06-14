//! # coding_adventures_vault_policy — VLT06
//!
//! ## What this crate does
//!
//! The pluggable **policy engine** layer of the Vault stack.
//! Authentication (VLT05) says *who*; policy says *what they can
//! do*. This crate hosts the trait and ships a small set of
//! built-in engines + decorators that cover the common cases:
//!
//! - **`SimpleRbacEngine`** — role × permission table. Fits a
//!   Bitwarden-class app where users have roles like
//!   `"member"`, `"admin"`, `"owner"` and resources are vault
//!   items.
//! - **`AllOf`** — boolean AND of inner engines. All must allow.
//! - **`AnyOf`** — boolean OR of inner engines. Any allow wins.
//! - **`RequireFactor`** — wraps an inner engine and additionally
//!   requires a specific authentication-factor `kind` to be
//!   present in the context (e.g. require WebAuthn for
//!   `delete-vault` actions).
//! - **`TimeBound`** — wraps an inner engine and only allows
//!   within a `[start, end]` UNIX-time window.
//!
//! Future engines (HCL, Cedar, Rego) plug in via the same trait
//! and compose with these decorators identically.
//!
//! ## Decision shape
//!
//! ```ignore
//! pub enum Decision { Allow, Deny(Reason) }
//! ```
//!
//! `Reason` is a `&'static str` chosen from a fixed table — the
//! engine never quotes attacker-controlled bytes back to the
//! caller, so a malicious principal name in a deny message
//! cannot inject content into logs.
//!
//! ## Threat model
//!
//! - The policy engine doesn't see secrets — only metadata
//!   (principal, action, resource, factor list, time, optional
//!   metadata bag). Decisions are deterministic given the same
//!   context.
//! - Decisions fail closed — anything the engine can't
//!   confidently allow is denied with a static reason.
//! - All `Display` strings come from this crate's literals.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::{HashMap, HashSet};

// ─────────────────────────────────────────────────────────────────────
// 1. Context & decision
// ─────────────────────────────────────────────────────────────────────

/// A decision request. The engine reads this and returns
/// [`Decision`]. Fields are deliberately narrow — anything the
/// engine doesn't read can't influence the decision.
#[derive(Debug, Clone)]
pub struct PolicyContext {
    /// Stable identifier of the requester. For users it's a user
    /// id; for machine workloads it's the workload identity (e.g.
    /// service account name, AWS-IAM ARN). Opaque to the engine.
    pub principal: String,
    /// What the requester is trying to do. e.g. `"read"`,
    /// `"write"`, `"delete"`, `"share"`, `"rotate-kek"`,
    /// `"issue-db-cred"`. Opaque to the engine.
    pub action: String,
    /// Which resource the action applies to. e.g.
    /// `"vault/login/abc123"`, `"engine/database/prod-postgres"`.
    /// Opaque to the engine.
    pub resource: String,
    /// Authentication-factor `kind` strings that backed this
    /// session — typically populated from the matching
    /// `AuthAssertion::kind` from VLT05.
    pub factors: Vec<String>,
    /// UNIX seconds at the moment of the decision. Caller passes
    /// `SystemTime::now()` or a test-pinned value.
    pub time: u64,
    /// Free-form metadata bag (IP address, geo, device id, …).
    /// Opaque to the engine; specific engines may inspect specific
    /// keys.
    pub metadata: HashMap<String, String>,
}

/// The engine's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Permitted.
    Allow,
    /// Denied. `Reason` carries a static literal explaining the
    /// rule that fired, never attacker-controlled bytes.
    Deny(Reason),
}

impl Decision {
    /// Returns true when the policy allowed the request.
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow)
    }

    /// Returns true when the policy denied the request.
    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny(_))
    }

    /// Returns the static denial reason, if this is a denial.
    pub fn reason(&self) -> Option<Reason> {
        match self {
            Self::Allow => None,
            Self::Deny(reason) => Some(*reason),
        }
    }
}

/// Static-literal reason for a `Deny`. The engine never
/// constructs these from input bytes; they all come from a fixed
/// per-rule table so a malicious principal name in logs cannot
/// inject content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reason(pub &'static str);

impl Reason {
    /// Default catch-all reason ("policy denies").
    pub const POLICY_DENIES: Reason = Reason("policy denies");
    /// Caller's principal is unknown to the RBAC table.
    pub const UNKNOWN_PRINCIPAL: Reason = Reason("unknown principal");
    /// Principal's role does not grant this `(action, resource)`.
    pub const ROLE_LACKS_PERMISSION: Reason = Reason("role lacks permission");
    /// Required authentication factor was absent from the context.
    pub const FACTOR_REQUIRED: Reason = Reason("required authentication factor missing");
    /// Time-bound decorator: the decision time was outside the
    /// configured window.
    pub const OUTSIDE_TIME_WINDOW: Reason = Reason("outside time window");
    /// `AllOf` decorator: at least one inner engine denied.
    pub const ANY_INNER_DENIED: Reason = Reason("at least one inner engine denied");
    /// `AnyOf` decorator: every inner engine denied.
    pub const ALL_INNER_DENIED: Reason = Reason("every inner engine denied");
}

/// Owned, audit-friendly view of one policy decision.
///
/// The record intentionally copies only identifiers and counts from
/// [`PolicyContext`]. Callers can hand it to audit or telemetry sinks without
/// granting those sinks access to the original metadata bag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDecisionRecord {
    /// Stable kind of the policy engine that produced the decision.
    pub engine_kind: &'static str,
    /// Principal being evaluated.
    pub principal: String,
    /// Requested action.
    pub action: String,
    /// Requested resource.
    pub resource: String,
    /// Decision time in UNIX seconds.
    pub time: u64,
    /// Number of authentication factors present on the request.
    pub factor_count: usize,
    /// Number of metadata keys present on the request.
    pub metadata_count: usize,
    /// Final policy decision.
    pub decision: Decision,
}

impl PolicyDecisionRecord {
    /// Build a record from a precomputed decision.
    pub fn from_decision(
        engine_kind: &'static str,
        ctx: &PolicyContext,
        decision: Decision,
    ) -> Self {
        Self {
            engine_kind,
            principal: ctx.principal.clone(),
            action: ctx.action.clone(),
            resource: ctx.resource.clone(),
            time: ctx.time,
            factor_count: ctx.factors.len(),
            metadata_count: ctx.metadata.len(),
            decision,
        }
    }

    /// Returns true when this record captured an allow decision.
    pub fn is_allowed(&self) -> bool {
        self.decision.is_allow()
    }

    /// Returns the static denial reason, if this record captured a denial.
    pub fn denial_reason(&self) -> Option<Reason> {
        self.decision.reason()
    }
}

// ─────────────────────────────────────────────────────────────────────
// 2. The trait
// ─────────────────────────────────────────────────────────────────────

/// Pluggable policy engine. Implementations: `SimpleRbacEngine`,
/// the decorator wrappers below, and future HCL / Cedar / Rego.
pub trait PolicyEngine: Send + Sync {
    /// Stable kind string for telemetry, e.g. `"rbac"`,
    /// `"all-of"`, `"hcl"`. Not load-bearing for security.
    fn kind(&self) -> &'static str;

    /// Decide. Pure function over the context. Implementations
    /// must NOT touch the network, the filesystem, or the wall
    /// clock — `ctx.time` is the time of record.
    fn decide(&self, ctx: &PolicyContext) -> Decision;
}

/// Decide a request and return a compact owned record of the decision.
pub fn decide_with_record<E>(engine: &E, ctx: &PolicyContext) -> PolicyDecisionRecord
where
    E: PolicyEngine + ?Sized,
{
    let decision = engine.decide(ctx);
    PolicyDecisionRecord::from_decision(engine.kind(), ctx, decision)
}

// ─────────────────────────────────────────────────────────────────────
// 3. SimpleRbacEngine — role × (action, resource_pattern) table
// ─────────────────────────────────────────────────────────────────────

/// A simple role-based access-control engine.
///
/// Each principal is mapped to a single role. Each role is mapped
/// to a set of `(action, resource_pattern)` permissions. A
/// permission's `resource_pattern` matches:
///
/// - exact string match, OR
/// - `*` for any resource (the wildcard).
///
/// Future versions can add prefix or glob matching; v0.1 keeps
/// it deliberately small.
#[derive(Debug, Default, Clone)]
pub struct SimpleRbacEngine {
    /// principal → role
    principals: HashMap<String, String>,
    /// role → set of (action, resource_pattern)
    role_perms: HashMap<String, HashSet<(String, String)>>,
}

impl SimpleRbacEngine {
    /// New empty engine — denies everything by default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a principal to a single role. Replaces any existing
    /// binding for the same principal.
    pub fn assign_role(&mut self, principal: impl Into<String>, role: impl Into<String>) {
        self.principals.insert(principal.into(), role.into());
    }

    /// Grant a `(action, resource_pattern)` permission to a role.
    /// `resource_pattern` is either an exact string or `"*"`.
    pub fn grant(
        &mut self,
        role: impl Into<String>,
        action: impl Into<String>,
        resource_pattern: impl Into<String>,
    ) {
        let entry = self.role_perms.entry(role.into()).or_default();
        entry.insert((action.into(), resource_pattern.into()));
    }

    /// Return a count-only view of the RBAC table shape.
    pub fn summary(&self) -> SimpleRbacSummary {
        let assigned_roles: HashSet<&str> = self.principals.values().map(String::as_str).collect();
        let permission_roles: HashSet<&str> = self.role_perms.keys().map(String::as_str).collect();
        let mut summary = SimpleRbacSummary {
            principal_bindings: self.principals.len(),
            assigned_roles: assigned_roles.len(),
            roles_with_permissions: permission_roles.len(),
            unique_roles: assigned_roles.union(&permission_roles).count(),
            permission_roles_without_principals: permission_roles
                .difference(&assigned_roles)
                .count(),
            assigned_roles_without_permissions: assigned_roles
                .difference(&permission_roles)
                .count(),
            ..SimpleRbacSummary::empty()
        };

        for permissions in self.role_perms.values() {
            summary.permission_grants += permissions.len();
            for (_, resource_pattern) in permissions {
                if resource_pattern == "*" {
                    summary.wildcard_resource_grants += 1;
                } else {
                    summary.exact_resource_grants += 1;
                }
            }
        }

        summary
    }
}

/// Count-only read model for a [`SimpleRbacEngine`] table.
///
/// This intentionally reports only table shape and grant counts. It does not
/// expose principal ids, role names, actions, or resource patterns.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SimpleRbacSummary {
    /// Number of principal-to-role bindings.
    pub principal_bindings: usize,
    /// Number of distinct roles referenced by principal bindings or grants.
    pub unique_roles: usize,
    /// Number of distinct roles assigned to at least one principal.
    pub assigned_roles: usize,
    /// Number of distinct roles with at least one permission grant.
    pub roles_with_permissions: usize,
    /// Total number of role permission grants.
    pub permission_grants: usize,
    /// Number of grants whose resource pattern is the wildcard `"*"`.
    pub wildcard_resource_grants: usize,
    /// Number of grants whose resource pattern is an exact resource string.
    pub exact_resource_grants: usize,
    /// Number of roles with permissions but no assigned principal.
    pub permission_roles_without_principals: usize,
    /// Number of assigned roles with no permission grants.
    pub assigned_roles_without_permissions: usize,
}

impl SimpleRbacSummary {
    /// Return an empty summary.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Returns true when at least one principal is bound to a role.
    pub fn has_principal_bindings(&self) -> bool {
        self.principal_bindings > 0
    }

    /// Returns true when at least one permission is granted.
    pub fn has_permission_grants(&self) -> bool {
        self.permission_grants > 0
    }

    /// Returns true when a granted role is not assigned to any principal.
    pub fn has_permission_roles_without_principals(&self) -> bool {
        self.permission_roles_without_principals > 0
    }

    /// Returns true when an assigned role has no permission grants.
    pub fn has_assigned_roles_without_permissions(&self) -> bool {
        self.assigned_roles_without_permissions > 0
    }
}

impl PolicyEngine for SimpleRbacEngine {
    fn kind(&self) -> &'static str {
        "rbac"
    }
    fn decide(&self, ctx: &PolicyContext) -> Decision {
        let role = match self.principals.get(&ctx.principal) {
            Some(r) => r,
            None => return Decision::Deny(Reason::UNKNOWN_PRINCIPAL),
        };
        let perms = match self.role_perms.get(role) {
            Some(p) => p,
            None => return Decision::Deny(Reason::ROLE_LACKS_PERMISSION),
        };
        for (action_pat, resource_pat) in perms {
            if action_pat == &ctx.action && (resource_pat == "*" || resource_pat == &ctx.resource) {
                return Decision::Allow;
            }
        }
        Decision::Deny(Reason::ROLE_LACKS_PERMISSION)
    }
}

// ─────────────────────────────────────────────────────────────────────
// 4. Decorators: AllOf, AnyOf, RequireFactor, TimeBound
// ─────────────────────────────────────────────────────────────────────

/// Boolean AND. Allows only if *every* inner engine allows.
pub struct AllOf {
    inner: Vec<Box<dyn PolicyEngine>>,
}

impl AllOf {
    /// Build from a list of inner engines.
    pub fn new(inner: Vec<Box<dyn PolicyEngine>>) -> Self {
        Self { inner }
    }
}

impl PolicyEngine for AllOf {
    fn kind(&self) -> &'static str {
        "all-of"
    }
    fn decide(&self, ctx: &PolicyContext) -> Decision {
        if self.inner.is_empty() {
            // Vacuously deny — an empty list can't allow.
            return Decision::Deny(Reason::POLICY_DENIES);
        }
        for e in &self.inner {
            match e.decide(ctx) {
                Decision::Allow => continue,
                Decision::Deny(_) => return Decision::Deny(Reason::ANY_INNER_DENIED),
            }
        }
        Decision::Allow
    }
}

/// Boolean OR. Allows if *any* inner engine allows.
pub struct AnyOf {
    inner: Vec<Box<dyn PolicyEngine>>,
}

impl AnyOf {
    /// Build from a list of inner engines.
    pub fn new(inner: Vec<Box<dyn PolicyEngine>>) -> Self {
        Self { inner }
    }
}

impl PolicyEngine for AnyOf {
    fn kind(&self) -> &'static str {
        "any-of"
    }
    fn decide(&self, ctx: &PolicyContext) -> Decision {
        if self.inner.is_empty() {
            return Decision::Deny(Reason::POLICY_DENIES);
        }
        for e in &self.inner {
            if let Decision::Allow = e.decide(ctx) {
                return Decision::Allow;
            }
        }
        Decision::Deny(Reason::ALL_INNER_DENIED)
    }
}

/// Wraps an inner engine and additionally requires a specific
/// authentication-factor `kind` to be present in the context.
///
/// Useful for "step-up" rules: e.g. allow `read` with just a
/// password, but require WebAuthn-PRF for `rotate-kek`.
pub struct RequireFactor {
    inner: Box<dyn PolicyEngine>,
    factor_kind: &'static str,
}

impl RequireFactor {
    /// Build a wrapper that allows the inner engine's decision
    /// only if `factor_kind` appears in `ctx.factors`.
    pub fn new(inner: Box<dyn PolicyEngine>, factor_kind: &'static str) -> Self {
        Self { inner, factor_kind }
    }
}

impl PolicyEngine for RequireFactor {
    fn kind(&self) -> &'static str {
        "require-factor"
    }
    fn decide(&self, ctx: &PolicyContext) -> Decision {
        if !ctx.factors.iter().any(|f| f == self.factor_kind) {
            return Decision::Deny(Reason::FACTOR_REQUIRED);
        }
        self.inner.decide(ctx)
    }
}

/// Wraps an inner engine and only forwards the decision when the
/// caller's `ctx.time` is in `[start, end]` (UNIX seconds,
/// inclusive). Outside the window → `Deny(OUTSIDE_TIME_WINDOW)`.
pub struct TimeBound {
    inner: Box<dyn PolicyEngine>,
    start: u64,
    end: u64,
}

impl TimeBound {
    /// Build a time-bounded wrapper.
    pub fn new(inner: Box<dyn PolicyEngine>, start: u64, end: u64) -> Self {
        Self { inner, start, end }
    }
}

impl PolicyEngine for TimeBound {
    fn kind(&self) -> &'static str {
        "time-bound"
    }
    fn decide(&self, ctx: &PolicyContext) -> Decision {
        if ctx.time < self.start || ctx.time > self.end {
            return Decision::Deny(Reason::OUTSIDE_TIME_WINDOW);
        }
        self.inner.decide(ctx)
    }
}

// ─────────────────────────────────────────────────────────────────────
// 5. Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(
        principal: &str,
        action: &str,
        resource: &str,
        factors: &[&str],
        time: u64,
    ) -> PolicyContext {
        PolicyContext {
            principal: principal.into(),
            action: action.into(),
            resource: resource.into(),
            factors: factors.iter().map(|s| (*s).to_string()).collect(),
            time,
            metadata: HashMap::new(),
        }
    }

    fn rbac_alice_admin_bob_member() -> SimpleRbacEngine {
        let mut e = SimpleRbacEngine::new();
        e.assign_role("alice", "admin");
        e.assign_role("bob", "member");
        e.grant("admin", "read", "*");
        e.grant("admin", "write", "*");
        e.grant("admin", "delete", "*");
        e.grant("member", "read", "*");
        e
    }

    // --- SimpleRbacEngine ---

    #[test]
    fn rbac_admin_can_delete() {
        let e = rbac_alice_admin_bob_member();
        let c = ctx("alice", "delete", "vault/login/abc", &["password"], 0);
        assert_eq!(e.decide(&c), Decision::Allow);
    }

    #[test]
    fn rbac_member_cannot_delete() {
        let e = rbac_alice_admin_bob_member();
        let c = ctx("bob", "delete", "vault/login/abc", &["password"], 0);
        match e.decide(&c) {
            Decision::Deny(r) => assert_eq!(r, Reason::ROLE_LACKS_PERMISSION),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn rbac_unknown_principal_denied_with_specific_reason() {
        let e = rbac_alice_admin_bob_member();
        let c = ctx("eve", "read", "vault/login/abc", &[], 0);
        match e.decide(&c) {
            Decision::Deny(r) => assert_eq!(r, Reason::UNKNOWN_PRINCIPAL),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn rbac_member_can_read_with_wildcard_grant() {
        let e = rbac_alice_admin_bob_member();
        let c = ctx("bob", "read", "vault/login/anything", &[], 0);
        assert_eq!(e.decide(&c), Decision::Allow);
    }

    #[test]
    fn rbac_exact_resource_grant() {
        let mut e = SimpleRbacEngine::new();
        e.assign_role("alice", "narrow");
        e.grant("narrow", "read", "vault/login/specific");
        let c1 = ctx("alice", "read", "vault/login/specific", &[], 0);
        let c2 = ctx("alice", "read", "vault/login/other", &[], 0);
        assert_eq!(e.decide(&c1), Decision::Allow);
        match e.decide(&c2) {
            Decision::Deny(r) => assert_eq!(r, Reason::ROLE_LACKS_PERMISSION),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn rbac_role_with_no_perms_denies() {
        let mut e = SimpleRbacEngine::new();
        e.assign_role("alice", "no-perms");
        let c = ctx("alice", "read", "x", &[], 0);
        match e.decide(&c) {
            Decision::Deny(r) => assert_eq!(r, Reason::ROLE_LACKS_PERMISSION),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn rbac_summary_counts_table_shape_without_values() {
        let e = rbac_alice_admin_bob_member();

        let summary = e.summary();

        assert_eq!(
            summary,
            SimpleRbacSummary {
                principal_bindings: 2,
                unique_roles: 2,
                assigned_roles: 2,
                roles_with_permissions: 2,
                permission_grants: 4,
                wildcard_resource_grants: 4,
                exact_resource_grants: 0,
                permission_roles_without_principals: 0,
                assigned_roles_without_permissions: 0,
            }
        );
        assert!(summary.has_principal_bindings());
        assert!(summary.has_permission_grants());
        assert!(!summary.has_permission_roles_without_principals());
        assert!(!summary.has_assigned_roles_without_permissions());
    }

    #[test]
    fn rbac_summary_counts_orphaned_and_empty_roles() {
        let mut e = SimpleRbacEngine::new();
        e.assign_role("alice", "admin");
        e.assign_role("carol", "empty");
        e.grant("admin", "read", "vault/login/specific");
        e.grant("orphan", "read", "*");

        let summary = e.summary();

        assert_eq!(summary.principal_bindings, 2);
        assert_eq!(summary.unique_roles, 3);
        assert_eq!(summary.assigned_roles, 2);
        assert_eq!(summary.roles_with_permissions, 2);
        assert_eq!(summary.permission_grants, 2);
        assert_eq!(summary.wildcard_resource_grants, 1);
        assert_eq!(summary.exact_resource_grants, 1);
        assert_eq!(summary.permission_roles_without_principals, 1);
        assert_eq!(summary.assigned_roles_without_permissions, 1);
        assert!(summary.has_permission_roles_without_principals());
        assert!(summary.has_assigned_roles_without_permissions());
    }

    #[test]
    fn rbac_empty_summary_is_default() {
        let summary = SimpleRbacEngine::new().summary();

        assert_eq!(summary, SimpleRbacSummary::empty());
        assert!(!summary.has_principal_bindings());
        assert!(!summary.has_permission_grants());
    }

    #[test]
    fn decision_helpers_expose_outcome_and_reason() {
        assert!(Decision::Allow.is_allow());
        assert!(!Decision::Allow.is_deny());
        assert_eq!(Decision::Allow.reason(), None);

        let denial = Decision::Deny(Reason::POLICY_DENIES);
        assert!(denial.is_deny());
        assert_eq!(denial.reason(), Some(Reason::POLICY_DENIES));
    }

    #[test]
    fn decide_with_record_captures_compact_allow_shape() {
        let e = rbac_alice_admin_bob_member();
        let mut c = ctx(
            "alice",
            "delete",
            "vault/login/abc",
            &["password", "webauthn-prf"],
            1_700,
        );
        c.metadata.insert("ip".into(), "127.0.0.1".into());

        let record = decide_with_record(&e, &c);

        assert_eq!(record.engine_kind, "rbac");
        assert_eq!(record.principal, "alice");
        assert_eq!(record.action, "delete");
        assert_eq!(record.resource, "vault/login/abc");
        assert_eq!(record.time, 1_700);
        assert_eq!(record.factor_count, 2);
        assert_eq!(record.metadata_count, 1);
        assert!(record.is_allowed());
        assert_eq!(record.denial_reason(), None);
    }

    #[test]
    fn decide_with_record_captures_static_denial_reason() {
        let e = rbac_alice_admin_bob_member();
        let c = ctx("bob", "delete", "vault/login/abc", &["password"], 1_700);

        let record = decide_with_record(&e, &c);

        assert!(!record.is_allowed());
        assert_eq!(record.denial_reason(), Some(Reason::ROLE_LACKS_PERMISSION));
        assert_eq!(
            record.decision,
            Decision::Deny(Reason::ROLE_LACKS_PERMISSION)
        );
    }

    // --- AllOf ---

    #[test]
    fn all_of_allows_only_when_every_inner_allows() {
        let e1 = Box::new(rbac_alice_admin_bob_member());
        let e2 = Box::new({
            let mut x = SimpleRbacEngine::new();
            x.assign_role("alice", "admin");
            x.grant("admin", "delete", "*");
            x
        });
        let all = AllOf::new(vec![e1, e2]);
        let c_ok = ctx("alice", "delete", "x", &[], 0);
        assert_eq!(all.decide(&c_ok), Decision::Allow);

        let c_bad = ctx("bob", "delete", "x", &[], 0);
        match all.decide(&c_bad) {
            Decision::Deny(r) => assert_eq!(r, Reason::ANY_INNER_DENIED),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn all_of_empty_denies() {
        let all = AllOf::new(vec![]);
        let c = ctx("alice", "read", "x", &[], 0);
        match all.decide(&c) {
            Decision::Deny(_) => {}
            _ => panic!("expected deny on empty AllOf"),
        }
    }

    // --- AnyOf ---

    #[test]
    fn any_of_allows_if_any_inner_allows() {
        // First denies (member can't delete), second allows
        // (admin can).
        let e1 = Box::new({
            let mut x = SimpleRbacEngine::new();
            x.assign_role("bob", "member");
            x.grant("member", "read", "*");
            x
        });
        let e2 = Box::new({
            let mut x = SimpleRbacEngine::new();
            x.assign_role("bob", "owner");
            x.grant("owner", "delete", "*");
            x
        });
        let any = AnyOf::new(vec![e1, e2]);
        let c = ctx("bob", "delete", "x", &[], 0);
        assert_eq!(any.decide(&c), Decision::Allow);
    }

    #[test]
    fn any_of_denies_when_all_inner_deny() {
        let e1 = Box::new(SimpleRbacEngine::new());
        let e2 = Box::new(SimpleRbacEngine::new());
        let any = AnyOf::new(vec![e1, e2]);
        let c = ctx("alice", "read", "x", &[], 0);
        match any.decide(&c) {
            Decision::Deny(r) => assert_eq!(r, Reason::ALL_INNER_DENIED),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn any_of_empty_denies() {
        let any = AnyOf::new(vec![]);
        let c = ctx("alice", "read", "x", &[], 0);
        match any.decide(&c) {
            Decision::Deny(_) => {}
            _ => panic!("expected deny on empty AnyOf"),
        }
    }

    // --- RequireFactor ---

    #[test]
    fn require_factor_allows_when_factor_present() {
        let inner = Box::new(rbac_alice_admin_bob_member());
        let r = RequireFactor::new(inner, "webauthn-prf");
        let c = ctx("alice", "delete", "x", &["password", "webauthn-prf"], 0);
        assert_eq!(r.decide(&c), Decision::Allow);
    }

    #[test]
    fn require_factor_denies_when_factor_absent() {
        let inner = Box::new(rbac_alice_admin_bob_member());
        let r = RequireFactor::new(inner, "webauthn-prf");
        let c = ctx("alice", "delete", "x", &["password", "totp"], 0);
        match r.decide(&c) {
            Decision::Deny(reason) => assert_eq!(reason, Reason::FACTOR_REQUIRED),
            _ => panic!("expected deny"),
        }
    }

    // --- TimeBound ---

    #[test]
    fn time_bound_inside_window_allows() {
        let inner = Box::new(rbac_alice_admin_bob_member());
        let t = TimeBound::new(inner, 1000, 2000);
        let c = ctx("alice", "read", "x", &[], 1500);
        assert_eq!(t.decide(&c), Decision::Allow);
    }

    #[test]
    fn time_bound_outside_window_denies() {
        let inner = Box::new(rbac_alice_admin_bob_member());
        let t = TimeBound::new(inner, 1000, 2000);
        let early = ctx("alice", "read", "x", &[], 999);
        let late = ctx("alice", "read", "x", &[], 2001);
        match t.decide(&early) {
            Decision::Deny(r) => assert_eq!(r, Reason::OUTSIDE_TIME_WINDOW),
            _ => panic!("expected deny early"),
        }
        match t.decide(&late) {
            Decision::Deny(r) => assert_eq!(r, Reason::OUTSIDE_TIME_WINDOW),
            _ => panic!("expected deny late"),
        }
    }

    #[test]
    fn time_bound_inclusive_endpoints() {
        let inner = Box::new(rbac_alice_admin_bob_member());
        let t = TimeBound::new(inner, 1000, 2000);
        assert_eq!(
            t.decide(&ctx("alice", "read", "x", &[], 1000)),
            Decision::Allow
        );
        assert_eq!(
            t.decide(&ctx("alice", "read", "x", &[], 2000)),
            Decision::Allow
        );
    }

    // --- Composition: nested decorators ---

    #[test]
    fn nested_all_of_require_factor_time_bound() {
        // Allow `delete` only if (admin) AND (has webauthn-prf)
        // AND (within window).
        let rbac = Box::new(rbac_alice_admin_bob_member());
        let with_factor = Box::new(RequireFactor::new(rbac, "webauthn-prf"));
        let with_time = Box::new(TimeBound::new(with_factor, 1000, 2000));
        let composite = AllOf::new(vec![with_time]);

        // All conditions met.
        let c_ok = ctx("alice", "delete", "x", &["password", "webauthn-prf"], 1500);
        assert_eq!(composite.decide(&c_ok), Decision::Allow);

        // Missing webauthn-prf.
        let c_no_factor = ctx("alice", "delete", "x", &["password"], 1500);
        match composite.decide(&c_no_factor) {
            Decision::Deny(_) => {}
            _ => panic!("expected deny — factor missing"),
        }

        // Outside time.
        let c_late = ctx("alice", "delete", "x", &["password", "webauthn-prf"], 9999);
        match composite.decide(&c_late) {
            Decision::Deny(_) => {}
            _ => panic!("expected deny — outside time window"),
        }
    }

    // --- Reason inertness ---

    #[test]
    fn reasons_are_static_literals() {
        // Each reason's inner str is a 'static literal that doesn't
        // come from input. We can't introspect lifetime at runtime,
        // but we can confirm the textual content matches a fixed
        // table.
        assert_eq!(Reason::POLICY_DENIES.0, "policy denies");
        assert_eq!(Reason::UNKNOWN_PRINCIPAL.0, "unknown principal");
        assert_eq!(Reason::ROLE_LACKS_PERMISSION.0, "role lacks permission");
        assert_eq!(
            Reason::FACTOR_REQUIRED.0,
            "required authentication factor missing"
        );
        assert_eq!(Reason::OUTSIDE_TIME_WINDOW.0, "outside time window");
        assert_eq!(
            Reason::ANY_INNER_DENIED.0,
            "at least one inner engine denied"
        );
        assert_eq!(Reason::ALL_INNER_DENIED.0, "every inner engine denied");
    }
}
