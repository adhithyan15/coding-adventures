# KILN03 — Keys for Legacy Shell BUILDs (M1)

## Status

Accepted (2026-11-12), after an adversarial design review and a security review. Implements
KILN00 §15.1 on top of KILN02 key encoding, the `Source`/`Host` interfaces
(KILN00 §13) and the pass ledger (§10). **Shadow mode only:** nothing in M1
or M2 skips a build; keys are computed, recorded and compared.

## 1. The problem in one paragraph

Kiln's real keys (KILN00 §8) need declared inputs and isolated work roots.
Today's 5,500 shell BUILD files have neither: they run in place, inherit the
CI job's environment, and read whatever they like — sibling packages
(`cd ../dep && npm ci`), configuration in ancestor directories (`Cargo.toml`
workspaces, `.npmrc`, `mise.toml`), scripts that walk upward with `dirname`.
To cache them anyway, a legacy key must **over-approximate** everything a
BUILD could observe. Over-approximation only lowers the hit rate;
under-approximation produces false hits, the one failure Kiln must not have.
Every rule below errs toward the first, and the rules that are still
heuristic are named as such and backed by the M2 audit (§8).

## 2. Where the knowledge comes from: the legacy graph export

Kiln's core knows no language (KILN00 §5). The language knowledge needed here
— dependency edges per OS, test-only dependencies, repository-input
consumers, extra toolchains, which BUILD file each OS runs — already lives in
the current Go build tool. That tool gains one flag; Kiln consumes its output
as plain data:

```
build-tool -root <repo> -force -emit-kiln-graph <out.json>
```

It lists **every** discovered package for all three platforms at once
(ignoring `-language` and sharding). Schema `kiln-legacy-graph/1`:

```json
{
  "schema": "kiln-legacy-graph/1",
  "packages": [
    {
      "name": "rust/parser",
      "rel_path": "code/packages/rust/parser",
      "language": "rust",
      "build_files": {"linux": "BUILD", "darwin": "BUILD", "windows": "BUILD_windows"},
      "deps":     {"linux": ["rust/lexer"], "darwin": ["rust/lexer"], "windows": ["rust/lexer"]},
      "dev_deps": ["rust/test-support"],
      "repo_inputs": ["code/grammars/ruby"],
      "repo_input_globs": [],
      "extra_toolchains": ["dotnet"]
    }
  ]
}
```

| Field | Source in the old tool | Meaning |
|---|---|---|
| `build_files` | `GetBuildFileForPlatform` candidate order (`BUILD_mac`, `BUILD_linux`, `BUILD_mac_and_linux`, `BUILD_windows`, `BUILD`) | The single file each OS runs, relative to `rel_path`, or `null`. |
| `deps` | resolver graph incl. platform overrides | **Direct** build/runtime edges. |
| `dev_deps` | the test-trigger relation, inverted | Packages this package's tests link against (Cargo dev-deps and equivalents). |
| `repo_inputs` | the `repositoryInputConsumers` table | Repo-relative paths outside the package that it reads. |
| `repo_input_globs` | the same table, for consumers whose inputs are a predicate over the whole tree | Repo-relative glob patterns (`**` any depth, `*` within a segment), e.g. `code/**/*.mll`. |
| `extra_toolchains` | `discovery.Package.ExtraToolchains` | Toolchains beyond its own language. |

Lists are sorted and de-duplicated; output is deterministic for a tree. A
golden fixture pins the format. Discovery for the export looks at all three
platforms' BUILD variants at once, so a package whose only BUILD file is
`BUILD_windows` is still listed when the export runs on Linux or macOS.

A consumer in the old table that declares neither paths nor globs cannot be
keyed; the export lists it with `"repo_inputs_unknown": true`, and Kiln marks
every package whose closure contains it `uncacheable` (reason
`unknown-repo-inputs:<name>`).

## 3. The legacy key

For package P on platform OS, the key is a KILN02 key with schema 1 and
**semantics version 0**, which is reserved for legacy keys (§3.5).

### 3.1 Own digest

`D(p)` is SHA-256 of the KILN02 `inputs`-list encoding (§3.5) of **every
file `Source.Files` reports under `rel_path`**, excluding files under any
nested directory that is itself a package. There is no extension filter —
deliberately unlike the old hasher's per-language selectors, which miss files
a BUILD can read.

### 3.2 Closure

```
closure(P, OS) = {P} ∪ N(P) ∪ E(P) ∪ deps*( deps[OS](P) ∪ dev_deps(P) ∪ N(P) ∪ E(P) )
```

- `deps*` is the transitive closure over `deps[OS]` edges only.
- `N(P)` is every package whose `rel_path` lies under P's — containment is an
  edge, because a BUILD can `cd ext/foo && cargo build` with no `../` at all.
