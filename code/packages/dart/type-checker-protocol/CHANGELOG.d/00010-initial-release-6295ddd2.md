## 0.1.0

- Initial Dart lane of the type-checker-protocol identity: `TypeChecker`
  contract, `TypeCheckResult`/`TypeErrorDiagnostic` reporting, and the
  `GenericTypeChecker` base with phase/kind hook dispatch, wildcard tier,
  diagnostics with overridable positions, and kind normalization.
- Dart adaptations, both documented in the README: hook arguments are an
  explicit list (no variadic parameters), and wildcard registration stores
  the exact kind `'*'` under the literal `"phase:*"` key, fixing a
  reference bug where normalized-away wildcards never fire.
- One runtime dependency (`meta`, for `@protected` on the subclass
  extension points). Tests mirror the reference suite (27 unit tests), add 6
  seeded property tests over the in-lane `pbt` package and a 2,000-iteration
  differential fuzz campaign over the in-lane `fuzz` package; 100%
  line/branch/function coverage.
