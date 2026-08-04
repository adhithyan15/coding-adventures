// vault_policy.hpp — pluggable authorization policy engine (VLT06).
// ==================================================================
//
// A faithful, header-only port of the Rust `vault-policy` crate (namespace
// `ca::vault_policy`). Authentication says *who*; this layer says *what they
// can do*. You build a policy engine out of a role-based table plus composable
// decorators, then ask it to `decide` a request described by a `PolicyContext`.
//
//   SimpleRbacEngine   role × (action, resource-pattern) table
//   AllOf              boolean AND of inner engines (all must allow)
//   AnyOf              boolean OR of inner engines (any allow wins)
//   RequireFactor      wrap an engine, also require an auth factor present
//   TimeBound          wrap an engine, only allow within [start, end] UNIX secs
//
// Engines compose via `std::unique_ptr<PolicyEngine>`; decorators own their
// inner engines. Decisions fail closed and every denial reason is a static
// literal drawn from a fixed table (`Reason::*`) — never attacker-controlled
// bytes, so a malicious principal name cannot inject content into logs.
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no compiler extensions.
#ifndef VAULT_POLICY_HPP
#define VAULT_POLICY_HPP

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <memory>
#include <optional>
#include <set>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

namespace ca::vault_policy {

// ── Reasons ────────────────────────────────────────────────────────────────
//
// A static-literal denial reason. The engine never constructs one from input
// bytes; they all come from the fixed table below.
struct Reason {
    const char* text;

    friend bool operator==(const Reason& a, const Reason& b) {
        if (a.text == b.text) return true;
        if (a.text == nullptr || b.text == nullptr) return false;
        return std::strcmp(a.text, b.text) == 0;
    }
    friend bool operator!=(const Reason& a, const Reason& b) { return !(a == b); }

    static const Reason PolicyDenies;
    static const Reason UnknownPrincipal;
    static const Reason RoleLacksPermission;
    static const Reason FactorRequired;
    static const Reason OutsideTimeWindow;
    static const Reason AnyInnerDenied;
    static const Reason AllInnerDenied;
};

inline const Reason Reason::PolicyDenies{"policy denies"};
inline const Reason Reason::UnknownPrincipal{"unknown principal"};
inline const Reason Reason::RoleLacksPermission{"role lacks permission"};
inline const Reason Reason::FactorRequired{
    "required authentication factor missing"};
inline const Reason Reason::OutsideTimeWindow{"outside time window"};
inline const Reason Reason::AnyInnerDenied{"at least one inner engine denied"};
inline const Reason Reason::AllInnerDenied{"every inner engine denied"};

// ── Decision ─────────────────────────────────────────────────────────────────
//
// The engine's verdict: Allow, or Deny carrying a static reason.
class Decision {
   public:
    static Decision allow() { return Decision(true, Reason{nullptr}); }
    static Decision deny(Reason reason) { return Decision(false, reason); }

    bool is_allow() const { return allow_; }
    bool is_deny() const { return !allow_; }
    std::optional<Reason> reason() const {
        if (allow_) return std::nullopt;
        return reason_;
    }

    friend bool operator==(const Decision& a, const Decision& b) {
        if (a.allow_ != b.allow_) return false;
        if (a.allow_) return true;
        return a.reason_ == b.reason_;
    }
    friend bool operator!=(const Decision& a, const Decision& b) {
        return !(a == b);
    }

   private:
    Decision(bool allow, Reason reason) : allow_(allow), reason_(reason) {}
    bool allow_;
    Reason reason_;
};

// ── Context ──────────────────────────────────────────────────────────────────
//
// A decision request. Fields are deliberately narrow — anything the engine
// doesn't read can't influence the decision.
struct PolicyContext {
    std::string principal;
    std::string action;
    std::string resource;
    std::vector<std::string> factors;
    std::uint64_t time = 0;  // UNIX seconds at decision time
    std::unordered_map<std::string, std::string> metadata;
};

// ── The trait ────────────────────────────────────────────────────────────────

/// Pluggable policy engine base class.
class PolicyEngine {
   public:
    virtual ~PolicyEngine() = default;
    /// Stable kind string for telemetry (not load-bearing for security).
    virtual const char* kind() const = 0;
    /// Decide. Pure over the context — never touches the clock, network, or FS.
    virtual Decision decide(const PolicyContext& ctx) const = 0;
};

// ── SimpleRbacEngine ─────────────────────────────────────────────────────────

/// Count-only read model for an RBAC table (exposes no names).
struct SimpleRbacSummary {
    std::size_t principal_bindings = 0;
    std::size_t unique_roles = 0;
    std::size_t assigned_roles = 0;
    std::size_t roles_with_permissions = 0;
    std::size_t permission_grants = 0;
    std::size_t wildcard_resource_grants = 0;
    std::size_t exact_resource_grants = 0;
    std::size_t permission_roles_without_principals = 0;
    std::size_t assigned_roles_without_permissions = 0;

