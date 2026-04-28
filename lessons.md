# Lessons Learned

This file tracks mistakes made during development so they are not repeated. Check this file before starting any new work.

---

### 2026-04-23: QR format info `write_format_info` — bit ordering is MSB-first in row 8

ISO/IEC 18004 places format information bits **MSB-first** (f14 → f9) going
left-to-right across row 8 (cols 0–5) and **LSB-first** (f0 → f5) going
top-to-bottom down col 8 (rows 0–5).  Copy 2 mirrors this: f0 → f7 going
right-to-left across row 8 (cols n−1 → n−8), and f8 → f14 going top-to-bottom
down col 8 (rows n−7 → n−1).

**The bug:** `write_format_info` was placing bits in LSB-first order everywhere,
producing a reversed 15-bit word.  Both copies read `0x1F3D` instead of `0x5E7C`
for ECC=M/mask=2.  BCH remainder was `0x3DA` (non-zero) for both copies, so
every standard decoder (zbarimg, iPhone camera, ZXing) rejected the format info
and could not determine the correct mask pattern or ECC level — the QR code
appeared structurally correct but was completely unscannable.

**Root cause:** The manual decoder written to debug the issue used the same
reversed reading order, so it incorrectly confirmed the format info as valid.
The bug was only caught by comparing pixel values at specific grid positions
against the expected standard layout.

**Fix:**
- Copy 1, row 8 cols 0–5: use `(fmt >> (14 - i))` (f14 at col 0, not f0).
- Copy 1, (8,7) = f8, (8,8) = f7, (7,8) = f6.
- Copy 1, col 8 rows 0–5: use `(fmt >> i)` (f0 at row 0 … f5 at row 5).
- Copy 2, row 8 cols n−1..n−8: use `(fmt >> i)` for i=0..7 (f0 at rightmost).
- Copy 2, col 8 rows n−7..n−1: use `(fmt >> i)` for i=8..14.

**Always verify with `zbarimg` (or equivalent standard decoder) immediately
after implementing format info — the BCH check is the ground truth.**

---

### 2026-04-21: BUILD files must be updated whenever package.json dependencies change

The build-tool validates that every `BUILD` / `BUILD_windows` shell script lists
all transitive local packages as `npm install` preludes, in leaf-to-root order.
Changing a package's `package.json` dependencies without updating its `BUILD`
(and the `BUILD` of every program/package that transitively depends on it) is
a CI failure.

**Symptom:** `detect` job fails with:

```
undeclared local package refs: typescript/<removed>;
missing prerequisite refs for standalone builds: typescript/<added>
```

**Rule:** When editing `dependencies` in a `package.json`, immediately:

1. Update that package's `BUILD` and `BUILD_windows` to reflect the new deps.
2. Grep all `BUILD*` files for the OLD dep name across the repo.
3. For every hit, replace the old dep line(s) with the new deps in correct
   leaf-to-root order.

Applies to Python/Ruby/Go/Rust/Elixir/Perl BUILD files too — same rule, same
format.

---

### 2026-04-21: Full Rust workspace builds can include platform-only crates

`cargo build --workspace` is useful for catching missing Rust exports, but this
workspace currently includes crates that intentionally compile only on specific
operating systems.

**Symptom:** On macOS, a full workspace build reaches `paint-vm-direct2d` or
`paint-vm-gdi` and fails with a compile-time message that the crate requires
Windows.

**Rule:** Treat platform-only compile errors as workspace configuration scope,
not as regressions in the package under test. Still run the focused package
`BUILD` scripts and any directly affected dependency builds before pushing.

---

### 2026-04-20: Commit or otherwise expose intended diffs before relying on build-tool diff mode

The Go build tool's default changed-package path depends on `git diff` against
the configured base. If the intended package changes are only uncommitted in a
fresh worktree, the tool may be unable to compute the intended diff and fall
back to hash/cache mode. In a large checkout, that can begin a monorepo-scale
build and create unrelated coverage or build artifacts before the mistake is
noticed.

**Symptom:** `./build-tool --diff-base origin/main` prints `Git diff
unavailable — falling back to hash-based cache` and starts building thousands of
packages instead of the small affected set.

**Rule:** Before using the build tool for PR verification, either commit the
intended diff or first verify `git diff --name-only origin/main...HEAD` returns
the changed files the tool should see. If the tool announces a hash/cache
fallback unexpectedly, stop it immediately and clean generated artifacts before
continuing.

---

### 2026-04-20: Downstream package tests should not pin exact dependency patch versions

When a foundational package intentionally bumps its version, dependent packages
may rebuild in the same affected set. If a downstream test asserts the exact
dependency version string, the downstream package fails even when its declared
dependency range and runtime behavior are still valid.

**Symptom:** A package depending on `logic-engine>=0.3.0` failed only because a
test asserted `logic_engine.__version__ == "0.4.0"` after `logic-engine` moved
to `0.5.0`.

**Rule:** Downstream smoke tests should assert a minimum compatible dependency
version or a capability, not an exact dependency version, unless the package
truly requires that exact release.

---

### 2026-04-20: Compiler-generated data segments need source-stage size caps

Even when a frontend only emits internal IR, any IR data declaration that a
backend materializes as bytes can become a host-memory exhaustion path. Source
size and type-checking success do not automatically bound semantic frame plans
or generated runtime images.

**Symptom:** A compiler sums frame sizes and emits one data declaration, while
the WASM backend later expands it with `bytes(...) * size`.

**Rule:** Put explicit byte caps at the earliest compiler stage that computes
the generated data size, and test the rejection path with a synthetic semantic
model rather than a huge source file.

---

### 2026-04-20: CI setup-job failures can be infrastructure flakiness, not code failures

GitHub Actions can fail before checkout or before any repository command runs
if the runner cannot download a pinned action archive. These failures look red
on the PR, but there is no package, test, or build output to fix in the repo.

**Symptom:** A job fails during "Set up job" with an error such as
`Failed to download archive` for an action repository tarball, while sibling
matrix jobs and earlier push runs pass.

**Rule:** Inspect the failing job log before changing code. If the failure is
limited to downloading an action archive during setup, rerun or retrigger CI and
avoid inventing an application-code fix for an infrastructure failure.

---

### 2026-04-20: Do not leak local machine state in commits or PR descriptions

CI fixes sometimes involve local environment problems, but commit messages and
PR descriptions are permanent project history. They should explain the portable
engineering lesson without naming private paths, host setup details, account
state, or other machine-specific facts that do not belong in the repository.

**Symptom:** A fix message or PR description mentions a developer workstation
condition instead of the general failure mode that future contributors need to
understand.

**Rule:** Write history in terms of reproducible repository behavior. If a
local detail helped diagnose the issue, translate it into a general rule before
committing or opening the PR.

---

### 2026-04-20: Tests that need a CLI tool must verify the tool actually runs

Checking `exec.LookPath("git")` only proves that a binary exists on `PATH`. A
tool can still fail every invocation because of host configuration, permissions,
or setup policy, which makes tests fail after they already decided the tool was
available.

**Symptom:** A test guarded by `exec.LookPath("git")` still fails at the first
real Git command.

**Rule:** Tests that depend on an external CLI should run a harmless probe such
as `git --version` and skip when that command fails. Presence is not usability.

---

### 2026-04-20: CodeQL flags unchecked CLI integer downcasts from `int64` to `int`

CodeQL treats parsed CLI integer values as untrusted numeric input. If a Go
program converts a signed 64-bit parsed value to `int` without checking the
current platform's `int` width, CodeQL raises a high-severity
`go/incorrect-integer-conversion` alert even when tests pass on 64-bit local
machines.

**Symptom:** A PR's CodeQL workflow job succeeds, but GitHub Advanced Security
adds a separate failing `CodeQL` check with annotations like "Incorrect
conversion between integer types" on `return int(value)`.

**Rule:** When converting parsed or decoded numeric input to `int`, add
explicit platform-sized bounds checks first or route through `strconv.Atoi`
after formatting/validating the value. For `float64`, reject NaN, infinity, and
non-integral values before attempting any `int` conversion.

---

### 2026-04-21: Use body files for GitHub PR text containing Markdown backticks

When passing a Markdown PR body directly to `gh pr create --body "..."` or
`gh pr edit --body "..."`, shell command substitution still applies inside the
double-quoted string. Inline code spans such as `` `goal_from_term(...)` `` can
therefore be executed by `zsh` before `gh` receives the body, producing noisy
shell errors and a mangled pull request description.

**Rule:** For PR descriptions or comments that contain Markdown backticks, write
the body to a temporary file with a single-quoted heredoc and pass it via
`--body-file`. Do not put Markdown-heavy PR bodies directly in a double-quoted
shell argument.

---

### 2026-04-20: Compiler runtime specs must bound execution and captured environment lifetimes

When designing a compiler/runtime for a language with recursion, nested procedures,
closures, thunks, or explicit stack frames, source-size and AST-depth limits are
not enough. The runtime also needs execution fuel or timeout policy, dynamic call
depth limits, frame stack byte limits, heap allocation limits, stack/heap
collision checks, and clear captured-environment lifetime rules.

**Symptom:** Security review flags a roadmap or implementation because recursive
programs can exhaust frame memory or run forever, and descriptors that capture raw
frame pointers could outlive the activation they point into.

**Rule:** For compiler runtimes, specify and test runtime resource caps and
captured environment handling before implementing procedures, closures, or
call-by-name thunks. Either reject escaping descriptors, prove they cannot escape,
or heap-lift captured environments with explicit lifetime management.

---

### 2026-04-19: JVM composite Gradle BUILD files need a shared lock when they reuse included builds

Java and Kotlin packages that include the same local Gradle builds can corrupt
shared `gradle-build` class outputs when the repo build tool runs sibling
packages in parallel.

**Symptom:** CI fails with a Java compiler error like `bad class file ...
class file truncated at offset 0` inside a shared dependency such as
`java/wasm-types`, even though each package passes when run alone.

**Rule:** When adding multiple JVM packages that point at the same local
included Gradle builds, serialize the Unix `BUILD` commands with a shared
repo-local lock and run Gradle with `--no-daemon --no-build-cache --max-workers=1`.
Keep the lock in the package BUILD scripts so CI's parallel package scheduler
cannot race the shared composite outputs.

---

### 2026-04-18: LuaRocks CI installs may need patched GitHub archive URLs for old rockspecs

Some published LuaRocks rockspecs still point at legacy GitHub archive URLs
like `https://github.com/<owner>/<repo>/archive/<tag>.tar.gz`. Those URLs can
be flaky or return gateway errors in CI even when the corresponding tag still
exists.

**Symptom:** shared CI setup fails before any package build runs, typically
while installing `busted` or one of its transitive dependencies, with an error
like `Failed downloading https://github.com/.../archive/0.08.tar.gz`.

**Rule:** When a LuaRocks dependency fails because of an old GitHub archive
URL, patch the downloaded rockspec in CI to use the stable
`archive/refs/tags/<tag>.tar.gz` form and install from that patched rockspec
before proceeding with the rest of the Lua test tool bootstrap.

### 2026-04-18: CI workflow classifier must recognize helper shell lines in toolchain-scoped hunks

The build tool analyzes `.github/workflows/ci.yml` diffs to decide whether a
workflow change is limited to one language toolchain or should force a full
monorepo rebuild. If a toolchain-scoped hunk includes ordinary helper commands
that the classifier does not recognize, such as `sed` or `rm`, the whole PR can
fall back to a full build.

**Symptom:** a small Lua setup fix triggers every language toolchain and builds
thousands of packages, surfacing unrelated failures and making CI take hours.

**Rule:** Whenever a CI setup hunk adds helper shell commands, add a regression
test in `internal/gitdiff/ci_workflow_test.go` proving the hunk remains
toolchain-scoped. Keep the allowlist narrow and tied to commands that only
support the already-detected language setup.

### 2026-04-18: Haskell cabal.project files must list transitive local packages

Cabal does not discover sibling packages from another sibling's
`cabal.project`. If a package includes `../lexer`, and `lexer.cabal` depends on
the local `grammar-tools` package, the top-level package's own
`cabal.project` must also list `../grammar-tools`.

**Symptom:** a full Haskell build fails with `unknown package: grammar-tools`
while trying to solve dependencies for `lexer`, even though
`code/packages/haskell/grammar-tools` exists in the repo.

**Rule:** When adding or maintaining a Haskell `cabal.project`, include every
local package in the transitive dependency closure. Do not rely on nested
`cabal.project` files from dependencies to make their local dependencies
visible.

### 2026-04-18: TypeScript BUILD files that touch the paint stack must install `pixel-container` explicitly

The build-plan validator checks standalone BUILD prerequisites by looking at
the actual sibling refs installed by the package's BUILD script. In the
TypeScript paint stack, `paint-instructions`, `paint-vm`, `paint-vm-ascii`, and
`format-doc-to-paint` all rely on `pixel-container` during clean standalone
builds even when the top-level package does not mention it directly in its own
`package.json`.

**Symptom:** CI fails in the detect/build-plan stage with an error like
`missing prerequisite refs for standalone builds: typescript/pixel-container`
before any tests or type-checking run.

**Rule:** If a new TypeScript package installs paint-stack siblings in its
`BUILD` file, include `cd ../pixel-container && npm install --silent` before the
paint packages so the standalone prerequisite closure matches what CI expects.

### 2026-04-18: Platform-specific Python native packages still need platform-neutral coverage

When a Python package wraps native windowing or other OS APIs, one platform may
exercise the full smoke path locally while another CI platform skips it. If the
tests only cover the native happy path, Linux or Windows can still fail the
repo coverage gate even though the wrapper logic itself is correct.

**Symptom:** macOS passes because a native smoke test runs, but Ubuntu fails
coverage because the same test is skipped and the wrapper methods around the
native bindings are never exercised.

**Rule:** For platform-specific Python native packages, add mocked wrapper tests
for argument normalization, handle lifecycle, and method delegation so the
Python facade stays above coverage thresholds even when native smoke tests are
skipped off-platform.

### 2026-04-18: Downstream TypeScript BUILD files should not fail on unrelated upstream source errors

Many TypeScript packages in this repo depend on sibling packages through
source-first `file:` dependencies. A package-level `npx tsc --noEmit` can
therefore type-check far beyond the package being changed and fail on an
upstream error that already exists on `main`.

**Symptom:** CI fails in a downstream package build with type errors from a
shared sibling package that was not touched by the PR, even though the
downstream package's own tests and runtime behavior are correct.

**Rule:** Keep each TypeScript package `BUILD` focused on the verifications the
package actually owns. If a source-level sibling typecheck is already broken on
`main`, do not gate an unrelated package PR on `npx tsc --noEmit` until the
shared failure is repaired.

### 2026-04-18: New TypeScript packages should not commit local transpile outputs

TypeScript package-local `tsc` and test runs can emit `.js`, `.d.ts`, and
source-map files right beside the checked-in `.ts` sources. If those generated
files are staged, the PR ends up carrying duplicate runtime and test artifacts
that do not match the repo's source-first `main: "src/index.ts"` package
convention.

**Symptom:** a new package PR includes paired `src/*.ts` and generated
`src/*.js`, `src/*.d.ts`, `tests/*.js`, or `vitest.config.js` files even though
the package entrypoint already points at the `.ts` sources.

**Rule:** For new TypeScript packages, commit the `.ts` sources, metadata, and
lockfile only. Do not commit local transpile outputs such as `src/*.js`,
`src/*.d.ts`, `tests/*.js`, `vitest.config.js`, or their source maps unless the
package intentionally publishes prebuilt artifacts and the spec says so.

---

### 2026-04-18: Default to a fresh git worktree before starting substantive repo work

In this repo, the main checkout often contains unrelated untracked files, active
agent output, or in-progress specs on other branches. Trying to begin work in
that noisy tree can block the required `git merge origin/main` step or create a
risk of mixing unrelated changes into the feature.

**Symptom:** `git merge origin/main` fails because untracked local files would be
overwritten, or the worktree is too noisy to safely isolate a new package/spec
change.

**Rule:** Before doing substantive work, create a fresh `git worktree` from
`origin/main` on a dedicated feature branch and do the implementation there.
Treat this as the default, not an exception, whenever the source checkout is
shared or noisy.

### 2026-04-18: Dart decompressors must cap declared output size from untrusted headers

Compression formats often encode the original byte length in the payload
header. If the decoder trusts that field blindly, an attacker can declare a
huge output size and force the process to allocate or append toward that size,
turning decompression into a memory-exhaustion denial of service.

**Symptom:** Security review flags `decompress()` because a crafted payload can
claim an arbitrarily large `originalLength`, and the Dart implementation
attempts to honor it without any upper bound.

**Rule:** Any Dart decompressor that consumes a declared output length from
untrusted bytes must enforce a sane maximum decompressed size before decoding.
Expose the cap as an override for trusted callers, but fail closed by default
with `FormatException` when the header exceeds the limit.

### 2026-04-18: Dart decompressors must validate backreferences before indexing decoded output

For byte-oriented compression formats like LZ77, the decoder is a trust
boundary whenever it accepts token streams or compressed bytes from outside the
process. A malformed token with `offset == 0` or `offset > decoded_prefix_len`
can trigger a `RangeError` if the implementation blindly indexes into the
already-decoded output buffer.

**Symptom:** Security review flags a denial-of-service bug because `decode()`
crashes on hostile compressed input instead of rejecting it cleanly.

**Rule:** Every Dart decoder for backreference-based formats must validate
token fields before copying. Backreferences need `offset > 0` and
`offset <= output.length`, and malformed or truncated streams should throw
`FormatException` rather than indexing past buffer bounds.

---

### 2026-04-18: BUILD files must avoid shell quotes and line-continuation backslashes that confuse the repo runner

The repo build tool does not execute BUILD files exactly like an interactive
shell script file. Commands that look fine in isolation, such as quoted extras
like `'.[dev]'` or backslash-continued multi-line commands, can be mangled by
the runner's wrapper and fail in CI with errors like `unexpected end of file`
or `\: not found`.

**Symptom:** a BUILD file passes local spot checks but CI fails before the
package itself runs, usually with plain shell parse errors instead of package
test failures.

**Rule:** Keep BUILD scripts wrapper-safe: avoid embedded shell quotes when a
simple escaped token works, and prefer separate commands or subshells over
trailing `\` continuations.

When a Unix `BUILD` script needs a temporary path or other computed value, do
not put a heredoc inside command substitution like `VAR="$(python - <<'PY' ...
PY)"`. That form can pass on macOS shells and still fail under Linux `dash`
with `unexpected EOF` parse errors. Prefer `python -c '...'` or plain shell
loops for simple staging logic.

---

### 2026-04-18: Lua packages tested with `busted` must install or expose `LUA_PATH` first

Lua test files often `require("coding_adventures.<package>")`, which expects
the package to be installed via LuaRocks or the source tree to be visible
through `LUA_PATH`. Running `busted` from `tests/` without either setup makes
CI fail with `module 'coding_adventures.<package>' not found` even though the
source file exists in `src/`.

**Symptom:** the Unix or Windows BUILD passes control to `busted`, but the
test process cannot load the package module because it only sees the `tests/`
working directory and default Lua search paths.

**Rule:** For Lua packages in this repo, BUILD scripts must either run
`luarocks make --local` first or export a `LUA_PATH` that points at
`../src/?.lua` and `../src/?/init.lua` before invoking `busted`. Prefer doing
both when the package already ships a rockspec.

---

### 2026-04-18: Elixir coverage thresholds require tests for delegates and error branches too

Small Elixir packages can miss the repo's `80%` coverage threshold even when
their primary happy-path tests pass, because delegate helpers and negative
parsing branches still count toward the total module coverage.

**Symptom:** `mix test --cover` reports green tests but fails the package build
with coverage in the low 70s because helper modules such as request/response
heads or invalid-parse branches were never exercised.

**Rule:** When adding a new Elixir package with coverage enforcement, include
tests for delegate helpers (`header`, `content_length`, `content_type`) and
invalid input branches, not just the main success path.
### 2026-04-18: Dart binary deserializers should reject both short and padded payloads

When a package defines a fixed-width wire format, accepting undersized payloads
as "empty" messages or silently ignoring extra bytes creates a fail-open
boundary. That can hide tampering and lets callers treat malformed compressed
data as if it were valid.

**Symptom:** Security review flags insecure deserialization because a parser
accepts incomplete headers or trailing attacker-controlled bytes instead of
failing closed.

**Rule:** For Dart binary formats, validate the exact expected byte length from
the header before decoding any entries, and validate that the reconstructed
payload length exactly matches any declared output length. Reject incomplete
payloads, extra trailing bytes, underflow, and overflow with `FormatException`.

---

### 2026-04-18: Lua rockspecs must pin immutable source refs, not just HTTPS URLs

Switching a Lua rockspec from `git://` to `https://` fixes transport security,
but it does not make installs reproducible. If the rockspec points at a moving
branch tip with no immutable tag or commit, the published package version can
resolve to different source code over time.

**Symptom:** Security review flags a supply-chain integrity issue because a
`0.1.0-1` rockspec can fetch whatever happens to be at the repository head at
install time.

**Rule:** Every Lua rockspec that installs from GitHub must use `https://` and
must pin the `source` table to an immutable git ref, such as a release tag or
commit SHA.

---

### 2026-04-18: Never expose caller-controlled FFI input enums as Rust `repr(C)` enums

At the C ABI boundary, foreign callers can pass any integer bit pattern for an
enum field. If a Rust `repr(C)` enum is embedded directly in an input struct and
the caller supplies an out-of-range discriminant, Rust can observe invalid enum
values, which is undefined behavior before normal validation code ever runs.

**Symptom:** Security review flags FFI input structs that use Rust enums for
caller-controlled fields such as mount-target kind or surface preference.

**Rule:** For all FFI input structs, represent enum-like fields as primitive
integers (`u32`, `c_int`, etc.) in both the Rust ABI struct and the C header,
then validate/convert them explicitly with `match`/`TryFrom` before using them
as internal enums.

---

