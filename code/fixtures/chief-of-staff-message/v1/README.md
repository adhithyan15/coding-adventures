# Chief of Staff D18F message fixtures — version 1

`manifest.json` is the language-neutral compatibility lock for
[`D18F-chief-of-staff-message-profile.md`](../../../specs/D18F-chief-of-staff-message-profile.md).
It contains:

- eight deterministic positive messages covering empty, text, structured JSON,
  binary, multipart, rotated-key, and two-message streaming payloads
- their plaintext, canonical authenticated header, D18M v1 record, and
  canonical lossless JSON bytes, all encoded as RFC 4648 base64
- binary and JSON negative cases with one normative stable error code each
- compact recipes for length-boundary tests, avoiding checked-in 64 MiB blobs
- deterministic signing and channel keys labelled explicitly as test-only

Every language adapter must consume this same manifest. It must not regenerate
expected bytes with its own implementation during a conformance test.

## Central conformance gate

From the repository root, run:

```sh
python3 code/scripts/validate_d18f_message_conformance.py
```

The gate validates the closed manifest roster and generator blob identity,
requires registered consumers for Rust, Python, TypeScript, Go, Ruby, and
Elixir, executes each package's native `BUILD` front door, and regenerates the
manifest from the Rust baseline into a temporary file for a byte-for-byte
comparison. CI publishes this as the stable `D18F message conformance` check
and includes it in the aggregate CI gate.

## Regeneration

The manifest records the Git blob hash of the exact generator source. Unlike a
commit ID, that identifier survives rebases and squash merges. From the
repository root, compute the blob hash and regenerate with:

```sh
cargo run --manifest-path code/packages/rust/Cargo.toml \
  -p chief-of-staff-channel-crypto \
  --example generate_d18f_fixtures -- \
  code/fixtures/chief-of-staff-message/v1/manifest.json GENERATOR_BLOB_SHA1
```

Review byte changes deliberately. Changing any existing positive vector is a
wire-compatibility event and requires updating D18F and every language gate.

## Security warning

The manifest contains a private Ed25519 seed and channel master keys so every
runtime can reproduce the vectors. They are public, deterministic test values.
Never copy them into a deployed configuration, key store, example service, or
production test environment.
