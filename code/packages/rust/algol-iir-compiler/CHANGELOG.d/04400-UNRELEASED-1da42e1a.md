## [Unreleased]

### Fixed

- Require runtime-real assignment provenance to remain valid after RHS lowering. A selector's by-name actual can promote a previously local branch root to shared storage; that transition now rejects the stale formatter proof. Preserve value-formal selectors and ordinary unaliased local snapshots, with exact refusal, alias-storage and native output controls (#17350).
