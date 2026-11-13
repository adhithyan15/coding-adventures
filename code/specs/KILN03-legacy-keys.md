# KILN03 — Keys for Legacy Shell BUILDs (M1)

## Status

Accepted (2026-11-12), after an adversarial design review and a security review. Implements
KILN00 §15.1 on top of KILN02 key encoding, the `Source`/`Host` interfaces
(KILN00 §13) and the pass ledger (§10). **Shadow mode only:** nothing in M1
or M2 skips a build; keys are computed, recorded and compared.

*Revision (2026-11-12): traced read-sets.* The static key defined here is
now a **diagnostic upper bound**, not the basis for reuse. Reuse is decided
by [KILN04](KILN04-traced-read-sets.md): a weak key over what a tracer
cannot see (this spec's recipe, tools, environment and executor digest) and
a strong key over what a trusted run was observed to read. The file scans
of §6 are advisory hints; the symlink rule of §3.3 and the hard reasons of
§7 remain preconditions for both keys. Three things differ between the two
keys:
- A traced `pkg:` entry carries `T(q)`, not this spec's `D(q)`.
- The traced weak key also keys `PATH`, `HOME` and `TMPDIR`, which §5
  leaves unkeyed.
- `pinned.json` comes to mean "ecosystems whose fetchers and download
  state are allowed" (KILN04 §5.3–§5.4), and `never.json` shrinks to
  clock-dependent packages, since tracing reports network use.

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
| `repo_inputs` | **every** package-to-external-input relation the old tool's selection uses: the `repositoryInputConsumers` table, the fixture-consumer tables (discovery, CI-gate, test-trigger, package-metadata, toolchain-detection, source-collection, graph-diff), the native-fixture reader and case maps, and hard-coded seeds | Repo-relative paths outside the package that it reads. |
| `repo_input_globs` | the same table, for consumers whose inputs are a predicate over the whole tree | Repo-relative glob patterns (`**` any depth, `*` any run and `?` exactly one character within a segment — the same matcher as the `Source` layer), e.g. `code/**/?*.mll` (the `?` keeps a bare `.mll` out, as the old predicate does). |
| `extra_toolchains` | `discovery.Package.ExtraToolchains` | Toolchains beyond its own language. |

Lists are sorted and de-duplicated; output is deterministic for a tree. A
golden fixture pins the format. Discovery for the export looks at all three
platforms' BUILD variants at once, so a package whose only BUILD file is
`BUILD_windows` is still listed when the export runs on Linux or macOS.

The export must cover every relation the old tool's own selection relies on,
not only the first table: a package whose tests read a fixture under
`code/specs` (outside every package) is otherwise keyed without it, and a
Go test that builds the path with `filepath.Join("..", "..")` has no `../`
token for §6 to find. A test enumerates every entry of every such table and
asserts the export gives it inputs or the unknown flag, and each relation's
paths live in one place that both selection and the export read, so they
cannot drift.

A consumer that declares neither paths nor globs cannot be
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

`closure(P, OS)` is the **least set** that contains `{P} ∪ dev_deps(P)` and
is closed under three relations — for every member q, all of these are
members too:

- `deps[OS](q)` — q's direct build/runtime dependencies;
- `N(q)` — every package whose `rel_path` lies under q's. Containment is an
  edge, because a BUILD can `cd ext/foo && cargo build` with no `../` at all,
  and `D(q)` deliberately excludes nested packages' files (§3.1);
- `E(q)` — every package whose `rel_path` *encloses* q's. Tools search
  upward, so an enclosing package's own manifest (`Cargo.toml`,
  `package.json`, `go.mod`, `tsconfig.json`) is configuration for q.

It is computed by a worklist until nothing new is added. All three relations
apply to **every** member, not only to P: P's BUILD can `cd ../dep/ext/foo`,
or build a dependency that is itself nested inside another package, and the
`../` scan (§6) cannot catch either because the path lands inside a closure
member.

