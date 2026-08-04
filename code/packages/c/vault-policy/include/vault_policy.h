/*
 * vault_policy.h — pluggable authorization policy engine, pure ISO C17.
 * ====================================================================
 *
 * A faithful port of the Rust `vault-policy` crate (VLT06). Authentication
 * says *who*; this layer says *what they can do*. You build a policy engine
 * out of a role-based table plus composable decorators, then ask it to decide
 * a request described by a `VpContext`.
 *
 *   VP_RBAC            role × (action, resource-pattern) table
 *   VP_ALL_OF          boolean AND of inner engines (all must allow)
 *   VP_ANY_OF          boolean OR of inner engines (any allow wins)
 *   VP_REQUIRE_FACTOR  wrap an engine, also require an auth factor present
 *   VP_TIME_BOUND      wrap an engine, only allow within [start, end] UNIX secs
 *
 * ## Ownership
 *
 * An engine (`VpEngine *`) is an owned tree: the decorator constructors TAKE
 * OWNERSHIP of the inner engines handed to them, so you compose bottom-up and
 * release the whole tree with a single `vp_engine_free`. A constructor returns
 * NULL on allocation failure, having freed the inner engines you passed in.
 *
 * A `VpContext` is a borrowed view: it holds pointers to caller-owned strings
 * and arrays and is not freed. `vp_engine_decide` only reads it.
 *
 * ## Decisions fail closed
 *
 * Anything the engine cannot confidently allow is denied with a static reason
 * string drawn from a fixed table (`vp_reason_*`) — never attacker-controlled
 * bytes, so a malicious principal name cannot inject content into logs.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no compiler extensions.
 */
#ifndef VAULT_POLICY_H
#define VAULT_POLICY_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Reasons ───────────────────────────────────────────────────────────────
 *
 * Static-literal denial reasons. Each is a fixed string constant; the engine
 * never constructs a reason from input bytes. Compare returned reasons by
 * pointer identity (they are the same objects) or by strcmp. */
extern const char *const vp_reason_policy_denies;        /* "policy denies" */
extern const char *const vp_reason_unknown_principal;    /* "unknown principal" */
extern const char *const vp_reason_role_lacks_permission;/* "role lacks permission" */
extern const char *const vp_reason_factor_required;      /* required factor missing */
extern const char *const vp_reason_outside_time_window;  /* "outside time window" */
extern const char *const vp_reason_any_inner_denied;     /* AllOf: one inner denied */
extern const char *const vp_reason_all_inner_denied;     /* AnyOf: every inner denied */

/* ── Decision ──────────────────────────────────────────────────────────────
 *
 * `allow` is 1 for Allow, 0 for Deny. On a Deny, `reason` points at one of the
 * vp_reason_* literals; on an Allow it is NULL. */
typedef struct {
    int allow;
    const char *reason;
} VpDecision;

VpDecision vp_decision_allow(void);
VpDecision vp_decision_deny(const char *reason);
int vp_decision_is_allow(VpDecision d);
int vp_decision_is_deny(VpDecision d);
/* Returns the denial reason, or NULL when this is an allow. */
const char *vp_decision_reason(VpDecision d);
/* Structural equality: same allow flag and (for denials) same reason text. */
int vp_decision_equal(VpDecision a, VpDecision b);

/* ── Context ───────────────────────────────────────────────────────────────
 *
 * A borrowed decision request. All pointers reference caller-owned memory that
 * must outlive any call that reads the context. Nothing here is freed by the
 * library. */
typedef struct {
    const char *principal;      /* stable requester id */
    const char *action;         /* e.g. "read", "delete" */
    const char *resource;       /* e.g. "vault/login/abc123" */
    const char *const *factors; /* auth-factor kind strings */
    size_t n_factors;
    uint64_t time;                 /* UNIX seconds at decision time */
    const char *const *meta_keys;  /* metadata bag keys (values unused here) */
    const char *const *meta_vals;  /* metadata bag values */
    size_t n_metadata;
} VpContext;