### 2026-04-17: Use a fresh git worktree before editing shared manifests in a noisy repo

When a worktree already contains unrelated untracked package directories or
other agents are actively building in the same checkout, shared files like
workspace manifests can pick up accidental references to crates that are not
part of the intended change. In this case, a Rust event-loop PR accidentally
committed `job-*` workspace members from the surrounding dirty tree, and CI
failed because those package directories were never pushed with the branch.

**Symptom:** CI fails with errors like `failed to load manifest for workspace member ... No such file or directory`, even though the feature itself builds locally in isolation.

**Rule:** If the source worktree has unrelated untracked files, package
directories, or active agent work, create a fresh `git worktree` from
`origin/main` before staging or committing shared manifest changes. Replay only
the intended commits there, then push from the clean worktree.

---

### 2026-04-17: Socket tests should not assume immediate accept batching or instant EOF propagation

Local TCP tests can be timing-sensitive even on loopback. A listener may not
see every new connection in the very first `accept()` burst, and a client that
was refused server-side may not observe EOF immediately on the next `read()`.
Tests that assume either behavior become flaky under the repo build tool and CI.

**Symptom:** a readiness or connection-cap test passes in one direct `cargo test`
run but fails under CI or under the repo build tool with missing accepted
connections or a refused client that never reports close quickly enough.

**Rule:** In socket tests, wait for the expected accepted-connection count with
bounded retries, and assert the stable invariant you actually care about
(`connections.len()`, state transitions, explicit errors) instead of requiring
an immediate EOF from the peer.

---

### 2026-04-18: Event-loop tests must not require independent sources to co-occur in one poll batch

Native event loops report what is ready now, not what was ready "as a group"
across the whole scenario. A wakeup, a timer expiry, and two client-readable
streams may be delivered across separate `poll()` calls or separate event
batches even when all of them happen during the same short test window.

**Symptom:** a transport test flakes or fails by asserting that two different
resources both appear in the same returned `Vec<Event>` instead of tracking
whether each resource was observed at least once before the deadline.

**Rule:** For event-loop and reactor tests, accumulate observations across
multiple `poll()` iterations and assert the stable outcome ("did we ever see A
and B?") rather than requiring unrelated readiness sources to appear in the
same poll batch.

---

### 2026-04-12: Never commit build artifacts — agents running tests will generate them

When agents run tests locally (e.g., `swift test`, `mix test`, `bundle exec rake test`), they generate build artifacts in directories like `.build/`, `cover/`, `vendor/`, `node_modules/`, `_build/`, `deps/`, `blib/`, `MYMETA.*`, `pm_to_blib`. If the agent then runs `git add .` or `git add <package-dir>/`, these artifacts get committed.

**Symptom:** Windows CI fails with "Filename too long" for deeply nested Swift `.build/` paths. Repo bloats by thousands of files.

**Rule:** After agents complete, always check `git status` for build artifacts before committing. Use specific file paths in `git add` rather than directory globs. Never commit: `.build/`, `cover/`, `vendor/`, `node_modules/`, `.venv/`, `deps/`, `_build/`, `__pycache__/`, `blib/`, `MYMETA.*`, `pm_to_blib`, `Makefile` (Perl-generated), `go.sum`.

**Prevention:** Every new Swift package MUST include a `.gitignore` containing `.build/` and `.swiftpm/`. These directories are created by `swift test` and can contain thousands of deeply nested files that break Windows CI with "Filename too long" errors. The `.gitignore` prevents this even if an agent runs tests and does `git add .`.

---

### 2026-04-12: Security fixes that change error messages require updating test assertions

When unifying error messages for security (e.g., generic "Invalid PKCS#7 padding" to prevent padding oracle attacks), tests that assert specific old messages (`match="Invalid padding value"`, `toThrow("inconsistent padding bytes")`) will fail. Always grep all test files for the old messages after making security changes.

**Rule:** After changing error messages in source code, run `grep -r "old message pattern" */test* */t/` to find all test assertions that need updating.

---

### 2026-04-12: Build tool validator requires declared deps in metadata files, not just BUILD

---

### 2026-04-18: Linux `epoll_event` FFI mirrors must use the kernel's packed layout

The Linux kernel declares `struct epoll_event` as packed. Modeling it as a
plain `#[repr(C)]` Rust struct can appear to work for single events but corrupt
or drop readiness information once `epoll_wait` returns multiple events.

**Symptom:** Linux CI flakes or fails in higher-level readiness tests with
missing readable streams, even though the logic above `epoll` looks correct.

**Rule:** Any Rust FFI mirror of Linux `epoll_event` must use the kernel's
packed layout and should be covered by a test that waits on multiple ready file
descriptors at once.

When a BUILD file references a sibling package (e.g., `cd ../json-rpc`), the build tool's validator (`-validate-build-files`) checks that the referenced package is a declared predecessor in the dependency graph. The graph edges come from **metadata files**, not BUILD files:

- **Python**: `dependencies` array in `pyproject.toml`
- **Ruby**: `spec.add_dependency` in `.gemspec` (regex requires `spec.`, not `s.`)
- **Perl**: `requires 'coding-adventures-xxx';` in `cpanfile`
- **Go**: `require` in `go.mod`
- **Swift**: `.package(path: "...")` in `Package.swift`
- **Rust**: `[dependencies]` in `Cargo.toml`
- **TypeScript**: `dependencies` in `package.json`

**Rule:** Every package that references a sibling in its BUILD file must also declare the dependency in the language-appropriate metadata file. The Ruby gemspec must use `spec` (not `s`) as the block variable, matching the build tool's regex `spec\.add_dependency`.

---

### 2026-04-27: Structural protocol test deps must be installed in BUILD — but protocol adapters don't need them at runtime

When a package implements a structural protocol from a sibling package (e.g., `codegen-core`'s
`CodeGenerator`) and its TESTS import that sibling for `isinstance()` checks, that sibling must
be installed in the BUILD venv even though the production code never imports it.

**Symptom:** CI fails with `ModuleNotFoundError: No module named 'codegen_core'` in test
collection, even though the adapter code itself does not import `codegen_core`.

**Root cause:** The `test_codegen_generator.py` files do `from codegen_core import CodeGenerator`
for `isinstance(gen, CodeGenerator)` checks.  Without `codegen-core` in the BUILD, the test
module cannot be collected.

**Fix:** Add the protocol package (and its transitive local deps) to the BUILD install line.
Also declare it in `pyproject.toml` `[project.optional-dependencies]` dev so the build
validator's `undeclared local package refs` check passes.

**Transitive deps must be explicit — in BOTH BUILD and pyproject.toml:**
uv does not auto-install local editable transitive deps from PyPI-registered names.  If
`codegen-core` depends on `interpreter-ir` and `ir-optimizer` (also local-only packages),
you must:
  1. Add `-e ../interpreter-ir -e ../ir-optimizer` to the BUILD install line explicitly.
  2. ALSO add `coding-adventures-interpreter-ir` and `coding-adventures-ir-optimizer` directly
     to dev extras in pyproject.toml.

Declaring `codegen-core` in dev extras is NOT sufficient to satisfy the build validator for
packages it in turn depends on — every package referenced directly by a `-e ../pkg` in a
BUILD file must be directly declared in the package's own pyproject.toml metadata.

**Order matters:** install in leaf-to-root order:
  1. `interpreter-ir` (no internal deps)
  2. `ir-optimizer` (depends on compiler-ir, already present)
  3. `codegen-core` (depends on 1, 2, and compiler-ir)

---

### 2026-04-17: Python build validation only whitelists sibling refs that appear in dependency metadata

For Python packages, adding a sibling path only under `[tool.uv.sources]` is not enough to satisfy the build validator's `undeclared local package refs` check. If a BUILD script installs a sibling package for tests or tooling, that sibling must also appear in `dependencies` or an appropriate `[project.optional-dependencies]` group so the validator can see the edge in the package metadata.

**Symptom:** CI detect or CodeQL fails in the build validation phase with `undeclared local package refs: python/<package>` even though the BUILD file and `tool.uv.sources` both mention the sibling path.

**Rule:** For Python packages, declare every BUILD-installed sibling package in `pyproject.toml` dependency metadata as well as in `[tool.uv.sources]`. If the sibling is test-only, put it in the `dev` extra and prefer installing `.[dev]` from BUILD rather than duplicating a standalone `-e ../package` entry.

---

### 2026-04-06: Perl `reverse LIST, $extra` vs `(reverse LIST), $extra` — precedence trap

When building a list that is the reverse of one list plus an extra item, the expression
`(reverse @$list, $extra)` in Perl is parsed as `reverse(@$list, $extra)` — reversing the
ENTIRE list including `$extra`. Use explicit double parens: `((reverse @$list), $extra)`.

**Symptom:** `lineage()` returned the entity itself as the FIRST element instead of the last,
because `$cv_id` was being reversed into the list along with the ancestors.

**Rule:** When using `reverse` in a list construction that includes extra elements, always
wrap the `reverse` call in its own parens: `my @result = ((reverse @arr), $extra_item);`

---

### 2026-04-06: JSON null sentinel from JsonValue comes back as object, not Perl undef

`CodingAdventures::JsonSerializer::decode` returns `CodingAdventures::JsonValue::Null`
blessed objects for JSON `null` values — not Perl `undef`. When deserializing stored data
that contains null fields (e.g., `parent_cv_id`, `deleted`, `origin`), the decoded values
are Null sentinel objects. Tests like `is($e->{parent_cv_id}, undef)` fail because the
sentinel is not undef.

**Solution:** Use `CodingAdventures::JsonSerializer::is_null($v)` to detect nulls and
normalize them back to Perl `undef` in any deserialization code:

```perl
sub _to_perl_undef {
    my ($v) = @_;
    return undef if !defined $v;
    return undef if CodingAdventures::JsonSerializer::is_null($v);
    return $v;
}
```

**Rule:** Any Perl module that deserializes JSON and stores the result in internal data
structures must normalize JSON nulls to Perl undef using `is_null()` checks. Never assume
a JSON null field will come back as Perl undef.

---

### 2026-04-05: Always verify all agent-written files are staged before committing

When using multiple background agents to write files in parallel, some files may be written after the initial `git add` command. Always run `git status --short` after all agents complete and before committing to catch untracked or unstaged files. In this case, Rust's `src/lib.rs`, Ruby/Elixir/Lua/Perl test updates, and workspace Cargo.toml changes were missed.

**Rule:** After collecting agent results, run `git diff --name-only` and `git status --short` to verify ALL changes are staged. Don't trust a single `git add` command to catch everything when agents run concurrently.

---

### 2026-04-05: Hand-written parsers need manual token type updates

The Perl python-parser is hand-written (not grammar-driven). It checks `$type eq 'NUMBER'` directly. When the lexer grammar changed from `NUMBER` to `INT`, the grammar-driven parsers (Go, TypeScript, Lua, Ruby) picked up the fix from `python.grammar`, but the Perl parser didn't — it never reads that file.

**Rule:** When changing token names, check for BOTH grammar-driven parsers (which load `python.grammar`) AND hand-written parsers (which have hardcoded type checks). Grep for the old token name across ALL parser packages.

---

### 2026-04-05: Changing lexer token names breaks downstream parsers

When updating the python-lexer to use versioned grammar files (python3.12.tokens), the token name for integers changed from `NUMBER` to `INT`. This broke the python-parser, which still loaded the old `python.grammar` containing `factor = NUMBER | ...`. The parser received `INT` tokens but had no grammar rule matching `INT`.

**Rule:** When changing token names in a lexer grammar, always check all downstream parsers that consume those tokens. Either update the parser grammar simultaneously, or make the grammar accept both old and new token names during the transition period (`factor = INT | FLOAT | NUMBER | ...`).

---

### 2026-04-05: Lua regex does not support \v and \f escapes in character classes

The ECMAScript .tokens grammar files use `/[ \t\r\n\v\f]+/` for whitespace skip patterns. In Lua's regex engine, `\v` and `\f` are not recognized escape sequences inside character classes. They are interpreted as literal `v` and `f`, causing the whitespace skip to match the letters `v` and `f` in source code, silently consuming characters from keywords like `var` and `function`.

**Symptom:** `var` tokenizes as NAME with value `ar` (the `v` is consumed by the skip pattern).

**Solution:** In Lua lexer wrappers that load .tokens files with `\v` or `\f` in patterns, replace these escapes with actual control characters before parsing:

```lua
content = content:gsub("\\v", "\x0B")
content = content:gsub("\\f", "\x0C")
```

**Rule:** Every Lua lexer package that loads a .tokens grammar with `\v` or `\f` in skip patterns must sanitize the content before calling `grammar_tools.parse_token_grammar`.

---

### 2026-04-05: Swift GrammarLexer emits generic "KEYWORD" type for all keywords

The Swift `GrammarLexer` emits tokens with type `"KEYWORD"` for all keywords, with the actual keyword text in the `value` field. This differs from the Lua GrammarLexer which uses `type_name` set to the uppercased keyword (e.g., `"VAR"`, `"IF"`).

**Solution:** Swift lexer wrappers must post-process the token stream to promote `KEYWORD` tokens:

```swift
return raw.map { token in
    if token.type == "KEYWORD" {
        return Token(type: token.value.uppercased(), value: token.value, ...)
    }
    return token
}
```

**Rule:** All Swift language lexer packages need keyword promotion in their `tokenize()` method.

---

### 2026-04-04: Gradle "build" directory conflicts with BUILD file on case-insensitive filesystems

Gradle's default output directory is `build/`. On macOS and Windows (case-insensitive filesystems), this collides with our `BUILD` file — Gradle sees `BUILD` as a file where it expects to create a `build/` directory, causing `IllegalArgumentException: Could not create problems-report directory`.

**Solution:** In every `build.gradle.kts` for Java/Kotlin packages in this monorepo, add this line BEFORE the plugins block:

```kotlin
layout.buildDirectory = file("gradle-build")
```

This redirects Gradle's output to `gradle-build/` instead of `build/`. Also add `gradle-build` to the skip dirs in all build tool implementations so the build tool doesn't recurse into Gradle output directories.

**Checklist for every new Java/Kotlin package:**
- [ ] `layout.buildDirectory = file("gradle-build")` in build.gradle.kts
- [ ] BUILD file exists for the monorepo build tool
- [ ] `gradle-build/` in .gitignore
- [ ] Do NOT use `java { toolchain { languageVersion.set(...) } }` — let Gradle use the running JDK

---

### 2026-04-04: Java toolchain block causes CI failure when JDK is not pre-installed

Using `java { toolchain { languageVersion.set(JavaLanguageVersion.of(21)) } }` in `build.gradle.kts` causes Gradle to search for a JDK 21 installation matching that exact version. If the CI runner doesn't have JDK 21 pre-installed and toolchain auto-provisioning isn't configured, the build fails with "Cannot find a Java installation on your machine."

**Solution:** Do NOT specify an explicit Java toolchain version. Let Gradle use whatever JDK is on the PATH (set up by `actions/setup-java` in CI). This matches the lesson from the hello-world programs.

---

### 2026-04-04: CI detect outputs must use steps.toolchains, not steps.detect

The CI workflow has a "Normalize toolchain requirements" step (id: `toolchains`) that sits between the detect step and the job outputs. On main branch pushes, it forces all languages to `true` for the full rebuild. On other branches, it passes through the detect outputs.

When adding a new language to CI, you must add it in THREE places:
1. `allLanguages` in the build tool (`main.go`)
2. The detect job `outputs:` section (using `steps.toolchains.outputs.needs_<lang>`)
3. The `steps.toolchains` normalization step — in BOTH the `is_main=true` branch AND the `else` branch

If you only add it to `outputs:` using `steps.detect.outputs` instead of `steps.toolchains.outputs`, the validator will fail with: "detect outputs for forced main full builds are not normalized through steps.toolchains."

---

### 2026-04-04: elixir_make chicken-and-egg: do not use `:make` compiler in mix.exs when BUILD builds the NIF externally

When `mix.exs` lists `compilers: Mix.compilers() ++ [:make]`, Mix tries to
load `Mix.Tasks.Compile.Make` at startup — before `elixir_make` has been compiled
from deps. This causes `** (Mix) The task "compile.make" could not be found` on
every `mix` command in CI (including `mix deps.get`), making the BUILD fail.

The error appears even though the subsequent `mix compile` may ultimately succeed
(after auto-compiling elixir_make), because Mix exits non-zero from the first command.

**Rule:** If the BUILD file already calls `cargo build --release` and copies the
`.so` into `priv/`, do NOT also use `elixir_make` in `mix.exs`. Remove the
`:make` compiler, `make_targets`, `make_clean`, `make_cwd`, and the
`{:elixir_make, "~> 0.7", runtime: false}` dep. Use plain `Mix.compilers()`.

---

### 2026-04-01: kern Format 0 coverage — format is in HIGH byte (bits 8-15)

The `coverage` field of a kern subtable header is a 16-bit value. Bits 0-7
contain directional flags (bit 0 = horizontal). **Bits 8-15 contain the
subtable format number.** To check for Format 0 (sorted pairs): `coverage >> 8 == 0`.
Using `coverage & 0xFF == 0` (the low byte) checks the flags, not the format,
and will skip all valid Format 0 subtables since the horizontal flag (bit 0)
is usually set (making the low byte == 1, not 0).

**Rule:** Always extract format as `coverage >> 8` when parsing kern subtable headers.

---

### 2026-04-01: Elixir ranges — always use explicit step //1 when count may be zero

In Elixir, `0..(n - 1)` when `n = 0` creates the range `0..-1` which
**defaults to step -1** and iterates `[0, -1]`. This causes out-of-bounds
`binary_part/3` calls (negative offsets) → `ArgumentError` → `ParseError` in
the error handler, masking the real issue.

**Rule:** All ranges over font table entries must use `//1`: `0..(n - 1)//1`.
An ascending range `0..-1//1` is correctly empty (zero iterations).

---

### 2026-04-01: Swift XCTestCase shadows module-level `load` function

`XCTestCase` (via `NSObject`) has a static `load()` class method. Inside a
test class that subclasses `XCTestCase`, calling `load(data)` resolves to
the inherited class method, not the `FontParser.load(_:)` module function.
The error is "static member 'load' cannot be used on instance of type '...'".

**Rule:** In Swift test files that use `FontParser.load`, always qualify the
call as `FontParser.load(...)` to bypass the `XCTestCase` shadow.

---

### 2026-04-01: Swift .build/ directory must be gitignored before first test run

`swift build` / `swift test` creates a `.build/` directory with thousands of
binary files. If you commit before adding a `.gitignore`, these get staged.
Add `.gitignore` containing `.build/` **before** running any Swift build commands.

**Rule:** For every new Swift package, create `.gitignore` with `.build/` as
the very first file — before `swift test`.

---

### 2026-04-01: OpenType synthetic font builder — head table needs 54 bytes exactly

The `head` table in a minimal synthetic OpenType font has this exact layout (54 bytes):
- 4×u32 (version, fontRevision, checkSumAdjust, magicNumber) = 16 bytes
- flags u16 + unitsPerEm u16 = 4 bytes
- created i64 + modified i64 = 16 bytes
- xMin i16 + yMin i16 + xMax i16 + yMax i16 = 8 bytes
- macStyle u16 + lowestRecPPEM u16 + fontDirectionHint i16 + indexToLocFormat i16 + glyphDataFormat i16 = 10 bytes

Missing the xMin/yMin/xMax/yMax fields (8 bytes) makes the table only 46 bytes,
causing all subsequent table offsets to be wrong, leading to `ParseError` when
loading the synthetic font.

**Rule:** When building a synthetic OpenType font in tests, verify total table
sizes match the declared lengths in the directory. Assert `buf.count == expected_size`
at the end of the builder if your language supports it.

---

### 2026-03-18: Cannot create a PR when remote has no main branch

When working with a completely empty GitHub repo, you can't create a PR because there's no base branch. The `gh pr create` command fails with "no history in common." Solution: push an initial commit to main first (even an empty one), then create PRs from feature branches. For the very first content, merging directly to main is acceptable.

---

### 2026-03-18: Ruby 3.4 build requires libyaml on macOS

Building Ruby 3.4 from source via mise fails with `psych` extension error if `libyaml` is not installed. The `psych` extension (YAML parser) is required by Bundler. Solution: `brew install libyaml` before `mise install ruby@3.4`. Also pass `RUBY_CONFIGURE_OPTS="--with-libyaml-dir=/opt/homebrew"` on Apple Silicon Macs.

---

### 2026-03-18: mise ruby.compile=false still compiles from source

Setting `mise settings ruby.compile=false` did not use precompiled binaries as of mise 2026.3. The precompiled binary feature is noted as "coming in 2026.8.0." For now, always install build dependencies (libyaml, openssl) before installing Ruby via mise.

---

### 2026-03-18: Always add BUILD files and verify discovery for new packages

When creating a new package, you MUST:
1. Create a `BUILD` file in the package directory with the test command
2. Verify the build tool discovers the new package

At the time of the original mistake, some tooling still relied on `DIRS` routing. The current build system uses recursive `BUILD` discovery instead, so the important invariant now is that every package has a valid `BUILD` file and shows up in a dry run. This was missed for fp-arithmetic, Go logic-gates, Ruby sequential logic, and clock packages — they passed locally but were invisible to CI.

**Checklist for every new package:**
- [ ] BUILD file with test command
- [ ] `./build-tool -dry-run` shows the package

**IMPORTANT — multi-language packages:** When implementing the same package across all 5 languages (Python, TypeScript, Ruby, Go, Rust), you MUST add a BUILD file for EVERY language variant. On 2026-03-19 this mistake recurred: Go and Rust compute-unit packages were missing BUILD files while Python, TypeScript, and Ruby had them. The build tool only detected 3 out of 5 packages. After finishing all language implementations, always verify the count matches:
```
find code/packages/*/package-name -name BUILD | wc -l   # should equal number of languages
```

---

### 2026-03-19: TypeScript package.json main must point to src/index.ts for Vitest

When TypeScript packages depend on each other via `"file:../other-pkg"` references, Vitest resolves the dependency using the `main` field in `package.json`. If `main` points to `dist/index.js` (the compiled output), resolution fails because we don't compile before testing — Vitest transforms TypeScript on the fly.

**Solution:** Set `"main": "src/index.ts"` in every TypeScript package's `package.json`. This lets Vitest resolve and transform the TypeScript source directly. Do NOT use `"main": "dist/index.js"` unless you have a pre-build step.

**Checklist for every new TypeScript package:**
- [ ] `"main": "src/index.ts"` (not `dist/index.js`)
- [ ] `"type": "module"` for ESM
- [ ] `file:../` dependencies for internal packages
- [ ] `"@vitest/coverage-v8": "^3.0.0"` in devDependencies (missed on 5 packages in the S-series work — display, interrupt-handler, rom-bios, bootloader, os-kernel, system-board)
- [ ] BUILD file uses `npm install --silent` (not `npm ci`) unless package-lock.json is committed and in sync
- [ ] Run the real coverage gate locally with `npx vitest run --coverage` (or the package `BUILD` file) for every changed TypeScript package, not just `tsc` or plain `vitest run`

---

### 2026-03-19: JavaScript 32-bit integer gotchas in CPU simulation

JavaScript bitwise operators (`&`, `|`, `<<`, `>>`) work on **signed 32-bit integers**. This causes two issues when porting CPU simulation code:

1. **`(1 << 32)` wraps to `1`** — bit shifts are modulo 32, so `1 << 32 === 1` instead of `2^32`. Use a conditional: `bitWidth >= 32 ? 0xFFFFFFFF : (1 << bitWidth) - 1`.

2. **`0xFFFFFFFF & 0xFFFFFFFF` yields `-1`** — the `&` operator returns a signed int, so the all-ones pattern is interpreted as `-1`. Use `>>> 0` to convert to unsigned: `(value & mask) >>> 0`.

These are critical when implementing register files, ALU operations, and memory addressing.

---

### 2026-03-19: Rust workspace Cargo.toml must match pushed packages

When adding new Rust packages, the workspace `Cargo.toml` lists all members. If a member is listed but its directory hasn't been pushed to the remote yet, ALL Rust packages in the workspace fail to compile in CI with "failed to load manifest" errors.

**Solution:** Only add packages to the workspace `members` list in the same commit where the package directory is pushed. Or push all new packages together in one commit.

---

### 2026-03-19: Always update PR description after each push

When working on a large PR with many commits, update the PR description after each push to reflect current progress. This lets the reviewer (and CI) see what's been done and what's left. Use `gh pr edit <number> --body "..."` to update the description programmatically.

---

### 2026-03-19: TypeScript file: deps need transitive installs on CI

When TypeScript package A depends on B (`"file:../B"`) and B depends on C (`"file:../C"`), running `npm ci` in A installs B but does NOT install C inside B's own `node_modules`. On a fresh CI runner (no pre-existing `node_modules`), this causes `ERR_MODULE_NOT_FOUND` at runtime when B tries to import C.

**Solution:** The BUILD file must chain installs from the bottom of the dependency tree upward:
```
cd ../C && npm ci --quiet && cd ../B && npm ci --quiet && cd ../A && npm ci && npx vitest run
```

**Why this only fails on CI:** Locally, if you've ever run `npm install` in package C, its `node_modules` already exists. The `file:` reference from B resolves because C's deps are already present. On CI, every directory starts clean — no `node_modules` anywhere.

**Checklist for TypeScript BUILD files:**
- [ ] Identify the full transitive `file:` dependency chain
- [ ] Install from leaves to root in the BUILD script
- [ ] Test from a clean state: `rm -rf node_modules ../dep/node_modules && bash BUILD`

---

### 2026-03-21: BUILD files must install ALL transitive dependencies explicitly

When creating packages that depend on sibling packages, the BUILD file must install every transitive dependency — not just direct ones. CI runs each package in isolation with a clean environment. If package A depends on B which depends on C, A's BUILD file must install C too.

This caused repeated CI failures across Python, Ruby, and Go during the S-series system software work. The same mistake was made 3 times before being fully resolved.

**Python:** `uv pip install` resolves deps from PyPI. If a sibling package (e.g., `state-machine`) isn't on PyPI, it must be installed with `-e ../state-machine` BEFORE any package that depends on it. Install order matters — install leaves first:
```
uv pip install -e ../directed-graph -e ../state-machine -e ../branch-predictor -e ../core -e ".[dev]"
```

**Ruby:** Gemfiles must list ALL transitive path dependencies. If `riscv_simulator` depends on `cpu_simulator`, then any package depending on `riscv_simulator` must also have:
```ruby
gem "coding_adventures_cpu_simulator", path: "../cpu_simulator"
gem "coding_adventures_riscv_simulator", path: "../riscv_simulator"
```

**Go:** When a transitive dependency adds a new module (e.g., `state-machine`), ALL packages up the chain need `go mod tidy` or manual additions to `go.mod`. A single missing entry in go.sum breaks the build. After adding a new Go package, run `go mod tidy` in EVERY package that transitively depends on it.

**TypeScript:** `npm ci` requires lock files in sync. After adding new dependencies to `package.json`, either regenerate `package-lock.json` with `npm install`, or use `npm install` instead of `npm ci` in BUILD files.

**Checklist for every new package with dependencies:**
- [ ] List ALL transitive deps in BUILD file (not just direct)
- [ ] Install deps in leaf-to-root order
- [ ] Test from a completely clean state (no cached installs)
- [ ] For Go: run `go mod tidy` in all dependent packages
- [ ] For TypeScript: regenerate lock files or use `npm install`

---

### 2026-03-21: Elixir reserved words cannot be used as variable names

Elixir reserves words like `after`, `rescue`, `catch`, `else` that cannot be used as variable names. When porting code from other languages, rename these variables (e.g., `after` → `rest`, `after_bytes`). This caused a compilation error in the Core memory_controller.ex that required two rounds of fixes because the first fix only caught one occurrence.

**Rule:** When porting to Elixir, grep for reserved words used as variables: `after`, `rescue`, `catch`, `else`, `end`, `fn`, `do`, `when`, `cond`, `try`, `receive`.

---

### 2026-03-21: Ruby predicate methods use `?` suffix — don't port method names literally

Ruby convention: methods that return a boolean end with `?` (e.g., `contains?`, `empty?`, `valid?`). When porting from Go/Python/TypeScript where the method is `contains()` or `is_empty()`, Ruby code must use `contains?()`. Tests calling `snap.contains("text")` instead of `snap.contains?("text")` will fail with `NoMethodError`.

**Rule:** When writing Ruby tests that call boolean methods, always add `?`. Grep test files for common predicates: `contains`, `empty`, `valid`, `halted`, `idle` — they all need `?` in Ruby.

---

### 2026-03-21: Python Enum rejects invalid values — don't construct with arbitrary integers

Python's `enum.Enum` raises `ValueError` if you call `MyEnum(99)` and 99 isn't a defined member. This differs from Go (where enums are just ints) and TypeScript (where enums allow any number). Tests that check behavior for "invalid enum values" by constructing `BootPhase(99)` will fail.

**Rule:** When testing "not found" or "invalid" enum cases in Python, use `None` or a sentinel value — don't construct the Enum with an invalid int. Or use `IntEnum` if arbitrary ints should be allowed.

---

### 2026-03-21: Ruby `include` inside a method body doesn't work as expected

In Ruby, `include SomeModule` is a class-level operation that adds the module's constants/methods to the current class. Calling `include` inside a test method (instance method) calls `Kernel#include` which doesn't exist as an instance method — it raises `NoMethodError`.

**Rule:** Either `include` the module at the class level (inside the test class but outside any method), or use fully qualified constant names like `CodingAdventures::SystemBoard::PHASE_NAMES`.

---

### 2026-03-21: Rust cpu-simulator must export ALL types other crates import

When creating a Rust crate that replaces or extends an existing one (e.g., cpu-simulator), check ALL downstream crates that import from it. The arm-simulator crate imported `CPU`, `DecodeResult`, `ExecuteResult`, `InstructionDecoder`, `InstructionExecutor`, `PipelineTrace` — but our fresh cpu-simulator only exported `Memory`, `RegisterFile`, and `SparseMemory`. This broke the entire Rust workspace in CI.

**Rule:** After creating or modifying a Rust crate, run `cargo build --workspace` to catch any missing exports. Don't just test the individual crate — test the whole workspace.

---

### 2026-03-21: Ruby require ordering matters for constant resolution

Ruby loads files in the order they are `require`d. If `system_board/config.rb` references `RomBios::BIOSConfig`, the `coding_adventures_rom_bios` gem must be required BEFORE the config file loads. This means the main entry point file (`coding_adventures_system_board.rb`) must `require "coding_adventures_rom_bios"` before requiring its own modules.

**Rule:** When a Ruby package depends on another, add the `require` for the dependency at the TOP of the entry point file, before any `require_relative` calls to the package's own modules.

---

### 2026-03-21: TypeScript BUILD files must chain-install transitive file: deps

TypeScript packages using `"file:../sibling"` dependencies need their transitive deps installed first. CI starts with a clean `node_modules`. The BUILD file must `cd` into each transitive dep and run `npm install` in leaf-to-root order before running the package's own tests.

This was already documented (2026-03-19) but continues to recur because new packages are added without following the pattern. The fix is mechanical — check the dependency chain and install from leaves to root.

---

### 2026-03-20: Use mise for all language runtimes — nothing is installed globally

This machine does not have many tools installed globally. Language runtimes (Ruby, Go, Rust, etc.) are managed by **mise** (configured in `mise.toml` at the repo root). The system Ruby is 2.6.10, but the project requires 3.4+.

**Problem:** Ruby BUILD files used bare `bundle` commands, which invoked system Ruby 2.6.10. All 39 Ruby packages failed to build.

**Fix:** Prefix language-specific commands with the **absolute path** to mise in BUILD files:
```
/Users/adhithya/.local/bin/mise exec -- bundle install --quiet
/Users/adhithya/.local/bin/mise exec -- bundle exec rake test
```

**Why absolute path?** The build tool runs BUILD commands via `sh -c`, which gets a minimal PATH that does NOT include `~/.local/bin` where mise is installed. Using just `mise exec` fails with `sh: mise: command not found`.

**Rule:** Always use `/Users/adhithya/.local/bin/mise exec --` (absolute path) when invoking language-specific tools in BUILD files. Never assume mise or any tool is on PATH in `sh -c` contexts. Check `mise.toml` for managed runtimes. This applies to `ruby`, `bundle`, `gem`, `go`, `cargo`, `rustc`, and any other runtime-managed tool.

---

### 2026-03-22: Rust workspace Cargo.toml must include ALL crates (except self-workspace crates)

When adding new Rust crates to the workspace, the `Cargo.toml` `members` list must include every crate in the directory that doesn't declare its own `[workspace]`. Crates like `node-bridge`, `python-bridge`, and `ruby-bridge` have their own `[workspace]` declarations (for FFI builds) and must be excluded — including them causes "multiple workspace roots" errors.

**Problem:** The workspace `Cargo.toml` was missing ~30 crates (grammar-tools, lexer, javascript-lexer, ruby-lexer, etc.). CI failed with "current package believes it's in a workspace when it's not" for every affected crate.

**Rule:** After adding new Rust crates, regenerate the members list from disk:
```bash
for d in code/packages/rust/*/; do
  if [ -f "$d/Cargo.toml" ] && ! grep -q '^\[workspace\]' "$d/Cargo.toml"; then
    basename "$d"
  fi
done | sort
```
This lists every crate that should be a workspace member.

---

### 2026-03-22: Always merge origin/main before reasoning about CI failures

When a PR branch is behind `origin/main`, CI merges them before building. Local reasoning about "what crates exist" will be wrong if main has added crates that aren't in your worktree. Always `git fetch origin main && git merge origin/main` before fixing CI issues.

---

### 2026-03-21: Rust coverage requires cargo-tarpaulin — always measure and report

Unlike Python (`pytest-cov`), Go (`go test -cover`), Ruby (`simplecov`), TypeScript (`@vitest/coverage-v8`), and Elixir (`mix test --cover`), Rust does NOT include a built-in coverage tool. You must use `cargo-tarpaulin`.

**Install:** `cargo install cargo-tarpaulin` (already installed on this machine).

**Measure coverage for a single package:**
```
cargo tarpaulin -p <package-name> --out stdout
```

Note: tarpaulin reports coverage for the ENTIRE workspace by default. To get package-specific numbers, look at the per-file breakdown in the output and sum only the lines for your package's `src/` files.

**Rule:** Every Rust package PR must include coverage numbers. Run `cargo tarpaulin -p <name> --out stdout`, sum the covered/total lines for that package's source files, and report the percentage. Don't leave coverage as "n/a" or "all passing" — compute the real number.

---

### 2026-03-21: BUILD files must NOT use absolute mise paths — CI doesn't have mise

**Update to the above rule:** The mise absolute path rule applies ONLY when running the build tool locally. In GitHub Actions CI, mise is NOT installed at `/Users/adhithya/.local/bin/mise` — the CI runner installs language runtimes via `actions/setup-go`, `actions/setup-ruby`, etc. BUILD files that hardcode the local mise path fail in CI with `sh: 1: /Users/adhithya/.local/bin/mise: not found`.

**Resolution:** BUILD files should use bare commands (`go test`, `bundle exec`, etc.) without any mise prefix. The CI environment already has the correct language runtimes on PATH. This affected 9 packages (4 Go, 5 Ruby) in the D12-D17 OS abstractions work.

**Rule:** Do NOT use `/Users/adhithya/.local/bin/mise exec --` in BUILD files. Use bare commands: `go test ./... -v -cover`, `bundle install --quiet && bundle exec rake test`, etc. CI sets up its own runtimes.


---

### 2026-03-22: Skip patterns consume newlines — update downstream tests

When the lexer's skip pattern evaluation order changes (skip patterns before newline check), any grammar with `\n` in its skip pattern (e.g., JSON's `WHITESPACE = /[ \t\r\n]+/`) will consume newlines silently instead of emitting NEWLINE tokens. This broke Go and Ruby json-lexer tests that expected NEWLINE tokens.

