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
