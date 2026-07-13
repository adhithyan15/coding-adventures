// Tests for the C++ vault-policy library, using the header-only iso_test.h
// harness (pure ISO). Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <memory>
#include <string>
#include <vector>

#include "vault_policy.hpp"

namespace vp = ca::vault_policy;
using vp::Decision;
using vp::PolicyContext;
using vp::PolicyEngine;
using vp::Reason;
using vp::SimpleRbacEngine;

// A context with empty metadata unless a test fills it in.
static PolicyContext mk_ctx(std::string principal, std::string action,
                            std::string resource,
                            std::vector<std::string> factors,
                            std::uint64_t time) {
    PolicyContext c;
    c.principal = std::move(principal);
    c.action = std::move(action);
    c.resource = std::move(resource);
    c.factors = std::move(factors);
    c.time = time;
    return c;
}

static SimpleRbacEngine rbac_fixture() {
    SimpleRbacEngine e;
    e.assign_role("alice", "admin");
    e.assign_role("bob", "member");
    e.grant("admin", "read", "*");
    e.grant("admin", "write", "*");
    e.grant("admin", "delete", "*");
    e.grant("member", "read", "*");
    return e;
}

static std::unique_ptr<PolicyEngine> rbac_fixture_ptr() {
    return std::make_unique<SimpleRbacEngine>(rbac_fixture());
}

