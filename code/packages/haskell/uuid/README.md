# uuid

A native Haskell implementation of UUID generation and parsing from first
principles, covering RFC 4122 and RFC 9562.

## API

- `uuidFromBytes`, `uuidBytes`, `uuidFromInteger`, and `uuidToInteger` convert
  the 128-bit value without changing its network byte order.
- `parse`, `render`, and `isValid` handle canonical, compact, braced, and URN
  text forms.
- `uuidVersion`, `uuidVariant`, `isNil`, and `isMax` expose UUID metadata.
- `v1`, `v4`, and `v7` generate time/random UUIDs in `IO`.
- `v3` and `v5` generate deterministic name-based UUIDs with the repository's
  local MD5 and SHA-1 packages.
- `namespaceDNS`, `namespaceURL`, `namespaceOID`, `namespaceX500`, `nilUUID`,
  and `maxUUID` provide the standard constants.

## Running the tests

```sh
cabal test all
```
