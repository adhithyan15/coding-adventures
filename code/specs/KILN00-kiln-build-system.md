# KILN00 — Kiln: a Language-Agnostic, Content-Addressed Build System

## Status

Accepted as the design direction (2026-11-11); implementation not started.
Supersedes B01 (BUILD.lark), B02 (build sandbox), B03 (vendoring) and B04
(Starlark migration). Once Kiln ships, it also replaces the change-detection and
execution sections of `12-build-system.md`.

Related: B06 (zero-dependency migration), B07 (shared-directory safety),
`build-plan-v1.md`, `build-plan-sharding.md`, `build-tool-conformance.md`.

---

## 1. What Kiln is, in one paragraph

Kiln is Make with three things Make never had: **content hashes instead of
timestamps**, **a private copy of the world for every build step**, and **a
planner that sees the whole graph before anything runs**. You describe *what*
goes in, *what* comes out and *which tools* turn one into the other. Kiln
works out the rest: where the tools are, which steps can be skipped because
exactly these inputs already passed, which steps are the same work reached by
two different paths, and in what order everything else must run. Kiln itself
knows nothing about Rust, Python, LaTeX or any other language. Everything it
knows about a language it learns from **rules** written in Kiln's own small
language, kept in the repository next to the code they build.

The name comes from pottery: raw material goes in, a finished piece comes
out, and nothing gets in while it is firing.

---

## 2. Why we need it

The current system is a shell BUILD file per package (5,581 files), a cmd.exe
`BUILD_windows` beside most of them (3,846 files), and a Go build tool that
picks affected packages from `git diff` and runs their BUILD scripts in place.
It got the repo a long way. These are the problems it can no longer absorb,
each measured on 2026-11-11:

| Problem | Evidence |
|---|---|
| **No isolation.** Every BUILD runs inside the real package directory (`internal/executor/executor.go`, `cmd.Dir = pkg.Path`), so builds overwrite each other. | 658 BUILD files `cd ../sibling && <install>` into another package. B07 documents 113 of them running `npm ci` (which deletes `node_modules` first) in `typescript/state-machine` while others read it. |
| **Shared global state is guarded by hand.** | ~10 hand-maintained `global:` locks in `executor.go` (opam, cabal, hex, luarocks, rubygems, gradle, dotnet, rustup…), each added after something broke; the luarocks lesson shows the list missing a writer. |
| **No caching.** CI rebuilds and retests every affected package from scratch, every run. | A change to `rust/parser` rebuilds ~750 packages on one runner (`ci.yml` comment near the PR build step). Nothing remembers that identical inputs already passed, so the 6-hourly full build re-runs everything that has not changed. |
| **Two files per package that drift.** | `BUILD` vs `BUILD_windows` drift is common enough that CI now audits it (OPS-012/013/014). |
| **Inputs are guessed, not declared.** | Change detection uses language file-extension lists; a separate 1,300-line test-trigger resolver exists because diff selection kept missing dependents. |
| **Downloads are unpinned.** | `Cargo.lock`, `uv.lock` and `Gemfile.lock` are gitignored; `mise.toml` pins Go, Rust and Dart to "latest"/"stable". Two runs of the same commit can test against different third-party code. |
| **Language knowledge is hard-coded in Go.** | Per-language dependency resolvers (~2,900 lines), a Haskell-linker validator, per-ecosystem locks. Every new language means editing the tool, and the 16 educational ports. |

A Starlark migration (B01–B04) was designed to fix some of this and stalled
at 85 files. Its rules declared only `srcs` and `deps`: converting a package
bought slightly better change detection and nothing else, so nothing pulled
the other 5,500 files across. Kiln's lesson from that: **the value has to
arrive before the migration**, which is why §15 starts by caching the
*existing* shell BUILD files.

---

## 3. Goals and non-goals

**Goals, in priority order**

1. **Caching in CI.** If a target's inputs are byte-for-byte what main already
   built and tested, do not build or test it again.
2. **Plain declarations.** Inputs, outputs, tools and steps that a newcomer can
   read without learning a programming language.
3. **Automatic isolation.** Every step runs in a private copy of exactly what
   it declared. Nothing writes to the real tree or to another package.
4. **Language-agnostic core.** LaTeX → PDF is as natural as compiling Rust.
5. **Supply-chain safety.** Everything first-party is built from source, every
   tool and download is pinned and verified, and only trusted runs may record
   results.
6. **Portable infrastructure.** Version control, CI host, result storage and
   tool provisioning sit behind interfaces, so leaving Git or GitHub means
   writing an adapter, not redesigning the system.

**Non-goals**

- **Storing build outputs between CI runs.** Only hashes cross runs (§10).
- **Interface-based cutoff** (skipping callers when only an implementation
  changed). Considered and dropped: it conflicts with building dependencies
  from source, and it turns into a risky test-selection policy.
- **File-level actions for ecosystem languages.** Kiln wraps cargo, npm, uv,
  mix and friends as whole-package steps; it does not reimplement them.
  Fine-grained actions are for tools built in this repo (grammar → parser,
  Twig → bytecode, LaTeX → PDF).
- **Remote execution.** Out of scope.
- **Making local builds fast.** A local speed-up falls out of the design
  (§10.4), but no decision is made for its sake.

---

## 4. Vocabulary

| Term | Meaning |
|---|---|
| **Target** | A named thing you can ask Kiln for: `rust/parser:test`, `books/tamil:pdf`. Declared in a BUILD file. |
| **Rule** | A reusable recipe that turns parameters into a target's fields and steps. Written in a `.rule` file. This is how Kiln "learns" a language. |
| **Step** | One command (a tool and its arguments) or one built-in file operation. |
| **Phase** | A named group of steps with a fixed meaning: `fetch`, `build`, `test` (§7). |
| **Action** | What the planner schedules: one phase of one target, fully expanded. The unit of caching. |
| **Key** | A SHA-256 over everything that can change an action's result (§8). Two actions with the same key are the same work. |
| **Ledger** | The record of keys that passed. Hashes only (§10). |
| **Work root** | The private directory an action runs in (§9). |
| **Tool** | A named, version-pinned program: `cargo`, `node`, `latexmk`, or a program built in this repo (§11). |

---

## 5. The core: six things and nothing else

Kiln's Go core understands exactly six concepts. If a feature cannot be
expressed with them, it belongs in a rule or a tool, not in the core.

1. **File sets.** Globs relative to a package (`src/**`, `*.tex`), resolved
   through the `Source` interface (§13), never by walking the disk directly.
2. **Actions.** Tool + arguments + declared inputs + declared outputs +
   environment, executed in a private work root.