**Rule:** When modifying the lexer's main loop order (skip vs newline vs token matching), check ALL downstream lexer wrapper packages (json-lexer, css-lexer, toml-lexer, etc.) for tests that depend on NEWLINE token emission. If a grammar's skip pattern includes `\n`, no NEWLINE tokens should be expected.

---

### 2026-03-22: Go BUILD files must run commands from within the package directory

Go modules with their own `go.mod` file cannot be built via parent directory patterns like `cd ../ && go build ./subdir/...`. This fails with "directory prefix does not contain main module or its selected dependencies." Instead, BUILD files should use `go build ./...`, `go test ./...`, `go vet ./...` which run from within the package directory (the build tool already `cd`s into the package directory before executing BUILD commands).

**Rule:** Go BUILD files should always use `./...` patterns, not `cd ../ && ./subdir/...` patterns. Match the existing convention used by `starlark-parser/BUILD`: `go test ./... -v -cover`.

---

### 2026-03-22: Elixir indentation-sensitive parser needs INDENT/DEDENT tokens

When building a parser for Python/Starlark in Elixir, the tokenizer MUST produce INDENT and DEDENT tokens for block boundaries. Without them, `parse_block` cannot determine where a function body or if-body ends. A simple "parse one statement" heuristic fails for multi-statement blocks (e.g., `def factorial(n):` with an if-return and a second return).

**Solution:** Post-process raw tokens to inject INDENT/DEDENT based on indentation levels from the source text. Use an indent stack (like Python's tokenize module). Additionally, `skip_newlines` must NOT skip DEDENT tokens — DEDENT is a block boundary marker. Create a separate `skip_whitespace` helper that skips NEWLINE + INDENT + DEDENT for contexts where indentation is noise (e.g., multiline function call arguments).

---

### 2026-03-22: Elixir GenericVM — function calls need fresh execution context

When calling a Starlark function via `GenericVM.execute`, the VM's pc, stack, call_stack, and halted state carry over from the caller. This causes function calls to fail silently (pc past end of function's code, empty call_stack triggers wrong RETURN behavior).

**Solution:** Save the entire caller state (pc, variables, locals, stack, call_stack, halted), reset them for the function call (pc=0, stack=[], call_stack=[], halted=false), execute the function's CodeObject, extract the return value from the function's stack, then restore all caller state and push the return value.

---

### 2026-03-22: Elixir `if` blocks must capture their result

In Elixir, `if` blocks return a value. If you write:
```elixir
if condition do
  {_idx, compiler} = emit(compiler, ...)
  compiler
end
# Compile body as nested code object
```
The result of the `if` is DISCARDED — `compiler` after the `if` is the OLD value. You must write:
```elixir
compiler = if condition do
  {_idx, comp} = emit(compiler, ...)
  comp
else
  compiler
end
```
This bit us in `handle_def_stmt` where defaults BUILD_TUPLE was never emitted.
### 2026-03-22: TypeScript file: deps require ALL transitive deps listed directly

When a TypeScript package has `file:` deps (e.g., `"@coding-adventures/lexer": "file:../lexer"`), `npm ci` creates symlinks to those packages but does NOT install their `file:` dependencies' node_modules. If lexer depends on `state-machine` via `file:../state-machine`, your package must ALSO list `state-machine` as a direct dependency.

Additionally, do NOT use `cd ../dep && npm ci` chain patterns in BUILD files — the build tool runs packages in parallel, and two packages running `npm ci` on the same shared dependency simultaneously causes esbuild install conflicts. Instead, use simple `npm ci --quiet` + `npx vitest run` patterns (matching starlark-lexer/starlark-parser BUILD convention) and list all transitive `file:` deps directly in package.json.

**Rule:** TypeScript BUILD files should be `npm ci --quiet\nnpx vitest run --coverage`. All transitive `file:` deps must be listed as direct deps in package.json.

---

### 2026-03-22: TypeScript JSDoc comments must not contain unescaped glob patterns

When writing JSDoc/TSDoc comments in TypeScript, never include raw glob patterns like `src/**/*.py` in `@example` blocks. The `**` sequence confuses esbuild's parser which treats the `*` after `*/` as a syntax error inside the comment block. This caused the TypeScript build-tool CI to fail with "Unexpected `*`" on the starlark-evaluator.ts JSDoc example.

**Rule:** In TSDoc `@example` blocks, either:
- Omit glob patterns entirely
- Use escaped/simplified patterns (e.g., `"src/*.py"` instead of `"src/**/*.py"`)
- Use backtick code fences within the comment

---

### 2026-03-22: Adding new source files without tests drops coverage below threshold

When adding a new source module to an existing package (e.g., `starlark_evaluator.py` to the Python build-tool), coverage drops because the new code has 0% coverage. The Python build-tool has `fail_under=80` in its pytest-cov config. Adding ~150 lines of untested code dropped coverage from 83% to 77%.

**Rule:** Every new source file must have a corresponding test file in the same commit. Plan tests alongside implementation, not as an afterthought.

---

### 2026-03-22: Elixir Starlark compiler may infinite-loop on certain inputs

The Elixir Starlark AST-to-bytecode compiler can get stuck in an infinite loop in `skip_newlines/1` when processing certain Starlark source patterns (particularly inline target declarations). Integration tests that call `evaluate_build_file` through the full interpreter pipeline should use `@tag timeout: :infinity` or be skipped entirely to avoid blocking CI for 60+ seconds.

**Rule:** When adding integration tests that exercise the full Starlark interpreter pipeline in Elixir, either skip them or set generous timeouts. Unit tests for detection, command generation, and extraction helpers should not need the interpreter.

### 2026-03-22: Pin uv version in CI to avoid missing platform binaries

The `astral-sh/setup-uv@v4` action with `version: latest` resolved to uv `0.10.12`, which was missing the `aarch64-apple-darwin` binary (404 error). This caused all macOS CI runs to fail on the "Install uv" step. The fix is to pin to a known stable version series (e.g., `version: "0.6.x"`) rather than relying on `latest`.

**Rule:** Always pin tool versions in CI actions. `latest` can resolve to broken or incomplete releases. Use version ranges like `"0.6.x"` that stay within a known-good series.

---

### 2026-03-22: Always use the scaffold generator for new packages

When creating new packages, ALWAYS use the scaffold generator (`code/programs/go/scaffold-generator/`). It:
1. Computes the full transitive dependency closure
2. Orders installs in leaf-to-root order in the BUILD file
3. Uses consistent naming conventions per language
4. Creates all required files (BUILD, README.md, CHANGELOG.md, package metadata, tests)

**Problem:** 22 Verilog/VHDL wrapper packages were hand-written by agents without using the scaffold generator. This led to:
- Missing transitive dependencies in TypeScript BUILD files
- Missing README.md and CHANGELOG.md files
- Inconsistent BUILD file format across packages

**Rule:** `scaffold-generator PACKAGE_NAME --language LANG --depends-on DEP1,DEP2` before writing any package code.

---

### 2026-03-22: Python BUILD files — do NOT quote .[dev] extras

Newer versions of uv reject `".[dev]"` (quoted extras syntax) with "Quoted extras are not permitted." Use unquoted `.[dev]` instead.

**Bad:**  `uv pip install -e ".[dev]" --quiet`
**Good:** `uv pip install -e .[dev] --quiet`

The scaffold generator has been updated to use the unquoted form. Existing packages using the quoted form may work on some uv versions but will break when uv is upgraded.

---

### 2026-03-22: Changing ALL BUILD files triggers full rebuild — avoid mass changes

The build-tool uses diff-based change detection. Touching a BUILD file marks that package (and its dependents) for rebuild. Changing ALL 82 Python BUILD files in one commit forces a full rebuild of every Python package, exposing pre-existing broken BUILD files that were previously untested.

**Rule:** When making a global change to BUILD files (like fixing a syntax issue), only change the files that are actually broken. Don't apply the fix to files that are working — they'll be fixed next time the scaffold generator is used.
### 2026-03-23: Never merge PRs without CI pass and explicit user sign-off

