# `coding_adventures_bounded_json`

A small, zero-dependency RFC 8259 parser and serializer for bounded JSON values.
It was built for OAuth authorization-server metadata and token responses, where
the caller already caps bytes and the parser must additionally cap nesting
before recursive descent. Serialization applies the same explicit depth model,
preserves object order and duplicates, and rejects non-finite numbers.

Objects preserve source order and duplicate names so a protocol layer can reject
ambiguous security-sensitive fields. Strings reject unescaped controls and lone
UTF-16 surrogates. Numbers enforce JSON's exact lexical grammar before conversion
and reject non-finite results. Errors expose only a closed class and byte offset,
never source text.

The package has no dependencies. It performs no I/O and owns no provider,
network, storage, clock, or audit authority. Parse and serialization failures
are closed and never include source strings or values.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_bounded_json --all-targets -- -D warnings
cargo doc -p coding_adventures_bounded_json --no-deps
```
