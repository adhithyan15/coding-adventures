# Kiln

Kiln is a language-agnostic, content-addressed build system. The design is in
[KILN00](../../../specs/KILN00-kiln-build-system.md); this program is its Go
implementation, built up one spec at a time.

## What is here today

- `internal/key` - the canonical **action key** encoder of
  [KILN02](../../../specs/KILN02-key-encoding.md) (key schema 1). A key is
  `SHA-256(preimage)`, where the preimage is a length-framed, section-named,
  bytewise-sorted encoding of everything that could change a build result.
  The encoder validates strictly and refuses (never repairs) non-canonical
  input.
- Golden vectors at
  [`code/specs/fixtures/kiln/key-vectors-v1.json`](../../../specs/fixtures/kiln/key-vectors-v1.json)
  that any implementation must reproduce.
- A tiny CLI.

It uses only the Go standard library and depends on no other package in this
repository.

## Usage

```
go build -o kiln .
./kiln version                                   # kiln 0.1.0 (key schema 1)
./kiln verify-vectors ../../../specs/fixtures/kiln/key-vectors-v1.json
```

```go
k, err := key.Key(action)   // [32]byte
fmt.Println(key.Hex(k))     // 64 lowercase hex characters
```

## Regenerating the vectors

The vectors are defined in `internal/key/vectors_def_test.go`:

```
go test ./internal/key -run TestGenerateVectors -update
```

Changing a vector's expected value is a key-schema change; bump the version
in KILN02 first.

## Testing

```
go vet ./... && go test ./... -cover
```

## Ledger

`internal/ledger` is the directory ledger of KILN00 section 10: a directory of
immutable JSON-lines segment files (`<10-digit sequence>-<16 hex>.jsonl`) that
record which action keys passed (`pass`) or were revoked (`tombstone`).
Hashes and provenance only; no build output.

- `Open(dir, ledger.Shared|ledger.Local).Lookup(keys)` returns each key's newest record if it is a pass.
  Newest means: later `at`, then higher segment sequence, then higher segment
  name, then later line; at equal `at` a tombstone beats a pass. A newer tombstone hides a pass; a newer pass un-shadows
  the key. Any unreadable or invalid segment fails the whole lookup (a skipped
  segment could hide a tombstone and resurrect a false hit).
- `Store.Record(records, auth, now)` writes one new segment atomically (temp
  file in the same directory, fsync, rename, directory fsync). `auth` is an
  `Authority` (`Trusted()`, `WritesLocalLedger()`, implemented by the Host): a
  shared ledger needs `Trusted()`, a local one `WritesLocalLedger()`, and nil or
  anything else yields `ErrUntrusted`. Records dated more than `MaxClockSkew`
  (5 minutes) after `now` are refused.
- `ledger.Merge(snapshotDir, deltaFile, outDir, auth, now, prune)` is the core
  of the `ledger-merge` job: it needs `auth.Trusted()`, strictly validates the
  delta (one bad record rejects it all), re-stamps every delta record's `at` to
  `now` (a writer cannot choose its own ordering against tombstones; `now` is a
  parameter, so output stays deterministic), refuses snapshot records dated
  after `now` plus the skew, unions with the snapshot, keeps one record per key,
  and writes deterministic sorted output to a `<outDir>.partial-*` sibling that
  is renamed into place (removed on error). Merge assumes the delta came from a
  run the Host marked trusted; a compromised trusted job can still forge passes,
  which re-stamping, the daily uncached run and tombstones bound. `OlderThan(cutoff)` prunes, but never drops a tombstone
  while a pass for its key survives.
- Decoding is strict and canonical: lowercase 64-hex key, known kind, UTF-8
  without control characters, target at most 512 bytes and other fields at most
  128, UTC whole-second `at`, no unknown fields, one JSON value per line that
  must equal the canonical encoding. Caps: 50,000 records and 16 MiB per
  segment, 10,000 segments and 1,000,000 records per directory, all enforced
  while reading.
- Segment names are pattern-checked before any path is built; symlinks and
  non-regular files are refused (regular-file check on the open descriptor).
