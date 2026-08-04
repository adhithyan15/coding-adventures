/*
 * vault_policy.c — implementation of the pure-ISO C policy engine.
 * ===============================================================
 *
 * An engine is a tagged union: a leaf RBAC table, or a decorator that owns one
 * or more inner engines. `vp_engine_decide` is a recursive interpreter over
 * that tree; `vp_engine_free` tears the same tree down. Decisions fail closed
 * and every denial reason is a pointer into the fixed `vp_reason_*` table.
 */
#include "vault_policy.h"

#include <stdlib.h> /* malloc, realloc, free, calloc */
#include <string.h> /* strcmp, strlen, memcpy, memset */

/* ── Reasons ───────────────────────────────────────────────────────────────*/

const char *const vp_reason_policy_denies = "policy denies";
const char *const vp_reason_unknown_principal = "unknown principal";
const char *const vp_reason_role_lacks_permission = "role lacks permission";
const char *const vp_reason_factor_required =
    "required authentication factor missing";
const char *const vp_reason_outside_time_window = "outside time window";
const char *const vp_reason_any_inner_denied = "at least one inner engine denied";
const char *const vp_reason_all_inner_denied = "every inner engine denied";

/* ── Decision ──────────────────────────────────────────────────────────────*/

VpDecision vp_decision_allow(void) {
    VpDecision d = {1, NULL};
    return d;
}
VpDecision vp_decision_deny(const char *reason) {
    VpDecision d = {0, reason};
    return d;
}
int vp_decision_is_allow(VpDecision d) { return d.allow ? 1 : 0; }
int vp_decision_is_deny(VpDecision d) { return d.allow ? 0 : 1; }
const char *vp_decision_reason(VpDecision d) { return d.allow ? NULL : d.reason; }
int vp_decision_equal(VpDecision a, VpDecision b) {
    if (a.allow != b.allow) return 0;
    if (a.allow) return 1;
    if (a.reason == b.reason) return 1;
    if (a.reason == NULL || b.reason == NULL) return 0;
    return strcmp(a.reason, b.reason) == 0;
}

/* ── Small helpers ─────────────────────────────────────────────────────────*/

static char *str_dup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p != NULL) memcpy(p, s, n);
    return p;
}

/* ── Engine representation ─────────────────────────────────────────────────*/

typedef enum {
    VP_RBAC,
    VP_ALL_OF,
    VP_ANY_OF,
    VP_REQUIRE_FACTOR,
    VP_TIME_BOUND
} VpEngineKind;

typedef struct {
    char *principal;
    char *role;
} VpBinding;

typedef struct {
    char *role;
    char *action;
    char *resource; /* exact string or "*" */
} VpPerm;

struct VpEngine {
    VpEngineKind kind;
    union {
        struct {
            VpBinding *bindings;
            size_t n_bindings, cap_bindings;
            VpPerm *perms;
            size_t n_perms, cap_perms;
        } rbac;
        struct {
            VpEngine **inner;
            size_t n;
        } combo; /* ALL_OF / ANY_OF */
        struct {
            VpEngine *inner;
            char *factor_kind; /* owned copy for memory safety */
        } require_factor;
        struct {
            VpEngine *inner;
            uint64_t start, end;
        } time_bound;
    } as;
};

static VpEngine *engine_alloc(VpEngineKind kind) {
    VpEngine *e = (VpEngine *)calloc(1, sizeof(VpEngine));
    if (e != NULL) e->kind = kind;
    return e;
}

/* ── SimpleRbacEngine ──────────────────────────────────────────────────────*/

VpEngine *vp_rbac_new(void) { return engine_alloc(VP_RBAC); }

