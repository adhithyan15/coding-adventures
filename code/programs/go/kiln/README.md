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
- `internal/source` - the **`Source`** interface of KILN00 section 13 (which
  files are we building, what is their snapshot id, what changed). `Git`
  shells out to `git` (never via a shell, NUL-separated output); `Folder` walks
  a plain directory honouring a `.kilnignore`, never following symlinks, and
  derives its `Revision` from file contents. Both share one glob language
  (`*`, `?`, whole-segment `**`) and return key-ready paths.
- `internal/host` - the **`Host`** interface (identity, host fingerprint,
  trust, shard, work dir, reporting). `GitHub` implements the KILN00 section
  10.2 trust rule exactly (default-branch ref AND push/schedule/dispatch, fail
  closed), `Local` is the developer's machine, `Detect` picks one, and
  `ScrubSecrets` drops credentials from a child environment (section 9.6).
- A tiny CLI.

`Ledger` and `Tools` (the other two section 13 interfaces) are not built yet.

### `.kilnignore`

One glob per line, `#` comments and blank lines skipped, matched against the
whole repo-relative path (no negation, no basename shortcut, no trailing `/`).
`*` stays inside a segment, `**` as a whole segment spans directories. A path
is ignored if it or any directory above it matches, so `build` ignores the
whole tree and `**/*.log` ignores logs at any depth. `.git` is always skipped.

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