- `E(P)` is every package whose `rel_path` *encloses* P's. Tools search
  upward, so an enclosing package's own manifest (`Cargo.toml`,
  `package.json`, `go.mod`, `tsconfig.json`) is configuration for P.
- `dev_deps` apply at P only: only P's own tests run, so only P's test-only
  dependencies are linked. Applying them at every member would merge large
  parts of the graph through dev-dependency cycles.
- The closure is a set, so cycles are harmless and order-independent.

### 3.3 Reads

`reads(P)` is the union, over every closure member q, of:

1. q's `repo_inputs`, and q's `repo_input_globs` — each glob is encoded as
   one `consumed` entry `glob:<pattern>` whose digest is the tree digest of
   every file `Source.Files` matches;
2. the inputs of every boundary in
   `code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json`
   whose `applies_to` matches q (an extra source, not the only one);
3. **ancestor configuration**: every regular file that sits *directly* in
   each ancestor directory of q's `rel_path`, up to and including the
   repository root, that is not inside a package and not on the ignore list
   (`*.md`, `LICENSE*`, `CHANGELOG*`, `.gitignore`). This catches the files
   tools search upward for — workspace `Cargo.toml`, `go.work`,
   `tsconfig.base.json`, `.npmrc`, `global.json`, `Directory.Build.props`,
   `mise.toml`, linter configs — without knowing any of their names.

A read path that is a file is encoded as a KILN02 `reads` entry. A read path
that is a directory is encoded as **one** `consumed` entry (§3.4) carrying
its **tree digest**: SHA-256 of the KILN02 `inputs`-list encoding of every
file under it — paths, kinds, executable bits *and content digests*, never
paths alone. So `code/grammars/**` costs 32 bytes per key, not thousands of
entries.

**Symlinks.** A link entry's digest covers only its target *path*. So every
symlink in P's inputs, and in every read, is resolved: a target inside P, a
closure member or a read path is already covered; any other target makes P
uncacheable with reason `escape:<link>`.

### 3.4 Key fields

| KILN02 section | Value |
|---|---|
| `platform` | OS, arch |
| `host` | always present: `Host.ImageID()` |
| `action` | target `P.name`, phase `test` |
| `recipe` | `LegacyRecipe([(name, contents)])` of the single file `build_files[OS]` (KILN02 §3.4, generalised) |
| `inputs` | P's own files (full entries) |
| `reads` | the file-valued reads of §3.3 |
| `consumed` | `pkg:<q.name>` → `D(q)` for every member q ≠ P; `dir:<path>` → tree digest for every directory-valued read; `glob:<pattern>` → tree digest of its matches; `tool:build-tool` → tree digest of `code/programs/go/build-tool` **plus every in-repo module it imports** (its `go.mod` `replace` targets) — the executor rewrites and splits BUILD lines, so its source is part of what runs |
| `tools` | §4 |
| `env` | §5 |

Including every member's own digest makes the key change whenever any file
anywhere in the closure changes. Each digest is computed once per run.

### 3.5 What semantics version 0 means

Under semantics version 0, a `consumed` entry's `key` is a **content
digest** (an own digest, a tree digest or a tool-source digest), not another
action's key, and its `phase` is nominal (`build`). Targets are prefixed
`pkg:`, `dir:`, `glob:` or `tool:` so the four kinds can never collide. The version
sits in the key preamble, so a legacy key can never equal a real one. KILN02
records this and pins it with a golden vector.

## 4. Tools: pins first, probes second

Toolchain identity comes from **committed exact pins**, not only from
whatever happens to be installed, so a key can be computed on a runner where
a toolchain is not installed. CI installs toolchains only for affected
languages.

`code/kiln/legacy/toolchains.json` maps each language and extra toolchain to:

```json
{
  "rust": {"pins": {"rustc": "1.83.0"},
           "probes": [["rustc", "-vV"], ["cargo", "-V"]]},
  "c":    {"pins": {"cc": "image"},
           "probes": [["cc", "--version"], ["ld", "-v"]]}
}
```

- **Identity** (in the key): the pins, plus `Host.ImageID()` for anything
  pinned to `"image"` (the runner image supplies it), plus the digest of
  `toolchains.json` itself.
