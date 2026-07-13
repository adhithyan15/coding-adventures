/*
 * Tests for vault-policy, using the header-only iso_test.h harness (pure ISO).
 * Cases mirror the Rust crate's own unit tests.
 */
#include "iso_test.h"

#include <stdlib.h> /* NULL */

#include "vault_policy.h"

/* Build a borrowed context. Metadata is empty unless a test overrides it. */
static VpContext mk_ctx(const char *principal, const char *action,
                        const char *resource, const char *const *factors,
                        size_t nf, uint64_t time) {
    VpContext c;
    c.principal = principal;
    c.action = action;
    c.resource = resource;
    c.factors = factors;
    c.n_factors = nf;
    c.time = time;
    c.meta_keys = NULL;
    c.meta_vals = NULL;
    c.n_metadata = 0;
    return c;
}

/* alice=admin (read/write/delete on *), bob=member (read on *). */
static VpEngine *rbac_fixture(void) {
    VpEngine *e = vp_rbac_new();
    vp_rbac_assign_role(e, "alice", "admin");
    vp_rbac_assign_role(e, "bob", "member");
    vp_rbac_grant(e, "admin", "read", "*");
    vp_rbac_grant(e, "admin", "write", "*");
    vp_rbac_grant(e, "admin", "delete", "*");
    vp_rbac_grant(e, "member", "read", "*");
    return e;
}

