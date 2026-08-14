# Chief of Staff D18P durable-channel fixtures — version 1

`manifest.json` is the language-neutral compatibility lock for
[`D18P-chief-of-staff-durable-channel-profile.md`](../../../specs/D18P-chief-of-staff-durable-channel-profile.md).
It covers:

- active and destroyed canonical D18C definitions, including binary receiver IDs
  and byte-sorted membership
- initial and pending D18S states, the embedded D18H reservation header, and D18A
  receiver cursors
- every normative storage key and content type
- create, publish, read, acknowledge, recovery, retry, abandon, permanent-gap,
  independent-cursor, and destroy transition traces
- malformed records, bounds, cross-record invariants, role failures, acknowledgement
  failures, and the closed stable-error roster

Every language adapter must consume this same manifest. It must not derive expected
bytes from its own codec during conformance tests. D18F remains the sole owner of
D18M message bytes; the D18P fixture embeds one D18F record only to lock the durable
store transition that produced it. Sealed-key cryptographic vectors remain owned by
D18G and the portable rotation work tracked in issue #141.

## Regeneration

The manifest records the Git blob hash of the exact Rust generator source. From the
repository root, regenerate it with:

```sh
GENERATOR_BLOB_SHA1=$(git hash-object \
  code/packages/rust/chief-of-staff-channel-endpoints/examples/generate_d18p_fixtures.rs)
cargo run --manifest-path code/packages/rust/Cargo.toml \
  -p chief-of-staff-channel-endpoints \
  --example generate_d18p_fixtures -- \
  code/fixtures/chief-of-staff-channel/v1/manifest.json "$GENERATOR_BLOB_SHA1"
```

The Rust integration test regenerates the manifest in memory with its recorded blob
identity and requires a byte-for-byte match. Review any byte change as a durable
storage compatibility event and update every registered language consumer together.

## Security warning

The manifest contains deterministic private keys and a channel master key so all
runtimes can reproduce the transition traces. They are public test-only material.
Never copy them into deployed configuration, a key store, an example service, or a
production test environment.