    bool has_principal_bindings() const { return principal_bindings > 0; }
    bool has_permission_grants() const { return permission_grants > 0; }
    bool has_permission_roles_without_principals() const {
        return permission_roles_without_principals > 0;
    }
    bool has_assigned_roles_without_permissions() const {
        return assigned_roles_without_permissions > 0;
    }

    friend bool operator==(const SimpleRbacSummary& a,
                           const SimpleRbacSummary& b) {
        return a.principal_bindings == b.principal_bindings &&
               a.unique_roles == b.unique_roles &&
               a.assigned_roles == b.assigned_roles &&
               a.roles_with_permissions == b.roles_with_permissions &&
               a.permission_grants == b.permission_grants &&
               a.wildcard_resource_grants == b.wildcard_resource_grants &&
               a.exact_resource_grants == b.exact_resource_grants &&
               a.permission_roles_without_principals ==
                   b.permission_roles_without_principals &&
               a.assigned_roles_without_permissions ==
                   b.assigned_roles_without_permissions;
    }
    friend bool operator!=(const SimpleRbacSummary& a,
                           const SimpleRbacSummary& b) {
        return !(a == b);
    }
};

/// Role-based access control: each principal maps to one role, each role to a
/// set of `(action, resource_pattern)` permissions. `resource_pattern` is an
/// exact string or `"*"` (wildcard).
class SimpleRbacEngine : public PolicyEngine {
   public:
    const char* kind() const override { return "rbac"; }

    /// Bind a principal to a single role (replaces any existing binding).
    void assign_role(std::string principal, std::string role) {
        principals_[std::move(principal)] = std::move(role);
    }

    /// Grant `(action, resource_pattern)` to a role. Duplicates collapse.
    void grant(std::string role, std::string action,
               std::string resource_pattern) {
        role_perms_[std::move(role)].insert(
            {std::move(action), std::move(resource_pattern)});
    }

    Decision decide(const PolicyContext& ctx) const override {
        auto it = principals_.find(ctx.principal);
        if (it == principals_.end())
            return Decision::deny(Reason::UnknownPrincipal);
        auto pit = role_perms_.find(it->second);
        if (pit == role_perms_.end())
            return Decision::deny(Reason::RoleLacksPermission);
        for (const auto& [action_pat, resource_pat] : pit->second) {
            if (action_pat == ctx.action &&
                (resource_pat == "*" || resource_pat == ctx.resource)) {
                return Decision::allow();
            }
        }
        return Decision::deny(Reason::RoleLacksPermission);
    }

    SimpleRbacSummary summary() const {
        std::set<std::string> assigned;
        for (const auto& [principal, role] : principals_) assigned.insert(role);
        std::set<std::string> perm_roles;
        for (const auto& [role, perms] : role_perms_) perm_roles.insert(role);

        SimpleRbacSummary s;
        s.principal_bindings = principals_.size();
        s.assigned_roles = assigned.size();
        s.roles_with_permissions = perm_roles.size();
        for (const auto& [role, perms] : role_perms_) {
            s.permission_grants += perms.size();
            for (const auto& [action, resource] : perms) {
                if (resource == "*")
                    ++s.wildcard_resource_grants;
                else
                    ++s.exact_resource_grants;
            }
        }
        std::set<std::string> uni = assigned;
        uni.insert(perm_roles.begin(), perm_roles.end());
        s.unique_roles = uni.size();
        for (const auto& r : perm_roles)
            if (assigned.count(r) == 0) ++s.permission_roles_without_principals;
        for (const auto& r : assigned)
            if (perm_roles.count(r) == 0) ++s.assigned_roles_without_permissions;
        return s;
    }