`dev_deps` are seeded **once, at P**: only P's own tests run, so only P's
test-only dependencies are linked. Applying them at every member would merge
large parts of the graph through dev-dependency cycles. The closure is a set,
so cycles in any relation are harmless and order-independent.

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
symlink in P's inputs, and in every read, is **fully** resolved — following
chains and directory links, with a 40-hop limit and loops refused — and the
final target is checked: inside P, a closure member or a read path is
already covered; anything else, a loop, or an exhausted hop limit makes P
uncacheable with reason `symlink:<link>`.

### 3.4 Key fields

| KILN02 section | Value |
|---|---|
| `platform` | OS, arch |
| `host` | always present: `Host.ImageID()` |
| `action` | target `P.name`, phase `test` |
| `recipe` | `LegacyRecipe([(name, contents)])` of the single file `build_files[OS]` (KILN02 §3.4, generalised) |
| `inputs` | P's own files (full entries) |
| `reads` | the file-valued reads of §3.3 |
| `consumed` | `pkg:<q.name>` → `D(q)` for every member q ≠ P; `dir:<path>` → tree digest for every directory-valued read; `glob:<pattern>` → tree digest of its matches; `tool:build-tool` → tree digest of `code/programs/go/build-tool` **plus every in-repo module it imports** (its `go.mod` `replace` targets) and, when present, `go.work` and `vendor/` — the executor rewrites and splits BUILD lines, so its source is part of what runs |
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
  `node_modules/.bin` or shims). A probe whose resolved binary (after following symlinks) lies inside
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

**Secrets.** After wildcard expansion, any name containing — compared
case-insensitively — `TOKEN`, `SECRET`, `PASSWORD`, `PASSWD`, `CREDENTIAL`,
`AUTH`, `PRIVATE`, or ending in `_KEY` or `_PAT`, is an **error**, not a
silent skip (`CARGO_*` would otherwise pull in
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
runs cheap static scans over the files it hashes. **The scans are advisory
hints for triage and debugging.** They make no soundness or security claim: a
text scan cannot prove that a build reads nothing outside its key, and every
review of one has found another way past it. A hint can neither make a
package `cacheable` nor make it `uncacheable`; hints never appear among a
package's `reasons` and never change its class (§7). Soundness for reuse
comes from **traced read-sets**: the read-set of a real run, recorded under
`strace` on trusted CI and checked against the key's inputs (KILN04,
forthcoming). Until a traced read-set manifest exists for a package it carries
`m1-incomplete:trace` and cannot be `cacheable`.

What the scans report, as entries of the plan's per-package `hints` array:

1. **`../` resolution over every text file.** Every text file (up to 256 KiB
   each, not just the BUILD) is scanned for path tokens containing `../`. Each
   is resolved against the file's directory. A result inside the allowed
   region (§12 item 17) is quiet; anything else is hint `escape:<path>`.
2. **Indirection markers in every scanned file.** Location markers
   (`dirname`, `realpath`, `readlink`, `$0`, `${BASH_SOURCE`, `$HOME`, `~/`,
   `git rev-parse`, a bare `cd ..`, `__file__`, `CARGO_MANIFEST_DIR`, ...) and
   absolute paths outside a small allowlist give hint `indirect:<file>`.
   The ~301 packages whose BUILD is `sh tools/run.sh` are the typical hit.
3. **Unscanned files.** A text file over the 256 KiB cap, or a file that is
   not text and not a known inert format, cannot be scanned and gives hint
   `unscanned:<file>`.

Hints are only a map of where a human, or the tracer, should look first. They
are sorted, de-duplicated and capped at 16 per kind per package, and the
histogram counts them apart from reasons.

