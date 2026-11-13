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

## Manifest store (`internal/ledger`, KILN04 stage 2)

Manifests are a store-only record kind in a `manifests/` subdirectory of the
ledger directory: for one weak key, the sorted names (`pkg:`, `path:`,
`list:`) a traced build read. They hold names only, never digests, and can
never cause a hit by themselves (the strong key still needs a pass record).
The pass-ledger reader never opens that subdirectory, and a manifest line
dropped among pass segments is refused (and the reverse).

- `OpenManifests(ledgerDir, scope).Record(manifests, auth, now)` writes one
  immutable segment through the same `Authority` gate as passes. Manifests
  are not restamped. A record over 8,192 entries, 512 bytes per name or 1 MiB
  per line is refused with `ErrManifestTooLarge` (`manifest-too-large`).
- `.Load(now, wanted)` returns a `ManifestSet`; `wanted` is an optional set of
  weak keys (`nil` keeps everything; a non-nil set ignores every other record
  while parsing, so memory follows the caller's packages, not the store), and
  `Retained(weak)` is `Manifests` with each verified id, computed once at load.
  `Manifests(weak)` gives at most 4
  manifests, newest first by `(run, at)`, with distinct ids (the id is the
  SHA-256 of the entries list, verified on read). Records older than 30 days
  are dropped, and a re-recorded id does not crowd out other read-sets.
- Reader policy: an over-cap record or segment is dropped and reported in the
  `ManifestReport`; structural corruption (non-canonical line, unknown field,
  id mismatch, bad entry, a store over 256 MiB) makes the whole store
  unusable and `Load` returns an error, so there are no hits.
- `MergeManifests(snapshotDir, deltaFile, outDir, auth, now)` is the manifest
  half of `ledger-merge`: it needs `Trusted()`, reads the delta strictly,
  applies retention, and writes a deterministic new snapshot.

## Legacy keys (`legacy-plan`)

`internal/legacy` implements the core of [KILN03](../../../specs/KILN03-legacy-keys.md):
keys for today's shell BUILD files, in **shadow mode** (nothing is skipped).

```
kiln legacy-plan --graph <graph.json> --out <plan.json> [--os linux|darwin|windows] [--root <dir>] [--source git|folder] [--probe]
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
- The key is a KILN02 key with **semantics version 0**. Every package carries
  `m1-incomplete:trace` (no traced read-set exists yet, KILN04), so none is
  `cacheable` yet. Packages whose closure has an unpinned language
  (`code/kiln/legacy/pinned.json`) are class `unpinned`;
  `unknown-repo-inputs:<name>`, `unknown-toolchain:<name>`, `never.json`
  entries and a missing BUILD file for the OS are `uncacheable`.
- **Tools** (KILN03 section 4). `code/kiln/legacy/toolchains.json` maps each
  closure language and extra toolchain to `pins` (an exact version from the
  repository's committed pins, or `image`, meaning the runner image supplies it
  and `Host.ImageID()` is the identity), `probes` and `roots` (the directories
  of each tool, for `ConstructPATH`). The `tools` section of the key holds the
  pins of every language in the package's closure plus the always-included set
  (`sh`/`bash`, `cc`, `ld`, `strace` on Linux, `xcode-select` on macOS) as KILN02
  kind-2 entries; `policy:toolchains.json` is a consumed entry. A closure
  language or extra toolchain with no entry is the hard reason
  `unknown-toolchain:<name>`, never "no tools". `ToolsFor` and `ConstructPATH`
  are plain functions for the KILN04 weak key.
- **Probes** (`--probe`, off by default, only for the platform the machine
  runs). Each probe runs with no shell, an environment of just a fixed `PATH`
  (toolchain directories only), `LANG=C`, `TZ=UTC` and a private `HOME` and
  working directory, a 10 s timeout and 64 KiB of output; a binary that resolves
  inside the checkout is refused. Output must carry the pin as a whole token, or
  the package gets the soft reason `probe-mismatch:<tool>` (reported, never a
  class). An absent tool is not an error. The plan's `probes` array shows each
  tool's status.
- **Environment** (KILN03 section 5). `code/kiln/legacy/env.json` is a strict
  allowlist (a trailing `*` makes a prefix). Each set, matching variable goes
  into the `env` section with its value; an unset one is omitted. A name
  containing `:` is reserved, and a name that is secret-like after expansion is
  an error that names the variable. `policy:env.json` is a consumed entry. Plans
  and `kiln explain` show tool names (`name@pin`), env variable names and the
  tools and env component **digests**, never values.
- `code/kiln/legacy/host-roots.json` (KILN04 section 5.3: `roots`, `ephemeral`,
  `absent-ok`) has a strict loader that cross-checks tool names against
  `toolchains.json`. Only its digest is for the KILN04 weak key; nothing in the
  static key reads it yet.
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

## Trace classification (`internal/traced`, KILN04 stage 1b-i)

`internal/traced` turns the parsed traces of one action into a **raw record**:
what the action observed, what it wrote, and every reason its trace cannot be
trusted. It is pure; everything outside the traces is injected.

```go
cfg := traced.Config{
    Root:      "/work/repo",                     // the real checkout path
    Packages:  traced.NewPackageIndex(relPaths), // nested-aware
    Tree:      tree,                             // tracked tree (SourceTree adapts a Source)
    FS:        disk,                             // post-run Lstat, Readlink, ReadHead
    HostRoots: hostRoots, Pinned: pinned, Tools: weakKeyTools,
    Home: home, Tmpdir: tmp, JobStart: start,
}
rec, err := traced.Classify(cfg, traced.Input{
    Package: "code/packages/go/parser",
    Traces:  stepTraces,   // one per BUILD step, in order
    Recipe:  recipeLines,  // required: the expected `sh -c` lines
})
// rec.Entries: ["list:code/packages/go", "path:go.work", "pkg:code/packages/go/lexer", ...]
// rec.Reasons: ["host-read:$HOME/.cache/x", "network:/usr/bin/curl", ...]
```

- **Entries** are `pkg:<package>` for a path inside a package (tracked or not,
  the deepest package wins), `path:<rel>` outside every package (an absent path
  at its first absent component) and `list:<rel>` for a listed directory. A
  path the action made itself, or that lies under a directory it made, is its
  own output and records nothing.
- **Resolution** walks each path through the post-run disk, expanding symlinks
  (40 hops) and applying `..` to the resolved directory. Every crossed link is
  an observation. A component that may have changed under the action is
  `trace-incomplete:unstable-path`, with three narrow exceptions (KILN04 5.5):
  a successful open settles its own final name, a stat or exec of a name whose
  whole change log is `O_CREAT` opens (and at most one later `unlink`) is
  accepted, and `$HOME` itself may be stat'ed when the checkout is under it.
- **Exec** reads the program, its `#!` chain and the ELF loader. `/proc` and
  `/dev/fd` follow the ephemeral list. Reads outside the repository follow
  `host-roots.json`. Network use is judged by address.
- **Writes** are coarsened to write roots; tracked paths written are listed
  separately and give `writes-tracked`. Untracked observations and traversed
  components are exposed for the merge job's cross-action checks (KILN04
  section 7). There are no digests in a raw record.
