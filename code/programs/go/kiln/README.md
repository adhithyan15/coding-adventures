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
  trust, shard, work dir, reporting). `Trusted()` means "may write the
  SHARED ledger" (KILN00 section 10.2). `GitHub` implements it exactly: the
  ref must be `refs/heads/<default branch>`, where the default branch comes
  from the GitHub-generated event JSON (`GITHUB_EVENT_PATH`,
  `repository.default_branch`) and `KILN_DEFAULT_BRANCH` can only narrow it,
  AND the event must be `push`, `schedule` or `workflow_dispatch`; everything
  else, including any error, is untrusted. That answer is only meaningful in a
  job that has run no untrusted code. `Local` (a developer machine) is never
  trusted but `WritesLocalLedger()`; `Untrusted` is what `Detect` returns when
  any CI indicator (`CI`, `GITHUB_*`, `RUNNER_*`, ...) is present but the
  environment is not exactly GitHub Actions, so a CI run is never mistaken for
  a laptop. Reports strip control characters and defang `::` / `##[` so
  untrusted text cannot inject workflow commands.
- `internal/envscrub` - `ScrubSecrets`: drops credentials (tokens, keys,
  passwords, `ACTIONS_*`, `INPUT_*`, `GITHUB_ENV`/`PATH`/`OUTPUT`/...) from a
  child environment (section 9.6 item 3). Git is always run with it, with all
  `GIT_*` removed and repo-configured hooks/fsmonitor disabled.
- A tiny CLI.

`Ledger` and `Tools` (the other two section 13 interfaces) are not built yet.

### `.kilnignore`

One glob per line, `#` comments and blank lines skipped, matched against the
whole repo-relative path (at most 64 KiB, 1000 patterns, 64 segments each; the
file is never followed through a symlink; no negation, no basename shortcut, no trailing `/`).
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

Source paths are stricter than KILN02 keys: a backslash, a colon or a Windows
reserved device name (`CON`, `NUL`, `COM1`, ...) in any path is refused on
every platform so listings, and therefore keys, agree across operating systems.