**Symlink rules stay hard.** A symlink is the one case where the key itself
is blind by construction (a link's digest covers only its target text), so
these remain uncacheable reasons, `symlink:<link>`, independent of any trace:
a link in a package's own files or reads that does not resolve, through any
chain or directory link, inside the closure and reads; a link that leaves the
repository or exhausts the 40-hop limit; and a symlinked selected BUILD file.
Unknown toolchains and unknown repository inputs are hard reasons too.

The implementation widens the scans beyond P's own files: they apply to every
closure member's files and to every read file, and non-text files are
`unscanned` unless a known inert format. §12 items 13 to 21 give the exact
definitions; they define what a hint means, not a guarantee.

## 7. Cacheability classes

Keys are **always** computed. Cacheability only gates **reuse** (M3), never
measurement. Each package gets one class:

| Class | Meaning |
|---|---|
| `cacheable` | Every closure language is in `code/kiln/legacy/pinned.json`, no symlink leaves the closure, no unknown toolchain or repository input, not on the never list. Reuse additionally needs a KILN04 manifest whose strong key has a pass. |
| `unpinned` | Only reason: some closure language is not yet pinned (its M0 PR has not landed). |
| `uncacheable` | Any other hard reason (listed). Advisory hints (§6) never affect the class. |

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
- **The resolver is the trust root — for the static key only.** Every edge
  comes from the old tool, which KILN00 §2 records has missed dependents
  before. *Revision (2026-11-12):* the audit below is no longer a one-off
  sample. It became the mechanism: every recorded pass carries its own trace
  (KILN04), and traced reads outside the static closure are reported as
  old-graph bugs (KILN04 §9). The original audit design: on one Linux full-build shard set, run each BUILD under
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
- **Gate for M3.** Superseded by KILN04 §13: zero unexplained
  disagreements on traced predictions, no unexplained result changes under
  the scrubbed environment (now enforced on every traced run, not only
  before M3), `probe-mismatch` fatal, and the recording run confined.
- **Ignored and ignore-listed files.** Files Git ignores are invisible to
  `Source`, so they are outside every key; this is safe only on fresh
  runners, which is why reuse is CI-only. The ancestor-directory ignore list
  (`*.md`, `LICENSE*`, `CHANGELOG*`) is a hit-rate choice; a test that reads
  such a file is a false-hit source the audit will report.

## 9. Commands and output

```
kiln legacy-plan --graph <graph.json> --out <plan.json> [--os linux|darwin|windows] [--probe]
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
- `../` escapes, indirection markers and unscanned files each produce the
  matching **hint** and leave the class unchanged; in-package and
  in-closure `../` produce no hint. (Read-set soundness is tested in KILN04
  §12, not here.)
- A symlink whose target leaves the closure and read paths makes the
  package uncacheable; one pointing inside the closure does not.
- An enclosing package's manifest change changes the nested package's key; a change in a package nested inside, or enclosing, a *dependency* of P changes P's key.
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

## 11. Implementation decisions (7c-1)

Points the first implementation had to settle; every other implementation
must match them.

1. **Digest encodings.** A package's own digest `D(p)` and every tree or glob
   digest is SHA-256 of the full KILN02 section encoding
   `str("inputs") || list(entries)`. The plan's component digests use the
   section encodings of `reads` and `consumed` from the key encoder.
2. **Class precedence.** The temporary `m1-incomplete:*` reason (the escape
   scans landed in 7c-2 and tool/env identity in 7c-3, so
   `m1-incomplete:scans` and `m1-incomplete:tools-env` no longer exist; only
   `m1-incomplete:trace` remains) does not count when choosing
   between `unpinned` and `uncacheable`. Hard reasons — `unknown-repo-inputs`,
   `never`, `unknown-toolchain:<name>`, `no-build-file:<os>` — always give `uncacheable`; otherwise an
   unpinned closure language gives `unpinned`; `cacheable` needs no reason at
   all, so it is unreachable while any `m1-incomplete` reason remains. The text scans
   of §6 (`escape:`, `indirect:`, `unscanned:`) are advisory `hints`, not
   reasons, and play no part in the class. The hard symlink reasons
   (`symlink:<link>`) do. `m1-incomplete:trace` (no traced read-set manifest,
   KILN04) is the standing reason, so `cacheable` stays unreachable. A package carrying a hard reason still gets
   its key.
3. **No BUILD for this OS.** A package whose `build_files[OS]` is `null` gets
   an empty key and reason `no-build-file:<os>`, but remains a closure member
   for others.
4. **Missing read paths.** A read path that names neither a file nor a
   directory is encoded as `dir:<path>` with the empty-tree digest, so the
   path appearing later changes the key.
5. **Boundary roots.** A descendant root includes the root itself.
   `excluded_roots` is parsed and validated but ignored for key reads: honouring
   an exclusion could only drop a read, which is the error a key must not make,
   while ignoring it costs only hit rate. A package's boundary inputs are the
   union over every boundary whose exact or descendant roots match it.
6. **Policy files are required.** A missing or malformed `pinned.json`,
   `never.json` or boundary file is an error, every `never.json` name must be
   a package in the graph, and lists must be sorted.
7. **Build-tool digest** follows `go.mod` `replace` targets transitively (with
   a cycle guard), follows the `use` and `replace` directories of a root
   `go.work` the same way, and includes `go.work`, `go.work.sum` and `vendor/`
   when present. A target that leaves the repository, names the repository
   root, or contains a backslash is an error, never skipped.
8. **Executable bit.** The Git `Source` takes it from the index mode; the
   plain-folder `Source` from the file system (always `0` on Windows). Plans
   from the two adapters agree on Unix; on Windows only the Git adapter is
   authoritative.
9. **Plan schema** is `kiln-legacy-plan/1`: `os`, `arch`, `host`, `packages`
   sorted by name, and `histogram` (named reasons grouped by prefix, except
   `unpinned:<language>`).
10. **Symlinked BUILD.** A selected BUILD file that is a symlink makes that
    package `uncacheable` with reason `symlink:<build-file>` and no key; the
    rest of the plan is still produced.
11. **Graph limits.** At most 100,000 packages and 5,000,000 edges (deps over
    all OSes plus dev_deps); `extra_toolchains` entries are non-empty UTF-8
    without control characters.
12. **Folder source.** The plain-folder `Source` cannot check for a clean tree;
    `legacy-plan` warns on stderr and continues.

## 12. Implementation decisions (7c-2: escape and indirection scans)

13. **Scope and timing.** The files examined are every closure member's own
    files (§3.1; the package's own among them) and every read file (a read
    file, every file under a read directory, every glob match, ancestor
    configuration). A dependency's indirection, unscanned file or dynamic `../`
    token therefore gives each dependent a hint, naming the member's file
    (`indirect:<member file>`); a dependency's escaping link gives each
    dependent the hard reason `symlink:<link>`. Hints are advisory (section 6)
    and never change a class. `../` targets
    and links from ALL examined files are re-judged against the dependent's own
    allowed region (item 17). Per-file facts are computed once, in the same pass
    that hashes the file, and summarised once per member and per read item. A
    symlink is never opened, so its target's content is not scanned; the
    link-target rule covers it.
14. **Text.** The kind of a file comes from its first 8 KiB (all of it when
    shorter), in this order: (1) a UTF-16 byte order mark (`FF FE` or `FE FF`):
    the file is decoded to text and scanned; (2) the magic bytes of an inert
    format (PNG, JPEG, GIF, WebP, ICO, PDF, TrueType, OpenType, WOFF, WOFF2):
    hashed but not scanned, unless the file is executable, in which case it is
    `unscanned`; (3) a NUL byte: binary, always `unscanned`; (4) otherwise text,
    scanned as bytes whether or not it is valid UTF-8. Text up to 256 KiB
    (262,144 bytes, inclusive) is scanned; larger text is `unscanned:<file>`.
    Residual risk, accepted: a script that begins with an inert magic and is run
    as `sh file.png` is not seen. Scanning works on bytes (or the decoded
    UTF-16): every marker and delimiter is ASCII.
15. **Tokens.** A token is a maximal run of bytes that are not whitespace,
    `"`, `'`, `` ` ``, `=`, `(`, `)`, `,`, `;`, `:`, `<`, `>`, `|` or `&`.
    Before tokenising, URLs (`scheme://` up to the next whitespace, quote,
    bracket or operator) are removed, so `https://x/y` is ignored. Kept (so
    scanned as ordinary text): `file:` URLs, which name a path, and any URL
    containing `..`.
16. **`../` tokens.** A token counts when, with `\` read as `/`, it contains
    `../`, ends in `/..`, or is exactly `..`, `../` or `..\` (the parent of the
    file's directory; Lua's spaced concatenation `a .. b` is an accepted false
    positive). `wait..` and `a..b` do not count. A glued flag prefix without a
    slash is dropped (`-I../inc` is `../inc`). The token is resolved against
    the file's directory and cleaned; a result that climbs above the repository
    root is an escape. A token containing any of `$ * ? [ ] { } ~ %` cannot be
    resolved and is an escape as written. Absolute tokens are not resolved:
    the absolute-path rule owns them. The reason names the cleaned target.
    Before the region check every target is walked through any symlinks it
    crosses (the walk of item 20); the walked path is what is checked.
17. **Allowed region.** A target is inside when it equals or lies under the
    package's `rel_path`, any closure member's `rel_path`, or a read path (file
    or directory). A read glob admits a target only when the target is a listed
    file the glob matches, or a directory with at least one file all of whose
    files match. A directory that merely contains the package (`../` from its
    root) is outside.
18. **Markers.** Case-sensitive substrings, deliberately without word
    boundaries (`__dirname`, `os.path.realpath`, `$0` inside `$01` all hit):
    - shell: `dirname`, `realpath`, `readlink`, `$0`, `${0`, `$BASH_SOURCE`,
      `${BASH_SOURCE`, `~/`, `pushd`, `Set-Location`, `Push-Location`, `chdir`,
      `Chdir`;
    - a file's own location: `__file__`, `__dir__`, `__FILE__`, `__filename`,
      `import.meta`, `CARGO_MANIFEST_DIR`, `MyInvocation`, `PSCommandPath`,
      `$PSScriptRoot`;
    - the working directory: `getcwd`, `Getwd`, `cwd()`, `Dir.pwd`,
      `current_dir`, `set_current_dir`, `filepath.Dir`, `filepath.Abs`,
      `runtime.Caller`, `Get-Location`;
    - CI checkouts: `GITHUB_WORKSPACE`, `RUNNER_`; Windows: `%~dp0`, `%CD%`.

    Matched case-insensitively (PowerShell variables are case-insensitive):
    `$(pwd`, `` `pwd` ``, `$pwd`, `${pwd`, `$oldpwd`, `${oldpwd`, `$home`,
    `${home`, `$psscriptroot`, `%~dp0`, `%cd%`. Also `git`, whitespace,
    `rev-parse` or `ls-files` (any whitespace run); and a bare `cd ..`: the
    word `cd` or `sl` not glued to a longer word, optional whitespace (so
    `cd..` counts), an optional quote, `..`, any number of `/`, an optional
    quote, then whitespace, `;`, `&`, `|`, `)` or the end (`cd ../x` is not a
    marker, it is a `../` token).
19. **Absolute paths.** A token is examined after: reading `\` as `/`; dropping
    a leading `#!`; dropping a leading run of `- + @ [ {` (so `${TMPDIR:-/tmp}`,
    `@/etc/x` and `[/etc` are seen); for a token that began with `-`, dropping
    the flag name up to the first `/` (`-isystem/usr/include`); and dropping
    trailing `} ) ]`. It is an absolute path outside the allowlist
    (`/dev/null`, `/usr/bin/env`, `/bin/sh`) when it starts with `~` and a
    letter or `_` (`~user/x`), or starts with `/` and: it contains `..`; or it
    starts with exactly one slash and its first segment (cut at the first
    character other than a letter, digit, `_`, `.`, `-`) is a well-known
    filesystem root (`etc usr opt home Users tmp var private Library System
    Applications Volumes mnt media proc sys dev root nix bin sbin lib lib64 snap
    srv run workspace github runner __w Windows`) or it has at least two
    non-empty segments (`/a/b`); or it starts with exactly two slashes and has
    at least two non-empty segments once they are removed (`//server/share`)
    or its first segment is a filesystem root (`//tmp`); three or more leading
    slashes (the tail of `file:///etc/x`) are judged as the one-slash case on
    what follows. A token made only of slashes is never a path, and neither
    are one-segment tokens with an unknown root: `</p>` (the token `/p`), `/W4`,
    `/permissive-`, `/*`, `/^re/`. Separately, before tokenising, a Windows
    drive prefix (a single letter not glued to a longer word, then `:` and `\`
    or `/`) or a `\\host\` UNC path anywhere in the text is indirection.
20. **Symlinks.** Resolved like a kernel (a stack of components; a link
    component is replaced by its target's components and costs one hop; `..`
    pops), over the listed tree: at most 40 hops, so a loop fails by running
    out. Refused (hard reason `symlink:<link>`): an empty, rooted, Windows-drive or
    backslash-containing target, climbing above the repository root, a regular
    file used as a directory, an exhausted hop budget; or a final path outside
    the allowed region (17). A dangling target inside the region is fine. The
    links checked are those in every examined file set of item 13. The target
    text comes from the same `Describe` that produced the link's digest.
21. **Reasons and hints.** Symlink findings are reasons: `symlink:<link>`
    (and `symlink:<build-file>` for a symlinked selected BUILD file). The text
    scans are hints in the record's `hints` array: `escape:<path>` (the cleaned
    `../` target or the dynamic token), `indirect:<file>`, `unscanned:<file>`.
    Control characters are removed from both. At most 16 of each kind are
    recorded per package (the first in sorted order); `hints` is sorted and
    de-duplicated, and the cap bounds plan size and changes no class. Hints are
    advisory: they make no soundness or security claim, never enter `reasons`,
    and cannot make a package `cacheable` or `uncacheable`; the key is always
    computed. The histogram has a `hints` map beside `reasons`, counting each
    hint kind once per package under its prefix. `kiln explain` prints hints in
    their own section, labelled advisory.

## 13. Implementation decisions (7c-3: tools and environment)

Items continue the numbering of §11 and §12. (The loop item asked for a
"§12"; §12 was taken by 7c-2, so this is §13.)

22. **`toolchains.json` shape.** A bare object from entry name to entry,
    sorted by name, with no duplicate keys. An entry has `pins` (required, may
    be empty; tool name to version, sorted), `probes` (argv lists), `roots`
    (tool name to PATH directories, sorted; each tool must be pinned in the same
    entry) and the optional `always` (bool) and `os` (a sorted subset of
    `linux`, `darwin`, `windows`, in the graph's spelling). Unknown fields are
    refused. Names (entries and tools) are 1 to 64 bytes of letters, digits and
    `._+-`, not starting with punctuation: no `:` (reserved for Kiln) and no `/`.
    A pin is `image` or an exact version (a digit first, then letters, digits and
    `._+-`); `latest`, ranges and empty pins are refused. At most 256 entries, 8
    probes of at most 8 arguments each (a probe's `argv[0]` must be a tool the
    same entry pins; `image` counts. Its arguments are a flag such as `-v`,
    `--version` or `--pkg-info=x`, or a bare word such as `version`; no path, no
    space, and never an interpreter eval flag: `-c`, `-e`, `-E`, `-r`, `-eval`,
    `--eval`, `--exec`, `--command`, with or without `=value`), 16 directories per tool. A tool pinned in
    two entries must carry the same pin and the same roots, or the file is
    refused. Every policy file (`toolchains.json`, `env.json`,
    `host-roots.json`, `pinned.json`, `never.json`) is decoded case-exactly and
    with duplicate keys refused (Go's decoder would match `"Pins"` and let the
    last of two equal keys win), so what a reviewer reads is what is parsed.
    Paths in policy files may not contain `:` or `;`, so one entry cannot smuggle
    in a second PATH directory.
23. **Entry namespace.** Closure languages and `extra_toolchains` share one
    namespace of entry names (`rust` is both a language and an extra toolchain).
    The always-included set of §4 is the entries with `always: true` (`sh`, `cc`,
    `strace` on Linux, `xcode` on macOS); every package gets them whether or not
    its closure names them. A name with no entry is the hard reason
    `unknown-toolchain:<name>` for every package whose closure holds it. The
    language `unknown` (the two `code/sites` packages, which the old tool could
    not classify) deliberately has no entry, so it gives that reason.
24. **The `tools` section.** One KILN02 kind-2 entry per distinct tool name
    (the binary name), `version` = the pin string, `identity` = the pin string,
    or for `image` the `Host.ImageID()` bytes; sorted by name. The set is the
    union of the entries for the closure's languages and extra toolchains plus the
    always-included ones that apply to the OS. `ToolsFor(names, os, imageID)` is the
    one function; it returns the unknown names separately. The digest of
    `toolchains.json` is the consumed entry `policy:toolchains.json` (SHA-256 of the
    file's bytes) in the static key; KILN04's weak key leaves it out.
25. **Pin sources.** An exact pin is the version committed in `mise.toml`
    (`go`, `python`, `uv`, `node`, `ruby`, `rust`, `ghc`, `cabal`, `dart`,
    `gradle`, `kotlin`, `lua`), in `code/specs/data/toolchain-pins.json`
    (`dotnet`, `elixir`, `erlang`, `bundler`) or asserted in `ci.yml` shell steps
    (`opam`, `ocamlc`). Everything else (the C toolchain, `java` whose workflow
    input is only a major version, `swift`, `perl`, `npm`, `cpanm`) is `image`.
    A repository test compares each exact pin with these files, so a toolchain
    bump that forgets Kiln fails.
26. **Probe semantics.** A probe checks the pin whose tool name equals its
    `argv[0]`; a probe whose tool has no pin, or an `image` pin, is run but
    compared with nothing (`ok`). The pin must be a whole token of the normalised
    output: tokens split at white space and `,;()[]"'<>`, trailing `.` and `:`
    are cut, and a letters-only prefix is cut (`v22.1.0`, `go1.26.1`), so
    `1.2.3` does not match `1.2.30` or `1.2.3-rc1`. Normalisation removes
    absolute paths (recognised at a token start) and timestamps first. Statuses:
    `ok`, `absent` (not found on the fixed PATH, or a dangling link),
    `refused` (the real path is inside the checkout, or cannot be related to it:
    the check fails closed, compares case-folded on macOS and Windows, and `Probe`
    refuses to run at all without a checkout root), `mismatch` (the output lacks
    the pin, or the run failed to start, timed out or exited non-zero even if the
    pin is in the output; a short normalised excerpt is kept as `detail`) and
    `timeout` (the 120 s limit for all probes of a plan passed before this one
    ran). `mismatch` and `timeout` are findings and give the soft reasons
    `probe-mismatch:<tool>` and `probe-timeout:<tool>` to every package whose key
    holds that tool. They, and `refused`, are soft only in M2: they become
    fatal before M3 enables reuse. A tool's
    reported status is the worst of its probes. The binary that is run is the
    real path that was checked (so it cannot be swapped between check and run),
    with `argv[0]` set to the name the PATH gave (rustup proxies and multi-call
    binaries read it). On Unix each probe leads its own process group and a
    timeout kills the group. A `HOME` that is not absolute and clean, or holds the
    list separator, is treated as unset, so its roots are skipped. Failing to
    remove the probe's private directory is an error. The run's environment is exactly `PATH`, `LANG=C`,
    `LC_ALL=C`, `TZ=UTC` and a private `HOME`, in a private working directory.
    Probes run once per plan over the entries any package uses, and only for the
    platform the machine runs (`--probe` with another `--os` is a usage error).
    A tool whose entry lists no `roots` cannot be found on the fixed PATH and
    reads as `absent`; adding roots is how a tool becomes probe-able.
27. **`env.json`.** `{"names": [...]}`, strictly sorted; a name is
    letters, digits and `_` (not starting with a digit) with an optional trailing
    `*`. A bare `*`, a name with `:`, and any name or prefix that is secret-like
    are refused when the file loads. Secret-like is §5's list (`TOKEN`, `SECRET`,
    `PASSWORD`, `PASSWD`, `CREDENTIAL`, `AUTH`, `PRIVATE` anywhere, `_KEY` and
    `_PAT` at the end, case-insensitively; plus `PASSPHRASE`, `COOKIE`,
    `SESSION`, `APIKEY`, `SIGNING` and `ACCESS_KEY`) joined with what `envscrub` removes
    from child environments, so the two lists cannot disagree. After wildcard
    expansion a matching set variable that is secret-like is an error naming the
    variable (never its value). A keyed VALUE that looks like a credential is an
    error naming the variable too: URL userinfo (`://user@`), the token shapes
    `ghp_`, `gho_`, `github_pat_`, `AKIA`, `xoxb-` and `xoxp-`, and a
    `-D...password=` flag. Names match case-sensitively, except on Windows where
    they are case-insensitive; the LAST of two entries with one name wins (as in
    `os/exec`); and a value must be valid UTF-8.
    Origin is always 0 (`env`). `PATH`, `HOME` and `TMPDIR` are not listed: the
    KILN04 weak key owns them.
28. **`ConstructPATH(tools, home)`.** Directories come from the `roots` of
    exactly the given tools, in tool-name order and each tool's file order,
    non-system directories first, then system ones (`/usr`, `/bin`, `/sbin`,
    `/opt/homebrew`), first occurrence wins, joined with the OS list separator.
    A leading `$HOME` is replaced by `home`; with no home such roots are left
    out. It is not wired into anything yet. In the seed file `go`, `node`,
    `python3` and `ruby` use `/opt/hostedtoolcache/<name>/<version>/x64/bin`,
    `rustc` and `cargo` use `$HOME/.cargo/bin`, `ghc` uses `$HOME/.ghcup/bin`,
    `dotnet` uses `$HOME/.dotnet`, and the always-included tools `/bin` and
    `/usr/bin`; every other tool has no roots yet.
29. **`host-roots.json`.** `roots` (strictly sorted by path; `path`, `kind`
    `image`, `tool`, `state` or `ephemeral`, and `tool`, `ecosystem`,
    `fetchers` only where the kind uses them), `ephemeral` and `absent-ok`
    (strictly sorted). An `ephemeral` glob needs at least two literal segments
    before its first wildcard (`/dev/fd/*` is fine, `/proc/*` is not). Paths are absolute or begin with `$HOME` (or `$TMPDIR`),
    are clean, and contain no `..`; `ephemeral` allows `*` and `**`; `/`, `$HOME`
    and `$TMPDIR` alone are too broad to be roots. A `tool` root names a tool
    that `toolchains.json` pins (the tool's binary name, not the entry name), and
    every `/opt/hostedtoolcache` directory on a tool's PATH must lie under a
    `tool` root of that tool, so the two files cannot drift. The loader is
    required (a missing file is an error). Its digest (`policy:host-roots.json`)
    is for the KILN04 weak key; the static key does not use it, so editing the
    file changes no static key. The seed lists only `image` and `tool` roots: no
    `state` root, because state roots need `pinned.json` ecosystems and the
    fetchers of KILN04 stage 5, and the image roots are the Linux layout.
30. **Plan and explain.** The plan gains `env` (the sorted names that entered
    every key), `probes` (only when `--probe` ran), per-record `tools`
    (`name@pin`), and the component digests `tools` and `env` (the KILN02 section
    encodings of §3.8 and §3.9). Environment values never appear. `explain`
    prints the same. Since the environment is one for the whole run, `env` sits at
    the plan level; each record carries only the digest.
31. **Where policy is read from.** On trusted runs the policy files are to be
    read from the base revision through `Source`, never from the working tree
    (KILN03 D6), so a PR cannot choose its own probes or allowlist. 7c-3 reads
    them from the tree it is given; the base-revision read is a later item.
