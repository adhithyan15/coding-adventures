# Changelog

## Unreleased

- Exposed audited authored API-key conflict merge with a hidden token prompt,
  opaque base retention, closed scope/expiry validation, and all-current-parent
  publication.
- Exposed audited authored payment-card conflict merge with hidden PAN/CVV,
  opaque base retention, closed validation, and all-current-parent publication.
- Exposed audited authored secure-note conflict merge with hidden body input,
  opaque base retention, and all-current-parent publication.
- Exposed audited authored login conflict merge with opaque base selection,
  complete hidden form collection, durable failure ordering, and an
  all-current-parent success mutation.
- Exposed audited `conflict reveal ITEM REVISION FIELD` and extended the
  restart drill through terminal-confirmation denial plus an unconflicted
  candidate failure, empty stdout, secret exclusion, and durable audit-chain
  advancement.
- Exposed audited `search QUERY` and extended the primary restart drill through
  a URL metadata match, non-echoed query, redacted result row, audit-chain
  advancement, and closed audit-field verification.
- Extended the real-process login lifecycle through two ordered URLs, hidden
  notes creation and replacement, notes-presence redaction, separate audited
  notes reveal, history restoration, and plaintext-tree exclusion.
- Exposed audited TOTP creation with hidden canonical Base32 input,
  restart-backed metadata-only rendering, separate audited Base32 reveal,
  closed audit rows, and encoded/raw plaintext-tree exclusion in a real PTY
  drill.
- Exposed audited static database-credential creation with restart-backed
  redaction, separate password reveal, closed audit rows, and plaintext-tree
  exclusion in a real PTY drill.
- Exposed audited API-key creation and added a real PTY drill for hidden token
  input, restart-backed metadata-only rendering, separately authorized token
  reveal, closed-field audit advancement, and plaintext-tree exclusion.
- Exposed audited payment-card creation and added a second real PTY drill for
  hidden PAN/CVV input, restart-backed redaction, separate direct-terminal
  reveal, closed-field audit advancement, and full-PAN plaintext-tree
  exclusion.
- Exposed audited interactive current-secret reveal and extended the real PTY
  drill to prove direct controlling-terminal delivery with empty captured
  process stdout and restart-backed audit advancement.
- Exposed audited redacted conflict listing and choose-existing-candidate
  resolution through the thin executable.
- Exposed explicit-target `restore FILE` and moved the real-process PTY drill to
  one artifact authentication, audited import, independent target reopen, and
  completed-and-verified aggregate output with no intermediate import claim.
- Exposed `vault create NAME` and command-scoped `--vault NAME`, and moved the
  real-process restore drill onto a separately keyed named target in the same
  profile with a final restart-backed source-isolation check.
- Exposed retryable audited `restore verify FILE` and extended the real-process
  PTY drill through another independent target reopen and hidden artifact
  prompt before aggregate verification output.
- Exposed audited `import FILE` and extended the real-process PTY suite through
  an independent initialized target, hidden artifact input, and restarted
  redacted observation.
- Exposed audited encrypted `export FILE` and extended the real-process PTY
  suite through two hidden export-passphrase prompts and plaintext-tree checks.
- Exposed secure-note creation and redacted show, with real-process PTY proof
  that the body is hidden during input and absent from the storage tree.
- Extended the real-process PTY audit ceremony through an invalid item-create
  prompt and exact trace selection of its durable failed event.
- Exposed authenticated `audit list` and `audit show TRACE`, and extended the
  real-process PTY suite through trace selection plus later verification that
  both audit-history accesses became durable.
- Exposed `audit enable` and extended the real-process PTY suite through audit
  activation, an invalid edit prompt, and later verification of its event.
- Exposed reversible item delete/restore and extended the real-process PTY
  suite through tombstone observation and exact historical restoration.
- Exposed redacted revision history listing and extended the PTY suite through
  canonical newest-first history after a durable edit.
- Exposed revision-safe login edit and extended the PTY restart suite through
  replacement plus a later redacted show.
- Exposed login add and redacted list/show through the thin executable.
- Extended the PTY suite through encrypted item persistence across processes.
- Exposed authenticated `audit verify` and `doctor --unlock` through the thin
  executable.
- Extended the real-process PTY suite across restart and redirected-stdin
  injection for both authenticated commands.

## 0.1.0

- Added the `vault-pm` executable composition root.
- Added real-process pseudo-terminal initialization and restart coverage.