   private:
    std::unordered_map<std::string, std::string> principals_;
    std::unordered_map<std::string,
                       std::set<std::pair<std::string, std::string>>>
        role_perms_;
};

// ── Decorators ───────────────────────────────────────────────────────────────

/// Boolean AND: allows only if every inner engine allows. Empty list denies.
class AllOf : public PolicyEngine {
   public:
    explicit AllOf(std::vector<std::unique_ptr<PolicyEngine>> inner)
        : inner_(std::move(inner)) {}
    const char* kind() const override { return "all-of"; }
    Decision decide(const PolicyContext& ctx) const override {
        if (inner_.empty()) return Decision::deny(Reason::PolicyDenies);
        for (const auto& e : inner_)
            if (e->decide(ctx).is_deny())
                return Decision::deny(Reason::AnyInnerDenied);
        return Decision::allow();
    }

   private:
    std::vector<std::unique_ptr<PolicyEngine>> inner_;
};

/// Boolean OR: allows if any inner engine allows. Empty list denies.
class AnyOf : public PolicyEngine {
   public:
    explicit AnyOf(std::vector<std::unique_ptr<PolicyEngine>> inner)
        : inner_(std::move(inner)) {}
    const char* kind() const override { return "any-of"; }
    Decision decide(const PolicyContext& ctx) const override {
        if (inner_.empty()) return Decision::deny(Reason::PolicyDenies);
        for (const auto& e : inner_)
            if (e->decide(ctx).is_allow()) return Decision::allow();
        return Decision::deny(Reason::AllInnerDenied);
    }

   private:
    std::vector<std::unique_ptr<PolicyEngine>> inner_;
};

/// Requires a specific authentication-factor `kind` present in `ctx.factors`,
/// then defers to the inner engine.
class RequireFactor : public PolicyEngine {
   public:
    RequireFactor(std::unique_ptr<PolicyEngine> inner, std::string factor_kind)
        : inner_(std::move(inner)), factor_kind_(std::move(factor_kind)) {}
    const char* kind() const override { return "require-factor"; }
    Decision decide(const PolicyContext& ctx) const override {
        for (const auto& f : ctx.factors)
            if (f == factor_kind_) return inner_->decide(ctx);
        return Decision::deny(Reason::FactorRequired);
    }

   private:
    std::unique_ptr<PolicyEngine> inner_;
    std::string factor_kind_;
};

/// Only forwards the inner decision when `ctx.time` is in `[start, end]`
/// (UNIX seconds, inclusive).
class TimeBound : public PolicyEngine {
   public:
    TimeBound(std::unique_ptr<PolicyEngine> inner, std::uint64_t start,
              std::uint64_t end)
        : inner_(std::move(inner)), start_(start), end_(end) {}
    const char* kind() const override { return "time-bound"; }
    Decision decide(const PolicyContext& ctx) const override {
        if (ctx.time < start_ || ctx.time > end_)
            return Decision::deny(Reason::OutsideTimeWindow);
        return inner_->decide(ctx);
    }

   private:
    std::unique_ptr<PolicyEngine> inner_;
    std::uint64_t start_, end_;
};

// ── Decision record ──────────────────────────────────────────────────────────

/// A compact owned view of one decision, safe to hand to an audit sink without
/// exposing the original metadata. Copies only identifiers and counts.
struct PolicyDecisionRecord {
    const char* engine_kind;
    std::string principal;
    std::string action;
    std::string resource;
    std::uint64_t time;
    std::size_t factor_count;
    std::size_t metadata_count;
    Decision decision;

    bool is_allowed() const { return decision.is_allow(); }
    std::optional<Reason> denial_reason() const { return decision.reason(); }
};

/// Decide with `engine` and capture a compact record.
inline PolicyDecisionRecord decide_with_record(const PolicyEngine& engine,
                                               const PolicyContext& ctx) {
    Decision decision = engine.decide(ctx);
    return PolicyDecisionRecord{engine.kind(),
                                ctx.principal,
                                ctx.action,
                                ctx.resource,
                                ctx.time,
                                ctx.factors.size(),
                                ctx.metadata.size(),
                                decision};
}

}  // namespace ca::vault_policy

#endif  // VAULT_POLICY_HPP
