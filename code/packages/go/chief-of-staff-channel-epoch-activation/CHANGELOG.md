# Changelog — chief-of-staff-channel-epoch-activation

## Unreleased

- Validate that a plan's `channel_id` is a real UUID v7 — version nibble 7 in
  byte 6, variant bits `0b10` in byte 8 — not merely 16 octets. The Rust
  reference, Python, Ruby, and Elixir all check this; Go checked only the
  length, so a plan record with a malformed channel identifier decoded here but
  was `corrupt_record` everywhere else. Two conforming implementations
  disagreed about whether the same bytes were valid, which the six-language
  conformance gate (#11788) would have surfaced as a CI failure rather than a
  decision.
- Add the Go D18T durable epoch-activation adapter, consuming the canonical
  Rust-authored manifest at `code/fixtures/chief-of-staff-channel-epoch-activation/v1/`
  directly. Go reproduces the canonical D18T plan bytes and every D18G grant
  byte-for-byte from the labelled test-only secrets.
- Add the exact `D18S` version 2 state codec (active epoch, next sequence,
  optional pending D18H reservation) and the immutable `D18T` version 1
  activation-plan codec, with canonical ordering enforced on decode rather than
  normalized on construction.
- Add the injected atomic originator-key custody boundary: three-valued
  selection (`selected` / `idempotent` / `conflict`), indivisible
  plan-plus-grants-plus-CMK bundles, constant-time CMK comparison, redacted
  handles, and an explicitly non-durable in-memory implementation that the
  production constructor refuses.
- Add the orchestration surface: create, migrate from D18P version 1, prepare,
  recover, activate, reserve-publish against the active epoch, abandon pending,
  read the public plan, and apply destruction.
- Enforce the shared-CAS invariant: the active epoch and the pending publish
  reservation live in one versioned record, so publication and activation
  contend on a single revision and exactly one wins.
- Report Go's honest `best_effort` secret-erasure capability rather than
  echoing the manifest's Rust-authored `guaranteed`.
- Perform the D18Q grant epoch comparison locally. `VerifyGrantSignature`
  deliberately takes no expected epoch, so a validly signed grant for another
  epoch would otherwise pass; D18T step 5 owns that check.

Prerequisite landed separately: `VerifyGrantSignature` in
`go/chief-of-staff-channel-crypto` (#11882).
