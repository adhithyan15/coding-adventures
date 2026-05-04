# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT06
  (`code/specs/VLT06-vault-policy.md`).
- `PolicyEngine` trait + `PolicyContext` (principal / action /
  resource / factors / time / metadata) + `Decision { Allow,
  Deny(Reason) }`.
- `Reason` is a `&'static str` chosen from a fixed table —
  attacker-controlled bytes never appear in deny messages.
- `SimpleRbacEngine` — role × permission table. Each principal
  binds to one role; each role grants `(action, resource_pattern)`
  where `resource_pattern` is either an exact string or `"*"`.
  Distinct deny reasons for unknown-principal vs role-lacks-perm
  so debugging is easy without leaking content.
- `AllOf`, `AnyOf` — boolean composition decorators. Empty
  inner-list denies vacuously.
- `RequireFactor` — wraps an inner engine and requires a specific
  authentication-factor `kind` (matches `AuthAssertion::kind` from
  VLT05) to be present in the context.
- `TimeBound` — wraps an inner engine and only allows within a
  `[start, end]` UNIX-time window (inclusive endpoints).
- 18 unit tests covering: RBAC admin/member matrix, wildcard vs
  exact resource match, unknown-principal vs role-lacks-perm
  distinct deny reasons, role-with-no-perms, AllOf composition
  with one allow / one deny, AllOf empty denies, AnyOf composition
  including the deny-then-allow case, AnyOf empty denies,
  RequireFactor present/absent paths, TimeBound inside / outside /
  inclusive endpoints, nested-decorator composition (RBAC +
  RequireFactor + TimeBound + AllOf), and the
  reasons-are-static-literals invariant.

### Out of scope (future PRs)

- HCL policy DSL (HashiCorp Vault compatibility).
- Cedar (AWS).
- Rego / OPA bindings.
- ABAC / dynamic attribute fetch.
- Quorum / two-admin-must-approve decorator.
- Audit-trail integration (the policy engine is pure today; an
  optional sink hook could be added at a later layer).
