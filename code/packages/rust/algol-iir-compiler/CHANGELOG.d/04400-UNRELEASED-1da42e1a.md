## [Unreleased]

### Fixed

- Recheck direct real-output provenance after expression lowering as well as before it (#17363). Preserve value-formal selectors and all bounded-family/depth limits; by-name selector promotion can no longer retain a cached formatter proof. Expanded local-root integration retains assignment guard, exact root-position/value controls and native selected-output checks.

- Require runtime-real assignment provenance to remain valid after RHS lowering. A selector's by-name actual can promote a previously local branch root to shared storage; that transition now rejects the stale formatter proof. Preserve value-formal selectors and ordinary unaliased local snapshots, with exact refusal, alias-storage and native output controls (#17350).