int vp_rbac_assign_role(VpEngine *e, const char *principal, const char *role) {
    if (e == NULL || e->kind != VP_RBAC) return 0;
    /* Replace an existing binding for the same principal. */
    for (size_t i = 0; i < e->as.rbac.n_bindings; i++) {
        if (strcmp(e->as.rbac.bindings[i].principal, principal) == 0) {
            char *nr = str_dup(role);
            if (nr == NULL) return 0;
            free(e->as.rbac.bindings[i].role);
            e->as.rbac.bindings[i].role = nr;
            return 1;
        }
    }
    if (e->as.rbac.n_bindings == e->as.rbac.cap_bindings) {
        size_t nc = e->as.rbac.cap_bindings ? e->as.rbac.cap_bindings : 4;
        if (nc > ((size_t)-1) / 2 / sizeof(VpBinding)) return 0;
        nc *= 2;
        VpBinding *nb =
            (VpBinding *)realloc(e->as.rbac.bindings, nc * sizeof(VpBinding));
        if (nb == NULL) return 0;
        e->as.rbac.bindings = nb;
        e->as.rbac.cap_bindings = nc;
    }
    char *dp = str_dup(principal);
    char *dr = str_dup(role);
    if (dp == NULL || dr == NULL) {
        free(dp);
        free(dr);
        return 0;
    }
    e->as.rbac.bindings[e->as.rbac.n_bindings].principal = dp;
    e->as.rbac.bindings[e->as.rbac.n_bindings].role = dr;
    e->as.rbac.n_bindings++;
    return 1;
}

int vp_rbac_grant(VpEngine *e, const char *role, const char *action,
                  const char *resource_pattern) {
    if (e == NULL || e->kind != VP_RBAC) return 0;
    /* Dedup: a grant set never holds the same triple twice. */
    for (size_t i = 0; i < e->as.rbac.n_perms; i++) {
        if (strcmp(e->as.rbac.perms[i].role, role) == 0 &&
            strcmp(e->as.rbac.perms[i].action, action) == 0 &&
            strcmp(e->as.rbac.perms[i].resource, resource_pattern) == 0) {
            return 1; /* already present */
        }
    }
    if (e->as.rbac.n_perms == e->as.rbac.cap_perms) {
        size_t nc = e->as.rbac.cap_perms ? e->as.rbac.cap_perms : 4;
        if (nc > ((size_t)-1) / 2 / sizeof(VpPerm)) return 0;
        nc *= 2;
        VpPerm *np = (VpPerm *)realloc(e->as.rbac.perms, nc * sizeof(VpPerm));
        if (np == NULL) return 0;
        e->as.rbac.perms = np;
        e->as.rbac.cap_perms = nc;
    }
    char *dr = str_dup(role);
    char *da = str_dup(action);
    char *dres = str_dup(resource_pattern);
    if (dr == NULL || da == NULL || dres == NULL) {
        free(dr);
        free(da);
        free(dres);
        return 0;
    }
    e->as.rbac.perms[e->as.rbac.n_perms].role = dr;
    e->as.rbac.perms[e->as.rbac.n_perms].action = da;
    e->as.rbac.perms[e->as.rbac.n_perms].resource = dres;
    e->as.rbac.n_perms++;
    return 1;
}

/* A tiny grow-only string set (dedup by content) for summary computations. */
typedef struct {
    const char **items;
    size_t n, cap;
} StrSet;

static int strset_contains(const StrSet *s, const char *v) {
    for (size_t i = 0; i < s->n; i++)
        if (strcmp(s->items[i], v) == 0) return 1;
    return 0;
}
/* Add if absent. Returns 0 on OOM (caller aborts the summary defensively). */
static int strset_add(StrSet *s, const char *v) {
    if (strset_contains(s, v)) return 1;
    if (s->n == s->cap) {
        size_t nc = s->cap ? s->cap : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(const char *)) return 0;
        nc *= 2;
        const char **ni =
            (const char **)realloc(s->items, nc * sizeof(const char *));
        if (ni == NULL) return 0;
        s->items = ni;
        s->cap = nc;
    }
    s->items[s->n++] = v;
    return 1;
}

