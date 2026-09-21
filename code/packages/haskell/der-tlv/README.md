# der-tlv

Bounded, authority-free DER identifier and definite-length framing for
Haskell. The decoder rejects BER-only encodings, reports the portable stable
error identifiers and byte offsets, and exposes transactional cursor reads.
Returned headers, values, and complete elements are strict `ByteString`
slices of the caller-provided input.

## Development

```bash
cabal test
cabal build der-tlv:lib:der-tlv der-tlv:test:spec \
  --ghc-options=-Wall --ghc-options=-Werror
```

The tests exercise the shared language-neutral DER-TLV fixture corpus. Cabal
also emits an HPC report when coverage is enabled in `cabal.project`.