- **Probes** (verification): when a tool is present, its probe runs with no
  shell, a scrubbed environment, a 10 s timeout, output capped at 64 KiB, and
  a **fixed minimal `PATH`** (the toolchain install directories, never `.`,
  `node_modules/.bin` or shims). A probe whose resolved binary lies inside
  the checkout is refused — otherwise the PR under test would choose what
  runs at planning time. Probes never run in a job that holds secrets or
  ledger-write credentials. Output is normalised (stdout and stderr captured,
  absolute paths and timestamps stripped) and must contain the pinned
  version **as a whole token** (so `1.83.0` does not match `11.83.01`);
  otherwise the package is marked `probe-mismatch` — a real error in M3, a
  reported disagreement in M2. An absent tool is not an error; its identity
  still comes from the pin. Because the key records the pin, not the
  installed version, a drifted toolchain is only caught by this probe, which
  is why `probe-mismatch` must be fatal before M3 enables reuse.
- Every closure member's language and extra toolchains must have an entry; a
  missing entry makes P uncacheable (`unknown-toolchain:<name>`), never "no
  tools".
- The always-included entries are `sh`/`bash` and the C toolchain and linker
  (`cc`, `ld`, and on macOS `xcode-select -p` plus the command-line-tools
  version) — the Haskell-linker failure class.

M1 does not hash `ci.yml`. Pins plus the image identify the toolchains;
things neither covers (apt packages, pip pins) are what the M2 audit exists
to find.

## 5. Environment

Legacy BUILDs inherit the job's environment. `code/kiln/legacy/env.json`
lists the variable names whose values can change a build or test result
(`RUSTFLAGS`, `CARGO_*`, `NODE_OPTIONS`, `DEVELOPER_DIR`, `PYTHON*`,
`JAVA_HOME`, `DOTNET_*`, `CI`, `GOFLAGS`, `LANG`, `TZ`, …; trailing `*`
matches a prefix). Each matching name that is set goes into the `env`
section with its value; an unset name is simply omitted (absence means
unset, and an empty value stays distinct from unset — no sentinel string
that a real value could collide with). The list's own digest is in every
key.

**Secrets.** After wildcard expansion, any name containing `TOKEN`,
`SECRET`, `PASSWORD`, `PASSWD`, `CREDENTIAL` or ending in `_KEY` is an
**error**, not a silent skip (`CARGO_*` would otherwise pull in
`CARGO_REGISTRY_TOKEN` and put a brute-forceable hash of it in the plan).

**The allowlist is complete by construction, not by hope.** A BUILD can read
any variable it likes, and no syscall trace can see a `getenv`. So in M2 the
allowlist is a measurement, and **before M3 enables reuse, legacy BUILDs run
with their environment scrubbed to exactly the key's `env` section** plus
`PATH`, `HOME` and `TMPDIR`. Whatever a BUILD cannot see cannot change its
result. M2 runs a scrubbed shadow execution on the full-build shards and
reports every package whose result changes, which is how the allowlist is
grown.

## 6. Escapes and indirection

A legacy BUILD can reach outside its closure in ways a key cannot see. M1
uses two cheap, conservative rules. Both are heuristics; §8 is the real check.

1. **`../` resolution over every text file.** Every text file in P's own
   inputs (up to 256 KiB each, not just the BUILD) is scanned for path tokens
   containing `../`. Each is resolved against the file's directory. A result
   inside P is fine (source-relative imports stay quiet); a result inside
   another closure member or a read path is fine; anything else makes P
   uncacheable with reason `escape:<path>`.
2. **Indirection markers in every scanned file.** The selected BUILD *and*
   every text file in P's own inputs are scanned for: `git rev-parse`,
   `git ls-files`, `dirname`, `realpath`, `readlink`, `$(pwd)`, `$PWD`,
   `$OLDPWD`, `$0`, `${BASH_SOURCE`, `$HOME`, `~/`, a bare `cd ..` (with or
   without a trailing `/` or separator), or an absolute path outside a small
   allowlist (`/dev/null`, `/usr/bin/env`, `/bin/sh`). Any hit makes P
   uncacheable with reason `indirect:<file>`. In particular the ~301
   packages whose BUILD is `sh tools/run.sh` stay uncacheable until the
   audit proves them.
3. **Unscanned means uncacheable.** A text file over the 256 KiB cap, or a
   file with an executable bit that is not text, cannot be scanned, so it
   makes P uncacheable with reason `unscanned:<file>`. Failing closed is the
   point: these rules are heuristics, so their blind spots must cost hit rate,
   not correctness.

## 7. Cacheability classes

Keys are **always** computed. Cacheability only gates **reuse** (M3), never
measurement. Each package gets one class:

| Class | Meaning |
|---|---|
| `cacheable` | Every closure language is in `code/kiln/legacy/pinned.json`, no escape, no indirection, no unknown toolchain, not on the never list. |
| `unpinned` | Only reason: some closure language is not yet pinned (its M0 PR has not landed). |
| `uncacheable` | Any other reason (listed). |