VpRbacSummary vp_rbac_summary(const VpEngine *e) {
    VpRbacSummary sum;
    memset(&sum, 0, sizeof sum);
    if (e == NULL || e->kind != VP_RBAC) return sum;

    StrSet assigned = {NULL, 0, 0};
    StrSet perm_roles = {NULL, 0, 0};
    int ok = 1;
    for (size_t i = 0; i < e->as.rbac.n_bindings && ok; i++)
        ok = strset_add(&assigned, e->as.rbac.bindings[i].role);
    for (size_t i = 0; i < e->as.rbac.n_perms && ok; i++)
        ok = strset_add(&perm_roles, e->as.rbac.perms[i].role);

    if (ok) {
        sum.principal_bindings = e->as.rbac.n_bindings;
        sum.assigned_roles = assigned.n;
        sum.roles_with_permissions = perm_roles.n;
        sum.permission_grants = e->as.rbac.n_perms;
        for (size_t i = 0; i < e->as.rbac.n_perms; i++) {
            if (strcmp(e->as.rbac.perms[i].resource, "*") == 0)
                sum.wildcard_resource_grants++;
            else
                sum.exact_resource_grants++;
        }
        /* unique = |assigned ∪ perm_roles|, split into the two differences. */
        size_t only_perm = 0, only_assigned = 0, in_both = 0;
        for (size_t i = 0; i < perm_roles.n; i++) {
            if (strset_contains(&assigned, perm_roles.items[i]))
                in_both++;
            else
                only_perm++;
        }
        for (size_t i = 0; i < assigned.n; i++)
            if (!strset_contains(&perm_roles, assigned.items[i])) only_assigned++;
        sum.unique_roles = in_both + only_perm + only_assigned;
        sum.permission_roles_without_principals = only_perm;
        sum.assigned_roles_without_permissions = only_assigned;
    }
    free(assigned.items);
    free(perm_roles.items);
    return sum;
}

/* ── Decorators ────────────────────────────────────────────────────────────*/

static void free_inner_array(VpEngine **inner, size_t n) {
    for (size_t i = 0; i < n; i++) vp_engine_free(inner[i]);
}

static VpEngine *make_combo(VpEngineKind kind, VpEngine **inner, size_t n) {
    VpEngine *e = engine_alloc(kind);
    if (e == NULL) {
        free_inner_array(inner, n);
        return NULL;
    }
    if (n > 0) {
        e->as.combo.inner = (VpEngine **)malloc(n * sizeof(VpEngine *));
        if (e->as.combo.inner == NULL) {
            free_inner_array(inner, n);
            free(e);
            return NULL;
        }
        memcpy(e->as.combo.inner, inner, n * sizeof(VpEngine *));
    }
    e->as.combo.n = n;
    return e;
}

VpEngine *vp_all_of(VpEngine **inner, size_t n) {
    return make_combo(VP_ALL_OF, inner, n);
}
VpEngine *vp_any_of(VpEngine **inner, size_t n) {
    return make_combo(VP_ANY_OF, inner, n);
}

VpEngine *vp_require_factor(VpEngine *inner, const char *factor_kind) {
    VpEngine *e = engine_alloc(VP_REQUIRE_FACTOR);
    char *fk = str_dup(factor_kind);
    if (e == NULL || fk == NULL) {
        vp_engine_free(inner);
        free(e);
        free(fk);
        return NULL;
    }
    e->as.require_factor.inner = inner;
    e->as.require_factor.factor_kind = fk;
    return e;
}

VpEngine *vp_time_bound(VpEngine *inner, uint64_t start, uint64_t end) {
    VpEngine *e = engine_alloc(VP_TIME_BOUND);
    if (e == NULL) {
        vp_engine_free(inner);
        return NULL;
    }
    e->as.time_bound.inner = inner;
    e->as.time_bound.start = start;
    e->as.time_bound.end = end;
    return e;
}

/* ── Interpreter ───────────────────────────────────────────────────────────*/

const char *vp_engine_kind(const VpEngine *e) {
    if (e == NULL) return "";
    switch (e->kind) {
        case VP_RBAC: return "rbac";
        case VP_ALL_OF: return "all-of";
        case VP_ANY_OF: return "any-of";
        case VP_REQUIRE_FACTOR: return "require-factor";
        case VP_TIME_BOUND: return "time-bound";
    }
    return "";
}

static int factors_contain(const VpContext *ctx, const char *kind) {
    for (size_t i = 0; i < ctx->n_factors; i++)
        if (strcmp(ctx->factors[i], kind) == 0) return 1;
    return 0;
}

static VpDecision rbac_decide(const VpEngine *e, const VpContext *ctx) {
    const char *role = NULL;
    for (size_t i = 0; i < e->as.rbac.n_bindings; i++) {
        if (strcmp(e->as.rbac.bindings[i].principal, ctx->principal) == 0) {
            role = e->as.rbac.bindings[i].role;
            break;
        }
    }
    if (role == NULL) return vp_decision_deny(vp_reason_unknown_principal);
    for (size_t i = 0; i < e->as.rbac.n_perms; i++) {
        const VpPerm *p = &e->as.rbac.perms[i];
        if (strcmp(p->role, role) == 0 && strcmp(p->action, ctx->action) == 0 &&
            (strcmp(p->resource, "*") == 0 ||
             strcmp(p->resource, ctx->resource) == 0)) {
            return vp_decision_allow();
        }
    }
    return vp_decision_deny(vp_reason_role_lacks_permission);
}