- `CheckRecipe(steps, lines)` compares each step's exec argv with the expected
  `sh -c` lines.

The decisions are in KILN04 section 5.5. `FuzzClassify` checks the classifier
never panics and always returns sorted, unique, clean output.

## Traced keys (`internal/key/traced.go`, `internal/legacy/traced.go`, KILN04 stage 1b-ii)

A traced action has two KILN02 keys of semantics version 2. The **weak key**
covers what a trace cannot see (platform, image, BUILD recipe, the
`tool:build-tool` and `policy:` digests, the static closure's tool pins, and the
scrubbed environment). The **strong key** is the weak key with `consumed`
replaced by `weak:` -> W plus one entry per manifest entry, each paired with its
digest as it is now.

```go
tr, err := legacy.NewTraced(graph, opts, "/abs/checkout", []string{"-exec-wrapper=/k"})
w, err := tr.WeakKey("code/packages/go/example") // *legacy.Refusal if not traceable
env := w.Environ()                               // exactly what the recorder runs with
s, err := tr.StrongKey(w, []string{"pkg:code/packages/go/dep", "path:code/x.py", "list:code/shared"})
```

- **Digests** (`internal/key`): `path:` states `00` absent, `01` directory,
  `02||exec||SHA-256(content)` file, `03||bytes(target)` link; the missing-package
  digest; the sorted listing digest; and `T(q)`, which adds the names of directly
  nested packages to `D(q)`.
- **Digest provider** (`Traced.CurrentDigests`): memoised, reads only the tracked
  tree through `Source`, never the disk.
- **`ScrubbedEnv`**: allowlist, then `HOME`/`TMPDIR`, then a constructed `PATH`,
  then `LANG=C.UTF-8` and `TZ=UTC`, later steps winning. One source of truth for
  the key and the recorder.
