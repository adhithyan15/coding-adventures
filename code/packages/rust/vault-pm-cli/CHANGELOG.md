# Changelog

## Unreleased

- Added authenticated `audit list` and canonical `audit show TRACE`; both
  publish one durable `AuditRead` before rendering verified trace-aware rows.
- Added closed canonical trace parsing, bounded newest-first output, audited
  missing-trace results, tamper rejection, and ambiguous-provider recovery
  coverage for the explicit audit surface.
- Exposed idempotent authenticated `audit enable`, installing the one durable
  `AuditEpochStart` migration event before any active-epoch command can run.
- Route `item edit ITEM` through an opaque application-owned preparation so
  active-epoch precondition, prompt, entropy, and document-validation failures
  become durable before their CLI errors, while success stays one atomic
  `ItemUpdate` mutation.
- Collapse active-epoch `history restore ITEM REVISION` into one item-bound
  audited application mutation, including durable missing, cross-item,
  tombstone, same-revision, and conflict failures.
- Collapse active-epoch `item delete ITEM` into one application-selected
  audited mutation: successful tombstones and failed authenticated
  preconditions now become durable before the CLI reveals their outcome.
- Route list, show, history list, audit verify, and unlocked doctor through
  signed publish-before-render access events whenever the vault audit epoch is
  active, while retaining backward-compatible pre-audit behavior.
- Added reversible authenticated `item delete ITEM` and
  `history restore ITEM REVISION` mutations with strict item-bound selectors,
  causal tombstones, and restore-as-new-revision semantics.
- Added authenticated `history list ITEM` with canonical revision selectors,
  newest-first causal metadata, and redacted record titles.
- Added revision-safe `item edit ITEM` for complete login-field replacement
  while preserving identity, metadata, notes, and causal history.
- Added strict `item add login`, `item list`, and `item show ITEM` commands.
- Added controlling-terminal item input, fresh mutation identities, durable
  application publication, escaped redacted rendering, and restart coverage.
- Added one-shot authenticated `audit verify` with aggregate-only output.
- Extended that output with a secret-free count of fully authenticated
  encrypted operation-audit events; pre-audit vaults report zero.
- Added opt-in full repository health verification through `doctor --unlock`.
- Added strict parser, wrong-passphrase, synchronous re-lock, and real-process
  controlling-terminal coverage for authenticated verification.

## 0.1.0

- Added the closed `init`, `status`, and `doctor` command grammar.
- Added stable exit classes and payload-free text/JSON rendering.
- Composed secure local roots, exact configuration, durable application state,
  immutable filesystem storage, fixed terminal prompts, and OS entropy.
- Added crash-resumable generation-zero activation and restart tests.
