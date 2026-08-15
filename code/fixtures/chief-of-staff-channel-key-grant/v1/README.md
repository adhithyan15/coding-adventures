# Chief of Staff D18Q key-grant fixtures - version 1

`manifest.json` is the language-neutral compatibility lock for
[`D18Q-chief-of-staff-channel-key-grant-profile.md`](../../../specs/D18Q-chief-of-staff-channel-key-grant-profile.md).
It records deterministic test-only keys and nonces, every X25519/HKDF/AAD/
signature intermediate, exact D18G records, negative validation-order cases,
receiver epoch traces, and the A+B to B-only rotation result.

Every language adapter must consume this manifest directly. Conformance tests
must not regenerate expected bytes with their own implementation, shell out to
another language, or substitute host cryptography for repository primitives.

## Regeneration

The manifest stores the Git blob SHA-1 of the exact Rust generator so its
identity survives rebases and squash merges. From `code/packages/rust`, run:

```sh
generator_sha=$(git hash-object chief-of-staff-channel-crypto/examples/generate_d18q_fixtures.rs)
cargo run -p chief-of-staff-channel-crypto \
  --example generate_d18q_fixtures -- \
  ../../fixtures/chief-of-staff-channel-key-grant/v1/manifest.json \
  "$generator_sha"
```

Review byte changes deliberately. Any change to an existing positive vector is
a D18Q/D18G compatibility event and must be coordinated across all six
language consumers.

## Security warning

The manifest intentionally publishes CMKs, private keys, shared secrets,
wrapping keys, and nonces for deterministic tests. Never copy these values into
a deployed configuration, key store, example service, log, audit record, or
production test environment.