- Only strong keys hold `weak:`, so the two can never collide. An entry that is
  not `pkg:`, `path:` or `list:` is refused.
- Golden vectors for all of this are in `key-vectors-v1.json`; `kiln
  verify-vectors` checks them.

## Sandbox (`kiln sandbox-exec`, `internal/sandbox`, KILN04 stage 3b)

`kiln sandbox-exec --config <abs path> -- <argv...>` runs one traced BUILD step
so that the trace is the whole story: the build tool's `-exec-wrapper` is
`["/abs/kiln","sandbox-exec","--config","/abs/cfg.json","--"]`, with
`KILN_ACTION` (the package `rel_path`) in the wrapper's environment.

```json
{
  "repo_root": "/home/runner/work/repo",
  "home": "/home/runner",
  "tmpdir": "/home/runner/_tmp",
  "ro_roots": ["/usr", "/etc", "/bin", "/lib", "/lib64", "/opt/hostedtoolcache/go/1.26.1"],
  "state_roots": ["/home/runner/.cargo/registry"],
  "masks": ["/home/runner/work/repo/.git"],
  "env_by_action": {"code/packages/go/a": ["HOME=/home/runner", "PATH=/usr/bin:/bin"]},
  "kiln_exe": "/opt/kiln/bin/kiln"
}
```

The new root is an **allowlist**, built on an empty tmpfs. The step sees only
the `ro_roots` (the `image` and `tool` roots of `host-roots.json`, read-only;
a symlink such as `/bin` is recreated), the checkout writable at its own
absolute path, the writable `state` roots, fresh empty tmpfs for `$HOME`,
`TMPDIR`, `/tmp`, `/var/tmp` and `/run`, a minimal `/dev`, and its own `/proc`
(`hidepid=2`). Nothing else of the host is there, because a pathname Unix
socket or FIFO is reachable through the filesystem whatever the mount flags.
An entry may not be, or contain, a directory the sandbox builds itself, so
`/var` and `/var/lib` are refused: list specific subdirectories. `masks` are the checkout's own direct children (`.git`) made empty; a path
outside the checkout is simply absent unless it is under an allowed root, and
the configuration refuses a mask there. The step runs as uid 1000 in a new user
namespace with no capabilities, `no_new_privs` and a seccomp filter (EPERM for
the refused calls of KILN04 section 4.4, for `CLONE_UNTRACED` and for every
`CLONE_NEW*`; ENOSYS for `clone3`; any non-x86-64 ABI is killed). Its
environment is exactly the configured list, standard input is `/dev/null`, and
when it exits (it is PID 1) the kernel kills everything left behind.

- **No network, no host IPC.** The step runs in empty network, IPC and UTS
  namespaces (only `lo`, up), `/run` is empty, `/dev` holds
  only `null`, `zero`, `full`, `random`, `urandom` and `tty`, and it runs in a
  session of its own, so it cannot signal strace. The recorder runs strace with
  `--kill-on-exit` and must make the step's descriptors 1 and 2 pipes.
- **linux/amd64 only.** Elsewhere the command prints why and exits 125.
- **Needs unprivileged user namespaces.** On hosted Ubuntu runners:
  `sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0`.
- The pure parts (configuration, mount plan, BPF program, privilege-drop
  sequence) are tested on every OS. The end-to-end tests are Linux-only and
  skip when namespaces are unavailable.
- CI: the required `kiln-sandbox` job runs `KILN_REQUIRE_SANDBOX=1 go test -run
  Sandbox` (a skip is a failure) including a run under the real `strace`.
- Exit status: the step's; 125 when the sandbox could not be built; 127 when the
  step's command was not found.

The design and every decision are in KILN04 section 4.7.

## Record formats and the merge engine (`internal/recfmt`, `internal/recmerge`, KILN04 stage 4)

Stage 4 records traced read-sets in shards and merges them in one job. These two
packages are the pure half (no I/O, no clock, no CLI yet; the recorder and
`kiln record-merge` come in later PRs):

- `internal/recfmt`: the strict wire formats (shard binding, host facts,
  pre-BUILD inventory, shard header, record line) with decoders and canonical
  encoders, plus `StepsDigest` and `PartitionDigest`. Every decoder refuses what
  a hostile shard could use to hide an observation: a missing key, a null list,
  a duplicate or case-variant key, an unsorted list, a malformed path.