`pinned.json` starts empty and gains a language when its pinning PR lands.
`code/kiln/legacy/never.json` lists packages known to depend on the network
or the clock in tests; they are always `uncacheable`. The two files have
different owners and are kept separate.

M2 records shadow passes for `unpinned` packages in a separate scope and
reports hit and disagreement rates per class: the unpinned disagreement rate
is the measured value of M0.

## 8. Known limits and the M2 audit

- **Order dependence.** In a full build, earlier BUILDs leave `node_modules`,
  `.venv` and shared Rust `target/` directories in siblings, so a pass in a
  full build is not the same experiment as a pass in a PR subset. This cannot
  be keyed. M2 records the run kind (`full` or `affected`) with each shadow
  record and reports disagreement split by run kind.
- **The resolver is the trust root.** Every edge comes from the old tool,
  which KILN00 §2 records has missed dependents before. The definitive check
  is an M2 audit: on one Linux full-build shard set, run each BUILD under
  `strace -f -e trace=%file,%process,connect -e signal=none` — `%file`
  includes `stat`, `access`, `readlink` and directory listing, so existence
  checks and globs are seen, not only opens; `connect` finds network use.
  Map every touched repository path to a package and diff against
  `closure ∪ reads`; every miss is a bug in the export or in this spec, and
  every package that connects to the network joins `never.json`
  automatically instead of by hand.
- **Audit hygiene.** strace runs unprivileged (no `--privileged`, no added
  `SYS_PTRACE`, no `sudo`), without `-v` and with a short `-s`, because
  verbose `execve` lines print the whole environment. Its logs are treated
  as sensitive and never uploaded as artifacts; only the derived
  path-to-package diff is.
- **Gate for M3.** Reuse is not enabled until the audit is clean (no
  unexplained misses), the scrubbed-environment shadow run (§5) shows no
  result changes, and `probe-mismatch` is fatal.
- **Ignored and ignore-listed files.** Files Git ignores are invisible to
  `Source`, so they are outside every key; this is safe only on fresh
  runners, which is why reuse is CI-only. The ancestor-directory ignore list
  (`*.md`, `LICENSE*`, `CHANGELOG*`) is a hit-rate choice; a test that reads
  such a file is a false-hit source the audit will report.

## 9. Commands and output

```
kiln legacy-plan --graph <graph.json> --out <plan.json> [--os linux|darwin|windows]
kiln explain --plan <plan.json> <package>
```

- `legacy-plan` must run on a **clean tree** — before any BUILD in the same
  job. With the Git `Source` it fails if any untracked, non-ignored file
  exists, because build products would otherwise enter `D(p)`.
- Per package it writes: `key`, `class`, `reasons`, `closure_size`, and the
  component digests (own, reads, consumed set, tools, env). `explain` prints
  them; comparing two plans shows which component changed.
- The plan ends with a **histogram**: packages per class, per reason, and
  closure sizes (min, median, p90, max). It is the first number M2 needs.
- Output is deterministic: identical tree, graph, pins and environment give
  byte-identical plans. Hashing is parallel and memoised within a run.

## 10. Tests that must exist

- Golden graph export for a fixture tree (old tool), including a
  `BUILD_mac_and_linux` selection.
- The key changes when any of these changes: a file in P; a file in a deep
  dependency; a file in a dev-dependency of P; a file in a nested package; an
  ancestor configuration file; a boundary input; a `repo_inputs` file; the
  selected BUILD text; the old build tool's source; an allowlisted
  environment variable; a pin; `toolchains.json`; `env.json`; the host image.
- The key does **not** change when: a file in an unrelated package changes;
  a dev-dependency of a closure member other than P changes; another OS's
  BUILD file changes text that is not this OS's selected file *and* is not in
  P's inputs (it always is, so this is asserted only for the recipe
  component).
- Dependency and dev-dependency cycles terminate and are order-independent.
- `../` escapes are detected in a non-BUILD text file; in-package and
  in-closure `../` are allowed; each indirection marker, in the BUILD or in
  any other text file, makes the package uncacheable; an over-cap text file
  and an executable binary each make it `unscanned`.
- A symlink whose target leaves the closure and read paths makes the
  package uncacheable; one pointing inside the closure does not.
- An enclosing package's manifest change changes the nested package's key.
- An unset allowlisted variable and one set to any string (including
  `<unset>` and the empty string) give three different keys; a secret-like
  name after wildcard expansion is an error.
- A probe whose binary resolves inside the checkout is refused; a pinned
  version that only appears as a substring of a longer version is a
  mismatch.
- An unpinned language makes the package `unpinned`, not `uncacheable`, and
  its key is still computed.
- A dirty tree is refused.
- Determinism: two runs produce byte-identical plans.