int main() {
    // ── SimpleRbacEngine ─────────────────────────────────────────────────────
    {
        auto e = rbac_fixture();
        ISO_CHECK(e.decide(mk_ctx("alice", "delete", "vault/login/abc",
                                  {"password"}, 0)) == Decision::allow());
    }
    {
        auto e = rbac_fixture();
        auto d = e.decide(mk_ctx("bob", "delete", "vault/login/abc",
                                 {"password"}, 0));
        ISO_CHECK(d.is_deny());
        ISO_CHECK(d.reason() == Reason::RoleLacksPermission);
    }
    {
        auto e = rbac_fixture();
        auto d = e.decide(mk_ctx("eve", "read", "vault/login/abc", {}, 0));
        ISO_CHECK(d.reason() == Reason::UnknownPrincipal);
    }
    {
        auto e = rbac_fixture();
        ISO_CHECK(e.decide(mk_ctx("bob", "read", "vault/login/anything", {}, 0))
                      .is_allow());
    }
    {
        SimpleRbacEngine e;
        e.assign_role("alice", "narrow");
        e.grant("narrow", "read", "vault/login/specific");
        ISO_CHECK(e.decide(mk_ctx("alice", "read", "vault/login/specific", {}, 0))
                      .is_allow());
        auto d = e.decide(mk_ctx("alice", "read", "vault/login/other", {}, 0));
        ISO_CHECK(d.reason() == Reason::RoleLacksPermission);
    }
    {
        SimpleRbacEngine e;
        e.assign_role("alice", "no-perms");
        auto d = e.decide(mk_ctx("alice", "read", "x", {}, 0));
        ISO_CHECK(d.reason() == Reason::RoleLacksPermission);
    }

    // ── Summary ──────────────────────────────────────────────────────────────
    {
        auto e = rbac_fixture();
        vp::SimpleRbacSummary want;
        want.principal_bindings = 2;
        want.unique_roles = 2;
        want.assigned_roles = 2;
        want.roles_with_permissions = 2;
        want.permission_grants = 4;
        want.wildcard_resource_grants = 4;
        want.exact_resource_grants = 0;
        ISO_CHECK(e.summary() == want);
        ISO_CHECK(e.summary().has_principal_bindings());
        ISO_CHECK(e.summary().has_permission_grants());
        ISO_CHECK(!e.summary().has_permission_roles_without_principals());
        ISO_CHECK(!e.summary().has_assigned_roles_without_permissions());
    }
    {
        SimpleRbacEngine e;
        e.assign_role("alice", "admin");
        e.assign_role("carol", "empty");
        e.grant("admin", "read", "vault/login/specific");
        e.grant("orphan", "read", "*");
        auto s = e.summary();
        ISO_CHECK(s.principal_bindings == 2);
        ISO_CHECK(s.unique_roles == 3);
        ISO_CHECK(s.assigned_roles == 2);
        ISO_CHECK(s.roles_with_permissions == 2);
        ISO_CHECK(s.permission_grants == 2);
        ISO_CHECK(s.wildcard_resource_grants == 1);
        ISO_CHECK(s.exact_resource_grants == 1);
        ISO_CHECK(s.permission_roles_without_principals == 1);
        ISO_CHECK(s.assigned_roles_without_permissions == 1);
        ISO_CHECK(s.has_permission_roles_without_principals());
        ISO_CHECK(s.has_assigned_roles_without_permissions());
    }
    {
        SimpleRbacEngine e;
        ISO_CHECK(e.summary() == vp::SimpleRbacSummary{});
        ISO_CHECK(!e.summary().has_principal_bindings());
        ISO_CHECK(!e.summary().has_permission_grants());
    }

    // ── Decision helpers ─────────────────────────────────────────────────────
    {
        ISO_CHECK(Decision::allow().is_allow());
        ISO_CHECK(!Decision::allow().is_deny());
        ISO_CHECK(!Decision::allow().reason().has_value());
        auto denial = Decision::deny(Reason::PolicyDenies);
        ISO_CHECK(denial.is_deny());
        ISO_CHECK(denial.reason() == std::optional<Reason>(Reason::PolicyDenies));
    }

    // ── decide_with_record ───────────────────────────────────────────────────
    {
        auto e = rbac_fixture();
        auto c = mk_ctx("alice", "delete", "vault/login/abc",
                        {"password", "webauthn-prf"}, 1700);
        c.metadata.emplace("ip", "127.0.0.1");
        auto rec = vp::decide_with_record(e, c);
        ISO_CHECK(std::string(rec.engine_kind) == "rbac");
        ISO_CHECK(rec.principal == "alice");
        ISO_CHECK(rec.action == "delete");
        ISO_CHECK(rec.resource == "vault/login/abc");
        ISO_CHECK(rec.time == 1700);
        ISO_CHECK(rec.factor_count == 2);
        ISO_CHECK(rec.metadata_count == 1);
        ISO_CHECK(rec.is_allowed());
        ISO_CHECK(!rec.denial_reason().has_value());
    }
    {
        auto e = rbac_fixture();
        auto c = mk_ctx("bob", "delete", "vault/login/abc", {"password"}, 1700);
        auto rec = vp::decide_with_record(e, c);
        ISO_CHECK(!rec.is_allowed());
        ISO_CHECK(rec.denial_reason() ==
                  std::optional<Reason>(Reason::RoleLacksPermission));
        ISO_CHECK(rec.decision == Decision::deny(Reason::RoleLacksPermission));
    }

    // ── AllOf ────────────────────────────────────────────────────────────────
    {
        auto e2 = std::make_unique<SimpleRbacEngine>();
        e2->assign_role("alice", "admin");
        e2->grant("admin", "delete", "*");
        std::vector<std::unique_ptr<PolicyEngine>> v;
        v.push_back(rbac_fixture_ptr());
        v.push_back(std::move(e2));
        vp::AllOf all(std::move(v));
        ISO_CHECK(all.decide(mk_ctx("alice", "delete", "x", {}, 0)).is_allow());
        auto d = all.decide(mk_ctx("bob", "delete", "x", {}, 0));
        ISO_CHECK(d.reason() == Reason::AnyInnerDenied);
    }
    {
        vp::AllOf all(std::vector<std::unique_ptr<PolicyEngine>>{});
        ISO_CHECK(all.decide(mk_ctx("alice", "read", "x", {}, 0)).is_deny());
    }

    // ── AnyOf ────────────────────────────────────────────────────────────────
    {
        auto e1 = std::make_unique<SimpleRbacEngine>();
        e1->assign_role("bob", "member");
        e1->grant("member", "read", "*");
        auto e2 = std::make_unique<SimpleRbacEngine>();
        e2->assign_role("bob", "owner");
        e2->grant("owner", "delete", "*");
        std::vector<std::unique_ptr<PolicyEngine>> v;
        v.push_back(std::move(e1));
        v.push_back(std::move(e2));
        vp::AnyOf any(std::move(v));
        ISO_CHECK(any.decide(mk_ctx("bob", "delete", "x", {}, 0)).is_allow());
    }
    {
        std::vector<std::unique_ptr<PolicyEngine>> v;
        v.push_back(std::make_unique<SimpleRbacEngine>());
        v.push_back(std::make_unique<SimpleRbacEngine>());
        vp::AnyOf any(std::move(v));
        auto d = any.decide(mk_ctx("alice", "read", "x", {}, 0));
        ISO_CHECK(d.reason() == Reason::AllInnerDenied);
    }
    {
        vp::AnyOf any(std::vector<std::unique_ptr<PolicyEngine>>{});
        ISO_CHECK(any.decide(mk_ctx("alice", "read", "x", {}, 0)).is_deny());
    }

    // ── RequireFactor ────────────────────────────────────────────────────────
    {
        vp::RequireFactor r(rbac_fixture_ptr(), "webauthn-prf");
        ISO_CHECK(r.decide(mk_ctx("alice", "delete", "x",
                                  {"password", "webauthn-prf"}, 0))
                      .is_allow());
    }
    {
        vp::RequireFactor r(rbac_fixture_ptr(), "webauthn-prf");
        auto d = r.decide(mk_ctx("alice", "delete", "x", {"password", "totp"}, 0));
        ISO_CHECK(d.reason() == Reason::FactorRequired);
    }

    // ── TimeBound ────────────────────────────────────────────────────────────
    {
        vp::TimeBound t(rbac_fixture_ptr(), 1000, 2000);
        ISO_CHECK(t.decide(mk_ctx("alice", "read", "x", {}, 1500)).is_allow());
    }
    {
        vp::TimeBound t(rbac_fixture_ptr(), 1000, 2000);
        ISO_CHECK(t.decide(mk_ctx("alice", "read", "x", {}, 999)).reason() ==
                  Reason::OutsideTimeWindow);
        ISO_CHECK(t.decide(mk_ctx("alice", "read", "x", {}, 2001)).reason() ==
                  Reason::OutsideTimeWindow);
    }
    {
        vp::TimeBound t(rbac_fixture_ptr(), 1000, 2000);
        ISO_CHECK(t.decide(mk_ctx("alice", "read", "x", {}, 1000)).is_allow());
        ISO_CHECK(t.decide(mk_ctx("alice", "read", "x", {}, 2000)).is_allow());
    }

    // ── Nested composition ───────────────────────────────────────────────────
    {
        auto wf = std::make_unique<vp::RequireFactor>(rbac_fixture_ptr(),
                                                      "webauthn-prf");
        auto wt = std::make_unique<vp::TimeBound>(std::move(wf), 1000, 2000);
        std::vector<std::unique_ptr<PolicyEngine>> v;
        v.push_back(std::move(wt));
        vp::AllOf composite(std::move(v));

        ISO_CHECK(composite
                      .decide(mk_ctx("alice", "delete", "x",
                                     {"password", "webauthn-prf"}, 1500))
                      .is_allow());
        ISO_CHECK(composite.decide(mk_ctx("alice", "delete", "x", {"password"},
                                          1500))
                      .is_deny());
        ISO_CHECK(composite
                      .decide(mk_ctx("alice", "delete", "x",
                                     {"password", "webauthn-prf"}, 9999))
                      .is_deny());
    }

    // ── Reason inertness ─────────────────────────────────────────────────────
    {
        ISO_CHECK(std::string(Reason::PolicyDenies.text) == "policy denies");
        ISO_CHECK(std::string(Reason::UnknownPrincipal.text) ==
                  "unknown principal");
        ISO_CHECK(std::string(Reason::RoleLacksPermission.text) ==
                  "role lacks permission");
        ISO_CHECK(std::string(Reason::FactorRequired.text) ==
                  "required authentication factor missing");
        ISO_CHECK(std::string(Reason::OutsideTimeWindow.text) ==
                  "outside time window");
        ISO_CHECK(std::string(Reason::AnyInnerDenied.text) ==
                  "at least one inner engine denied");
        ISO_CHECK(std::string(Reason::AllInnerDenied.text) ==
                  "every inner engine denied");
    }

    return ISO_TEST_RESULT();
}