- `internal/recmerge`: `Merge(Input) Result`. It checks the binding of every
  shard (B1-B6), then applies the cross-action rules (M1-M15) to every record of
  every shard and decides, per action, whether it may become a manifest and a
  pass. An unknown write set anywhere voids the whole run; an action that runs in
  several shards is checked in all of them and emitted from the assigned shard
  only. The design is KILN04 section 7.1.

```go
res := recmerge.Merge(recmerge.Input{Run: run, Expected: expected, Shards: shards, Keys: keys, Now: now, Kiln: id})
if res.Emitted { /* res.Manifests and res.Passes go to the ledger */ }
```

`FuzzMerge` (`go test -fuzz FuzzMerge ./internal/recmerge`) compares `Merge`
with an independent brute-force oracle; its seed corpus runs in a normal
`go test`.

## The shard recorder, first part (`internal/record`, KILN04 stage 4a)

The checks the recorder runs on the checkout before any BUILD does (KILN04
section 4.8). No sandbox and no CLI yet (`kiln record` is a later PR).

- `record.VerifyTree(root, expectedCommit)` proves the working tree is
  byte-for-byte one commit and returns its id (pass it back later to catch a
  moved HEAD; the root must be the repository's top level; a submodule is an
  error for now; replace refs are off): `git ls-tree -r -z --full-tree <id>`
  equals `git ls-files --stage -z` (path, mode, id,
  stage 0 only); no skip-worktree or assume-unchanged bit and no
  `core.sparseCheckout`; then every entry is `lstat`ed and hashed as a Git blob
  (SHA-1 or SHA-256 by id length) through an `O_NOFOLLOW|O_NONBLOCK|O_NOCTTY`
  open, symlink targets compared, exec bits compared, ancestor directories
  checked for symlinks. Smudge filters, `eol=crlf`, LFS pointers and a FIFO in
  place of a file are all caught. At most 16 paths are named in the error; limits are 1 GiB per file, 20 minutes,
  8 workers.
- `record.Inventory(root)` lists what the commit does not hold, as
  `kiln-record-inventory/1`: a directory with no tracked file is one entry,
  symlinks are never followed, the root `.git` is skipped, the order is byte
  order.
- `legacy.DecodeSteps` / `legacy.ReadStepsFile` strictly decode the build tool's
  `-emit-kiln-steps` file (`kiln-legacy-steps/1`: the lines
  `executor.EffectiveStepLines` returns per package), and `legacy.RecordTmpdir`
  is the `TMPDIR` keys are derived under.

```go
res, err := record.VerifyTree(root, "") // nil error: the disk is the commit
inv, err := record.Inventory(root)       // inv.Paths, ready for recfmt.EncodeInventory
```

## Lookup in the plan (`kiln legacy-plan --ledger`, KILN04 stage 5a)

```
kiln legacy-plan --graph g.json --out plan.json --root . --os linux \
    --ledger <ledger-dir> --scope shared|local [--executor-arg -flag=value]...
kiln explain --plan plan.json <package>
```

Shadow only: it reports predicted hits and **skips nothing**; without
`--ledger` the plan is unchanged. Each Linux package gets one result (other
OSes: `untraced-os`):

- `ineligible:<reason>`: the never list, a KILN03 hard reason or precondition,
  a `probe-mismatch`/`probe-timeout`/`probe-refused` (with `--probe`), or a
  refused weak key such as `host-unknown`. Gating is applied **first**, before
  any key or ledger read, because the traced keys carry pins only.
- `no-manifest`: nothing recorded under the weak key.
- `hit`: a retained manifest (newest first, at most 4) whose strong key, from
  the entries' **current** digests, has a pass. The manifest id and pass record
  are shown.
- `miss`: otherwise. Manifests store names, not digests, so the *first
  differing entry* cannot be computed; a miss lists each manifest tried, the
  strong key looked up and every entry with its state in this tree (`present`,
  `absent`, `directory`, `no-package`, `not-directory`). That is a description,
  not a diff.

Each package with manifests also lists `outside_static_closure`: traced `pkg:`
entries the old graph does not know about (bugs in the old graph). The
histogram counts every class and those packages.

`--ledger` is refused with `--source folder` (a plain folder cannot vouch for a
clean tree). The weak key contains `kiln:root` (the absolute `--root`) and
`kiln:executor-args` (the `--executor-arg` flags), so they must match the
recorder or every lookup says `no-manifest`. Skipping (M3) must run Kiln and its
policy from the base revision; this stage reads whatever tree it is given. See
KILN04 section 9.1.