int main(void) {
    static const char *const f_pw[] = {"password"};
    static const char *const f_none[] = {NULL}; /* unused; nf=0 */

    /* ── SimpleRbacEngine ───────────────────────────────────────────────────*/
    { /* admin can delete */
        VpEngine *e = rbac_fixture();
        VpContext c = mk_ctx("alice", "delete", "vault/login/abc", f_pw, 1, 0);
        ISO_CHECK(vp_engine_decide(e, &c).allow);
        vp_engine_free(e);
    }
    { /* member cannot delete -> role lacks permission */
        VpEngine *e = rbac_fixture();
        VpContext c = mk_ctx("bob", "delete", "vault/login/abc", f_pw, 1, 0);
        VpDecision d = vp_engine_decide(e, &c);
        ISO_CHECK(vp_decision_is_deny(d));
        ISO_CHECK(d.reason == vp_reason_role_lacks_permission);
        vp_engine_free(e);
    }
    { /* unknown principal -> specific reason */
        VpEngine *e = rbac_fixture();
        VpContext c = mk_ctx("eve", "read", "vault/login/abc", f_none, 0, 0);
        VpDecision d = vp_engine_decide(e, &c);
        ISO_CHECK(vp_decision_is_deny(d));
        ISO_CHECK(d.reason == vp_reason_unknown_principal);
        vp_engine_free(e);
    }
    { /* member reads any resource via wildcard grant */
        VpEngine *e = rbac_fixture();
        VpContext c = mk_ctx("bob", "read", "vault/login/anything", f_none, 0, 0);
        ISO_CHECK(vp_engine_decide(e, &c).allow);
        vp_engine_free(e);
    }
    { /* exact-resource grant matches only that resource */
        VpEngine *e = vp_rbac_new();
        vp_rbac_assign_role(e, "alice", "narrow");
        vp_rbac_grant(e, "narrow", "read", "vault/login/specific");
        VpContext c1 =
            mk_ctx("alice", "read", "vault/login/specific", f_none, 0, 0);
        VpContext c2 = mk_ctx("alice", "read", "vault/login/other", f_none, 0, 0);
        ISO_CHECK(vp_engine_decide(e, &c1).allow);
        VpDecision d = vp_engine_decide(e, &c2);
        ISO_CHECK(vp_decision_is_deny(d));
        ISO_CHECK(d.reason == vp_reason_role_lacks_permission);
        vp_engine_free(e);
    }
    { /* assigned role with no perms denies */
        VpEngine *e = vp_rbac_new();
        vp_rbac_assign_role(e, "alice", "no-perms");
        VpContext c = mk_ctx("alice", "read", "x", f_none, 0, 0);
        VpDecision d = vp_engine_decide(e, &c);
        ISO_CHECK(vp_decision_is_deny(d));
        ISO_CHECK(d.reason == vp_reason_role_lacks_permission);
        vp_engine_free(e);
    }

    /* ── Summary ────────────────────────────────────────────────────────────*/
    { /* counts the table shape */
        VpEngine *e = rbac_fixture();
        VpRbacSummary s = vp_rbac_summary(e);
        ISO_CHECK(s.principal_bindings == 2);
        ISO_CHECK(s.unique_roles == 2);
        ISO_CHECK(s.assigned_roles == 2);
        ISO_CHECK(s.roles_with_permissions == 2);
        ISO_CHECK(s.permission_grants == 4);
        ISO_CHECK(s.wildcard_resource_grants == 4);
        ISO_CHECK(s.exact_resource_grants == 0);
        ISO_CHECK(s.permission_roles_without_principals == 0);
        ISO_CHECK(s.assigned_roles_without_permissions == 0);
        vp_engine_free(e);
    }
    { /* orphaned and empty roles */
        VpEngine *e = vp_rbac_new();
        vp_rbac_assign_role(e, "alice", "admin");
        vp_rbac_assign_role(e, "carol", "empty");
        vp_rbac_grant(e, "admin", "read", "vault/login/specific");
        vp_rbac_grant(e, "orphan", "read", "*");
        VpRbacSummary s = vp_rbac_summary(e);
        ISO_CHECK(s.principal_bindings == 2);
        ISO_CHECK(s.unique_roles == 3);
        ISO_CHECK(s.assigned_roles == 2);
        ISO_CHECK(s.roles_with_permissions == 2);
        ISO_CHECK(s.permission_grants == 2);
        ISO_CHECK(s.wildcard_resource_grants == 1);
        ISO_CHECK(s.exact_resource_grants == 1);
        ISO_CHECK(s.permission_roles_without_principals == 1);
        ISO_CHECK(s.assigned_roles_without_permissions == 1);
        vp_engine_free(e);
    }
    { /* empty engine -> all-zero summary */
        VpEngine *e = vp_rbac_new();
        VpRbacSummary s = vp_rbac_summary(e);
        ISO_CHECK(s.principal_bindings == 0 && s.unique_roles == 0 &&
                  s.permission_grants == 0);
        vp_engine_free(e);
    }

    /* ── Decision helpers ───────────────────────────────────────────────────*/
    {
        ISO_CHECK(vp_decision_is_allow(vp_decision_allow()));
        ISO_CHECK(!vp_decision_is_deny(vp_decision_allow()));
        ISO_CHECK(vp_decision_reason(vp_decision_allow()) == NULL);
        VpDecision denial = vp_decision_deny(vp_reason_policy_denies);
        ISO_CHECK(vp_decision_is_deny(denial));
        ISO_CHECK(vp_decision_reason(denial) == vp_reason_policy_denies);
    }

    /* ── decide_with_record ─────────────────────────────────────────────────*/
    { /* compact allow shape */
        VpEngine *e = rbac_fixture();
        static const char *const f2[] = {"password", "webauthn-prf"};
        static const char *const mk[] = {"ip"};
        static const char *const mv[] = {"127.0.0.1"};
        VpContext c = mk_ctx("alice", "delete", "vault/login/abc", f2, 2, 1700);
        c.meta_keys = mk;
        c.meta_vals = mv;
        c.n_metadata = 1;
        VpDecisionRecord rec;
        ISO_CHECK(vp_decide_with_record(e, &c, &rec));
        ISO_CHECK_STR_EQ(rec.engine_kind, "rbac");
        ISO_CHECK_STR_EQ(rec.principal, "alice");
        ISO_CHECK_STR_EQ(rec.action, "delete");
        ISO_CHECK_STR_EQ(rec.resource, "vault/login/abc");
        ISO_CHECK(rec.time == 1700);
        ISO_CHECK(rec.factor_count == 2);
        ISO_CHECK(rec.metadata_count == 1);
        ISO_CHECK(vp_record_is_allowed(&rec));
        ISO_CHECK(vp_record_denial_reason(&rec) == NULL);
        vp_record_free(&rec);
        vp_engine_free(e);
    }
    { /* static denial reason */
        VpEngine *e = rbac_fixture();
        VpContext c = mk_ctx("bob", "delete", "vault/login/abc", f_pw, 1, 1700);
        VpDecisionRecord rec;
        ISO_CHECK(vp_decide_with_record(e, &c, &rec));
        ISO_CHECK(!vp_record_is_allowed(&rec));
        ISO_CHECK(vp_record_denial_reason(&rec) ==
                  vp_reason_role_lacks_permission);
        ISO_CHECK(vp_decision_equal(
            rec.decision, vp_decision_deny(vp_reason_role_lacks_permission)));
        vp_record_free(&rec);
        vp_engine_free(e);
    }

    /* ── AllOf ──────────────────────────────────────────────────────────────*/
    { /* allows only when every inner allows */
        VpEngine *e2 = vp_rbac_new();
        vp_rbac_assign_role(e2, "alice", "admin");
        vp_rbac_grant(e2, "admin", "delete", "*");
        VpEngine *inner[2] = {rbac_fixture(), e2};
        VpEngine *all = vp_all_of(inner, 2);
        VpContext c_ok = mk_ctx("alice", "delete", "x", f_none, 0, 0);
        ISO_CHECK(vp_engine_decide(all, &c_ok).allow);
        VpContext c_bad = mk_ctx("bob", "delete", "x", f_none, 0, 0);
        VpDecision d = vp_engine_decide(all, &c_bad);
        ISO_CHECK(vp_decision_is_deny(d));
        ISO_CHECK(d.reason == vp_reason_any_inner_denied);
        vp_engine_free(all);
    }
    { /* empty AllOf denies */
        VpEngine *all = vp_all_of(NULL, 0);
        VpContext c = mk_ctx("alice", "read", "x", f_none, 0, 0);
        ISO_CHECK(vp_decision_is_deny(vp_engine_decide(all, &c)));
        vp_engine_free(all);
    }

    /* ── AnyOf ──────────────────────────────────────────────────────────────*/
    { /* allows if any inner allows */
        VpEngine *e1 = vp_rbac_new();
        vp_rbac_assign_role(e1, "bob", "member");
        vp_rbac_grant(e1, "member", "read", "*");
        VpEngine *e2 = vp_rbac_new();
        vp_rbac_assign_role(e2, "bob", "owner");
        vp_rbac_grant(e2, "owner", "delete", "*");
        VpEngine *inner[2] = {e1, e2};
        VpEngine *any = vp_any_of(inner, 2);
        VpContext c = mk_ctx("bob", "delete", "x", f_none, 0, 0);
        ISO_CHECK(vp_engine_decide(any, &c).allow);
        vp_engine_free(any);
    }
    { /* denies when all inner deny */
        VpEngine *inner[2] = {vp_rbac_new(), vp_rbac_new()};
        VpEngine *any = vp_any_of(inner, 2);
        VpContext c = mk_ctx("alice", "read", "x", f_none, 0, 0);
        VpDecision d = vp_engine_decide(any, &c);
        ISO_CHECK(vp_decision_is_deny(d));
        ISO_CHECK(d.reason == vp_reason_all_inner_denied);
        vp_engine_free(any);
    }
    { /* empty AnyOf denies */
        VpEngine *any = vp_any_of(NULL, 0);
        VpContext c = mk_ctx("alice", "read", "x", f_none, 0, 0);
        ISO_CHECK(vp_decision_is_deny(vp_engine_decide(any, &c)));
        vp_engine_free(any);
    }

    /* ── RequireFactor ──────────────────────────────────────────────────────*/
    { /* allows when factor present */
        VpEngine *r = vp_require_factor(rbac_fixture(), "webauthn-prf");
        static const char *const f[] = {"password", "webauthn-prf"};
        VpContext c = mk_ctx("alice", "delete", "x", f, 2, 0);
        ISO_CHECK(vp_engine_decide(r, &c).allow);
        vp_engine_free(r);
    }
    { /* denies when factor absent */
        VpEngine *r = vp_require_factor(rbac_fixture(), "webauthn-prf");
        static const char *const f[] = {"password", "totp"};
        VpContext c = mk_ctx("alice", "delete", "x", f, 2, 0);
        VpDecision d = vp_engine_decide(r, &c);
        ISO_CHECK(vp_decision_is_deny(d));
        ISO_CHECK(d.reason == vp_reason_factor_required);
        vp_engine_free(r);
    }

    /* ── TimeBound ──────────────────────────────────────────────────────────*/
    { /* inside window allows */
        VpEngine *t = vp_time_bound(rbac_fixture(), 1000, 2000);
        VpContext c = mk_ctx("alice", "read", "x", f_none, 0, 1500);
        ISO_CHECK(vp_engine_decide(t, &c).allow);
        vp_engine_free(t);
    }
    { /* outside window denies (early and late) */
        VpEngine *t = vp_time_bound(rbac_fixture(), 1000, 2000);
        VpContext early = mk_ctx("alice", "read", "x", f_none, 0, 999);
        VpContext late = mk_ctx("alice", "read", "x", f_none, 0, 2001);
        VpDecision de = vp_engine_decide(t, &early);
        VpDecision dl = vp_engine_decide(t, &late);
        ISO_CHECK(de.reason == vp_reason_outside_time_window);
        ISO_CHECK(dl.reason == vp_reason_outside_time_window);
        vp_engine_free(t);
    }
    { /* inclusive endpoints */
        VpEngine *t = vp_time_bound(rbac_fixture(), 1000, 2000);
        VpContext lo = mk_ctx("alice", "read", "x", f_none, 0, 1000);
        VpContext hi = mk_ctx("alice", "read", "x", f_none, 0, 2000);
        ISO_CHECK(vp_engine_decide(t, &lo).allow);
        ISO_CHECK(vp_engine_decide(t, &hi).allow);
        vp_engine_free(t);
    }

    /* ── Nested composition ─────────────────────────────────────────────────*/
    { /* AllOf( TimeBound( RequireFactor( rbac ) ) ) */
        VpEngine *rbac = rbac_fixture();
        VpEngine *wf = vp_require_factor(rbac, "webauthn-prf");
        VpEngine *wt = vp_time_bound(wf, 1000, 2000);
        VpEngine *arr[1] = {wt};
        VpEngine *composite = vp_all_of(arr, 1);
        static const char *const f_ok[] = {"password", "webauthn-prf"};

        VpContext c_ok = mk_ctx("alice", "delete", "x", f_ok, 2, 1500);
        ISO_CHECK(vp_engine_decide(composite, &c_ok).allow);
        VpContext c_no_factor = mk_ctx("alice", "delete", "x", f_pw, 1, 1500);
        ISO_CHECK(vp_decision_is_deny(vp_engine_decide(composite, &c_no_factor)));
        VpContext c_late = mk_ctx("alice", "delete", "x", f_ok, 2, 9999);
        ISO_CHECK(vp_decision_is_deny(vp_engine_decide(composite, &c_late)));
        vp_engine_free(composite);
    }

    /* ── Reason inertness ───────────────────────────────────────────────────*/
    {
        ISO_CHECK_STR_EQ(vp_reason_policy_denies, "policy denies");
        ISO_CHECK_STR_EQ(vp_reason_unknown_principal, "unknown principal");
        ISO_CHECK_STR_EQ(vp_reason_role_lacks_permission,
                         "role lacks permission");
        ISO_CHECK_STR_EQ(vp_reason_factor_required,
                         "required authentication factor missing");
        ISO_CHECK_STR_EQ(vp_reason_outside_time_window, "outside time window");
        ISO_CHECK_STR_EQ(vp_reason_any_inner_denied,
                         "at least one inner engine denied");
        ISO_CHECK_STR_EQ(vp_reason_all_inner_denied, "every inner engine denied");
    }

    return ISO_TEST_RESULT();
}