/* ── Engines ───────────────────────────────────────────────────────────────*/

typedef struct VpEngine VpEngine;

/* -- SimpleRbacEngine builders -- */

/* New empty RBAC engine — denies everything until roles/grants are added.
 * Returns NULL on allocation failure. */
VpEngine *vp_rbac_new(void);
/* Bind `principal` to a single `role`, replacing any existing binding.
 * Copies the strings. Returns 0 on allocation failure (engine unchanged). */
int vp_rbac_assign_role(VpEngine *e, const char *principal, const char *role);
/* Grant `(action, resource_pattern)` to `role`; `resource_pattern` is either an
 * exact resource string or "*". Duplicate grants collapse. Copies the strings.
 * Returns 0 on allocation failure. */
int vp_rbac_grant(VpEngine *e, const char *role, const char *action,
                  const char *resource_pattern);

/* Count-only view of an RBAC table's shape (exposes no names). */
typedef struct {
    size_t principal_bindings;
    size_t unique_roles;
    size_t assigned_roles;
    size_t roles_with_permissions;
    size_t permission_grants;
    size_t wildcard_resource_grants;
    size_t exact_resource_grants;
    size_t permission_roles_without_principals;
    size_t assigned_roles_without_permissions;
} VpRbacSummary;

/* Summarise an RBAC engine's table shape. If `e` is not an RBAC engine, returns
 * an all-zero summary. */
VpRbacSummary vp_rbac_summary(const VpEngine *e);

/* -- Decorators (take ownership of their inner engines) -- */

/* Boolean AND: allows only if every inner engine allows. Consumes `inner[0..n)`
 * (the array itself is not freed). Empty list denies. NULL on OOM. */
VpEngine *vp_all_of(VpEngine **inner, size_t n);
/* Boolean OR: allows if any inner engine allows. Consumes `inner[0..n)`. Empty
 * list denies. NULL on OOM. */
VpEngine *vp_any_of(VpEngine **inner, size_t n);
/* Require `factor_kind` (a static string) present in ctx.factors, then defer to
 * `inner`. Consumes `inner`. NULL on OOM. */
VpEngine *vp_require_factor(VpEngine *inner, const char *factor_kind);
/* Only forward `inner`'s decision when ctx.time is in [start, end] inclusive.
 * Consumes `inner`. NULL on OOM. */
VpEngine *vp_time_bound(VpEngine *inner, uint64_t start, uint64_t end);

/* Stable kind string for telemetry: "rbac", "all-of", "any-of",
 * "require-factor", "time-bound". */
const char *vp_engine_kind(const VpEngine *e);
/* Decide a request. Pure over `ctx` — never touches the clock, network, or
 * filesystem. */
VpDecision vp_engine_decide(const VpEngine *e, const VpContext *ctx);
/* Release an engine and, recursively, every engine it owns. NULL-safe. */
void vp_engine_free(VpEngine *e);

/* ── Decision record ───────────────────────────────────────────────────────
 *
 * A compact owned view of one decision, safe to hand to an audit sink without
 * exposing the original metadata. Copies only identifiers and counts. */
typedef struct {
    const char *engine_kind; /* static string, borrowed from the engine */
    char *principal;         /* owned copy */
    char *action;            /* owned copy */
    char *resource;          /* owned copy */
    uint64_t time;
    size_t factor_count;
    size_t metadata_count;
    VpDecision decision;
} VpDecisionRecord;

/* Decide with `e` and capture a compact record. Returns 1 on success (fills
 * *out), 0 on allocation failure. Release with vp_record_free. */
int vp_decide_with_record(const VpEngine *e, const VpContext *ctx,
                          VpDecisionRecord *out);
int vp_record_is_allowed(const VpDecisionRecord *r);
/* Denial reason, or NULL if the record captured an allow. */
const char *vp_record_denial_reason(const VpDecisionRecord *r);
void vp_record_free(VpDecisionRecord *r);

#ifdef __cplusplus
}
#endif

#endif /* VAULT_POLICY_H */
