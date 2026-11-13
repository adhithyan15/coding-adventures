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
  (1 minute, `MaxRecordSkew`) after `now` are refused.
- `ledger.Merge(snapshotDir, deltaFile, outDir, auth, now, prune)` is the core
  of the `ledger-merge` job: it needs `auth.Trusted()`, strictly validates the
  delta (one bad record rejects it all), restamps the delta (a pass keeps its
  earlier `at`, capped at `now`, so a stale pass cannot undo a newer tombstone;
  a tombstone is stamped `now`; `now` is a parameter, so output stays
  deterministic), refuses snapshot records dated
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

## Legacy keys (`legacy-plan`)

`internal/legacy` implements the core of [KILN03](../../../specs/KILN03-legacy-keys.md):
keys for today's shell BUILD files, in **shadow mode** (nothing is skipped).

```
kiln legacy-plan --graph <graph.json> --out <plan.json> [--os linux|darwin|windows] [--root <dir>] [--source git|folder]
kiln explain --plan <plan.json> <package>
```

- The graph is the `kiln-legacy-graph/1` export of the old build tool. It is
  loaded strictly (schema, unknown fields, size cap, sorted and unique lists,
  every edge names a listed package, every path valid per KILN02).
- Per package: the **closure** (the package, its `dev_deps`, and everything
  reachable through `deps[OS]`, nested packages and enclosing packages, for
  every member), each member's **own digest** (every file under its directory
  except nested packages'), and its **reads** (`repo_inputs`, globs, the
  repository-source-input boundary file, and configuration files sitting in
  ancestor directories). Directory and glob reads become one `consumed` entry
  holding a tree digest that covers paths, kinds, executable bits and content.
  `tool:build-tool` digests the old build tool's source, its in-repo `replace`
  targets, `go.work` and `vendor/`.
- The key is a KILN02 key with **semantics version 0**. Tools and env are empty
  in this milestone, so every package carries `m1-incomplete:tools-env` and
  `m1-incomplete:trace` and none is `cacheable` yet. Packages whose closure has
  an unpinned language (`code/kiln/legacy/pinned.json`) are class `unpinned`;
  `unknown-repo-inputs:<name>`, `never.json` entries and a missing BUILD file
  for the OS are `uncacheable`.
- **Advisory hints** (KILN03 section 6) come from static scans of each
  package's own files, its closure members' files and its reads, in the same
  pass that hashes them: a `../` path that leaves the package, closure and
  reads is `escape:<path>`; indirection markers (`dirname`, `$PWD`,
  `git rev-parse`, a bare `cd ..`, an absolute path, ...) are `indirect:<file>`;
  text over 256 KiB or an executable that is not text is `unscanned:<file>`.
  They land in each record's sorted, de-duplicated `hints` array (at most 16
  per kind), are counted in `histogram.hints`, and make **no soundness or
  security claim**: they are for triage and debugging, and can neither make a
  package cacheable nor uncacheable. `kiln explain` prints them in a separate
  section labelled advisory. Soundness for reuse will come from traced
  read-sets recorded under `strace` on trusted CI (KILN04, forthcoming), so
  every package also carries `m1-incomplete:trace` and none is `cacheable`.
- **Symlink rules stay hard.** A symlink (in the package, a closure member or a
  read) that does not resolve, through any chain or directory link, to the
  package, closure or reads, that leaves the repository, or that exhausts the
  40-hop limit, is the reason `symlink:<link>`; a symlinked BUILD file is
  `symlink:<build-file>` and has no key. These make the package `uncacheable`.
- The Git source refuses to plan when any untracked, non-ignored file exists
  (build products would enter the digests). `--source folder` has no such
  check: it prints a warning to stderr and keeps going (shadow mode), so run it
  only on a tree that is known to be clean. Output is deterministic JSON,
  written atomically, ending with a histogram of classes, reasons and closure
  sizes.
- Glob semantics are pinned by
  [`glob-cases-v1.json`](../../../specs/fixtures/kiln/glob-cases-v1.json), which
  the old build tool's matcher must also satisfy.

## Trace parser (`internal/trace`, KILN04 stage 1a)

`internal/trace` is the pure parser for traced read-sets: it turns the text
written by `strace -f --quiet=attach -y -s 4096 -o <file>` into an ordered list
of path events plus the process tree. It never runs `strace` and never touches
the filesystem.

```go
tr, err := trace.Parse(file, "/work/repo", trace.Limits{})
// tr.Events: Pid, Syscall, Kind, Path, Path2, DirfdPath, ReturnFDPath, Result, Flags, Addr
// tr.Procs / tr.Exits: the process tree and exit statuses
```

- **Closed table.** Every syscall name is handled (open, metadata, listing,
  directory, write, process, network), ignored (no path), or refused. An
  unknown name is `trace-incomplete:unknown-syscall:<name>`, a refused one
  `trace-incomplete:refused:<name>`; `clone` with `CLONE_UNTRACED` or any
  `CLONE_NEW*` is `trace-incomplete:clone-flags`.
- **A real scanner, no regular expressions.** `<unfinished ...>` and
  `<... X resumed>` halves are paired per pid; strings are unescaped exactly
  (`\"`, `\\`, `\n`, octal, hex); `-y` annotations such as `3</a b/c>` are read
  with their `\<` and `\>` escapes. Truncated (`"..."...`), non-UTF-8,
  `(deleted)` and ambiguous paths are errors.
- **Process model.** Each process has a working directory (copied by
  fork/clone, shared under `CLONE_FS`, changed by `chdir`/`fchdir`). Every pid
  must have an exit line and every split call must be resumed.
- **Fail closed.** Every refusal is a `*trace.Error` whose message is
  `trace-incomplete:<detail>`, with a stable detail and no path text. Line,
  total-byte and record caps (defaults 1 MiB, 64 MiB, 1M records, a
  resident-memory bound) give `trace-incomplete:size`; argument counts are
  bounded while scanning (`arity`).
- **Working directory.** A child's cwd is snapshotted when the parent's
  `fork`/`clone` succeeds; a split call uses the cwd at its start; a `chdir`
  on a cwd shared by live `CLONE_FS` processes is `cwd-race`. A trace whose
  last line lacks its newline is incomplete.
- **Sandbox helper.** `trace.ParseOptions(r, cwd, trace.Options{HelperRoot: true, HelperExe: kiln})`
  models the helper tree: the root (and its threads) spawns exactly one init
  child, which sets up, may re-exec `HelperExe`, and then execs the step; the
  first successful exec of a non-Kiln path is the step. Root events and the
  init child's setup are discarded (failed execs of other paths are kept);
  `unshare`, `mount`, `pivot_root` and the like are tolerated there only. The
  step's cwd must equal `cwd`. `Trace.StepExit` and `Trace.InitPid` give the
  init child's status and pid. Violations are `helper-spawn`,
  `helper-activity`, `helper-exe`, `helper-no-init`, `helper-no-exec`,
  `helper-cwd` and `helper-config` (KILN04 section 4.5).
- **Boundary.** Paths are reported as printed, with the working directory or
  `dirfd` path they are relative to. Resolving them against a tree (symlinks,
  `..`, stability) is stage 1b.

Fixtures in `internal/trace/testdata/` are hand-written in strace 6.x format;
`FuzzParse` checks the parser never panics and only ever returns
`trace-incomplete` errors.