VpDecision vp_engine_decide(const VpEngine *e, const VpContext *ctx) {
    switch (e->kind) {
        case VP_RBAC:
            return rbac_decide(e, ctx);
        case VP_ALL_OF:
            if (e->as.combo.n == 0)
                return vp_decision_deny(vp_reason_policy_denies);
            for (size_t i = 0; i < e->as.combo.n; i++)
                if (vp_engine_decide(e->as.combo.inner[i], ctx).allow == 0)
                    return vp_decision_deny(vp_reason_any_inner_denied);
            return vp_decision_allow();
        case VP_ANY_OF:
            if (e->as.combo.n == 0)
                return vp_decision_deny(vp_reason_policy_denies);
            for (size_t i = 0; i < e->as.combo.n; i++)
                if (vp_engine_decide(e->as.combo.inner[i], ctx).allow)
                    return vp_decision_allow();
            return vp_decision_deny(vp_reason_all_inner_denied);
        case VP_REQUIRE_FACTOR:
            if (!factors_contain(ctx, e->as.require_factor.factor_kind))
                return vp_decision_deny(vp_reason_factor_required);
            return vp_engine_decide(e->as.require_factor.inner, ctx);
        case VP_TIME_BOUND:
            if (ctx->time < e->as.time_bound.start ||
                ctx->time > e->as.time_bound.end)
                return vp_decision_deny(vp_reason_outside_time_window);
            return vp_engine_decide(e->as.time_bound.inner, ctx);
    }
    return vp_decision_deny(vp_reason_policy_denies); /* unreachable */
}

void vp_engine_free(VpEngine *e) {
    if (e == NULL) return;
    switch (e->kind) {
        case VP_RBAC:
            for (size_t i = 0; i < e->as.rbac.n_bindings; i++) {
                free(e->as.rbac.bindings[i].principal);
                free(e->as.rbac.bindings[i].role);
            }
            free(e->as.rbac.bindings);
            for (size_t i = 0; i < e->as.rbac.n_perms; i++) {
                free(e->as.rbac.perms[i].role);
                free(e->as.rbac.perms[i].action);
                free(e->as.rbac.perms[i].resource);
            }
            free(e->as.rbac.perms);
            break;
        case VP_ALL_OF:
        case VP_ANY_OF:
            for (size_t i = 0; i < e->as.combo.n; i++)
                vp_engine_free(e->as.combo.inner[i]);
            free(e->as.combo.inner);
            break;
        case VP_REQUIRE_FACTOR:
            vp_engine_free(e->as.require_factor.inner);
            free(e->as.require_factor.factor_kind);
            break;
        case VP_TIME_BOUND:
            vp_engine_free(e->as.time_bound.inner);
            break;
    }
    free(e);
}

/* ── Decision record ───────────────────────────────────────────────────────*/

int vp_decide_with_record(const VpEngine *e, const VpContext *ctx,
                          VpDecisionRecord *out) {
    VpDecision decision = vp_engine_decide(e, ctx);
    char *p = str_dup(ctx->principal);
    char *a = str_dup(ctx->action);
    char *r = str_dup(ctx->resource);
    if (p == NULL || a == NULL || r == NULL) {
        free(p);
        free(a);
        free(r);
        return 0;
    }
    out->engine_kind = vp_engine_kind(e);
    out->principal = p;
    out->action = a;
    out->resource = r;
    out->time = ctx->time;
    out->factor_count = ctx->n_factors;
    out->metadata_count = ctx->n_metadata;
    out->decision = decision;
    return 1;
}

int vp_record_is_allowed(const VpDecisionRecord *r) {
    return vp_decision_is_allow(r->decision);
}
const char *vp_record_denial_reason(const VpDecisionRecord *r) {
    return vp_decision_reason(r->decision);
}
void vp_record_free(VpDecisionRecord *r) {
    if (r == NULL) return;
    free(r->principal);
    free(r->action);
    free(r->resource);
    r->principal = NULL;
    r->action = NULL;
    r->resource = NULL;
}