Three PRs (#201, #203, #207) were squash-merged immediately after creation without waiting for CI to run or getting user approval. While the fixes happened to work, merging without CI verification risks shipping broken code to main.

**Rule:** After creating a PR, wait for the full CI pipeline to pass. Report CI results to the user and only merge after receiving explicit approval. Even if a change looks trivial, CI catches regressions that are easy to miss — especially in a monorepo where changes can have far-reaching effects across packages and workflows.
### 2026-03-22: Unix-specific tools must compile on Windows CI

Tools that use Unix-specific syscalls (syscall.Stat_t, libc::getgroups, libc::statvfs, etc.) will fail to compile on the Windows CI runner. This affected chown, df, ls, groups, id, uname, and tty tools across Go and Rust.

**Go solution:** Use build tags to split platform-specific code:
- `tool_unix.go` with `//go:build !windows` — contains the real implementation
- `tool_windows.go` with `//go:build windows` — contains stubs that return errors/defaults
- The main `tool.go` file calls platform-abstracted helper functions

**Rust solution:** Use `#[cfg(unix)]` and `#[cfg(not(unix))]` conditional compilation:
```rust
#[cfg(unix)]
pub fn get_user_info() -> Result<UserInfo, String> { /* real impl */ }

#[cfg(not(unix))]
pub fn get_user_info() -> Result<UserInfo, String> {
    Err("id: not supported on this platform".to_string())
}
```

**Elixir/Python solution:** Provide `BUILD_windows` files that avoid Unix shell syntax (`2>/dev/null`) and handle dependency paths correctly.

**Rule:** When writing tools that use OS-specific APIs, always add platform guards from the start. Don't wait for CI failures. Check: `syscall.Stat_t`, `syscall.Statfs`, `os.Chown`, `libc::getuid`, `libc::statvfs`, etc.

---

### 2026-03-23: uv pip install cannot resolve local editable deps on Windows

When a Python package's `pyproject.toml` declares a dependency on another local package (e.g., `dependencies = ["coding-adventures-json-parser"]`), `uv pip install -e ../json-parser -e ".[dev]"` works on Linux/macOS but **fails on Windows** with "not found in the package registry." This is because uv on Windows doesn't resolve editable package names from the same command line during dependency resolution.

**Workaround:** Remove unpublished local package dependencies from `pyproject.toml` (since BUILD files handle all transitive deps anyway). Document them in comments for future PyPI publishing. This makes `uv pip install -e ".[dev]"` succeed because there are no deps to resolve from PyPI.

**Also learned:**
- `".[dev]"` with double quotes works in `sh -c` (bash strips quotes) but fails in `cmd /C` (cmd.exe passes quotes literally to uv, causing "Failed to parse" error)
- `.venv/bin/python` doesn't exist on Windows — use `uv run python` for cross-platform compatibility
- Always provide `BUILD_windows` files for Python packages that have shell-specific syntax
- The build tool's `GetBuildFileForPlatform` correctly selects `BUILD_windows` on Windows via `runtime.GOOS`

---

### 2026-03-23: .gitattributes eol=lf fixes Elixir heredoc CRLF test failures

Elixir heredoc strings in test files (triple-quoted `"""`) embed literal line endings. When git's `autocrlf` converts `\n` to `\r\n` on Windows checkout, the heredoc strings contain `\r\n` but the serializer produces `\n`, causing test assertion failures.

**Fix:** Add `.gitattributes` with `* text=auto eol=lf` to force LF line endings on all platforms. This prevents CRLF-related test failures across all languages (Elixir, Ruby, Python doctests, etc.).

---

### 2026-03-23: CPython type slot numbers must match typeslots.h exactly

When building native Python extensions via python-bridge, the slot numbers passed to `PyType_FromSpec` must exactly match CPython's `Include/typeslots.h`. Wrong slot numbers cause silent memory corruption during module loading -- typically manifesting as `UnicodeDecodeError` or access violation crashes.

**Correct slot numbers (from CPython typeslots.h):**
- `Py_sq_contains` = 41, `Py_sq_length` = 45
- `Py_tp_dealloc` = 52, `Py_tp_methods` = 64, `Py_tp_new` = 65, `Py_tp_repr` = 66
- `Py_tp_hash` = 59 (NOT 56), `Py_tp_iter` = 62 (NOT 58), `Py_tp_richcompare` = 67
- `Py_nb_and` = 8 (NOT 1), `Py_nb_invert` = 27 (NOT 5), `Py_nb_or` = 31 (NOT 12), `Py_nb_xor` = 38 (NOT 17)

**Rule:** Always verify slot numbers against the CPython source (`Include/typeslots.h`) or the official docs. Do not guess from memory. The numbers in `typeslots.h` are NOT sequential per category -- they are assigned globally across all protocols.

### 2026-03-23: Always use the scaffold generator to create new packages

Hand-crafting multi-language packages causes 12+ recurring CI failure categories: missing BUILD files, TypeScript `main` pointing to `dist/` instead of `src/`, missing transitive dependencies, Ruby require ordering issues, Rust workspace `Cargo.toml` not updated, missing README or CHANGELOG, etc.

**Rule:** Always build and use `code/programs/go/scaffold-generator/` before writing any implementation code for a new package.

```bash
cd code/programs/go/scaffold-generator
go build -o scaffold-generator .
./scaffold-generator <package-name> --language all --depends-on <dep1,dep2> --description "..."
```

The scaffold generator handles: correct directory naming per language (Ruby/Elixir use underscores), BUILD files for all platforms, `go.mod` with `replace` directives, TypeScript `package.json` with `src/index.ts` as main, Rust workspace membership, and transitive dependency resolution.

---

### 2026-03-23: Comprehensive BUILD file rules — the definitive guide

After 6+ rounds of CI failures on PR #165 (infrastructure fixes for Verilog/VHDL), every BUILD file pitfall has been encountered. This section consolidates ALL BUILD file rules into one place.

#### Rule 1: Each BUILD line runs as a SEPARATE shell process

The build tool (`code/programs/go/build-tool/`) executes each line of a BUILD file as a separate `sh -c` (Unix) or `cmd /C` (Windows) invocation. **`cd` does NOT persist between lines.** The working directory resets to the package directory before each line.

**Wrong:**
```
cd ../directed-graph && npm ci --quiet
cd ../state-machine && npm ci --quiet     ← starts in PACKAGE dir, not directed-graph
cd ../cli-builder && npm ci --quiet       ← starts in PACKAGE dir, not state-machine
```

**Right — chain on one line:**
```
cd ../directed-graph && npm ci --quiet && cd ../state-machine && npm ci --quiet && cd ../cli-builder && npm ci --quiet
npm ci --quiet
npx vitest run --coverage
```

Shell variables do not persist either. If one line does `PYTHON_BIN=.venv/bin/python`,
the next line sees an empty variable unless the assignment and its use are in the
same line.

**Or use full paths from the package dir on each line:**
```
cd ../../../packages/typescript/directed-graph && npm ci --quiet
cd ../../../packages/typescript/state-machine && npm ci --quiet
cd ../../../packages/typescript/cli-builder && npm ci --quiet
npm ci --quiet
npx vitest run --coverage
```

#### Rule 2: TypeScript — `npm ci` resolves `file:` deps transitively

For TypeScript **packages** (under `code/packages/typescript/`), a simple `npm ci --quiet` resolves all `file:` dependencies transitively via `package-lock.json`. No need for explicit `cd ../dep && npm ci` lines. The package-lock.json already encodes the full dependency tree.

**Preferred BUILD file for TypeScript packages:**
```
npm ci --quiet
npx vitest run --coverage
```

**Exception: TypeScript programs** (under `code/programs/typescript/`) with deep relative paths (`../../../packages/...`) need explicit dep installation because `npm ci` can't always resolve deeply nested `file:` references. Chain dep installs on a single line (see Rule 1).

#### Rule 3: TypeScript — never install sibling deps in parallel from BUILD files

When multiple packages run `cd ../state-machine && npm ci` in parallel (e.g., verilog-lexer and vhdl-lexer building simultaneously), they race to install into `state-machine/node_modules`, causing `ETXTBSY` (esbuild binary busy) or missing `package.json` errors. The build tool handles parallelism — BUILD files should only install their OWN deps.

#### Rule 4: Python — uv workspace discovery causes shared `.venv`

A workspace `pyproject.toml` exists at `code/packages/python/pyproject.toml`. By default, `uv venv` discovers this workspace and creates `.venv` at the workspace root, not in the individual package directory. When packages build in parallel, they all share this `.venv`, causing dependency conflicts.

**Fix:** Use `--no-project` and `--python .venv` flags:
```
uv venv .venv --quiet --no-project
uv pip install --python .venv -e ".[dev]" --quiet
uv run --no-project python -m pytest tests/ -v
```

- `uv venv .venv --no-project` — creates `.venv` in the package dir, ignoring workspace
- `uv pip install --python .venv` — installs into the correct local `.venv`
- `uv run --no-project python` — runs python from the local `.venv`, cross-platform

#### Rule 5: Python — `.venv/bin/python` does not exist on Windows

Windows uses `.venv\Scripts\python`, not `.venv/bin/python`. The cross-platform alternative is `uv run --no-project python` which works on all platforms.

**Never use:** `.venv/bin/python -m pytest tests/ -v`
**Always use:** `uv run --no-project python -m pytest tests/ -v`

#### Rule 6: Python — `".[dev]"` quoting fails on Windows

On Windows, `cmd /C` passes double quotes literally to uv, causing path resolution errors. The `"` character becomes part of the package specifier.

**Fails on Windows:** `uv pip install -e ".[dev]" --quiet`
**Works everywhere:** `uv pip install -e .[dev] --quiet`

Note: Some existing packages still use `".[dev]"` and work because they haven't been rebuilt on Windows yet. When touching any Python BUILD file, fix the quoting.

#### Rule 7: Mass BUILD file changes trigger full rebuild

The build tool uses `git diff --name-only` to detect changed files. Modifying ALL BUILD files in one commit causes the build tool to mark ALL packages for rebuild. This exposes pre-existing broken BUILD files that were previously untested.

**Rule:** When making a global BUILD file fix, only change the files that are actually broken or that your PR touches. Do not proactively fix working BUILD files — they'll be fixed when the scaffold generator is next used.

#### Rule 8: Canonical BUILD file templates

**TypeScript packages:**
```
npm ci --quiet
npx vitest run --coverage
```

**TypeScript programs (with deep deps):**
```
cd ../../../packages/typescript/dep-a && npm ci --quiet && cd ../dep-b && npm ci --quiet && cd ../dep-c && npm ci --quiet
npm ci --quiet
npx vitest run --coverage
```

**Python packages (multi-line):**
```
uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../dep-a -e ../dep-b -e .[dev] --quiet
uv run --no-project python -m pytest tests/ -v
```

**Python packages (single-line, for packages in the uv workspace):**
```
uv venv .venv --quiet --no-project && uv pip install --python .venv -e ../dep-a -e ../dep-b -e ".[dev]" --quiet && uv run --no-project python -m pytest tests/ -v --tb=short
```

**Go packages:**
```
go test ./... -v -cover
go vet ./...
```

**Rust packages:**
```
cargo test -p PACKAGE_NAME
```

**Ruby packages:**
```
bundle install --quiet
bundle exec rake test
```

**Elixir packages:**
```
mix deps.get --quiet
mix test --cover
```

---

### 2026-03-24: Python BUILD files — Windows requires a completely different pattern

**Problem**: Python BUILD files that work on Linux/macOS fail on Windows because:

1. **`.venv/bin/python` doesn't exist on Windows** — Windows uses `.venv\Scripts\python.exe`. Any BUILD file using `.venv/bin/python` will fail on Windows.
2. **`uv pip install -e ".[dev]"` resolves transitive deps from PyPI on Windows** — Even when local editable deps are installed first, uv on Windows sometimes tries to resolve transitive dependencies from the package registry instead of using locally installed packages. This causes `No solution found when resolving dependencies` errors for packages that only exist locally.

**Solution**: Every Python package needs TWO build files:

**BUILD** (Linux/macOS):
```
uv venv --quiet --clear
uv pip install -e ../dep-a -e ../dep-b -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
```

**BUILD_windows** (Windows):
```
uv venv --quiet --clear
uv pip install -e ../dep-a -e ../dep-b --quiet
uv pip install --no-deps -e .[dev] --quiet
uv pip install pytest pytest-cov ruff mypy --quiet
uv run --no-project python -m pytest tests/ -v
```

Key differences in BUILD_windows:
- Install local deps FIRST in a separate command
- Use `--no-deps` when installing the package itself (prevents PyPI resolution)
- Do NOT quote `.[dev]` (newer uv rejects `".[dev]"` on Windows)
- Explicitly install test tools (pytest, etc.) in a separate command
- Use `uv run --no-project python` instead of `.venv/bin/python`

**Rule**: Every new Python package MUST have both BUILD and BUILD_windows files. The scaffold generator must generate both.

---

### 2026-03-24: Changing shared infrastructure packages triggers cascading rebuilds

**Problem**: When you modify a widely-depended-on package (like `grammar-tools` or `lexer`), the build tool's diff detection marks ALL dependent packages for rebuild. If those downstream packages have broken BUILD files (e.g., missing BUILD_windows), the CI fails on packages you didn't intend to touch.

**Impact**: A one-line change to `grammar-tools` can trigger rebuilds of 50+ packages across all languages. If even one of those packages has a broken BUILD file, CI fails.

**Mitigation**:
1. Before modifying shared infrastructure packages, check that ALL downstream packages have correct BUILD and BUILD_windows files
2. Never mass-change BUILD files in a PR that also changes code — the blast radius is too large
3. If CI fails on a package you didn't change, fix its BUILD file in a separate focused commit
4. Use `./build-tool --list-affected --diff-base origin/main` to preview what will be rebuilt before pushing

---

### 2026-03-24: TypeScript shared source restructuring (code/src/)

**Problem**: TypeScript packages were restructured so that the actual source lives in `code/src/typescript/` and package-level files in `code/packages/typescript/*/src/` are re-exports:
```typescript
export * from "../../../../src/typescript/grammar-tools/token-grammar.js";
```

**Impact**: If you modify a TypeScript package's `src/` file directly, you're editing a re-export stub — your changes will be overwritten or cause conflicts on merge with main. The actual implementation lives at `code/src/typescript/<package>/`.

**Rule**: Always check if a TypeScript source file is a re-export before editing. If it is, apply changes to the shared source at `code/src/typescript/` instead.

---

### 2026-03-24: Always use the scaffold generator for new packages

**Problem**: Hand-writing package scaffolding (BUILD, BUILD_windows, pyproject.toml, mix.exs, etc.) is error-prone. Missing or incorrect BUILD files cause CI failures that are hard to diagnose, especially on Windows where the pattern is different from Linux/macOS.

**Solution**: Always use the scaffold generator at `code/programs/*/scaffold-generator/`. It:
- Computes transitive dependency closures automatically
- Generates correct BUILD and BUILD_windows files for all platforms
- Creates properly structured pyproject.toml / package.json / mix.exs / Cargo.toml
- Includes README.md, CHANGELOG.md, and test scaffolding

If the scaffold generator wouldn't produce the right output for your use case, fix the scaffold generator FIRST, then use it. Never hand-write what can be generated.

---

### 2026-03-24: Regex delimiter escaping in .tokens files

**Problem**: The `.tokens` file format uses `/` as regex delimiters (`/pattern/`). If a regex contains `/` inside a `[...]` character class (e.g., `[^/]`), the naive parser treats it as the closing delimiter and truncates the pattern.

**Example**: `BLOCK_COMMENT = /\/\*([^*]|\*[^/])*\*\//` — the `[^/]` contains a `/` that was being misinterpreted as the closing delimiter.

**Workaround before fix**: Escape as `[^\/]` — but this changes the regex semantics.

**Proper fix**: All 6 language implementations of the `.tokens` parser now use bracket-aware scanning that tracks `[...]` depth and doesn't treat `/` inside character classes as the closing delimiter. The scanner also has a fallback to `lastIndexOf("/")` for edge cases like unclosed brackets.

**Rule**: Never escape `/` inside `[...]` in .tokens files. The parser handles it correctly.

---

### 2026-03-23: uv workspace membership causes shared venv on Windows, breaking parallel builds

When a Python package is added to the `[tool.uv.workspace]` members list in `code/packages/python/pyproject.toml`, uv creates the virtual environment at the **workspace root** (`code/packages/python/.venv`) instead of the package directory. This causes two problems on Windows:

**Problem 1 — Directed-graph resolution:** If any workspace member depends on a package that is NOT in the workspace sources (e.g., `state-machine` depends on `directed-graph`), uv tries to find it in PyPI, fails, and aborts. Example error: `coding-adventures-directed-graph was not found in the package registry and coding-adventures-state-machine depends on coding-adventures-directed-graph`.

**Problem 2 — Shared venv race condition:** When multiple packages are workspace members and run `uv venv --quiet --clear` in parallel, they all clear and rebuild the SAME workspace-root venv. A package running `uv pip install -e .[dev]` installs only 1 package (itself) into the shared venv, then another parallel build clears it again. Result: `No module named pytest`.

**Rules:**
1. **Do NOT add new packages to the uv workspace members list** unless you explicitly want them to share a workspace-root venv.
2. The correct fix for directed-graph resolution failures is to **remove the problematic workspace member** (e.g., `state-machine`) from the workspace — not to add the missing dep to the workspace.
3. Packages that install deps via explicit `-e ../dep` paths in their BUILD files don't need to be workspace members. The workspace is only needed for packages that have no explicit dep paths (like `grammar-tools` which only does `uv pip install -e .[dev]`).
4. **Grammar-tools is correctly a workspace member** since it has no runtime deps, so no workspace resolution issues arise (as long as other workspace members don't have unresolvable deps).

---

### 2026-03-23: Python BUILD_windows for packages with no runtime deps must use python -m venv + pip

Packages where `pyproject.toml` has `dependencies = []` (no runtime deps) and the BUILD file only has `uv pip install -e ".[dev]"` fail on Windows with `No module named pytest` because:
1. `uv venv --quiet --clear` creates the workspace-root venv (not package-local)
2. `uv pip install -e .[dev]` installs pytest into that venv
3. `uv run python -m pytest` syncs the project env (only runtime deps → 0 pkgs) → removes pytest

**Fix for grammar-tools and similar no-dep packages:** Use `python -m venv` (creates package-local .venv) + direct pip paths:
```
python -m venv .venv --clear
.venv\Scripts\pip install -e .[dev] --quiet
.venv\Scripts\python -m pytest tests/ -v
```

**Critical quoting rule:** Use `.[dev]` WITHOUT double quotes in `BUILD_windows` files. Go's `exec.Command("cmd", "/C", command)` escapes arguments via Windows CreateProcess API, causing CMD to receive the literal string `".[dev]"` (with quotes) instead of `.[dev]`. Pip then rejects it: `ERROR: ".[dev]" is not a valid editable requirement`. The Unix BUILD files can use `".[dev]"` because bash strips quotes before passing to pip. Always check existing working BUILD_windows files (e.g., `lexer/BUILD_windows`) — they all use unquoted `.[dev]`.

**Chain reaction rule:** When a low-level Python package (grammar-tools, lexer) changes, ALL packages that depend on it are added to the affected set by the build tool and will be rebuilt. This means every Python package in `git diff origin/main...HEAD` that has a BUILD_windows file using `uv run python -m pytest` will fail on Windows. Before pushing any Python changes, grep all modified packages' BUILD_windows files for `uv run python` and fix them with the `python -m venv` pattern above. In this PR, grammar-tools and lexer both changed → both needed BUILD_windows fixes independently.

---

### 2026-03-28: Vitest + jsdom: vi.stubGlobal("crypto", ...) must include getRandomValues

When a test stubs the global `crypto` to control `randomUUID` for deterministic
IDs, the stub object must also include `getRandomValues` — any library that uses
the Web Crypto API (e.g., `@coding-adventures/uuid`'s `v7()`) will fail with
`crypto.getRandomValues is not a function` if the stub doesn't provide it.

**Wrong:**
```typescript
vi.stubGlobal("crypto", { randomUUID: () => "mock-uuid" });
// breaks newEdgeId() → v7() → crypto.getRandomValues
```

**Correct:**
```typescript
import { webcrypto } from "node:crypto";
vi.stubGlobal("crypto", {
  randomUUID: () => "mock-uuid",
  getRandomValues: (b: Uint8Array) => webcrypto.getRandomValues(b),
  subtle: webcrypto.subtle,
});
```

Note: `{ ...webcrypto }` and `Object.create(webcrypto)` do NOT work because
`webcrypto`'s methods are on the prototype and require `this` to be an actual
`Crypto` internal slot holder. Always bind with `.bind(webcrypto)` or pass
through an arrow function as shown above.

Also add a `setupFiles` entry in vitest.config.ts that calls
`vi.stubGlobal("crypto", webcrypto)` unconditionally, so all test workers get
a functional Web Crypto API regardless of jsdom's setup order.

---

### 2026-03-28: BUILD file validator requires ALL transitive deps listed in leaf-to-root order

The build tool's validator checks that every package referenced transitively by
a program is listed as an explicit prerequisite in the BUILD file. When a new
dependency is added (e.g., `directed-graph` which pulls in `uuid` → `md5` + `sha1`),
all transitive packages must be added to the program's BUILD file in leaf-to-root
order, not just the immediate dependency.

**Example — adding directed-graph to todo-app:**
```
# Wrong: only lists the direct dep
cd ../../../packages/typescript/directed-graph && npm install --quiet

# Correct: lists all transitives first
cd ../../../packages/typescript/md5 && npm install --quiet
cd ../../../packages/typescript/sha1 && npm install --quiet
cd ../../../packages/typescript/uuid && npm install --quiet
cd ../../../packages/typescript/directed-graph && npm install --quiet
```

**Rule:** When merging main into a feature branch that introduced new transitive
deps via the build system update, expect this validator error and add the missing
refs immediately.

---

### 2026-03-28: BUILD files must not use backslash line continuations

The CI build runner executes each line of a BUILD file as a separate `sh -c`
command. Backslash line continuations (`cmd1 && \` / `cmd2`) cause `sh` to
see `\` as a standalone command and fail with `sh: 1: \: not found`.

**Wrong:**
```
cd ../sha1 && npm install --quiet && \
cd ../md5 && npm install --quiet && \
cd ../todo-app && npm install --quiet && \
npx vitest run
```

**Correct — one command per line:**
```
npm install --quiet
npx vitest run
```

The build tool already handles dependency ordering (it builds sha1 → md5 →
uuid → directed-graph → todo-app in topological order), so manually
chaining installs in the BUILD file is not needed. Each package's own BUILD
file runs `npm install` at the right time.

---

### 2026-03-28: GenericVM handlers MUST call advance_pc

Every opcode handler registered with `GenericVM.register_opcode` must call
`vn.advance_pc` at the end, or the VM will loop forever on the same instruction.

**Exceptions:**
- `HALT` — sets `vn.halted = true`, the while-loop condition stops execution
- Unconditional `JUMP` — calls `vn.jump_to(instr.operand)` instead
- Conditional jumps (`JUMP_IF_FALSE`, `JUMP_IF_TRUE`) — call `vn.advance_pc`
  when NOT jumping, and `vn.jump_to` when jumping

**Pattern for normal (non-jump) handler:**
```ruby
vm.register_opcode(OP::SOME_OP, lambda do |vn, instr, code|
  # ... do the operation ...
  vn.advance_pc
  nil
end)
```

**Pattern for conditional jump:**
```ruby
vm.register_opcode(OP::JUMP_IF_FALSE, lambda do |vn, instr, _c|
  cond = vn.stack.pop
  if cond == NIL || cond == false
    vn.jump_to(instr.operand)
  else
    vn.advance_pc
  end
  nil
end)
```

See `code/packages/ruby/starlark_vm/lib/coding_adventures/starlark_vm/handlers.rb`
for the canonical reference implementation.

---

### 2026-03-28: CALL_FUNCTION stack order — closure is on top

When the lisp compiler emits a function call `(fn arg0 arg1)`, it pushes
args first, then the closure:

```
stack: bottom → [arg0, arg1, ..., closure] ← top
```

The CALL_FUNCTION handler must pop the closure FIRST (top of stack), then
pop the args:

```ruby
vm.register_opcode(LispOp::CALL_FUNCTION, lambda do |vn, instr, _code|
  arg_count    = instr.operand
  closure_addr = vn.stack.pop          # top = closure
  args         = []
  arg_count.times { args.unshift(vn.stack.pop) }  # then args
  ...
end)
```

Getting this order wrong causes `deref(arg_value)` which raises `KeyError`
because the arg value (e.g., `42`) is not a valid heap address.

---

### 2026-03-28: GrammarLexer strips surrounding quotes from string tokens

When a `.tokens` grammar file defines a string pattern like:

```
STRING = /"([^"\\]|\\.)*"/
```

The `GrammarLexer` uses the capture group (the content without quotes) as the
token value. Tests that assert the raw quoted string (`'"hello"'`) will fail —
the actual value is `"hello"` (without quotes).

Fix tests to expect the unquoted content:
```ruby
# Wrong:
assert_equal '"serif"', tok.value
# Correct:
assert_equal "serif", tok.value
```

---

### 2026-03-28: Ruby module naming — StarlarkVM not StarlarkVm

The starlark_vm Ruby package uses `CodingAdventures::StarlarkVM` (capital VM),
not `StarlarkVm`. Calling `CodingAdventures::StarlarkVm.create_starlark_vm`
raises `NameError: uninitialized constant CodingAdventures::StarlarkVm`.

Always verify the exact module constant name by reading the gem's entry point
file (`lib/coding_adventures_starlark_vm.rb`) before referencing it.

---

### 2026-03-28: CRITICAL — Lua test files MUST set package.path before require

When running `busted . --verbose` from the `tests/` subdirectory, Lua cannot
find modules in the `src/` directory because it's not in `package.path`.
This causes "module not found" errors, especially on **Windows CI** where
the rockspec install does NOT put modules into the default Lua search path.

**Every Lua test file MUST have this line before the first `require`:**

```lua
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
```

Without this line, tests pass on some platforms but fail on Windows with:
```
module 'coding_adventures.foo' not found:
    no file './src/coding_adventures\foo.lua'
```

This is NOT optional. It is NOT handled by BUILD files or rockspecs on Windows.
The test file itself must prepend the path. This lesson has been re-learned
multiple times (atbash_cipher, scytale_cipher). Do NOT create a Lua test file
without this line.

---

### 2026-03-28: Perl Test2::V0 does not export use_ok — use eval+require instead

The `t/00-load.t` scaffold uses `use_ok('Module::Name')`, which is a
`Test::More` function not available in `Test2::V0`. This causes the test to
fail with a "No plan found in TAP output" error.

Fix: replace `use_ok(...)` with:
```perl
ok( eval { require Module::Name; 1 }, 'Module::Name loads' );
```

---

### 2026-03-28: Perl bitwise NOT needs 0xFFFFFFFF mask on 64-bit systems

Perl's `~$x` produces a 64-bit bitwise complement on 64-bit platforms.
For 32-bit arithmetic (MD5, bitset, etc.), always mask after NOT:
```perl
(~$x) & 0xFFFFFFFF
```
Without the mask, `~0` = 18446744073709551615 (0xFFFFFFFFFFFFFFFF), not 0xFFFFFFFF.

---

### 2026-03-28: Perl >> on negative integers is NOT arithmetic right shift

In Perl, `$x >> 7` on a negative integer does NOT sign-extend. For signed
LEB128 encoding (and other algorithms needing arithmetic shift), use:
```perl
use POSIX 'floor';
$remaining = floor($remaining / 128.0);
```
Or equivalently: `int(($remaining - ($remaining % 128)) / 128)` with careful
sign handling. Test with -1: `(-1 >> 7)` in Perl is a large positive number,
not -1.

---

### 2026-03-28: Lua scaffold generator support — snake_case directories

Lua packages use snake_case directory names (like Ruby/Elixir), not kebab-case.
The scaffold generator's `dirName()` now includes `"lua"` in the snake_case
case. The BUILD file for Lua only contains:
```
cd tests && busted . --verbose --pattern=test_
```
Dep installation is handled by CI's luarocks commands, not the BUILD file.

---

### 2026-03-28: Perl BUILD files — cpanm --with-test is NOT valid on Strawberry Perl (Windows)

The scaffold generator initially generated BUILD files with `cpanm --with-test --installdeps --quiet .`.
This fails on Windows CI with Strawberry Perl because `--with-test` is a `cpm` option, NOT a `cpanm` option.
Use `cpanm --installdeps --quiet .` instead.

Also: Perl packages need a `BUILD_windows` file that is a no-op (`echo Perl testing is not supported on Windows - skipping`)
because the CI workflow skips Perl setup on Windows (`if: runner.os != 'Windows'`). Without BUILD_windows,
the build-tool falls back to BUILD and runs cpanm on Strawberry Perl, which fails.

---

### 2026-03-29: Vitest coverage includes build scripts by default

When adding build scripts (like `scripts/build-all-browsers.ts`) to a TypeScript package, vitest's v8 coverage provider includes them in the coverage report. If the script isn't imported by any test, it shows 0% coverage and can pull the overall coverage below the threshold, failing CI.

**Solution:** Add `"scripts/**"` to the vitest config's `coverage.exclude` array, alongside other non-testable files like `dist/**` and `vite.config.ts`.

---

### 2026-03-31: tsc -b follows imports into nested node_modules for .ts source files

When TypeScript packages export `.ts` source (not compiled `.d.ts`), `tsc -b` follows import chains into `node_modules`. With `file:` dependencies, npm creates copies (not symlinks on Windows), and the chain goes multiple levels deep: `node_modules/@scope/a/node_modules/@scope/b/node_modules/@scope/c/...`. At that depth, transitive deps aren't installed, causing `Cannot find module` errors.

`skipLibCheck: true` does NOT help because these are `.ts` files, not `.d.ts` declaration files. The `include: ["src"]` tsconfig option also doesn't help because `tsc -b` follows imports regardless.

**Fix:** Remove `tsc -b` from Vite app build scripts. Vite handles TypeScript compilation for bundling, and type checking happens through vitest. Change `"build": "tsc -b && vite build"` to `"build": "vite build"`.

**Rule:** For Vite-based TypeScript programs that use `file:` dependencies, never use `tsc -b` in the build script. Rely on Vite's TypeScript handling for production builds.

### 2026-04-16: Swift Windows CI requires explicit toolchain install, then real tests may run

Swift packages can run on GitHub-hosted Windows CI, but only after `.github/workflows/ci.yml` installs the Swift toolchain with `winget install --id Swift.Toolchain` and refreshes the job environment so `swift.exe` is visible in later steps.

**Rule:** For Windows-compatible Swift packages, use a real `swift test` in `BUILD_windows`. Only keep a Windows skip if the package genuinely depends on Apple-only frameworks or another macOS-only capability.

**Also required:** Every Swift package must have a `.gitignore` with `.build/` and `.swiftpm/` excluded, so running `swift test` locally does not pollute the git tree.

---

### 2026-03-30: Swift BUILD files must use `xcrun swift test`, not `swift test`

When `swift-actions/setup-swift` installs a Swift toolchain on macOS CI runners, the XCTest framework lives inside the Xcode app bundle, not on the toolchain's rpath. Running bare `swift test` fails with `Library not loaded: @rpath/XCTestCore.framework`. Using `xcrun swift test` instead resolves framework paths through Xcode automatically.

**Rule:** Use `xcrun swift test` on macOS (resolves XCTest framework paths via DEVELOPER_DIR) but `xcrun` doesn't exist on Linux. BUILD files must be platform-aware:
```
if command -v xcrun >/dev/null 2>&1; then xcrun swift test; else swift test; fi
```

---

### 2026-03-30: Lua BUILD_windows must use Windows cmd syntax for environment variables

Unix-style inline env vars (`LUA_PATH=... lua`) don't work on Windows cmd.exe — you get "'LUA_PATH' is not recognized as an internal or external command". Use `set "LUA_PATH=..." && lua ...` instead.

**Rule:** When a BUILD_windows file needs to set environment variables, use `set "VAR=value" && command` syntax, not Unix-style `VAR=value command`.

---

### 2026-04-03: Perl modules must add `use lib` for their own dependencies

When a Perl module (`.pm` file) uses another internal module via `use CodingAdventures::Trig`, the dependency's `lib/` directory must be in `@INC` at module load time — not just in the test file. `prove -l` only adds the local `lib/` directory, and `use lib` in test files doesn't help if the MODULE itself triggers the `use` at compile time.

**Pattern (from Point2D.pm):** Add `use lib '../trig/lib'` directly to the module file, before the `use CodingAdventures::Trig` line. The path is relative to the CWD when `prove` runs (the package directory), so `../trig/lib` resolves to the sibling package's lib directory.

```perl
use strict;
use warnings;
use lib '../trig/lib';
use lib '../point2d/lib';
use CodingAdventures::Trig qw(sin_approx cos_approx);
use CodingAdventures::Point2D qw(new_point);
```

**Rule:** Every Perl module that `use`s a sibling package must add `use lib '../sibling/lib'` to the module itself.

---

### 2026-04-03: Never use `@ file:` direct references in Python pyproject.toml dependencies

`file:../sibling` path references in `pyproject.toml` `dependencies` cause two cascading failures:

1. **Hatchling ≥ 1.18** rejects them: `cannot be a direct reference unless field` — adding `allow-direct-references = true` fixes hatchling, but then:
2. **uv** fails when building the wheel from a temp directory: `relative path without a working directory: ../trig` — uv resolves the relative path from the temp build dir, not the package dir.

**Fix:** Use bare package names in `pyproject.toml` and let the BUILD script pre-install sibling packages:

```toml
# pyproject.toml — bare name only
dependencies = [
    "coding-adventures-trig",
    "coding-adventures-point2d",
]
```

```bash
# BUILD — pre-install siblings in leaf-to-root order before the target
uv venv --quiet --clear
uv pip install --no-deps -e ../trig --quiet
uv pip install --no-deps -e ../point2d --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
```

When `uv pip install -e ".[dev]"` runs, it sees `coding-adventures-trig` in `dependencies`, finds it already installed in the venv, and skips the PyPI fetch.

**Rule:** Never put `@ file:../path` in pyproject.toml dependencies. Always use bare names + BUILD pre-installation.

---

### 2026-04-03: TypeScript trig package.json must use src/trig.ts as main

The `trig` package predates Vitest adoption and had `"main": "dist/trig.js"` pointing to compiled output. When other packages depend on `trig` via `"file:../trig"` and are tested with Vitest, Vite resolves the entry via `main` and fails because `dist/trig.js` doesn't exist.

**Fix:** Set `"main": "src/trig.ts"` (consistent with the rule for all TypeScript packages). This is the same rule from 2026-03-19 but applied to the leaf `trig` package that was created before the rule was established.

---

### 2026-04-03: Python 3.13 c_long type mismatch in Python C extension (Windows)

On Windows x64, `c_long` is `i32` (32-bit), not `i64`. The CPython C API functions
`PyLong_AsLong`, `PyLong_FromLong`, and `PyModule_AddIntConstant` use `long` in their
signatures, which maps to `c_long`.

If you declare them as returning/taking `i64` (hardcoded), Rust compiles fine on Linux/macOS
(where `c_long == i64`), but fails on Windows with type mismatch errors like:
```
error[E0308]: mismatched types
150 |     PyLong_FromLong(result as i64)
    |                     ^^^^^^^^^^^ expected `i32`, found `i64`
```

**Fix:** Always declare Python C API functions with `c_long` from `std::ffi::c_long`:
```rust
use std::ffi::c_long;

extern "C" {
    fn PyLong_AsLong(obj: PyObjectPtr) -> c_long;
    fn PyModule_AddIntConstant(module: PyObjectPtr, name: *const c_char, value: c_long) -> c_int;
}
// Return from functions:
PyLong_FromLong(result as c_long)
```

When comparing or doing arithmetic with `c_long` values that may differ in width:
```rust
// Safe on both platforms:
let exp = (exp_val as i64).min(u32::MAX as i64) as u32;
```

**Rule:** All Python C API functions that take or return `long` must use `c_long` in Rust, not `i64`.

---

### 2026-04-03: OTP 26 Linux — enif_get_int64/enif_make_int64 not exported from beam.smp

On OTP 26 (and some earlier versions) on Linux, `enif_get_int64` and `enif_make_int64` are NOT
reliably exported from `beam.smp`. Declaring them as `extern "C"` compiles fine but causes:
```
undefined symbol: enif_get_int64
```
at NIF load time (dlopen).

**Fix:** Use `enif_get_long` and `enif_make_long` (always exported). On 64-bit POSIX:
- `c_long == i64`, so `enif_get_long` / `enif_make_long` are semantically equivalent to the
  int64 variants.
- Cast to/from `c_long` when calling these functions.

```rust
// make_i64: use enif_make_long
pub unsafe fn make_i64(env: ErlNifEnv, i: i64) -> ERL_NIF_TERM {
    enif_make_long(env, i as c_long)
}

// get_i64: use enif_get_long
pub unsafe fn get_i64(env: ErlNifEnv, term: ERL_NIF_TERM) -> Option<i64> {
    let mut val: c_long = 0;
    if enif_get_long(env, term, &mut val) != 0 {
        Some(val as i64)
    } else {
        None
    }
}
```

**Rule:** Never use `enif_get_int64`/`enif_make_int64` in erl-nif-bridge. Use `enif_get_long`/`enif_make_long`.

---

### 2026-04-03: Node.js N-API macOS linking — cargo:rustc-cdylib-link-arg must come from cdylib crate

N-API addons (`.node` files) are cdylibs that need `-undefined dynamic_lookup` on macOS so the
linker defers N-API symbol resolution to dlopen() time (when Node.js loads the addon).

**Critical:** `cargo:rustc-cdylib-link-arg` from a dependency's build.rs does NOT propagate
to the final cdylib. The flag must be emitted by the cdylib crate's own build.rs.

Wrong (no effect):
```
node-bridge/build.rs → cargo:rustc-cdylib-link-arg=-undefined
```

Correct (applied to the cdylib link step):
```
gf256-native-node/build.rs → cargo:rustc-cdylib-link-arg=-undefined
```

Pattern for N-API cdylib build.rs:
```rust
fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        println!("cargo:rustc-cdylib-link-arg=-undefined");
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }
}
```

**Rule:** Every cdylib that wraps N-API, Ruby, Python, Erlang, or Lua APIs needs its own build.rs
emitting platform linker flags. Do not rely on the bridge library's build.rs.

---

### 2026-04-03: Windows CMD vs sh — BUILD files use `cmd /C` on Windows

The Go build tool (executor.go) runs BUILD lines with `cmd /C <line>` on Windows, not `sh -c`.
Shell syntax that works on Linux/macOS fails on Windows CMD:

- `if [ -f file ]` → `-f was unexpected at this time.`
- `elif`, `fi` → syntax errors
- `{ cmd1 || cmd2; }` → `{` and `||` may not work as expected

**Fix:** Create `BUILD_windows` files alongside `BUILD` files for packages that use sh-specific
syntax. Use CMD-compatible commands:

```
# BUILD (Linux/macOS) — sh syntax
if [ -f target/release/libfoo.so ]; then cp ...; fi

# BUILD_windows — CMD syntax  
copy target\release\foo.dll src\foo\foo.pyd
set PYTHONPATH=src&& .venv\Scripts\python.exe -m pytest tests -v
```

Key CMD facts:
- `set VAR=value&& command` sets VAR and runs command in the same subprocess (no space before &&)
- `copy source destination` copies files
- `where program >/dev/null 2>/dev/null && program || echo skip` checks for program availability
- `&&` and `||` work as conditional separators (same as sh)

**Rule:** For any package whose BUILD uses `if [ -f ... ]`, `elif`, or `fi`, create a BUILD_windows
with CMD-compatible syntax. Python native packages especially need this.


---

### 2026-04-04: TypeScript program package.json path depth from code/programs/

When a TypeScript program in `code/programs/typescript/<name>/` depends on a package
in `code/packages/typescript/<pkg>/`, the relative `file:` path must traverse **three**
directory levels up — not two:

```
code/programs/typescript/asciidoc-demo/   ← you are here
code/packages/typescript/asciidoc/        ← dependency
```

Correct: `"file:../../../packages/typescript/asciidoc"`
Wrong:   `"file:../../packages/typescript/asciidoc"`

The `commonmark-demo` package uses the same `../../../` depth. Always count:
`programs/typescript/<name>` → up 3 = repo root → down `packages/typescript/<pkg>`.

---

### 2026-04-03: Elixir NIF module name must use Elixir atom format (not short name)

When calling `:erlang.load_nif/2` from an Elixir module, Erlang checks that
`ErlNifEntry.name` exactly matches the calling module's Erlang atom name.

Elixir modules use the `Elixir.` prefix in their Erlang representation:
```
CodingAdventures.GF256Native       → 'Elixir.CodingAdventures.GF256Native'
CodingAdventures.PolynomialNative  → 'Elixir.CodingAdventures.PolynomialNative'
```

Using the short name (e.g., `"gf256_native"`) causes a `:bad_lib` error at load time:
```
{:bad_lib, "Library module name 'gf256_native' does not match calling module
 'Elixir.CodingAdventures.GF256Native'"}
```

**Fix:** Set the module name to the full Elixir atom format:
```rust
// In the Rust NIF src/lib.rs:
static MODULE_NAME_BYTES: &[u8] = b"Elixir.CodingAdventures.GF256Native\0";
```

**Rule:** For Elixir NIFs, always use the full `"Elixir.ModuleName"` format for
`ErlNifEntry.name`. For Erlang NIFs, use the short lowercase atom name.


### 2026-04-05: QOI encoder seen-table must update for CURRENT pixel, not previous

When implementing the QOI encoder, the 64-slot seen-pixels table must be updated
for the **current** pixel after emitting any op that is NOT QOI_OP_INDEX. Updating
the table for the *previous* pixel before processing the current one (a "lag-one"
strategy) diverges from the decoder state and causes round-trip failures for any
image where a pixel is referenced by INDEX after the first emit.

The decoder always updates `seen[hash(r,g,b,a)] = (r,g,b,a)` for each non-RUN
pixel *after* decoding it. The encoder must do the same: emit the op for the current
pixel, then update `seen[hash(current)]` — NOT `seen[hash(prev)]` first.

The reference Python, TypeScript, Go, and other correct implementations only update
`seen` when the pixel did NOT match INDEX (the value is already there for INDEX pixels,
so updating is a no-op — but the key point is that `prev` is never written to `seen`
as a separate step).

This bug appeared in the Lua QOI encoder and was fixed before committing.

### 2026-04-08: Parsing pyproject.toml with regex: comment lines containing "[" break section detection

When writing scripts to parse Python `pyproject.toml` files to detect which
packages are in `[project] dependencies`, a naive regex like:

```
^\[project\][^[]*?^dependencies\s*=\s*\[([^\]]*)\]
```

fails if any **comment line** between `[project]` and `dependencies` contains
a `[` character. For example:

```toml
[project]
# When publishing to PyPI, restore: dependencies = ["coding-adventures-foo"]
dependencies = ["coding-adventures-bar"]
```

The `[^[]*?` part stops at the `[` in the comment, so the regex never reaches
the actual `dependencies` line. The match returns `None` even though the
dependency is present.

**Fix:** Parse line by line, skip lines starting with `#`, and track section
headers explicitly. Never use `[^[]*?` in a cross-line regex over TOML content.

This bug caused a mass incorrect categorization of 54 packages that had
`coding-adventures-directed-graph` as a real main dependency as "dev-only",
leading to three rounds of CI failures and fixes on PR #610.

---

## Don't commit Elixir coverage HTML artifacts

When `mix test --cover` runs it generates a `cover/` directory containing HTML
coverage reports (e.g. `cover/Elixir.CodingAdventures.Scrypt.html`). These are
build artifacts and must **not** be committed.

**Fix:** Always add a `.gitignore` to every new Elixir package before running
tests locally. Minimum contents:

```
cover/
_build/
deps/
.elixir_ls/
```

When staging files with `git add`, never use `git add .` or `git add -A` on an
Elixir package directory — always stage files explicitly by path to avoid
accidentally including the `cover/` output. If caught after the fact, use
`git rm --cached cover/<file>` to remove from tracking.

---

### 2026-04-11: Lua `^` operator returns float — use `<<` for integer powers of 2 in tests

In Lua 5.4, the `^` operator always returns a **float**, even for integer operands.
`2^24` evaluates to `16777216.0` (a float), not `16777216` (an integer).

Any parameter validation that uses `math.type(x) ~= "integer"` will reject float values
before reaching later guards. This caused a test failure where `scrypt.scrypt("pw", "salt", 2, 1, 2^24, 32)` triggered "p must be a positive integer" instead of the expected "p * 128 * r exceeds memory limit" — because `2^24` was a float that failed the type check.

**Rule:** In Lua tests that pass powers of 2 as integer parameters, use the bitwise left shift
`1 << 24` instead of `2^24`. Bitwise operations in Lua 5.4 always return integers.

```lua
-- WRONG: 2^24 is a float in Lua
scrypt.scrypt("pw", "salt", 2, 1, 2^24, 32)

-- CORRECT: 1<<24 is an integer in Lua 5.4
scrypt.scrypt("pw", "salt", 2, 1 << 24, 1, 32)
```

---

### 2026-04-11: Swift `let bLen` redeclaration after adding overflow-safe guard

When adding an overflow-safe `multipliedReportingOverflow` guard for `bLen` in Swift,
the new `let (bLen, ...)` declaration introduced before the original `let bLen = p * 128 * r`
caused a "invalid redeclaration of 'bLen'" compile error.

**Rule:** When replacing a `let x = expr` with an overflow-checked version that also binds `x`,
always remove the original `let x` line. The compiler will catch both declarations at the same
scope level.

---

### 2026-04-12: TypeScript AES BUILD missing prerequisite for gf256

When implementing `typescript/aes` which depends on `typescript/gf256` via `"file:../gf256"`, the BUILD file omitted the transitive install step. CI failed with:

```
BUILD/CI validation failed:
  - typescript/aes: missing prerequisite refs for standalone builds: typescript/gf256
```

**Fix:** Add `npm install --prefix ../gf256 --silent` before the local `npm install` in the BUILD file. This matches the pattern used in `typescript/reed-solomon`.

**Rule:** Every TypeScript package whose `package.json` has a `"file:../X"` dependency must have `npm install --prefix ../X --silent` as the first line of its BUILD file, before the package's own `npm install`.

---

### 2026-04-12: Python packages with local deps must split `uv pip install` calls and declare the dep in pyproject.toml

When `python/aes` depends on `python/gf256`, the build validator requires that:

1. `pyproject.toml` `dependencies` must declare `"coding-adventures-gf256>=0.1.0"` — otherwise the build tool cannot infer the edge in the dependency graph and fails with `undeclared local package refs: python/gf256`.
2. The BUILD file must install gf256 FIRST with a separate `uv pip install -e ../gf256 --quiet` line, THEN install the package with `uv pip install -e ".[dev]" --quiet`.

Combining them into a single `uv pip install -e ../gf256 -e ".[dev]"` was previously tried but the split is required to match the pattern in `python/reed-solomon`.

**Rule:** Python packages with local deps → declare dep in pyproject.toml `dependencies` AND use two separate `uv pip install` calls in BUILD (local dep first, then the package).

**IMPORTANT UPDATE (2026-04-12):** The above alone is NOT sufficient. uv performs universal resolution across ALL extras (including optional groups), so if ANY optional-dependency group references `coding-adventures-gf256`, uv will attempt a PyPI lookup and fail. The correct pattern is:

1. `pyproject.toml` `dependencies = ["coding-adventures-gf256>=0.1.0"]` — valid PEP 440 for hatchling and provides dep graph edge for validator
2. `pyproject.toml` `[tool.uv.sources]` section: `coding-adventures-gf256 = { path = "../gf256", editable = true }` — redirects uv to local path, bypassing PyPI
3. BUILD: `uv pip install -e ../gf256 --quiet` first (explicit ref satisfies the validator's `requiresExplicitPrereqs` check), then `uv pip install -e ".[dev]" --quiet`

Do NOT use `@ file:../gf256` in `dependencies` — hatchling rejects this during wheel metadata build. Do NOT put gf256 in optional-dependency groups — uv resolves ALL extras universally.

---
## Python BUILD files: Always use Unix venv paths

**Date:** 2026-04-12

**What happened:** Python ls00 BUILD file used `.venv/Scripts/python` (Windows venv layout).
CI runs on Linux where the path is `.venv/bin/python`, causing `sh: .venv/Scripts/python: not found`.

**Rule:** Always use `.venv/bin/python` in BUILD files. CI runs on Linux. All other Python packages
in the repo use `.venv/bin/python`. The Windows path `.venv/Scripts/python` only works locally on
Windows and will fail in CI.

---

## BUILD_windows files required for Windows CI

**Date:** 2026-04-12

**What happened:** Python, Perl, and Ruby ls00 packages failed on Windows CI because the BUILD file
uses `sh` syntax (`.venv/bin/python`, `for f in ...; do ... done`, etc.) but the build tool runs
`cmd /C` on Windows. The build tool supports `BUILD_windows` as a platform-specific override.

**Rule:** When creating a new package, check if sibling packages (e.g., json-rpc) have `BUILD_windows`
files. If they do, create one for the new package too. Key differences for Windows:
- Python: use `uv run --no-project python` instead of `.venv/bin/python`
- Perl: skip tests on Windows (matches json-rpc pattern)
- Ruby: `bundle exec rake test` works on both platforms, but `cd` path separators may differ

---

## Swift POSIX bind() ambiguity in closures

**Date:** 2026-04-12

**What happened:** Swift tcp-client tests failed on macOS CI with "use of 'bind' refers to instance
method rather than global function 'bind' in module 'Darwin'". Inside `withMemoryRebound` closures,
Swift's type checker sees the Sequence.bind instance method before the Darwin.bind POSIX function.

**Rule:** Never call POSIX `bind()` directly inside Swift closures. Create a `posixBind()` wrapper at
module scope that dispatches to `Darwin.bind` or `Glibc.bind` via `#if canImport`. Same applies to
other POSIX functions that collide with Swift stdlib names (`read`, `write`, `close` — though those
are less ambiguous in practice).

---

## Lua packages with native dependencies need luarocks install in BUILD

**Date:** 2026-04-12

**What happened:** Lua tcp-client tests failed on macOS CI because `luasocket` was not installed. The
BUILD file only ran busted but didn't install dependencies first.

**Rule:** If a Lua package depends on a native LuaRocks dependency (like `luasocket`), the BUILD file
must install it: `luarocks install luasocket --local && cd tests && LUA_PATH=...`. Check the
rockspec's `dependencies` field and ensure BUILD installs all of them. On Windows, luasocket native
compilation may fail — use BUILD_windows to skip if needed.

---

## Swift SOCK_STREAM type differs between Darwin and Glibc

**Date:** 2026-04-12

**What happened:** Swift tcp-client failed on Linux CI with "initializer 'init(_:)' requires that
'__socket_type' conform to 'BinaryFloatingPoint'". On Darwin, `SOCK_STREAM` is an `Int32`. On Linux
(Glibc), it's a `__socket_type` enum requiring `.rawValue` to extract the integer.

**Rule:** Never use `Int32(SOCK_STREAM)` in cross-platform Swift code. Use platform conditionals:
```swift
#if canImport(Darwin)
let sockType = SOCK_STREAM
#elseif canImport(Glibc)
let sockType = Int32(SOCK_STREAM.rawValue)
#endif
```
Same applies to `SOCK_DGRAM` and other socket type constants.

---

### 2026-04-13: BUILD validator path resolution fails on subdirectory references

Perl BUILD files use `PERL5LIB=../sha512/lib` to set include paths, referencing a subdirectory of the sibling package rather than its root. The build tool validator's `resolvePackageRef` did exact-match lookups against `pathToPkg`, which only stores package root directories. The path `../sha512/lib` resolved to `/path/to/perl/sha512/lib` which didn't match `/path/to/perl/sha512`, causing false "missing prerequisite refs" errors.

**Fix:** Added `resolvePackageRefFuzzy` that walks up the directory tree to find the nearest ancestor in `pathToPkg`. Used only for the "missing prerequisite" check (Python, TypeScript, Perl) to avoid surfacing false "undeclared ref" positives on pre-existing packages.

**Rule:** When BUILD files reference sibling packages, they may point at subdirectories (e.g. `../sha512/lib`, `../hmac/src`). The validator must handle these by walking up to the package root. Keep the strict exact-match for "undeclared ref" detection to avoid false positives.
## Intel 4004 backend: AND_IMM is a no-op on 4-bit hardware

**Date:** 2026-04-13

**What happened:** The `intel-4004-backend` codegen emitted `AND R1` for `AND_IMM v, v, 15`. The
Intel 4004 assembler rejected this because the 4004 ISA has no bitwise AND instruction. The codegen
was trying to mask a register to 0xF to implement u4 wrapping, but this is unnecessary on 4004
hardware because all registers are 4-bit (0–15) — they can never hold a value > 15.

**Rule:** On the Intel 4004, `AND_IMM vR, vR, 15` and `AND_IMM vR, vR, 255` are both no-ops. Emit a
comment only. Any other mask value is unsupported (would require a RAM lookup table). Do not emit
an `AND` mnemonic — the 4004 ISA does not have one.

---

## Intel 4004 backend: ADD_IMM R1-corruption when source is v1

**Date:** 2026-04-13

**What happened:** The `_emit_add_imm` function used R1 as a scratch register unconditionally. When
the source virtual register was v1 (which maps to physical R1), the pattern was:
  `LDM k; XCH R1; LD R1; ADD R1; XCH Rdst`
The `XCH R1` loaded the scratch value (k) into R1, destroying the original source value. So
`ADD_IMM v2, v1, 0` (copy v1 into v2) produced v2=0 instead of v2=5.

**Rule:** In the Intel 4004 codegen, always check if the source register is R1 before selecting R1
as the scratch register. If src==R1, use R14 (or another safe scratch) instead. Also special-case
k==0 as a pure copy: `LD Rsrc; XCH Rdst` (no scratch needed at all).

---

## Intel 4004 simulator: use HLT opcode (0x01), not JUN $ self-loop, for HALT

**Date:** 2026-04-13

**What happened:** The backend emitted `JUN $` (jump to self) as the halt idiom. The Intel 4004
simulator (`intel4004-simulator` package) does not detect self-loops as a halt condition, so
`result.ok` was `False` and execution always hit `max_steps` with an error.

**Rule:** When targeting `intel4004-simulator`, emit `HLT` (assembled as opcode 0x01, which is not a
real 4004 instruction but is the simulator's halt sentinel) to terminate execution cleanly. The
assembler accepts `HLT` and encodes it as `0x01`. This gives `result.ok=True` after execution.

---

## BUILD_windows: use .venv\Scripts\python (backslash), not .venv/Scripts/python

**Date:** 2026-04-14

**What happened:** New Python packages in this PR used `.venv/Scripts/python -m pytest` in their
`BUILD_windows` files. The build tool (`executor.go:shellCommandForOS`) runs BUILD_windows commands
via `cmd /C <command>`. In cmd.exe, `/` is the switch delimiter, so `.venv/Scripts/python` is parsed
as command `.venv` with option `/Scripts/python`, causing:
  `'.venv' is not recognized as an internal or external command`

**Rule:** In ALL `BUILD_windows` files, ALWAYS use backslashes for the venv path:
  `.venv\Scripts\python -m pytest tests/ -v`
NOT:
  `.venv/Scripts/python -m pytest tests/ -v`

Look at any existing `BUILD_windows` file (e.g., `cas/BUILD_windows`, `bitset/BUILD_windows`) as
the reference — they all use `.venv\Scripts\python`.

---

## BUILD_windows: do NOT quote .[dev] extras on Windows

**Date:** 2026-04-14

**What happened:** `uv pip install -e ".[dev]"` fails on Windows because cmd.exe passes the literal
double-quote characters to uv, causing:
  `error: Failed to parse: '".[dev]"' — Expected package name starting with alphanumeric`

**Rule:** In `BUILD_windows` files, use `.[dev]` (no quotes, no `-e` flag for the current package):
  `uv pip install -e ../dep1 -e ../dep2 .[dev] --quiet`
NOT:
  `uv pip install -e ../dep1 -e ../dep2 -e ".[dev]" --quiet`

---

## BUILD_windows: -e .[dev] (no quotes) — not .[dev] alone — is the correct form

**Date:** 2026-04-14

**What happened:** Removing both the quotes AND the `-e` flag broke non-editable installs on
Windows. With `.[dev]` (no `-e`), `uv` installs to `.venv\Lib\site-packages\`. Modules that
compute relative paths via `__file__` parent-walking get the wrong depth:
- Windows site-packages: `.venv\Lib\site-packages\pkg\module.py` (4 levels from source)
- Linux site-packages: `.venv/lib/python3.x/site-packages/pkg/module.py` (6 levels from source)
A 6-parent walk from Windows site-packages lands at `code\packages\python\` instead of `code\`.

**Rule:** In `BUILD_windows`, always use `-e .[dev]` (editable, no quotes):
  `uv pip install -e ../dep .[dev]`   ← WRONG: non-editable breaks __file__ paths
  `uv pip install -e ".[dev]"`        ← WRONG: cmd.exe passes literal quotes to uv
  `uv pip install -e .[dev]`          ← CORRECT: editable install, no quotes ✓

---

## Rust workspace builds: keep the toolchain current when dependencies adopt new editions

**Date:** 2026-04-17

**What happened:** `cargo build --workspace` failed while resolving the wider Rust workspace because
an external dependency (`uefi-macros`) now requires the Edition 2024 manifest feature. Older Cargo
versions reject that manifest before our own crates even begin compiling.

**Rule:** Before declaring a Rust workspace build broken, check the active toolchain and upgrade
stable when necessary:
  `rustup toolchain install stable`
If a dependency has adopted a newer edition, rerun the workspace build with the refreshed stable
toolchain instead of assuming the local Cargo version is sufficient.

---

## Python BUILD files: include transitive local siblings required by editable installs

**Date:** 2026-04-17

**What happened:** The new `ir-to-jvm-class-file` package installed
`../brainfuck` as an editable sibling, but its `BUILD` file omitted
`../virtual-machine`, which `coding-adventures-brainfuck` depends on. `uv pip`
then tried to satisfy `coding-adventures-virtual-machine` from the package
registry and failed dependency resolution before tests even started.

**Rule:** When a Python `BUILD` file installs sibling packages with `-e ../pkg`,
include any additional local siblings that those editable packages require if
they are not available from PyPI. For repo-local packages, install leaf-to-root
so `uv pip` never falls back to the registry for a dependency that only exists
inside this repository.

---

## Pure markdown parsers need explicit nesting and input-size limits

**Date:** 2026-04-17

**What happened:** The new pure C# and F# CommonMark parsers recursively re-parsed nested blockquotes,
lists, emphasis, links, and images without any depth guard. A maliciously deep markdown payload could
drive unbounded recursion and risk `StackOverflowException` or disproportionate resource use.

**Rule:** For any parser that recursively descends into user-controlled structure, add hard limits at
the parser boundary for maximum input size and maximum nesting depth. Enforce the limit in every
recursive entry point, not just the top-level public API.

---

## .NET BUILD scripts for related packages must isolate artifacts for parallel CI

**Date:** 2026-04-17

**What happened:** The new C# and F# document packages built fine locally one at a time, but CI runs
affected packages in parallel. `commonmark-parser` and `gfm-parser` share transitive `ProjectReference`
graphs, so simultaneous `dotnet test` runs raced on shared `obj/` files like
`AssemblyInfoInputs.cache`, causing intermittent MSBuild failures on macOS.

**Rule:** When multiple .NET packages in the repo can build the same transitive project graph in
parallel, their `BUILD` scripts must use `dotnet test --artifacts-path .artifacts` (or an equivalent
isolated artifacts path) so each package invocation gets its own build outputs and intermediate files.

---

## Markdown ordered-list markers must parse integers without throwing

**Date:** 2026-04-17

**What happened:** The C# and F# CommonMark parsers used direct `int.Parse` conversion for ordered
list markers. Extremely large numeric markers like `999999999999999999999. item` caused overflow
exceptions instead of being treated as non-list input.

**Rule:** When parsing user-controlled numeric tokens in language tooling, use `TryParse`-style
conversion and treat overflow as invalid syntax or plain text. Never let oversized numeric literals
crash the parser.

---

## .NET CLI on Linux may need package-local HOME, not just DOTNET_CLI_HOME

**Date:** 2026-04-17

**What happened:** Even after isolating `.NET` build outputs with `--artifacts-path`, Ubuntu CI still
failed intermittently in the CLI startup path with a `NuGet-Migrations` mutex/first-run error while
parallel package builds invoked `dotnet test`. Setting only `DOTNET_CLI_HOME` was not enough.

**Rule:** For Linux `dotnet` BUILD scripts that may run in parallel, set both `HOME="$PWD/.dotnet"`
and `DOTNET_CLI_HOME="$PWD/.dotnet"` so the CLI's first-run state is fully package-local and does not
race with sibling package invocations.

---

## BUILD_windows: use `set "VAR=value"` for path-bearing env vars

**Date:** 2026-04-17

**What happened:** The Windows BUILD scripts for the new .NET document packages used plain
`set DOTNET_CLI_HOME=%CD%\.dotnet`. In `cmd.exe`, a working directory path containing characters like
`&`, `|`, `(`, or `)` can change how the command line is parsed.

**Rule:** In `BUILD_windows`, always use the defensive quoted assignment form for environment
variables, especially when the value includes `%CD%` or another path:
  `set "DOTNET_CLI_HOME=%CD%\.dotnet"`
Also quote path arguments passed to commands such as `dotnet test`.

---

## Inline parsers need bounded unmatched-delimiter searches

**Date:** 2026-04-17

**What happened:** The C# and F# CommonMark inline parsers tried link, image, emphasis, and code-span
parsing at each character and, on malformed input, could rescan the full remaining suffix looking for
closers that were not there. Depth and total input limits were not enough to prevent quadratic work on
delimiter-heavy hostile input.

**Rule:** Any inline parser that retries delimiter or bracket parsing character-by-character must cap
how far a failed unmatched-delimiter search can scan. If a closer is not found within the bounded
window, treat the opener as literal text and move on instead of rescanning the full tail again.

---

## F# unsigned ranges need a zero-count guard before subtracting one

**Date:** 2026-04-18

**What happened:** An F# deserialiser loop used `for index in 0u .. count - 1u` with a `uint32`
token count. When `count = 0u`, subtracting one underflowed to `UInt32.MaxValue`, turning the
empty-input path into a huge bogus loop range and causing an `ArgumentOutOfRangeException`.

**Rule:** When looping over an unsigned count in F#, always guard the zero case before writing
`count - 1u`. Prefer:
  `if count = 0u then [] else for index in 0u .. count - 1u do ...`

---

## Recursive local functions in Go need a `var` declaration before assignment

**Date:** 2026-04-18

**What happened:** A new Go `jvm-class-file` helper used `addConstant := func(...)` and then called
`addConstant(...)` recursively inside its own body to normalize `int` to `int32`. Go does not let a
function literal declared with short assignment refer to that identifier recursively during its own
initialization, so `go test`/`go vet` failed with `undefined: addConstant`.

**Rule:** When a local Go helper needs recursion, declare it in two steps:
  `var addConstant func(...) (...)`
  `addConstant = func(...) (...) { ... addConstant(...) ... }`

---

## Go binary parsers must validate attacker-controlled lengths before `int` conversion

**Date:** 2026-04-18

**What happened:** A new Go JVM class-file parser converted `u4` payload lengths straight to
platform `int` and let nested `Code` attributes recurse as though they were method-level code.
On malformed input, that combination can turn bogus lengths into slice panics on 32-bit builds or
drive stack growth through unbounded recursive attribute parsing.

**Rule:** In Go binary parsers, never cast attacker-controlled lengths to `int` until they pass an
explicit host-capacity check, and never recursively decode nested structures unless the format
requires it. When an attribute is only meaningful at one structural level, treat deeper copies as
opaque bytes.

---

## In large Rust workspaces, avoid `cargo fmt --all` for package-scoped feature work

**Date:** 2026-04-18

**What happened:** A Rust JVM rollout worker ran `cargo fmt --all` from the shared workspace while
working on a handful of new packages. That reformatted hundreds of unrelated crates and buried the
actual feature diff under incidental workspace churn.

**Rule:** In this monorepo, use package-scoped Rust formatting for feature work:
  `cargo fmt -p package-a -p package-b`
or format only the files inside the package write set. Do not run `cargo fmt --all` unless the PR
is intentionally a workspace-wide formatting change.
## C# tests using `BinaryPrimitives` need an explicit `using System.Buffers.Binary`

**Date:** 2026-04-18

**What happened:** A new C# compression test used `BinaryPrimitives.WriteUInt32BigEndian(...)`
to craft a malformed header case, but the test file omitted `using System.Buffers.Binary`. The
package code compiled, yet the test project failed with `CS0103: The name 'BinaryPrimitives' does
not exist in the current context`.

**Rule:** When a C# test or package uses `BinaryPrimitives`, always add
`using System.Buffers.Binary;` explicitly at the top of that file. Do not assume implicit usings
will cover low-level buffer helpers.

---

## F# deserialisers must cap header counts to the available payload, not just trust the header

**Date:** 2026-04-18

**What happened:** In the new F# compression ports, `lz78` and `lzss` deserialisers initially let
header-declared token/block counts drive unsigned loops directly. Even when the payload was tiny,
a crafted large count could force a huge useless loop or combine badly with unsigned range math.

**Rule:** In F# binary deserialisers, always derive a `maxPossible` item count from the remaining
payload bytes and cap the header count before looping. Then guard the zero case explicitly before
writing ranges like `0u .. count - 1u`.

---

## BUILD scripts that visit sibling packages should use subshells so later commands stay in the original package

**Date:** 2026-04-18

**What happened:** A new TypeScript `http1` BUILD script installed `../http-core` with `cd ../http-core && npm install ...` and then immediately ran `npm install` and `vitest` on the next lines. Because the `cd` changed the shell's working directory for the rest of the script, the later commands accidentally re-ran inside `http-core` instead of `http1`.

**Rule:** In BUILD scripts, when you need to temporarily run a command in a sibling package, wrap it in a subshell like `(cd ../dep && npm install ...)`. Do not rely on `cd ... && cmd` when more commands follow afterward.

## Nonblocking accept tests must try `accept()` before waiting for a fresh readiness edge

**Date:** 2026-04-18

**What happened:** While generalising `transport-platform` provider tests across BSD, Linux, and
Windows, an accept helper waited for a new listener-readiness event before attempting `accept()`.
That failed on macOS because an earlier poll had already observed readiness, yet queued
connections were still waiting to be accepted.

**Rule:** In nonblocking listener tests, always attempt `accept()` first and only fall back to
waiting for readiness when it returns `WouldBlock`. Do not require a second readiness edge before
draining already-queued connections.

## `git worktree add` inherits the current checkout unless you pin the base explicitly

**Date:** 2026-04-18

**What happened:** A new compression worktree was first created with `git worktree add ... -b ...`
from a checkout that was itself on a feature branch. The new worktree silently inherited that
feature branch's commit instead of starting from `origin/main`, which would have polluted the next
PR with unrelated history.

**Rule:** When creating a fresh implementation worktree in this repo, always pin the starting point
explicitly: `git worktree add <path> -b <branch> origin/main`. Do not rely on the current checkout's
HEAD being the correct base.

---

## C# packages that reference a type with the same name as its namespace need an explicit alias

**Date:** 2026-04-18

**What happened:** The new C# `reed-solomon` package referenced the `gf256` helper as `Gf256.*`
after importing `using CodingAdventures.Gf256;`. Because the referenced package exposes both the
namespace `CodingAdventures.Gf256` and the type `Gf256`, the compiler bound `Gf256` as the
namespace, not the class, and every static arithmetic call failed to compile.

**Rule:** In C# ports where a referenced package exposes a type with the same name as its
namespace, add an explicit type alias such as `using FieldMath = CodingAdventures.Gf256.Gf256;`
and call the aliased type. Do not rely on unqualified `Gf256.*` style references compiling.

---

## Trim trailing zero coefficients before exposing little-endian locator polynomials

**Date:** 2026-04-18

**What happened:** The first F# `reed-solomon` pass returned Berlekamp-Massey locator arrays with
their resized trailing zeros still attached. Decoding still worked, but the exposed
`ErrorLocator` polynomial had an inflated apparent degree and failed degree-sensitive tests.

**Rule:** When a little-endian polynomial builder grows arrays in place during iterative
algorithms like Berlekamp-Massey, trim trailing zero coefficients before returning the public
result. Keep at least one coefficient so the zero-error locator remains `[1]`.

---
## Lua BUILDs must stage sibling rocks and tests must prefer source lexers when grammars live in-tree

**Date:** 2026-04-18

**What happened:** The first Lua `nib_type_checker` / `nib_ir_compiler` / `nib_wasm_compiler`
BUILD files called `luarocks make` on sibling packages directly, which failed because those
packages depend on other local rocks that are not published. After installing the rocks, the
tests still failed because the installed `nib_lexer` resolved `grammars/nib.tokens` relative to
the LuaRocks install tree instead of the repo, so parser-driven tests could not find the grammar.

**Rule:** For Lua packages in this repo, BUILD files should invoke sibling package `BUILD`s when
those siblings already know how to install their transitive local rocks. In tests that exercise
grammar-backed lexers/parsers, put the sibling `src/` directories for those lexers ahead of
installed rocks on `package.path` so in-repo grammar files resolve correctly.

---

## Rust workspace merge resolutions must deduplicate member entries before pushing

**Date:** 2026-04-18

**What happened:** A PR branch merged `origin/main` after adding new Rust `http-core` and `http1`
packages. The conflict resolution kept both sides' `window-core`, `window-appkit`, and
`window-win32` entries in `code/packages/rust/Cargo.toml`, which older Cargo tolerated but the
CI detect step now rejects as duplicate workspace members before any package builds run.

**Rule:** After resolving Rust workspace `members` conflicts, scan the final list for duplicates
and run a workspace-level manifest check before pushing. A merged workspace must contain each
package exactly once, even when both branches added adjacent member blocks.

---

## Shell BUILD files are line-oriented, so multiline control flow breaks under the repo build tool

**Date:** 2026-04-18

**What happened:** The Python `http-core` and `http1` packages used a normal multiline POSIX `if`
block in their Unix `BUILD` files. Those scripts passed `sh -n`, but CI still failed because the
repo build tool reads shell BUILD files as separate command lines and executes each line with its
own `sh -c`, so `if`, `then`, and `fi` never reached the same shell process.

**Rule:** In shell BUILD files, treat each non-comment line as an independent command. Keep
control flow on a single line, or express it with standalone one-line conditionals and `&&`/`||`
chains that remain valid when each BUILD line runs in its own shell.

---

## Python package BUILD installs must not use `--no-deps` when tests rely on `dev` extras

**Date:** 2026-04-18

**What happened:** The Python `http-core` CI fix kept the shell BUILD file line-safe, but the
editable install still used `--no-deps`. In both `uv pip install -e .[dev]` and the fallback
`pip install .[dev]` flow, that flag suppresses the optional `dev` dependencies too, so the build
completed without `pytest` and then failed at test time with `No module named pytest`.

**Rule:** If a Python BUILD script installs a package via `.[dev]` so tests can run, do not add
`--no-deps` to that install step. Use explicit prerequisite installs only for local package
dependencies, and let the package's declared test extras install normally.

---

## .NET package BUILD coverage should target the package under test, not every referenced assembly

**Date:** 2026-04-18

**What happened:** The first `paint-instructions` BUILD run failed coverage even though its own test
surface was solid because coverlet also measured the referenced `pixel-container` assembly inside the
same run. That dragged the total below the repo threshold and hid the real package coverage signal.

**Rule:** For .NET package BUILD scripts in this repo, pass an explicit coverlet include filter such
as `"/p:Include=[CodingAdventures.PaintInstructions]*"` or `"/p:Include=[CodingAdventures.PaintVm]*"`
so coverage thresholds apply to the package being verified rather than all transitive project
references.

---

## F# tests that populate `Metadata` should build a concrete `Dictionary` before upcasting

**Date:** 2026-04-18

**What happened:** Two new F# paint tests populated `Metadata` with `dict [...] :> IReadOnlyDictionary<string, obj>`.
F# inferred the intermediate value as `IDictionary<string, objnull>`, which then failed the stricter
`IReadOnlyDictionary<string, obj>` type expected by the package records.

**Rule:** In F# tests and builders for these packages, create a concrete
`Dictionary<string, obj>`, populate it, and only then upcast to `IReadOnlyDictionary<string, obj>`.
Do not rely on `dict [...]` preserving the exact metadata interface type you need.

---

## Public recursive comparison helpers need cycle tracking before they reach shared runtime packages

**Date:** 2026-04-18

**What happened:** The first C# `paint-vm` pass exposed a public `DeepEqual(object?, object?)`
helper that recursively walked dictionaries, enumerables, and reflected properties without
tracking visited reference pairs. It worked for the intended paint records, but a caller could pass
cyclic object graphs and trigger unbounded recursion plus process-killing stack exhaustion.

**Rule:** Any public recursive comparison or traversal helper in shared runtime packages must track
visited reference pairs before descending into reference types. Do not assume consumers will only
pass the acyclic data structures you had in mind during implementation.

---

## Linux `.NET` BUILD scripts may need package-local `TMPDIR`, not just `HOME`

**Date:** 2026-04-18

**What happened:** The paint foundation PR still failed on Ubuntu after setting package-local
`HOME` and `DOTNET_CLI_HOME`. The failing F# package hit `NuGet-Migrations` again, and the CI log
showed the mutex trying to allocate shared state under `/tmp/.dotnet/shm/...`, which is still
shared across parallel package builds.

**Rule:** For Linux `.NET` BUILD scripts that can run in parallel, create a package-local temporary
directory and set `TMPDIR="$PWD/.dotnet/tmp"` alongside `HOME="$PWD/.dotnet"` and
`DOTNET_CLI_HOME="$PWD/.dotnet"`. Isolating only the home directory is not enough when the CLI also
uses temp-backed shared-memory state during first-run migrations.

---

## Python native package commits must exclude copied extension artifacts

**Date:** 2026-04-18

**What happened:** While committing the first `window-native` Python package, the local
`src/window_native/window_native.so` artifact that `BUILD` copies into the source tree for test-time
imports was accidentally staged along with the real source files.

**Rule:** For Python native-extension packages in this repo, keep copied `.so` and `.pyd` files out
of commits. Add a package-local `.gitignore` for the copied extension artifact path before staging,
and sanity-check `git show --name-only HEAD` after the first commit when a package BUILD writes back
into `src/`.

---

## Stateful TCP protocol servers must cap incomplete per-connection input buffers

**Date:** 2026-04-18

**What happened:** While preparing the `mini-redis` migration onto `tcp-runtime` for push, security
review caught that the new per-connection RESP session state buffered partial frames in a `Vec<u8>`
with no maximum size. A client could hold a socket open and stream an incomplete array or bulk
string forever, causing unbounded heap growth and eventual process OOM.

**Rule:** Any TCP server in this repo that buffers partial protocol frames per connection must enforce
an explicit maximum buffered-input size. When a client exceeds that cap, clear the buffered state,
return a protocol error when possible, and close the connection instead of allowing unbounded memory
growth.

## Python bytecode or pool decoders must reject negative indexes explicitly

**Date:** 2026-04-18

**What happened:** The first `logic-bytecode` decoder used tuple indexing inside `_pool_get()` and
relied on catching `IndexError` for bounds checks. Python accepts negative sequence indexes, so
malformed bytecode like `operand=-1` silently resolved to the last pool entry instead of raising a
decode error.

**Rule:** Any Python decoder for bytecode, constant pools, or table-indexed formats must reject
negative indexes before indexing. Do not rely on `IndexError` alone for bounds validation, because
Python sequence semantics treat negative values as valid offsets from the end.

---

## Zero-length decoders must validate the full canonical empty encoding

**Date:** 2026-04-18

**What happened:** The first Dart `deflate` decoder returned immediately when the declared output
length was zero. That skipped the normal end-of-stream validation, so malformed payloads could add
extra bytes after the empty end-of-block marker and still be accepted as a valid empty stream.

**Rule:** For compressed or serialized formats with a canonical empty representation, zero-length
decoders must validate the entire empty encoding, not just the declared output length. Reject any
extra table entries, payload bits, or trailing bytes before returning success.

## Lua BUILD validators require declared local deps and `--deps-mode=none` consistency

**Date:** 2026-04-18

**What happened:** The Ruby/Elixir/Lua convergence PR passed package-local Lua tests but failed the
monorepo BUILD validator. One new Lua package bootstrapped sibling rocks and then ran a final
`luarocks make` without `--deps-mode=none`, another Windows BUILD disabled dependency resolution but
forgot to bootstrap a local rockspec dependency, and a third BUILD bootstrapped `wasm_runtime` even
though it was only used by tests through direct `package.path` entries rather than as a declared
rockspec dependency.

**Rule:** For Lua packages, keep the BUILD bootstrap set aligned with declared local rockspec
dependencies. If sibling rocks are installed first, the final `luarocks make` should also use
`--deps-mode=none`, and any extra test-only source-path wiring should stay in the test file instead
of appearing as an undeclared sibling bootstrap in `BUILD`.

---

## Perl context VMs must re-read the active code object after call-style handlers switch programs

**Date:** 2026-04-18

**What happened:** The new Perl Nib-to-Wasm pipeline compiled correctly and simple functions ran,
but any Nib function that called another Nib function hung inside the Wasm runtime. The root cause
was the shared Perl `virtual-machine`: `execute_with_context()` captured the original code object's
instruction list once, then kept stepping that stale code even after a Wasm `call` handler swapped
`$vm->{_program}` to the callee. Internal calls therefore looped inside the caller instead of
following the switched program.

**Rule:** In Perl VM loops that support context handlers capable of swapping programs, always
re-read the active code object and instruction list on each step. Do not cache the starting code
object across the whole execution when handlers can change `$vm->{_program}` mid-run.

---

## WASI host shims must cap guest-controlled iovec counts and byte totals

**Date:** 2026-04-18

**What happened:** During the Perl convergence security review, the new `fd_write` host shim in
`wasm-runtime` trusted guest-controlled `iovs_len` and `buf_len` values and copied every requested
byte into host memory before invoking stdout/stderr callbacks. A malicious guest could request a
huge scatter/gather write and force excessive host CPU and memory usage. The same trust boundary
exists for `fd_read`.

**Rule:** In WASI host implementations, treat `iovs_len`, per-buffer lengths, and total read/write
bytes as untrusted input. Enforce explicit upper bounds before copying guest data, and prefer
streaming bounded chunks over accumulating arbitrarily large host-side buffers.

---

## IR-to-Wasm backends must bound function arity and data segment sizes before allocation

**Date:** 2026-04-18

**What happened:** During the Perl convergence security review, `ir-to-wasm-compiler` trusted
caller-provided `param_count` and IR data declaration sizes. A malicious IR producer could request
huge Wasm function parameter vectors or enormous repeated data blobs, forcing the compiler to spend
unbounded memory before rejecting anything.

**Rule:** Treat IR programs as a trust boundary in backend packages. Validate function arity, each
data declaration size, and aggregate static data size before building repeated arrays, strings, or
Wasm sections. Fail closed with a compiler error instead of materializing attacker-controlled
sizes.

---

## Wasm runtimes must validate raw data-section sizes before slicing payloads

**Date:** 2026-04-18

**What happened:** A Perl convergence security review found that `wasm-runtime` trusted the byte
count encoded inside a raw data section and sliced `$pos .. $pos + $size - 1` before proving those
bytes existed. A malformed module could advertise a large segment size with a short payload and
force the runtime to allocate a huge temporary range/list while instantiating.

**Rule:** Treat parsed-but-raw Wasm sections as untrusted until every length field is checked
against the remaining section bytes and package caps. Validate segment count, per-segment bytes,
aggregate data bytes, offset-expression termination, and exact section consumption before slicing
or copying payloads.

---

## WASI host calls must cap guest-controlled buffer sizes before provider calls

**Date:** 2026-04-18

**What happened:** A security review found that Perl `random_get` passed the guest-controlled
`buf_len` directly into the random provider. The default provider allocates returned entropy bytes,
so a hostile module could request an enormous buffer and exhaust host memory before any guest memory
write occurred.

**Rule:** For every WASI host call that accepts guest pointers and lengths, validate the length
against package caps and guest memory bounds before invoking host providers, reading guest buffers,
or allocating host-side arrays/strings. Keep `fd_read`, `fd_write`, `random_get`, and future
buffer-copying calls under the same bounded-allocation discipline.

---

## Reactor tests must tolerate deferred socket readability after a write-ready step

**Date:** 2026-04-18

**What happened:** The new `stream-reactor` state-persistence test passed locally but failed on
macOS CI because it assumed one `write_ready()` call meant the client socket would immediately yield
the echoed frame on the very next `read_exact()`. In practice, the write flush and client-side
readability can lag by a poll turn or scheduler slice.

**Rule:** In reactor/socket tests, do not assume client-visible readability immediately follows a
single server-side write-ready step. Use bounded retries around both the reactor progression and the
client read so the test asserts eventual delivery rather than same-tick delivery.

## Copying a `.NET` package skeleton requires renaming the project files, not just changing `PackageId`

**Date:** 2026-04-18

**What happened:** The first `paint-vm-ascii` pass copied the existing `paint-vm` package and only
changed namespaces plus `PackageId`. The copied backend still used the file name
`CodingAdventures.PaintVm.csproj`, so MSBuild treated the backend and the shared runtime as the
same project identity inside `.artifacts`, which broke test references and ref-assembly resolution.

**Rule:** When cloning a `.NET` package in this repo, rename the project files and test project
files to the new package name immediately, and set explicit `AssemblyName`/`RootNamespace` values.
Changing `PackageId` alone is not enough to keep build outputs distinct.

---

## F# interpolated strings get brittle when expressions contain quoted literals

**Date:** 2026-04-18

**What happened:** The first F# `paint-vm-svg` build failed with dozens of `FS3373` errors because
interpolated SVG strings embedded expressions like `safeNum value "field"` and
`defaultArg fill "none"` directly inside `$"..."`. The nested quoted literals inside the
interpolation made the parser unhappy even though the logic itself was fine.

**Rule:** In F#, avoid putting expressions with quoted string literals directly inside interpolated
strings. Bind those values first with `let`, or switch to `sprintf` when composing dense XML/HTML
attribute strings.

---

## Haskell package-local builds should not target `all`

**Date:** 2026-04-19

**What happened:** While validating an isolated Haskell package, running `cabal test all` from the
package directory picked up the parent `code/packages/haskell/cabal.project` and attempted to build
the whole Haskell package universe. The package-local `BUILD` command used plain `cabal test`, which
correctly limited the build plan to the current package and its test suite.

**Rule:** For single-package Haskell validation in this repo, run plain `cabal test` from the
package directory unless you explicitly want the parent project. Do not append the `all` target for
isolated package PRs.

---

## wasm-bindgen `JsValue::from_str` is not safe in native error-path tests

**Date:** 2026-04-19

**What happened:** A native `cargo test` for a WASM wrapper passed success-path tests but aborted
when an error-path test tried to convert a Rust error string with `JsValue::from_str`. On
non-wasm32 targets, that wasm-bindgen constructor can hit a "function not implemented" abort instead
of returning a normal Rust panic.

**Rule:** In wasm-bindgen wrapper crates that run native tests, keep JS error construction behind a
`#[cfg(target_arch = "wasm32")]` helper. Use a simple native placeholder such as `JsValue::NULL` for
test-only error values when the test only needs to assert that the wrapper returned `Err`.

---

## Typed epsilon transitions must not accept empty-string aliases at import boundaries

**Date:** 2026-04-20

**What happened:** Security review of the first Rust `StateMachineDefinition` import helpers caught
that `NFA::from_definition()` treated `Some("")` the same as `None`. The runtime NFA uses an empty
string sentinel internally for epsilon transitions, but the typed definition contract says epsilon is
represented by `None`. Accepting both shapes would let malformed definitions smuggle free moves past
the import validator.

**Rule:** At typed import or deserialization boundaries, reject empty-string event names explicitly.
Only lower `None` to the runtime epsilon sentinel inside the core automaton constructor path. Add
regression tests for this distinction whenever a runtime uses sentinel values internally.

---

## GitHub Actions archive download failures can be transient infrastructure, not package failures

**Date:** 2026-04-20

**What happened:** A Python-only PR failed before checkout/build execution on Ubuntu and Windows
because the runner could not download action archives from `api.github.com` during the
"Prepare all required actions" phase. The package-local builds and affected build-tool run were
already green, and the failed jobs never reached repository code.

**Rule:** When CI fails in job setup while downloading third-party action archives, inspect the job
logs before changing package code. If the failure happens before checkout or tool setup commands,
treat it as infrastructure/transient unless repeated runs prove otherwise.

---

### 2026-04-20: GrammarLexer returns TokenType enum values; test helpers must normalize

When writing tests for grammar-driven lexer wrappers (like `oct-lexer`, `nib-lexer`), the
`GrammarLexer` returns `Token` objects where:
- Keyword tokens (after promotion in `tokenize_*`) have `type` set to a **string** (e.g. `"fn"`,
  `"carry"`)
- All other tokens keep the `TokenType` **enum** value (e.g. `TokenType.LPAREN`, `TokenType.NAME`)

If test helpers compare `t.type` directly against strings, non-keyword token tests will fail
with diffs like:
```
At index 1 diff: (<TokenType.LPAREN: 11>, '(') != ('LPAREN', '(')
```

And the EOF filter `t.type != "EOF"` will never exclude EOF tokens because
`TokenType.EOF != "EOF"` is `True`.

**Fix:** Add a `_tok_type` normalizer helper that converts both forms to a plain string:
```python
def _tok_type(tok: Token) -> str:
    return tok.type if isinstance(tok.type, str) else tok.type.name
```

Use it everywhere `t.type` is compared or used for filtering. This pattern is established in
`nib-lexer` and must be replicated in every new `*-lexer` test file.

**Root cause:** The design intentionally keeps non-keyword token types as enums to preserve
the `TokenType` enum contract for downstream consumers (parsers). Only keyword tokens are
promoted to string types so the parser's grammar rules can match them by value.

**Rule:** Every grammar-driven lexer test file must use a `_tok_type` normalizer. Never compare
`t.type` directly against string literals unless you know the token is a keyword.

---

## Runtime failure guards in callable lowerings must unwind activation state

**Date:** 2026-04-20

**What happened:** Security review of ALGOL 60 dynamic array lowering caught that array bounds and
heap-exhaustion guards returned immediately from the current WASM function. That was fine for
`_start`, but inside a procedure it skipped normal frame and heap restoration before handing control
back to the caller.

**Rule:** Any generated runtime-failure path that returns from a callable lowering must emit the
same activation cleanup as the normal return path first. For frame-backed languages, unwind the
active lexical scope chain and restore block-lifetime heap marks before `RET`. Add a regression that
repeatedly triggers the failure inside a procedure and then proves the caller can still allocate a
new frame.

---

## Conservative call-by-name scans must track lexical procedure shadowing

**Date:** 2026-04-20

**What happened:** Security review of the first ALGOL 60 call-by-name metadata pass caught that the
pre-lowering write scan classified transitive calls by bare procedure name. A nested procedure that
shadowed a known read-only procedure, or shadowed the procedure currently being analyzed, could write
through a by-name formal while the outer formal stayed marked read-only.

**Rule:** Any conservative by-name write analysis that runs before full procedure resolution must
track procedure declarations lexically. If a call resolves to a locally declared procedure whose
descriptor is not available to the scan, treat the matching by-name actual as writable rather than
falling back to an outer read-only descriptor or to self-recursion handling.

If the pre-pass keeps a bare-name procedure lookup, duplicate procedure names are ambiguous and must
also be treated as writable. Recursive propagation must flow only through target parameters whose
mode is by-name; a value parameter assigned locally does not write back to the caller's actual.
## Python tests need imports for helper types used only in assertions

**Date:** 2026-04-21

**What happened:** A logic-engine test added an `isinstance(..., LogicVar)` assertion for
standardize-apart behavior but forgot to import `LogicVar`. The implementation and behavior were
fine, but the package `BUILD` failed during the test body with `NameError` after most tests had
already passed.

**Rule:** When adding Python tests that assert on concrete helper classes, update the test imports in
the same patch as the assertion. Do not rely on related packages or nearby tests importing the type;
pytest modules need every assertion-only type imported explicitly.

---

## Ruff import sorting is strict about similarly named Python builtins

**Date:** 2026-04-21

**What happened:** Adding the new `clauseo` builtin near existing `callo` and `callableo` imports
looked visually reasonable, but Ruff's import sorter rejected the order in both the package export
module and tests.

**Rule:** After adding similarly named Python symbols to grouped imports, run Ruff before assuming the
manual order is acceptable. Prefer letting `ruff check --fix` apply pure import-order fixes instead
of hand-sorting by eye.
## Music fixture tests must derive timing expectations from the score tokens

**Date:** 2026-04-20

**What happened:** While adding the first text-score music machine, an initial Happy Birthday test
guessed the event and sample counts instead of deriving them from the score's duration table. The
fixture had 28 note/rest events, 27 quarter-note beats, and therefore 13.5 seconds at 120 BPM, not
the shorter duration assumed by the first test.

**Rule:** For text-score fixtures, count tokens and beats from the same duration rules used by the
parser before asserting rendered sample counts. Prefer assertions that make the musical math visible:
event count, note count, total beats, tempo-derived seconds, and sample-rate-derived sample count.

---

## Python BUILD files must use .venv/bin/python, not system python3.12

**Date:** 2026-04-22

**What happened:** The aot-core BUILD file used `python3.12 -m pytest` (the system Python) instead of `.venv/bin/python -m pytest`. Locally this worked because the system Python had the packages installed globally. On CI runners, only the uv venv has pytest and the other dependencies — the system Python has none of them.

**Rule:** Python BUILD files must always run pytest via `.venv/bin/python -m pytest`, matching the pattern used by jit-core, interpreter-ir, and every other Python package in the repo. Never use `python3.12` or `python3` directly in a BUILD file.

---

## Rebase conflict resolution must not carry branch-side BUILD files onto main

**Date:** 2026-04-23

**What happened:** While rebasing the `claude/trusting-thompson-5867c0` branch (msg-crypto/curve25519) onto origin/main, seven BUILD files had merge conflicts. The branch's `our` version was chosen for all of them, which introduced two types of CI failures:

1. **`mise: command not found`** — Elixir and Ruby BUILD files gained `mise exec --` prefixes that don't exist on GitHub Actions runners.
2. **`sh: 1: set: Illegal option -o pipefail`** — Lua and Perl BUILD files gained `#!/usr/bin/env bash` + `set -euo pipefail` headers. On Ubuntu the build tool invokes BUILD via `/bin/sh` (dash), which rejects `-o pipefail`.

The branch's changes to those BUILD files were from earlier unrelated work that had already been merged or was being merged on main. Keeping the branch version created duplicate/stale changes.

**Rule:** During rebase conflict resolution, BUILD files that the branch did not *intentionally* change should be resolved to the `theirs` (main) side: `git checkout --theirs <file>` then `git add <file>`. Only keep the branch side for files that the current branch deliberately modified.

**Rule:** After completing a rebase, run `git diff origin/main...HEAD -- '**/BUILD'` to verify that only the BUILD files intentionally added or changed by this branch appear in the diff. Any unexpected BUILD file diffs indicate a bad conflict resolution.

---

## Ruby native extensions: QNIL = 0x04 on 64-bit Ruby (USE_FLONUM), not 0x08

**Date:** 2026-04-23

**What happened:** The `ruby-bridge` Rust crate had `QNIL = 0x08`, copied from pre-USE_FLONUM Ruby documentation. On every modern 64-bit Ruby (x86_64, aarch64) `USE_FLONUM` is enabled, which changes the special constant layout:

```
Qfalse = 0x00, Qnil = 0x04, Qtrue = 0x14, Qundef = 0x24
```

Returning `0x08` to Ruby caused it to treat the value as an object pointer. Ruby then read the `klass` field at `pointer + 8`, landing at address `0x10`, and crashed with SIGSEGV. The crash only manifested on the "no match" code path (where `None => QNIL` was returned), so the "match" path appeared to work.

This caused four CI workaround hacks in the `conduit` package — all of which masked the real bug rather than fixing it.

**Rule:** In `ruby-bridge` and any other Rust/C Ruby extension, always use:
- `QNIL = 0x04` (not 0x08)
- `QTRUE = 0x14`
- `QFALSE = 0x00`

Confirm against `ruby/internal/special_consts.h` in the Ruby installation. The `nil.object_id` value in Ruby (which returns 4 in Ruby 3.x) is computed as `LONG2FIX(QNIL)` and matches: `QNIL = nil.object_id` only if you account for the Fixnum encoding (`LONG2FIX(n) = n << 1 | 1`, so `LONG2FIX(4) >> 1 = 4`).

**Rule:** When a Ruby native extension starts crashing with SIGSEGV at a low address like `0x10`, suspect that a "nil" or other special constant is being returned with the wrong bit pattern, causing Ruby to dereference it as an object pointer.

---

## Lesson: `LUA_REGISTRYINDEX` in lua-bridge was set to the Lua 5.1 value (-10000) instead of Lua 5.4's (-1001000)

**Date:** 2026-04-24

**What happened:** The `lua-bridge` Rust crate had `LUA_REGISTRYINDEX = -10000`, which is the Lua 5.1 value. Lua 5.4 derives this constant from `LUAI_MAXSTACK`:

```c
#define LUAI_MAXSTACK  1000000
#define LUA_REGISTRYINDEX  (-LUAI_MAXSTACK - 1000)  /* = -1_001_000 */
```

Using `-10000` as the table index in `luaL_ref(L, LUA_REGISTRYINDEX)` caused Lua to treat it as a regular negative stack index (not a pseudo-index), landing 10000 slots below the current stack frame — far outside valid memory. This produced SIGBUS/SIGSEGV crashes whenever `app_add_route`, `app_add_before`, `app_add_after`, `app_set_not_found`, or `app_set_error_handler` were called (all of which store Lua function references in the registry).

**Rule:** In any Rust/C Lua extension targeting Lua 5.4, use:
```rust
pub const LUA_REGISTRYINDEX: c_int = -1_001_000;
```

The Lua 5.1 value (`-10000`) must never appear in Lua 5.4 code. Pseudo-indices in 5.4 start below `-LUAI_MAXSTACK - 1000`, so any value between `-10001` and `-1_000_999` is treated as a regular stack index — not the registry.

---

## Lesson: Extra `push_cstr` before `lua_rawset_str_top` corrupts the Lua stack when setting metatable `__gc`

**Date:** 2026-04-24

**What happened:** In `lua_new_app` (and `lua_new_server`), the code was:
```rust
luaL_newmetatable(L, APP_MT);        // stack: [ud | mt]
push_cstr(L, "__gc");                // stack: [ud | mt | "__gc"]  ← wrong
lua_pushcclosure(L, Some(app_gc), 0);// stack: [ud | mt | "__gc" | fn]
lua_rawset_str_top(L, -3, "__gc\0"); // sets stack[-3]["__gc"] = fn → [ud | mt | "__gc"]
lua_setmetatable(L, -2);             // sets stack[-2]'s metatable = top ("__gc" string) → crash
```

The extra `push_cstr` left a dangling string on the stack. After `lua_rawset_str_top` consumed `fn`, the top was `"__gc"` (a string), and `lua_setmetatable` tried to set that string as the metatable — which is invalid.

**Rule:** When attaching a `__gc` metamethod to a userdata metatable, the pattern is:
```rust
luaL_newmetatable(L, MT_NAME);        // stack: [ud | mt]
lua_pushcclosure(L, Some(gc_fn), 0); // stack: [ud | mt | fn]
lua_rawset_str_top(L, -2, "__gc\0"); // mt["__gc"] = fn; stack: [ud | mt]
lua_setmetatable(L, -2);             // ud.metatable = mt; stack: [ud]
```
Never push the key string manually before `lua_rawset_str_top` — the function already supplies the key.

---

## Lesson: Lua route errors must be caught inline and routed to the error handler, not via `on_handler_error`

**Date:** 2026-04-24

**What happened:** The web-core `on_handler_error` hook is for Rust-level errors (panics), not Lua errors. Lua errors from `error("...")` inside a route handler are caught by `lua_pcall` inside `dispatch()`, but `dispatch()` was returning a generic 500 instead of calling the registered Lua error handler. The test `GET /error returns 500 JSON via error handler` failed because the custom error handler body was never sent.

**Rule:** When a route handler's `lua_pcall` returns non-zero and the error is NOT a HaltError, the dispatch code must:
1. Extract the error message from the stack
2. Call the Lua error handler ref (if registered) with `(env, err_message)`
3. Only fall back to a generic 500 if no error handler is registered

Use a dedicated `dispatch_route` function that holds the Lua lock across both the route call and the error handler call.

---

## Lesson: Lua GC collects userdata when Lua-side variable goes out of scope — pin it in the registry when Rust holds raw integer refs into it

**Date:** 2026-04-24

**What happened:** `LuaConduitServer` was built from a `LuaConduitApp` userdata. After `setup()` returns, the Lua `app` variable goes out of scope. On Linux (glibc), Lua's incremental GC is more aggressive than on macOS and will collect the unreachable `LuaConduitApp` userdata between test runs. When `app_gc` fires, it calls `luaL_unref` on every handler registry slot. The server's Rust closures still hold those slot integers, but `lua_rawgeti` now pushes nil — causing all subsequent HTTP handler dispatches to fail with 500.

**Root cause:** Raw `i32` registry refs (integers from `luaL_ref`) are not Lua values — they don't prevent GC. The GC only tracks the Lua value graph (upvalues, tables, the registry itself). Once the `LuaConduitApp` userdata is unreachable from Lua's value graph, it will be collected even if Rust holds its slot integers.

**Fix:** In `lua_new_server`, before allocating the server userdata:
```rust
lua_pushvalue(L, 1);                         // push copy of app userdata (arg 1)
let app_ref = luaL_ref(L, LUA_REGISTRYINDEX); // pin it; pops the copy
```
Store `app_ref: i32` in `LuaConduitServer`. In `server_gc`, after stopping the server and joining the background thread:
```rust
luaL_unref(L, LUA_REGISTRYINDEX, (*srv).app_ref);
```
This keeps the `LuaConduitApp` reachable from the Lua registry (which is always a GC root) until the server is destroyed. The `luaL_unref` in `server_gc` runs only after all Rust closures will never fire again.

**Rule:** Any time Rust holds raw `luaL_ref` integers derived from a Lua object's internal state, and that object could become unreachable from Lua's value graph, you must keep a registry reference to the object alive for at least as long as the raw integer refs are in use. Use `luaL_ref` to pin the object and `luaL_unref` to release only after all integer refs are retired.

---

## Lesson: `napi_create_threadsafe_function` — pass C NULL for `async_resource`, not JS `undefined`

**Date:** 2026-04-25

**What happened:** Calling `napi_create_threadsafe_function` with `async_resource = napi_get_undefined()` (the JavaScript `undefined` value) causes Node.js v25 to return `napi_invalid_arg`. The panic message was `N-API error (status 2): napi_create_threadsafe_function`.

**Root cause:** Node.js source checks `v8_resource->IsObject()` on the `async_resource` parameter. The JavaScript `undefined` value is not an Object, so the check fails and `napi_invalid_arg` is returned. When `async_resource` is C NULL (the null pointer), Node.js instead creates a fresh internal Object — the correct way to opt out of async-hook tracking.

**Fix:** In `node-bridge/src/lib.rs` `tsfn_create()`, pass `ptr::null_mut()` for `async_resource` instead of the result of `napi_get_undefined`. Remove the `napi_get_undefined` call entirely for this parameter.

**Rule:** For optional `napi_value` parameters in N-API, "no value" must be represented as C NULL (`ptr::null_mut()`), not JavaScript `undefined`. The two are distinct: a C null pointer means "not provided"; JS `undefined` is a real JS value subject to type checks.

---

## Lesson: `napi_create_threadsafe_function` — pass C NULL for `func` when providing a custom `call_js_cb`

**Date:** 2026-04-25

**What happened:** On Node.js v25, passing both `func` (a JS function value) AND `call_js_cb` (a custom C callback) to `napi_create_threadsafe_function` caused `napi_invalid_arg`. The function value failed the internal `v8_func->IsFunction()` check even though it was a valid JS function.

**Root cause:** Platform-specific V8 value type checking; passing a non-null `func` triggers additional validation that can fail. When `func = NULL`, Node.js skips the function-type check and requires `call_js_cb != NULL` instead.

**Fix:** Pass `ptr::null_mut()` for `func`. Store the JS function as an `napi_ref` and pass it as the `context` pointer. In `call_js_cb`, cast `ctx` back to `napi_ref` and call `deref(env, handler_ref)` to retrieve the function.

**Rule:** When using a custom `call_js_cb`, always pass `func = NULL` to `napi_create_threadsafe_function` and carry the JS function reference via the `context` parameter as an `napi_ref`. This is the safe, N-API-documented pattern.

---

## Lesson: `WebResponse::internal_error` sets Content-Type header — breaks empty-headers sentinel check

**Date:** 2026-04-25

**What happened:** A route handler check `if resp.status == 500 && resp.headers.is_empty()` was used to detect an unhandled exception (to re-dispatch to the error handler TSFN). But `WebResponse::internal_error()` calls `.with_content_type("text/plain")`, which adds a `Content-Type` header — making `headers.is_empty()` false. The error handler was never invoked.

**Fix:** In `extract_halt_or_error`, for non-halt JS exceptions, return a bare `WebResponse { status: 500, headers: vec![], body: msg.into_bytes() }` instead of `WebResponse::internal_error(&msg)`. This makes the sentinel check `headers.is_empty()` reliable. If no error handler is registered, the bare 500 with the error message body is returned to the client as-is.

**Rule:** When using a `WebResponse` field value as a sentinel to distinguish "exception thrown, call error handler" from "handler returned a real 500 response", do NOT use the convenience constructors that add headers. Build the sentinel response manually with `headers: vec![]`.

---

## Lesson: Don't use `mise exec --` in BUILD files — CI Ubuntu doesn't have mise

**Date:** 2026-04-26

**What happened:** WEB05 PR #1426 build failed on `ubuntu-latest` with `sh: 1: mise: not found` for both `typescript/conduit` and `typescript/programs/conduit-hello`.

**Root cause:** The BUILD files prefixed every command with `mise exec --` (e.g. `mise exec -- npm ci`).  This works locally because `mise` is installed and on PATH.  In GitHub Actions, the workflow uses `actions/setup-node` and `dtolnay/rust-toolchain` to install tools directly into PATH; it does NOT install mise.  So the shell that runs the BUILD script can't find the `mise` binary.

**Fix:** Remove all `mise exec --` prefixes.  Just call `cargo`, `npm`, `npx`, `node`, etc. directly.  The Python conduit BUILD already documented this pattern explicitly in a comment.

**Rule:** BUILD files should call language tools (cargo, npm, npx, node, python, go, etc.) directly without any wrapper.  `mise` provides shims locally so direct invocation works in both environments.  CI installs tools into PATH itself.

---

## Lesson: In variable-length encodings, the FORMAT MARKER must come first — never write the length-distinguishing flag on the LOW byte of an LE16

**Date:** 2026-04-26

**What happened:** Several zstd ports (TS, Go, Lua) encoded the 2-byte sequence count as a little-endian u16 with `0x8000` ORed in, then wrote it LE — which puts the LOW byte of `count` first and the flag byte (with bit 7 set) second:

```ts
// BROKEN: low byte first
const v = count | 0x8000;
return [v & 0xFF, (v >>> 8) & 0xFF];
```

The decoder branches on `byte0` to choose form (`< 128` → 1 byte, `< 0xFF` → 2 bytes, `== 0xFF` → 3 bytes). For any count ≥ 128 whose LOW byte happened to be < 128 (e.g. count=515 → `[0x03, 0x82]`), the decoder mis-took the 1-byte path and silently returned a tiny garbage count, mis-aligning the modes byte and the FSE bitstream that followed. About half of all 2-byte counts triggered this; the other half worked. Round-trip tests with counts like 1000 (low byte 0xE8) or 0x7FFE (both high) silently passed despite the bug. Discovered when Lua's TC-8 (300 KB → 515 sequences) finally hit a count whose low byte was < 128 and threw `unsupported FSE modes` from a misaligned modes byte.

**Rule:** Any variable-length integer encoding whose decoder branches on `byte0` MUST place the byte that carries the format marker **first** in the wire stream — independent of the host's natural endianness. For zstd's seq_count specifically, the RFC 8878 §3.1.1.3.1 layout is:

```
count < 128:           [count]
128 ≤ count < 0x7FFF:  [(count >> 8) | 0x80, count & 0xFF]   # high byte first
count ≥ 0x7FFF:        [0xFF, low, high]                       # 3-byte form
```

Decode: `((b0 & 0x7F) << 8) | b1`, equivalent to RFC's `((byte0 - 128) << 8) | byte1`.

When testing variable-length codecs, the round-trip test parameter set MUST include at least one value in each form whose low byte is < 128 (e.g. 256, 300, 515, 768) — otherwise a low-byte-first regression silently passes. Pure round-trip tests on a self-consistent broken codec are blind to byte-order bugs by construction. The integration test that catches it reliably is "≥ 200 KB of repetitive text → ≥ 128 sequences in a single block" — that input distribution naturally produces counts spanning both halves of the 2-byte range.

---

## Lesson 92 — CI runners are ~25× slower for LZSS/compute-heavy tests; always set an explicit timeout

**Date:** 2026-04-26

**What happened:** The TypeScript ZStd TC-8 regression test (200 KB repetitive text → ≥ 128 sequences) ran in ~450 ms locally but took 12–15 seconds on CI runners. Vitest's default per-test timeout is 5 seconds. The CI job failed with a timeout error even though the test was functionally correct and passing locally.

**Rule:** Any test that triggers an LZSS/LZ77 pass over more than 50 KB should have an explicit timeout set to at least `30_000` ms (30 s) in vitest:

```ts
it("round-trips 200 KB ...", () => { ... }, 30_000);
```

CI runners (especially GitHub Actions free-tier) run at roughly 25× slower wall-clock for CPU-intensive loops. A test that takes < 1 s locally may take 25 s on CI. Default framework timeouts (5 s for vitest, 60 s for Go's `go test`) are often too tight for large compression round-trips. Always measure on CI before assuming the default is safe.

---

## Lesson 93 — `unpack('C*', ...)` in Perl amplifies memory before any size check

**Date:** 2026-04-26

**What happened:** The Perl ZStd `decompress` function called `my @data = unpack('C*', $input)` on the raw compressed bytes as its very first step, converting each byte into a full Perl scalar. A Perl scalar occupies ~56 bytes on 64-bit builds (SV header + IV/PV storage). A 64 MB compressed input therefore expands to ~3.5 GB of Perl scalars on the heap before any frame-header validation or size guard could fire — a classic unpack memory amplification attack.

**Rule:** In Perl, never `unpack('C*', ...)` a caller-supplied buffer without first checking its length:

```perl
die "input too large" if length($data) > 64 * 1024 * 1024;
my @bytes = unpack('C*', $data);
```

64 MB is a safe upper bound for all realistic ZStd frames (the compressor's MAX_BLOCK_SIZE is 128 KB). The same pattern applies to any language where unpacking bytes into an array of objects/scalars multiplies memory by a large constant factor. Always validate the *raw byte count* before the amplifying operation, not just the logical content-size field inside the frame.

---

## Lesson 94 — Trailing bytes after the last ZStd block must be rejected, not silently ignored

**Date:** 2026-04-26

**What happened:** The Lua ZStd decoder iterated blocks in a `while true` loop and broke on `last_block == 1`. Any bytes remaining in the input after the last block were silently ignored. A fuzz input consisting of a valid 5-byte frame followed by 1 MB of garbage would be accepted without complaint, masking corruption and making the decoder lenient about malformed or concatenated frames.

**Rule:** After the block-decoding loop exits (when `last_block == 1`), assert that the read cursor equals `#data` (or `data.length`, or the frame boundary). If any bytes remain, raise an error:

```lua
if pos <= #data then
  error("unexpected trailing data after last block")
end
```

The same check belongs in every language port. A strict decoder is far safer — it surfaces truncation and concatenation bugs immediately rather than silently returning partial output or accepting garbage.
