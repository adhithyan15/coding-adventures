# Changelog

## 0.1.0

- Add the Elixir D18T durable epoch-activation adapter, consuming the canonical
  Rust-authored manifest at
  `code/fixtures/chief-of-staff-channel-epoch-activation/v1/` directly. Elixir
  reproduces the canonical D18T plan bytes and every D18G grant byte-for-byte
  from the labelled test-only secrets. Completes the six-language surface.
- Add the exact `D18S` version 2 state codec (active epoch, next sequence,
  optional pending D18H reservation) and the immutable `D18T` version 1
  activation-plan codec, with canonical ordering enforced on decode rather than
  normalized on construction.
- Add the injected atomic originator-key custody boundary: three-valued
  selection (`selected` / `idempotent` / `conflict`), indivisible
  plan-plus-grants-plus-CMK bundles, constant-time CMK comparison delegated to
  `CodingAdventures.CtCompare`, handles and candidates that redact every field
  under `Inspect`, and an explicitly non-durable in-memory implementation that
  the production constructor refuses.
- Add the orchestration surface: create, migrate from D18P version 1, prepare,
  recover, activate, reserve-publish against the active epoch, abandon pending,
  read the public plan, and apply destruction.
- Enforce the shared-CAS invariant: the active epoch and the pending publish
  reservation live in one versioned record, so publication and activation
  contend on a single revision and exactly one wins.
- Enforce invariant 3, "all grants before visibility": replay re-reads every
  grant from public storage and byte-compares before activation may advance the
  epoch, rather than trusting the record the backend echoed from its own write.
- Settle the channel definition before importing the caller's CMK, so a
  mismatched definition cannot claim an unclaimed custody slot and wedge the
  legitimate import at `conflicting_active_key`.
- Validate that a plan's `channel_id` is a real UUID v7 — version nibble and
  variant bits — not merely 16 octets, matching Rust, Python, and Ruby.
- Report Elixir's honest `not_enforceable` secret-erasure capability rather than
  echoing the manifest's Rust-authored `guaranteed`. Immutable, garbage-collected
  BEAM values cannot promise physical overwrite.
- Perform the D18Q grant epoch comparison locally. `verify_grant_signature`
  deliberately takes no expected epoch, so a validly signed grant for another
  epoch would otherwise pass; D18T step 5 owns that check.