3. **Keys.** The hash in §8.
4. **The ledger.** "Key K passed" records (§10).
5. **Tool state.** An opt-in directory a tool may keep between actions for its
   *own* incremental work (cargo's `target/`). Never staged into another
   action, never treated as an output, never trusted for correctness by Kiln
   (§9.5).
6. **Scanners.** Actions whose output is a list of dependencies (like Make's
   generated `.d` files). Cached like any other action.

The core never mentions cargo, npm, Python or LaTeX. Today's per-language Go
code — dependency resolvers, the Haskell-linker validator, the ecosystem locks
— becomes rule files and small scanner tools.

---

## 6. The language

Kiln files are line-oriented and indentation-based, like a Makefile with the
tab trap removed. There are two tiers.

### 6.1 Guarantees that make caching possible

Evaluating any Kiln file, at either tier, must:

1. **Always finish.** No `while`, no recursion (the call graph of rule
   functions must be acyclic; Kiln checks this before evaluating).
2. **Be deterministic.** No reading files, clocks, environment variables or the
   network *while evaluating*. Globs are resolved by the core through
   `Source`, after evaluation, as data.
3. **Settle everything before anything runs.** Every input pattern, output,
   tool and step is known when planning ends, so the key covers all of it.

Variables, loops over fixed lists and (in rule files) functions are allowed
because they keep these three properties. Side effects and unbounded
computation are not, at any tier. Real computation — "which compilers are
installed?", "what does this manifest depend on?" — belongs in a **tool**,
run as a step or a scanner and hashed like any other input.

### 6.2 Tier 1: BUILD files

A BUILD file in Kiln format is marked by a version line: the **first line that
is neither blank nor a `#` comment** must match `^kiln [0-9]+$`, so literate
header comments may come first. That line is how every tool tells a Kiln BUILD
from a legacy shell BUILD during migration, and it versions the language for
the future:

```
kiln 1
```

Coexistence rules, which must be in force **before the first Kiln BUILD
lands**:

- The legacy Go build tool and its conformance fixtures recognise the marker
  and skip Kiln BUILD files. Otherwise the old tool would run `kiln 1` as a
  shell command.
- A Kiln `BUILD` beside a legacy `BUILD_windows` is a validator error, so a
  stale Windows script can never run in its place.
- The educational ports are frozen at the legacy level (§16); their fixtures
  gain one case asserting that Kiln files are skipped.

Indentation is spaces only; a tab is a parse error.

Most packages need one more line — a target name, a colon, a rule:

```
kiln 1

lib: rust_crate
```

Parameters follow the rule name as `name=value`. A target may also override or
extend what its rule produces with an indented block:

```
kiln 1

let native = "ext/conduit_native_node/target/release"

lib: ts_package
  tools +: cargo
  build +:
    in ext/conduit_native_node: cargo build --release
    copy $native/${libprefix}conduit_native_node$libext conduit_native_node.node
  outputs: conduit_native_node.node
  test:
    npx vitest run
```

A target with no rule at all is a one-off recipe — Make's plain rule:

```
kiln 1

tables:
  tools: //code/programs/go/grammar-gen
  inputs: grammar.ebnf
  outputs: tables.json
  build:
    grammar-gen grammar.ebnf -o tables.json
```

**Fields** (all optional; a rule supplies defaults):

| Field | Meaning |
|---|---|
| `inputs` | File patterns relative to the package, used by `fetch` and `build` (and `test`). Default: every source file in the package (as `Source` reports it), minus `.build/` and minus any nested directory that has its own BUILD file. |
| `test-inputs` | Extra patterns staged and keyed **only** for `test` (`tests/**`). Editing a test then re-runs that test without changing what consumers build against. |
| `reads` | Repo-relative paths outside the package that are inputs (`code/grammars/ruby`). |
| `deps` | Other targets this one consumes, or `scan <tool> <args>` to discover them (§5, item 6). |
| `test-deps` | Targets needed only by `test`. |
| `tools` | Tool names (§11). Only these are on `PATH` inside the work root. |
| `outputs` | Files or directories copied back to `<pkg>/.build/out/` after success. |
| `provides` | What a consumer receives when it depends on this target (§7.2). Default: the build-phase `inputs` plus `outputs`, which is what source nesting needs (`path = "../D"` must find D's source). |
| `env` | `NAME = "value"` pairs. Everything else in the environment is scrubbed. |
| `pass` | Host environment variable names allowed through unchanged, hashed by value. Only names on an allowlist in `code/kiln/lint.toml` may be passed, so a BUILD file in an untrusted PR cannot pull a secret into an action. Names that look like secrets are rejected, values that differ on every run (run ids, timestamps) are rejected, and values are never printed in plans, explanations or logs. `fetch` actions receive no `pass` variables at all. |
| `state` | Tool-state directories (§9.5). Experimental. |
| `cache` | `on` (default) or `off` (§12). |
| `timeout` | e.g. `20m`. |

Two settings are written **inside** a `fetch:` block rather than as target
fields, because they only mean something there: `network: on` and
`pinned-by: <lockfile>` (§12).

`field: values` replaces what the rule set; `field +: values` appends; the
same `+:` form appends steps to a phase. When a target `use`s a rule which
itself `use`s another, the innermost rule applies first and each layer
overrides the one beneath it.

**Steps** are one per line:

- `<tool> <args…>` — run a declared tool. No shell: no pipes, `&&`, `;`,
  globbing, or `$VAR` expansion by a shell. Words are bare or `"quoted"`;
  `$$` is a literal dollar sign. On Windows the tool table maps a command such
  as `npm` to the real executable or `.cmd` shim (§11.1), so steps stay the
  same on every OS.
- `in <dir>: <tool> <args…>` — same, with a working directory inside the work
  root. Replaces `cd x && …`.
- Built-ins that work identically on every OS: `copy <from> <to>`,
  `mkdir <dir>`, `remove <path>`, `write <path> "<text>"`.
- `( … )` lets one step span several lines, replacing backslash
  continuations.
- `shell: """…"""` — an explicitly non-portable escape hatch, counted and
  reported by `kiln lint`. It exists so that migration never blocks; the goal
  is for it to shrink to zero.

**Variables and loops (tier 1):** `let name = value` binds an immutable string
or list. `$name` interpolates. A list in argument position expands to one
argument per element; `[for t in targets: --test $t]` expands to pairs:

```
let targets = (cargo_archive_path array_ref_tracing beam_array_len)
lib: rust_crate
  test:
    cargo test -p lang-aot --lib [for t in targets: --test $t]
```

Tier 1 has no functions, no conditionals and no imports. If a BUILD file
wants one, the logic belongs in a rule where every package can reuse it.

**Platforms.** One file per platform, no in-file conditionals. `BUILD` is the
default; `BUILD.linux`, `BUILD.macos` or `BUILD.windows` is an **overlay**
applied on top of it on that platform, using the same `:` (replace) and `+:`
(append) operators. An overlay states only what differs, so the drift that
plagues today's whole-file `BUILD_windows` copies cannot recur. Rule files work
the same way (`python_library.windows.rule` overlays `python_library.rule`).
Built-ins `$exe`, `$libprefix` and `$libext` (`""/".dll"` on Windows,
`"lib"/".dylib"` on macOS, `"lib"/".so"` on Linux) absorb the most common
differences, so most packages need only `BUILD`. Because evaluation is pure,
`kiln lint` expands every platform's variant on every OS and catches a broken
Windows overlay on a Linux runner.

### 6.3 Tier 2: rule files

Rules live in `code/kiln/rules/<name>.rule`. Adding support for a new language
or file type means adding a rule file and, if dependencies need discovering, a
scanner tool. The Go core never changes.

```
# code/kiln/rules/latex_pdf.rule
rule latex_pdf(main):
  tools: latexmk
  inputs: **/*.tex **/*.bib **/*.sty figures/**
  env: SOURCE_DATE_EPOCH = "0"          # identical inputs → byte-identical PDF
  build:
    latexmk -pdf -interaction=nonstopmode $main.tex
  outputs: $main.pdf
```

```
# code/kiln/rules/rust_crate.rule
rule rust_crate(name = $package):
  tools: cargo
  inputs: src/** Cargo.toml build.rs
  test-inputs: tests/** benches/**
  reads: code/packages/rust/Cargo.toml code/packages/rust/Cargo.lock
  deps: scan cargo-path-deps Cargo.toml
  state: cargo-target shared-by toolchain profile
  fetch:
    network: on
    pinned-by: code/packages/rust/Cargo.lock
    cargo fetch --locked
  test:
    cargo test --offline --locked -p $name -- --nocapture
```

```
# code/kiln/rules/ts_package.rule
rule ts_package():
  tools: node
  inputs: src/** package.json package-lock.json tsconfig*.json
  test-inputs: tests/**
  deps: scan npm-file-deps package.json
  fetch:
    network: on
    pinned-by: package-lock.json
    npm ci --ignore-scripts --quiet     # install scripts must not run with network on
  build:
    npm rebuild                         # runs the install scripts, offline
    npx tsc
  provides: dist/** node_modules/** package.json
  test:
    npx vitest run
```

```
# code/kiln/rules/python_library.rule
rule python_library(extra = "dev", test_args = (tests/ -v)):
  tools: uv
  inputs: src/** pyproject.toml uv.lock
  test-inputs: tests/**
  deps: scan py-path-deps pyproject.toml
  fetch:
    network: on
    pinned-by: uv.lock
    # wheels only, dependencies only: no build backend runs with network on
    uv sync --locked --extra $extra --no-install-project --no-build
  build:
    uv sync --locked --offline --extra $extra
  test:
    uv run --locked --offline pytest $test_args
```

Tier 2 adds to tier 1:

- **Parameters** with defaults, and built-ins: `$package` (the package's
  directory name), `$target`, `$os`, `$arch`, `$exe` (`.exe` on Windows, empty
  elsewhere).
- **`if` on parameters** (never on the environment):
  `if coverage: …`.
- **Functions** that compute strings or lists from their arguments:
  `fn test_flags(names): [for n in names: --test $n]`. No recursion, no I/O.
- **`use <other_rule>`** to build on another rule.

A rule's fully expanded text is part of the key of every action that uses it
(§8), so editing a rule rebuilds exactly the targets whose expansion changed
and nothing else.

### 6.4 Grammar sketch

```ebnf
file        = { comment | blank } "kiln" INT NL { let | target | comment | blank } ;
let         = "let" IDENT "=" value NL ;
target      = NAME ":" [ IDENT { param } ] NL [ INDENT { field | phase } DEDENT ] ;
param       = IDENT "=" value ;
field       = FIELDNAME ( ":" | "+:" ) { item } NL ;
phase       = ( "fetch" | "build" | "test" ) ( ":" | "+:" ) NL
              INDENT { fetchsetting | step } DEDENT ;
fetchsetting= "network" ":" ( "on" | "off" ) NL | "pinned-by" ":" word NL ;
step        = [ "in" word ":" ] word { item } NL
            | "copy" word word NL | "mkdir" word NL | "remove" word NL
            | "write" word STRING NL | "shell" ":" [ "impure" ] TRIPLESTRING NL ;
item        = word | forexpr ;
forexpr     = "[" "for" IDENT "in" ( IDENT | list ) ":" { word } "]" ;
value       = word | list ;
list        = "(" { word } ")" ;
word        = BARE | STRING ;   (* $name, ${name} interpolate; $$ is a literal $ *)

rulefile    = { comment | blank } "kiln" INT NL { fn | rule } ;
rule        = "rule" IDENT "(" [ rparam { "," rparam } ] ")" ":" NL
              INDENT { field | phase | ifblock | "use" IDENT { param } NL } DEDENT ;
rparam      = IDENT [ "=" value ] ;
ifblock     = "if" IDENT ":" NL INDENT { field | phase | step } DEDENT ;
fn          = "fn" IDENT "(" [ IDENT { "," IDENT } ] ")" ":" expr NL ;
```

This sketch is deliberately incomplete. KILN01 specifies the full grammar, the
expansion algorithm and the canonical expanded form that gets hashed, and
ships golden vectors (§16). It must settle at least: the character set of
`BARE` words (`#`, `=`, `*`, `[`, `(` and Windows `C:` paths, which collide
with the `in <dir>:` prefix); `fn` bodies, call syntax and the expression
grammar; whether an `if` may appear inside a phase (allowed above, so a
conditional flag does not duplicate a whole phase); and whether a `-:`
removal operator is needed. In this sketch lists use parentheses — `(tests/
-v)` — so that `[` is reserved for `for` expressions; the examples in §6.2
and §6.3 should be read with that convention.

---

## 7. Phases and source nesting

### 7.1 The three phases

| Phase | Purpose | Network | Runs for |
|---|---|---|---|
| `fetch` | Bring in third-party packages named by a lockfile. | Allowed, only here | The target and every dependency that has one |
| `build` | Make the target consumable: compile, generate, install locally. | Never | The target and every dependency |
| `test` | Check the target. | Never | **Only** the target being tested |

Rules must keep `build` and `test` apart — running tests inside `build` would
make every consumer re-run them. `kiln lint` flags test runners in `build`,
using a list of test-runner commands kept as data in `code/kiln/lint.toml`
rather than in the Go core.

### 7.2 Source nesting (the "Russian doll")

A target consumes its dependencies **as source**, never as a prebuilt package
from outside. This is a supply-chain defence: a compromised low-level package
cannot be swapped in as a binary, because there is no binary to swap.

For a target P that depends on D:

1. D's `fetch` and `build` actions run in **D's own** work root, built from
   D's source and its own dependencies' outputs.
2. D's `provides` set is copied into P's work root at D's repo-relative path,
   so relative references such as npm `file:../D` and `path = "../D"` resolve
   exactly as they do in the repository.
3. D's `test` never runs on P's behalf.

### 7.3 The same work is done once

D's key does not depend on who asked for it. If 113 TypeScript packages need
`state-machine`, the planner sees one key and runs `state-machine`'s build
once per run; all 113 work roots receive the same staged `provides` set. This
is what makes source nesting affordable.

Be precise about what "consume as source" now means. P receives D's source
**plus whatever D's `build` produced in this same run** — for `ts_package`,
D's freshly compiled `dist/`. That is a deliberate refinement of "never
prebuilt": the outputs are produced by Kiln, in this run, from the exact
source D's key names, and their digests are verified when they are staged.
Nothing built outside the run, and nothing built in an earlier run, is ever
consumed. (The owner approved this on 2026-11-11: "if someone else had already
built it, we shouldn't be rebuilding.")

---

## 8. Keys

A key belongs to one **action** — one phase of one target — and is SHA-256
over a canonical, length-prefixed, domain-separated encoding of the following,
in this order:

1. `"kiln-key"`, the **key-schema version** and the **expansion-semantics
   version**. Both are bumped by hand when their meaning changes. The Kiln
   binary's own version is deliberately *not* included, so an unrelated Kiln
   release does not empty the ledger.
2. The platform: OS and architecture.
3. The **host fingerprint**, for any action that uses a tool not supplied by
   the Managed provider (§11): OS version and the runner image identifier from
   `Host.ImageID()`. Tools run things that are not on `PATH` — `/bin/sh`,
   `cc`, `ld`, the Xcode command-line tools, system libraries — and GitHub
   updates its runner images weekly, so the image is an input.
4. The action's phase and target name.
5. The **canonical expanded recipe of this phase only**, with arguments kept
   **in order**. (B02 §6 sorted its command strings; that was a bug —
   argument and step order are meaningful.) Scoping by phase is what keeps an
   edit to a `test` step from invalidating every consumer's build.
6. Every input file staged for this phase — `inputs`, plus `test-inputs` for
   `test` — as `(repo-relative path, kind, mode, SHA-256 of contents)`, sorted
   by path. Text files are hashed exactly as checked out; Kiln requires
   `.gitattributes` to fix line endings for inputs so a Windows checkout
   hashes the same bytes. A symlink is hashed as a link to its target path,
   and a link that leaves the package is an error.
7. Every `reads` path and `pinned-by` lockfile, the same way.
8. The **keys of every action this one consumes**, sorted by target and
   phase: `test` consumes its own `build` and the `build` of each `deps` and
   `test-deps` target; `build` consumes its own `fetch` and the `build` of
   each `deps` target. Because those keys include their own consumed keys, the
   whole source tree beneath a target is covered.
9. Every tool the action may run: name, pinned version, and an identity. For
   a Managed tool the identity is the verified archive digest. For a tool the
   host supplies it is the **full output of the tool's check command**
   (`rustc -vV`, `node -p process.versions`), never just the table entry. An
   optional tool that is absent is recorded as absent (and see §11.3).
10. Every `env` pair and every `pass` variable's name and value.

Rules for computing it:

- **Kiln hashes file contents itself.** It does not reuse Git's blob hashes,
  so a key means the same thing under any version-control system and a
  ledger survives a move away from Git.
- **Nothing undeclared enters the work root**, so anything an action can read
  is in its key. That is the property isolation (§9) exists to guarantee. The
  two exceptions are named: the host fingerprint (item 3) covers what the
  runner image provides, and tool state (§9.5) is unkeyed and audited.
- An action whose key cannot be fully determined — an unpinned download,
  `cache: off`, a `shell:` step marked `impure` — is **uncacheable**, and so
  is everything that consumes it.

---

## 9. Isolation

### 9.1 The work root

Each action runs in a fresh directory **outside the repository**, created per
run with an exclusive, owner-checked, mode-0700 create under a short base
directory the `Host` chooses (`os.MkdirTemp` semantics: a random name, never
derived from the key; the key is used only for lookup). Kiln refuses to run
if the directory already exists. Outside the repository, because cargo, npm,
tsc, git and mise walk up parent directories looking for configuration; a
work root inside the repo would find the real tree's configuration and break
isolation. A short path also keeps Windows under its 260-character limit.

The work root mirrors repo-relative layout:

```
<work root>/
  code/packages/typescript/foo/        ← the target's declared inputs
  code/packages/typescript/state-machine/   ← a dependency's provides set
  code/packages/rust/Cargo.toml        ← a declared `reads` file
  .kiln/tools/bin/                      ← the only directory on PATH
  .kiln/home/                           ← HOME, empty
  .kiln/tmp/                            ← TMPDIR
```

### 9.2 Staging

Files are **copied**, never symlinked or hard-linked: a symlink resolves back
into the real tree, and a hard link lets a write in the work root change the
real file. Kiln uses the cheapest safe copy each OS offers: `clonefile` on
APFS, reflinks where the filesystem supports them, plain copies elsewhere.
After an action succeeds, its `outputs` are copied back to
`<pkg>/.build/out/` — the only place Kiln ever writes in the repository.

Rules that keep staging and copy-back from becoming an escape route:

- **Containment.** Every path in `outputs`, `provides`, `in` and the built-in
  steps (`copy`, `mkdir`, `remove`, `write`) is cleaned, must be relative, and
  must stay inside the work root (or, for copy-back, inside
  `<pkg>/.build/out/`). Absolute paths and `..` are errors — including after
  `$name` interpolation.
- **No link following.** Staging and copy-back never follow symlinks: they
  walk with `lstat` and no-follow opens. A symlink in `outputs` or `provides`
  is rejected, unless it is a relative link that resolves inside the same
  tree, in which case it is recreated as that link and never dereferenced.
- **Regular files and directories only.** Device files, FIFOs and sockets are
  rejected; setuid and setgid bits are stripped.
- **Verify while copying.** Kiln hashes each input as it copies it into the
  work root and aborts if the digest differs from the one in the planned key.
  What runs is therefore exactly what was keyed, even if the real tree changes
  between planning and execution.

### 9.3 Environment

The environment is empty except for: `PATH` (the tool directory), `HOME`,
`TMPDIR`, locale variables fixed to `C.UTF-8`, `SOURCE_DATE_EPOCH`, the
target's `env` pairs and its `pass` variables. Each tool's own cache
locations (`CARGO_HOME`, `npm_config_cache`, `UV_CACHE_DIR`…) are set by the
`Tools` provider (§11) to directories inside the work root or to a
fetch output.

### 9.4 Network

Only `fetch` actions have network access. On Linux, Kiln enforces this by
starting every other action in a new network namespace (Go's standard
`syscall.SysProcAttr` supports this without third-party code). On macOS and
Windows it is enforced by convention and audited by the Linux runs.

### 9.5 Tool state: the one controlled exception

Some tools are much faster when they can reuse their own incremental work.
The canonical case is cargo: under strict nesting, each of `rust/parser`'s
~750 dependents would recompile it in its own work root. A rule may declare

```
state: cargo-target shared-by toolchain profile
```

and Kiln mounts one directory per distinct `(state name, toolchain, profile)`
into every action that declares it, within one run. Correctness of what is
reused there rests on the tool's own change tracking (cargo's fingerprints),
which is why the rule declares it in the open rather than Kiln hiding it.
State is never staged into consumers, never copied back as an output, never
persisted between CI runs, and never part of a key.

**`state` is experimental and is not in M1.** Two things must be proven first:

1. **That it helps at all.** Every work root has a different absolute path and
   fresh modification times, and cargo fingerprints path dependencies by
   path and time, so reuse across work roots may be close to zero. It may
   need a stable path — a Linux mount namespace, or one fixed directory reused
   serially — which must be measured on the `rust/parser` case before any
   rule relies on it.
2. **That it is audited.** Because state is unkeyed, the daily uncached run
   (§10.5) always runs with `--strict`, which ignores `state` entirely, so
   every state-assisted pass is re-checked without it at least daily.

### 9.6 The security boundary

A work root controls what an action can **see**. On its own it does not stop
a malicious step — a compromised third-party package's install script, say —
from reaching beyond it, because by default the action runs as the same user
as Kiln. "No shell" reduces injection risk but is not a sandbox either:
`node`, `uv` and `npx` run arbitrary code. The ledger is only as trustworthy
as this boundary, so Kiln requires the following.

1. **Actions cannot reach Kiln's own state.** No action may write the ledger
   directory, the delta, or the tool store, or read Kiln's environment or
   `/proc`. On Linux, actions run under a dedicated unprivileged user and in
   new user, mount and PID namespaces. The real repository is absent (or, for
   `isolation: none`, mounted writable — item 5), the tool store is mounted
   read-only, and the ledger and delta are absent.
2. **Kiln writes results itself.** Pass records are held in Kiln's memory and
   written to the delta only after an action's whole process group has
   exited and been killed, to a path no action can reach.
3. **Kiln drops secrets before spawning.** Tokens such as `GITHUB_TOKEN` are
   removed from Kiln's own environment before the first child starts; only
   the CI steps that need them (cache and artifact upload) see them.
4. **Tools are immutable once provided.** The tool store is made read-only
   after unpacking, and its tree digest is recorded and re-verified before
   each run, so tampering after unpack is detected.
5. **No enforcement, no trust.** A run on a host where items 1–4 cannot be
   enforced — today macOS and Windows — is **never trusted** and never writes
   the shared ledger, whatever its branch. `isolation: none` (legacy actions
   running in place, §15.1) skips only the private work-root copy, not items
   1–4: on Linux such an action still runs as the dedicated user inside the
   namespaces, with the repository mounted writable and the ledger, delta and
   tool store absent. Because it runs in the shared tree, Kiln also re-hashes
   every keyed input after it finishes and records nothing if any digest
   changed.
6. **`shell:` steps get no extra privilege** and are never cacheable unless
   the same confinement applies to them.

Consequence for M1–M3: shared-ledger hits come from Linux main runs first.
macOS and Windows targets get hits only once their confinement is in place
(open question 7).

---

## 10. The ledger: CI caching with hashes only

### 10.1 What is recorded

A record says "this key passed", plus enough provenance to answer "why was
this skipped?":

```json
{"key":"9f2c…","target":"rust/parser:test","os":"linux","arch":"x64",
 "kiln":"1.0.0","revision":"0bf84a8…","run":"43117337685","at":"2026-11-11T23:24:01Z"}
```

No build outputs, `node_modules` or tool state are ever stored. Records are
about 200 bytes; tens of thousands fit in a few megabytes.

### 10.2 Trust

- Only runs the `Host` reports as **trusted** (§13) write to the shared
  ledger. On GitHub, trusted means exactly: the ref is the default branch
  **and** the event is `push` or `schedule`. `workflow_dispatch` is trusted
  only on the default ref; `pull_request`, `pull_request_target` and
  merge-queue refs are never trusted.
- Pull-request runs **read** the shared ledger and never write to it. A PR
  that edits its workflow to fake passes can only skip its own checks — which
  is possible today anyway — and main re-checks every key it has not recorded
  itself after the merge. This means main still runs every target a PR
  changed, once, after the merge; making merge-queue runs trusted is a later
  option.
- A **missed** record only costs time. A **false** record is the real danger,
  so every design choice favours misses, and any doubt about a key makes it
  uncacheable.
- **Results that are never recorded:** SKIPPED (an optional tool was absent),
  failures, and anything from an untrusted run.

**Concurrent writers.** Merges land every few minutes and runs take longer
than that, so "restore the newest snapshot, save a new one" would lose most
additions. Instead each trusted run uploads only its **delta** (the records it
produced) as a run artifact, and a small `ledger-merge` job, in a concurrency
group per OS that never cancels in progress, restores the newest snapshot,
merges the delta and saves a new snapshot. Shards of one run contribute to
the same delta. Snapshots are named by sequence so old ones can be pruned
within the repository's cache quota; if a restore fails, the run simply
treats every key as a miss, and a failed restore on main raises an alert.

`ledger-merge` is the most trusted step in the system, so it is pinned down
tightly:

- It is a job in the **same workflow run** as the build (`needs:` the
  shards). It runs only when the ref is the default branch and the event is
  `push` or `schedule`. It never runs from `workflow_run`,
  `pull_request` or `pull_request_target` triggers.
- It uses code from that trusted checkout and consumes **only artifacts of
  its own run id**.
- It validates every record strictly before merging: a 64-hex key, bounded
  field lengths, a known schema, and caps on record count and delta size.
- **PR runs never save caches**, only restore, so they cannot evict main's
  snapshots by filling the quota.

**Kiln checking Kiln.** Kiln is compiled from source in every job,
including from a PR's checkout, so a PR that changes key computation or
trust logic is also the code that runs on main after the merge. Three
safeguards keep a self-validating change from hiding itself:

- The key-schema and expansion-semantics versions are enforced by golden
  vectors (§16) in CI. A change to key computation without a version bump
  fails.
- The daily `--ignore-ledger --strict` run is built from a **pinned
  prior-main** Kiln commit, not from the code it is checking.
- Required checks come from workflows defined on the base branch, not the PR
  head, where the host supports it. Requiring owner review for changes under
  `code/kiln/**` and the workflow files is recommended (open question 8).

### 10.3 Selection replaces `git diff`

CI no longer asks "which files changed since the diff base?". It computes
every target's key and asks the ledger which have passed. A PR branched from
recent main matches main's keys everywhere it changed nothing; only targets
whose keys differ — the changed packages and everything built on top of them —
run. This is more precise than the diff (toolchain and rule changes count) and
removes the need for the test-trigger resolver's special cases, which become
ordinary declared inputs.

### 10.4 Local runs

The same code with the `Local` host and a ledger in `<repo>/.kiln/ledger/`
(gitignored) skips anything already passed on your machine. Local runs cannot
read CI's ledger (GitHub offers no download API for Actions cache entries);
that is a `Ledger` adapter choice, not a design limit.

### 10.5 Keeping the cache honest

- **Daily uncached run.** Once a day CI runs everything with
  `--ignore-ledger --strict`, so flaky tests, runner drift and anything tool
  state (§9.5) masked still surface.
- **Revocation.** When the daily run disagrees with a ledger record, it writes
  a **tombstone** for that key, so the record stops producing hits at once
  instead of skipping a now-failing target on every PR forever. The target is
  reported, and its rule is flagged for an undeclared-input investigation.
- **Uncacheable by construction:** `cache: off`, an unpinned `fetch`, an
  `impure` shell step, and everything downstream of them.

---

## 11. Tools

### 11.1 Declared once

One table, `code/kiln/TOOLS`, replaces `mise.toml`, the setup steps in
`ci.yml`, the `# needs-toolchain:` comments and the per-language flags the
current `detect` job emits:

```
tool cargo:
  version: 1.83.0                   # exact; "latest" and "stable" are rejected
  provides: cargo rustc rustfmt
  check: rustc -vV                  # output must contain the version; hashed in full
  available: linux macos windows    # where it MUST be present (§11.3)
  linux-x64:   <url>  sha256 <hash>
  macos-arm64: <url>  sha256 <hash>
  windows-x64: <url>  sha256 <hash>

tool node:
  version: 22.11.0
  provides: node npm npx
  windows: npm = npm.cmd, npx = npx.cmd   # command → executable, per platform
  check: node -p process.versions

tool latexmk:
  version: 4.85
  provides: latexmk pdflatex
  check: latexmk --version
  available: linux
```

**Managed downloads** are fetched over HTTPS only, and the SHA-256 of the
downloaded bytes is verified **before** anything is unpacked. Extraction
rejects `..`, absolute paths and link entries that would escape the
destination, and caps total size. **Host-provided tools** (whatever the runner
has installed) are allowed only on hosted, ephemeral runners: their `check`
output is spoofable by a tampered binary, which is acceptable only where the
image itself is the trust root and is part of the host fingerprint (§8).

Tools built in this repo are named by target: `//code/programs/go/grammar-gen`.
Their identity is their own build action's key, so changing the tool's source
re-runs everything that uses it.

### 11.2 Resolution

A step's first word must be a command provided by a declared tool. The work
root's `PATH` contains only those commands, so calling an undeclared tool
fails immediately and loudly instead of working by accident. The plan lists
every tool it needs, so CI provides only those.

### 11.3 Optional tools

`tools: dotnet?` marks a tool optional in a rule. Whether it is present is
**declared**, not probed: the tool table's `available:` line says on which
platforms it must exist. That rule closes a trap the repo has fallen into
before (`lang-aot`'s tests silently never running): if presence were probed,
a tool missing on main *and* on the PR runner would give matching keys and
the tests would run nowhere.

- On a platform where the tool is declared available, its absence is an
  **error** on trusted runs, never a skip.
- On a platform where it is not declared available, the steps that use it are
  skipped and the result is **SKIPPED**, which is reported and **never
  recorded** in the ledger.

---

## 12. Cache policy

| Declaration | Effect |
|---|---|
| (default) | Cacheable: skipped when the ledger holds its key. |
| `cache: off` | Always runs; never recorded. For deploys, publishing, known-flaky suites. Its consumers are uncacheable. |
| `fetch` with `network: on` and `pinned-by: <lockfile>` | Cacheable; the lockfile is in the key and the tool verifies its integrity hashes (npm's `integrity`, cargo's checksums, uv's hashes). Kiln never parses lockfile formats. |
| `fetch` with `network: on` and no `pinned-by` | Runs, but it and everything downstream are uncacheable, and `kiln plan` lists it as unpinned. |
| `shell: impure """…"""` | Uncacheable, as above. |

Each distinct `fetch` key runs once per run; its output directory is staged
read-only into every work root that needs it. A target whose whole subtree is
a ledger hit — no miss anywhere above it — downloads nothing, which is the
largest download saving. A `fetch` that a miss consumes still runs (§14).

**Third-party code never runs with the network on.** `fetch` recipes use
no-build, no-script modes (`npm ci --ignore-scripts`,
`uv sync --no-build --no-install-project`) and run install scripts and build
backends offline in `build`. `fetch` actions receive no `pass` variables. On
Linux, a fetch's egress goes through a Kiln-run proxy that allows only a
per-ecosystem list of registry hosts, resolving and pinning their addresses
itself rather than trusting DNS inside the namespace; link-local and
cloud-metadata addresses are blocked.

The allowlists in `code/kiln/lint.toml` (`pass` names, registry hosts,
locked-mode commands) protect against a malicious PR only if the PR cannot
edit them. On trusted runs Kiln therefore reads `lint.toml` from the base
branch's revision via `Source`, not from the working tree.

**"Pinned" is checked, not taken on trust.** Kiln does not parse lockfiles,
so it verifies pinning another way:

- `pinned-by` is accepted only when the fetch step matches a locked-mode
  command allowlisted for that rule family in `code/kiln/lint.toml`
  (`cargo fetch --locked`, `npm ci`, `uv sync --locked`, …). A `fetch` that
  names a lockfile but runs `npm install` is treated as unpinned.
- A per-ecosystem lint, run as a scanner tool, rejects lockfile entries that
  lack integrity hashes (an npm `resolved` with no `integrity`, a git
  dependency pinned to a branch).
- The digest of each fetch's output tree is recorded for audit. Because the planner sees
every `fetch`, `kiln plan --third-party` prints the complete list of
third-party packages still being downloaded — a live B06 report.

---

## 13. Portable infrastructure: four interfaces

Kiln's core never names Git, GitHub, actions/cache or a download site. It
talks to four interfaces; moving to another provider means writing an
adapter.

```go
// Source answers questions about the files being built.
type Source interface {
    Files(root string, patterns []string) ([]string, error) // honours ignore rules
    Open(path string) (io.ReadCloser, error)
    Revision() (string, error)                               // opaque snapshot id
    Changed(since string) (paths []string, known bool, err error)
}

// Host describes where Kiln is running and who it is.
type Host interface {
    Identity() Identity   // revision, branch, event, run id, OS, arch
    ImageID() string      // OS version + runner image id, for the host fingerprint (§8)
    Trusted() bool        // may this run write to the shared ledger? (§10.2)
    Shard() (index, count int)
    WorkDir() string      // where work roots go
    Report(Event)         // summaries, annotations, log groups
}

// Ledger stores pass records. Hashes only.
type Ledger interface {
    Lookup(keys []Key) (map[Key]Record, error)
    Record(records []Record) error   // refuses unless Host.Trusted()
}

// Tools provides verified, pinned tools.
type Tools interface {
    Provide(spec ToolSpec, p Platform) (Provided, error) // paths + verified digest
}
```

| Interface | Implementations at launch | Later |
|---|---|---|
| `Source` | Git (`ls-files`, `diff --name-only`, `rev-parse`); **plain folder** (walks the tree with a `.kilnignore`; `Changed` returns unknown) | Perforce, others |
| `Host` | GitHub Actions; Local | Any CI |
| `Ledger` | Directory of record files (used directly locally; on GitHub the directory is restored and saved by `actions/cache` in one composite action) | Object store, HTTP |
| `Tools` | Managed (download pinned archive, verify SHA-256, unpack into a tool store); Host (use what is installed after checking the version) | — |

`Changed()` is used only to explain results ("this PR changed
`parser/src/lexer.rs`, so these 139 targets have new keys") and to skip
re-hashing unchanged files. It never decides correctness: if a source system
cannot answer, planning proceeds from keys alone. The plain-folder `Source` is
the proof — a build from a folder with no history must produce the same keys
and the same plan.

**GitHub Ledger adapter.** The Go tool reads and writes plain files in a
ledger directory. A composite action, `.github/actions/kiln-ledger`, restores
the newest snapshot with prefix `kiln-ledger-<os>-` before the build; on
trusted runs the delta is uploaded and merged by the `ledger-merge` job
(§10.2). GitHub scopes cache entries by branch: a `pull_request` run's saves
land in that PR's own scope, so it cannot write into main's scope even if the
PR edits the workflow. (Its own scope is consulted first on restore, which can
only affect that PR.) Kiln never calls GitHub's internal cache API.

**Required checks.** Never make a required check depend on job-level `if:`
skips. One always-running `kiln-gate` job reads the plan and the results and
reports the verdict, treating "skipped: passed on main under this key" as
success.

---

## 14. The planner

`kiln plan` writes a deterministic JSON plan that `kiln build` executes and a
human can read. Planning runs in two stages, because scanners are tools and
some are built in this repo:

- **Stage 0 — bootstrap.** Plan, build and key the scanner tools themselves
  (their own BUILD files may not use `scan`).
- **Stage 1 — plan.** Everything below.

1. **Load** every BUILD file through `Source`; treat legacy shell BUILDs as
   described in §15.
2. **Expand** each target through its rule into canonical actions (§6).
3. **Scan.** Run or reuse the cached scanner actions to fill in
   `deps: scan …`.
4. **Hash** inputs and **compute keys from the leaves up** (§8).
5. **Deduplicate.** Actions with identical keys become one.
6. **Consult the ledger.** For every miss, schedule the transitive closure of
   the `fetch` and `build` actions it consumes, regardless of ledger state.
   The ledger only ever removes `test` actions, and whole subtrees with no
   miss above them.
7. **Shard.** Assign actions to `Host.Shard()` slices so targets sharing large
   dependency trees land on the same runner, building `parser` once per
   runner rather than once per shard.
8. **Explain.** For every action that will run, record why: new key, which
   key component changed, `cache: off`, forced, or uncacheable upstream.

`kiln build` runs ready actions in parallel in dependency order. If two
workers want the same key at once, one builds and the other waits for it.

---

## 15. Migration

The order is chosen so the top goal — CI caching — pays off before any BUILD
file is rewritten.

| Stage | What happens | Exit criterion |
|---|---|---|
| **M0 — Pin** | Commit `Cargo.lock`, `uv.lock`, `Gemfile.lock`, `mix.lock`; exact tool versions. (In progress in a separate session.) | Every download is pinned; CI fails on a stale lockfile. |
| **M1 — Keys for legacy BUILDs** | The smallest Kiln that can cache: the key encoding with golden vectors, legacy key composition (below), the directory ledger with the `ledger-merge` job, the host fingerprint, the Git `Source` and the GitHub and Local `Host`s, and the shadow-comparison harness. **Not in M1:** the BUILD/rule language beyond what `legacy_shell` needs, scanners, `state`, optional tools, Managed tool provisioning, the plain-folder `Source`, `Changed()`, sharding changes and per-component explanations. | `kiln plan` over the whole repo is deterministic, and its package set matches the old tool's. |
| **M2 — Shadow** | CI computes keys beside the old flow and writes a ledger on main, but skips nothing. Before each 6-hourly forced full build, Kiln snapshots its predictions; afterwards every predicted hit is compared with the target's actual result. Each disagreement is re-run at the same commit: a pass means a flaky test, a repeat failure means an undeclared input to fix. Mutation tests touch a sibling file, a global file and a tool version and assert that the expected keys change. Each PR also reports "key hit but diff-affected": it should be near-empty, and every entry is a lead on a missing input. | At least N forced full builds per OS with zero unexplained disagreements. |
| **M3 — Skip** | CI skips ledger hits behind the `kiln-gate` required check. The daily `--ignore-ledger --strict` run and tombstones start. | CI time and queue time measurably down. |
| **M4 — Isolation + language** | Work roots, staging, scrubbed environment, network rules. Rule files for the most uniform languages first; a translator converts legacy BUILDs (strip dependency chains, match rules, map idioms such as `command -v X` → optional tool, `uname` → platform file, `a && b` → two steps, `cd d && x` → `in d: x`). `BUILD_windows` is translated separately and diffed, which should surface real drift bugs. | Per language: shadow-run old and new; switch when they agree. |
| **M5 — Tools + fetch** | `code/kiln/TOOLS`, managed provisioning, `fetch` semantics, network off elsewhere. `ci.yml` toolchain setup collapses to "provide what the plan needs". | — |
| **M6 — Retire** | Delete `BUILD_windows`, the `legacy_shell` path and the old build tool's selection logic; `ci.yml` becomes a thin wrapper around `kiln plan` / `kiln build`. | — |

### 15.1 Keys for legacy shell BUILDs (M1)

Legacy BUILDs still run **in place**, without isolation, so their keys must
over-approximate everything they might read. They are expressed as an
ordinary rule file, `code/kiln/rules/legacy_shell.rule`, which declares
`isolation: none` — a mode only legacy actions may use — so no ecosystem
knowledge enters the Go core. A legacy action's key contains:

1. **Every non-ignored file** of the package *and of every transitive
   dependency*, not filtered by file extension.
2. **The relations the current build tool already knows**: the dependency
   graph, the repository-input boundary and the test-trigger relations,
   imported from the old tool's plan output.
3. **Ecosystem globals**: `code/packages/rust/Cargo.toml` and its lockfile,
   shared `tsconfig` bases, and similar files, listed in the rule.
4. **The legacy `BUILD` and `BUILD_windows` text**, the old build tool's
   source digest, and the CI toolchain-setup definition. That definition
   moves out of `ci.yml` into one hashed file so it can be keyed.
5. **Resolved tool versions, probed at plan time** (`rustc -vV`, `node -v`,
   …), plus the host fingerprint (§8 item 3). The digest of today's
   `mise.toml` is **not** enough: it says `latest`, so it would not change
   when the toolchain does.
6. **A lexical check**: every `../` path in the BUILD must resolve inside the
   keyed closure. If one does not, the action is uncacheable.

**Downloads.** Legacy BUILDs install with the network on (`npm install`,
`uv pip install`, `bundle install`, `cpanm`, `luarocks`). By §12's rule those
actions are uncacheable. A legacy action becomes cacheable only when its
ecosystem is on a **pinned allowlist**: the lockfile is committed, the BUILD
installs from it in locked mode, and the lockfile is a keyed input. Each
ecosystem joins the allowlist as M0 lands for it. Until then its packages are
planned and reported, but never skipped.

### 15.2 Starting point

Measured starting point (2026-11-11, all 5,496 shell BUILDs): 89% have three
non-comment lines or fewer, 96% once dependency chains are removed; 100
distinct shapes cover 74% of files before rules take parameters. The 301
packages whose BUILD is `sh tools/run.sh` (C/C++ ISO harness) need a closer
look: only 57 of their scripts are identical once the package name is
normalised.

---

## 16. Conformance and the educational ports

The ~16 build-tool ports in other languages are **frozen at the legacy-BUILD
level**; they are not asked to implement work roots, provisioning or ledgers.
The new conformance surface is two pure functions with golden vectors,
which any port may implement:

1. **Parse and expand:** BUILD + rule files → canonical expanded actions.
2. **Key:** canonical actions + file digests + tool identities → keys.

Only the Go implementation stages, executes and talks to a ledger. Ports that
implement the two functions double as independent checks of the key
algorithm.

---

## 17. Implementation language: Go

Kiln is written in Go, under `code/programs/go/kiln/`, using only the standard
library.

- **Everything Kiln needs is in the standard library:** SHA-256, process
  execution, Linux namespaces (`syscall.SysProcAttr`), HTTP, tar/gzip/zip for
  tool archives, JSON. Rust would need in-house crates for several of these
  (the repo has `deflate`, `http1-client` and `bounded-json`), which would
  make the build tool depend on packages it builds.
- **Kiln is compiled from source in every CI job** (supply-chain: no prebuilt
  binary), and Go compiles it in seconds; the current Go tool compiles in
  2–14 s on the hosted runners.
- **Concurrency is not the deciding factor.** The workload is "start
  processes and wait", which either language handles well. Goroutines make
  the scheduler slightly simpler.
- **Continuity.** The current Go build tool is proven in this CI; its
  executor, plan format and CI integration are a starting point.

---

## 18. Open questions

1. **Cargo workspace staging.** `code/packages/rust/Cargo.toml` lists every
   member, so a staged subset will not load. Likely answer: a small in-repo
   tool, used by `rust_crate`, that writes a trimmed workspace manifest in the
   work root; confirm `--locked` accepts a lockfile with unused entries.
2. **Staging cost** on ext4 and NTFS for targets with ~750-target closures.
   Measure the top 20 closures before M4; mitigate with incremental syncing by
   content hash.
3. **Whether `state` works at all** for cargo across work roots (§9.5), and
   what stable-path mechanism it needs.
4. **Ledger growth and pruning** policy (keep records whose keys appear in the
   last N main plans?), within the Actions cache quota.
5. **What replaces the bespoke gate jobs in `ci.yml`** (conformance suites,
   grammar regeneration checks): most should become ordinary targets with
   rules; a few may stay as `cache: off` targets.
6. **The value of N** in M2's exit criterion, and how long shadow mode runs.
7. **Confinement on macOS and Windows** (§9.6): which mechanism (a separate
   user account, App Sandbox profiles, Windows job objects with a restricted
   token) lets those runs become trusted.
8. **Owner review for `code/kiln/**` and workflow files.** Recommended by
   the security review; it conflicts with the repo's auto-merge practice for a
   solo owner, so it is the owner's call.

---

## 19. Summary

Kiln is a small Go core that knows six things — file sets, actions, keys, a
ledger of passes, tool state and scanners — plus a two-tier language: BUILD
files that are usually one line, and rule files that teach Kiln each language.
Every action runs in a private copy of exactly what it declared, its key
covers everything it could have read, and CI skips any key main has already
passed. Only hashes cross CI runs. Git, GitHub, the cache store and tool
downloads all sit behind interfaces. Migration starts by caching the existing
shell BUILD files, so the value arrives before the rewrite.
