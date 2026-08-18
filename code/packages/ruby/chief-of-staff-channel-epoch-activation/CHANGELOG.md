# Changelog

## Unreleased

- Delegate the custody CMK comparison to `CodingAdventures::CtCompare.ct_eq_fixed`
  instead of hand-rolling the loop. The D18T spec requires using "the platform's
  constant-time primitive where one exists"; `ruby/ct-compare` did not exist
  until now, which is the only reason this package carried its own copy. The
  other five ports already delegate.

## 0.1.0

- Add the Ruby D18T durable epoch-activation adapter, consuming the canonical
  Rust-authored manifest at
  `code/fixtures/chief-of-staff-channel-epoch-activation/v1/` directly. Ruby
  reproduces the canonical D18T plan bytes and every D18G grant byte-for-byte
  from the labelled test-only secrets.
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
- Enforce invariant 3, "all grants before visibility": replay re-reads every
  grant from public storage and byte-compares before activation may advance the
  epoch, rather than trusting the record the backend echoed from its own write.
- Settle the channel definition before importing the caller's CMK, so a
  mismatched definition cannot claim an unclaimed custody slot and wedge the
  legitimate import at `conflicting_active_key`.
- Report Ruby's honest `best_effort` secret-erasure capability rather than
  echoing the manifest's Rust-authored `guaranteed`.
- Perform the D18Q grant epoch comparison locally. `verify_grant_signature`
  deliberately takes no expected epoch, so a validly signed grant for another
  epoch would otherwise pass; D18T step 5 owns that check.
