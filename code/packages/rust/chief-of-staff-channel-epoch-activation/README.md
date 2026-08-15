# chief-of-staff-channel-epoch-activation

Crash-safe durable D18T channel epoch activation over the production Rust
D18P channel store, D18Q grants, and D18C membership definition.

The package adds the version 2 D18S state record, whose `active_epoch` is
serialized by the same revision CAS as the next sequence and optional pending
D18H reservation. A rotation first selects one complete successor bundle in
injected originator custody, then idempotently replays its immutable D18T plan
and exact D18G grants. Only after every public record verifies does one CAS
advance the active epoch.

`EpochActivationStore::new` accepts only custody that reports itself durable.
`new_for_testing` is an explicit escape hatch for the included deterministic
`InMemoryKeyCustody`; it must not be used in production. Opaque
`EpochKeyHandle` values and error displays never reveal CMKs or custody
locators.

The canonical language-neutral transition corpus is checked in at
`code/fixtures/chief-of-staff-channel-epoch-activation/v1/manifest.json`. It
contains exact state migrations, activation-plan and grant bytes, crash/replay
and race traces, the closed stable error vocabulary, and clearly isolated
test-only secrets. The Rust tests prove that it is byte-identical to the
generator whose Git blob SHA-1 it records.

The normative contract lives in
`code/specs/D18T-chief-of-staff-durable-epoch-activation-profile.md`.

## Dependencies

- chief-of-staff-channel-crypto
- chief-of-staff-channel-store
- chief-of-staff-channel-endpoints
- storage-core
- json-value
- sha256
- ct-compare

## Development

```bash
bash BUILD
```

The build runs all package targets, including the fixture generator and
language-neutral manifest checks.
