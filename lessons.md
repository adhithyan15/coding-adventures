# Lessons Learned

- **After a code-generation or mechanical split script runs, parse every generated file before trusting the transformation.** While splitting the human-language cross-script integration test, patch-marker `+` characters accidentally survived inside a template string and were written into a shared helper. The structural statement-count check still passed because it measured moved assertions, not generated TypeScript syntax. Run the narrowest parser/test immediately after generation; statement counts prove coverage movement, not that wrapper text is valid source. Run it from the package directory too: invoking `npx vitest` at the repository root made Vitest discover an unrelated package whose local dependencies were not installed, turning a correct focused command into a false cross-repository failure.

A condensed quick-reference of mistakes made during development, grouped by category. Read this file before starting work that touches BUILD files, CI, native extensions, or any of the language-specific pitfalls below. Entries are kept short on purpose — when a rule recurs, the canonical entry is here, not buried in chronology.

---

## BUILD files & dependency management

- **A missing BUILD file is now a CI failure, not a silent gap — `code/BUILD-EXEMPTIONS` is the ledger.** `build-tool -validate-build-files` fails on any directory that has a `Cargo.toml`, no `BUILD`, and no entry in `code/BUILD-EXEMPTIONS`. Entries are `EXCLUDED <path>  # <reason>` (genuinely never built — a compile-only JNI bridge, a wasm-only cdylib) or `PENDING <path>  # <reason>` (a tracked backlog item), and the reason is mandatory. **Stale entries fail too**: land a BUILD for a `PENDING` crate and the same PR must delete its exemption line, so the ledger can never outlive the problem. This closed an 84-crate gap; it did NOT come from a scaffold-generator bug (that tool has templated BUILD since 2026-03-21) but from crates hand-rolled afterwards. If you add a crate and CI complains, the fix is a BUILD file — reach for an exemption only when the crate genuinely has nothing to run, and say where it IS covered.
- **Each BUILD line runs as a separate `sh -c` (Unix) / `cmd /C` (Windows) process.** `cd` and shell variables do NOT persist between lines. Chain with `&&` on one line, use subshells `(cd ../dep && ...)`, or keep each line absolute. Multiline `if/then/fi`, `for`, and backslash continuations all break — the runner sees `\` as a literal command and fails with `\: not found`.
- **A package with NO BUILD file is invisible to CI — it is never built and its tests never run.** The build tool discovers packages by their BUILD file, so a crate without one is silently unwatched no matter how many tests it has. `lang-aot` hid three red suites this way; `twig-aot` (PR #11264) hid both a red suite and a test target that did not compile at all. A workspace-wide `cargo check` elsewhere keeps such a crate *compiling* while nothing ever compiles its **test targets** or runs its assertions — which is exactly how an unused import in a test file sits on main indefinitely. When adding a BUILD to a long-unwatched package, expect to find existing breakage, and list test targets explicitly (`--test a --test b`) rather than running the whole crate, so any target you must exclude is visible and named in a comment. Excluding a target skips only its RUN, never its COMPILE — the check step still compiles every target with `-D warnings`.
- **BUILD files must install ALL transitive local deps in leaf-to-root order.** Single most-recurring repo-wide failure (re-learned 8+ times across Python/Ruby/Go/TypeScript/Lua/Perl/Elixir/Rust). CI starts with empty `node_modules`/venvs, so every transitive sibling needs an explicit install line before the package's own. After adding a new low-level package, every package up the chain needs its BUILD updated. Use the scaffold generator — it computes the closure for you.
- **Every TypeScript BUILD file must include `cd ../cli-builder && npm ci --quiet`.** The build-tool validator checks for this as a required prerequisite ref (`missing prerequisite refs for standalone builds: typescript/cli-builder`). It is a toolchain dep that every TS package needs implicitly — add it immediately after `cd ../directed-graph && npm ci --quiet`. Surfaced in PR #7659 for sql-planner, sql-optimizer, sql-codegen, sql-vm, mini-sqlite.
- **BUILD-file references must also appear in the language's metadata file** so the build-tool validator can see the dep edge: Python `[project] dependencies` (and `[tool.uv.sources]` for local paths), Ruby `.gemspec` `spec.add_dependency` (block var must be `spec`, not `s`), Perl `cpanfile`, Go `go.mod`, Swift `Package.swift`, Rust `Cargo.toml`, TypeScript `package.json`. Missing this raises `undeclared local package refs:` in the detect job.
- **Test-only sibling deps still need to be declared.** If a TEST file imports a sibling for `isinstance()` checks, install the sibling in BUILD AND declare it in `pyproject.toml` dev extras — otherwise the validator's prerequisite check fails. Every package referenced by `-e ../pkg` in a BUILD must be directly declared in that package's metadata; declaring the *parent* dep is not transitively sufficient.
- **Validator handles subdirectory refs** via `resolvePackageRefFuzzy` — paths like `../sha512/lib` walk up to the package root for the missing-prereq check, but exact-match for the undeclared-ref check.
- **Mass BUILD changes trigger a full rebuild.** The build tool diffs file paths; touching every BUILD in one commit forces every package to rebuild and exposes pre-existing broken BUILDs. Only edit BUILDs your PR actually needs.
- **Diff-based change detection requires a real diff.** Before `./build-tool --diff-base origin/main`, commit your changes (or verify `git diff --name-only origin/main...HEAD` returns the expected set). On hash/cache fallback the tool may attempt a monorepo-scale build — stop and clean artifacts.
- **Shared-infrastructure changes cascade.** Editing `grammar-tools`, `lexer`, etc. marks 50+ dependents for rebuild. Use `--list-affected` first; expect that any pre-existing broken BUILDs anywhere in the closure will surface.
- **Use the scaffold generator** (`code/programs/go/scaffold-generator/`) for every new package. It produces correct BUILD/BUILD_windows, metadata, leaf-to-root install order, language-specific dir naming (Ruby/Elixir/Lua use snake_case), and includes README/CHANGELOG. If output is wrong, fix the generator first.
- **Don't commit build artifacts.** After agents run tests, always `git status` for `.build/`, `.swiftpm/`, `cover/`, `_build/`, `deps/`, `node_modules/`, `.venv/`, `__pycache__/`, `blib/`, `MYMETA.*`, `pm_to_blib`, Perl-generated `Makefile`, `target/`, copied `.so`/`.pyd` files, etc. Stage by explicit path, never `git add .`. Every Swift package needs `.gitignore` with `.build/` and `.swiftpm/` BEFORE the first `swift test`.
- **Do not use `mise exec --` in BUILD files.** CI runners install language tools directly into PATH via `actions/setup-*`; they do not have mise. BUILDs that prefix `mise exec --` (or hardcode `/Users/adhithya/.local/bin/mise`) fail with `mise: not found`. Call `cargo`, `npm`, `python`, `go`, `bundle` directly — mise's local shims handle dispatch transparently. Re-learned during rebases; conflict resolution that picks the branch's `mise exec`-prefixed BUILD over main's bare-command BUILD reintroduces this break. After rebase: `git diff origin/main...HEAD -- '**/BUILD'` to verify only intentional BUILD diffs remain.
- **TypeScript program path depth.** Programs at `code/programs/typescript/<name>/` reach packages with `../../../packages/typescript/<pkg>` (three `..`, not two).
- **Don't install sibling deps in parallel** from inside a TS BUILD — two packages racing `cd ../state-machine && npm ci` corrupt each other's `node_modules` (ETXTBSY on esbuild). The build tool already handles topological order; only install what your own package needs.
- **BUILD scripts run as POSIX sh on CI, not bash.** `set -euo pipefail` errors with `Illegal option -o pipefail` on Ubuntu's `/bin/sh` (dash). Use `set -e` only; replace `[[ ]]` with `[ ]`; no arrays; no `local`. Shebangs are ignored when the script is sourced/dispatched by name. Verify locally with `sh ./BUILD`, not `bash ./BUILD`.
- **Multi-line `if`/`for`/`case` blocks inside BUILD itself break** because the build-tool dispatches each LINE through a fresh `sh -c`. Symptom: `Syntax error: end of file unexpected (expecting "fi")`. If you need a real shell script, put it in a sibling file (e.g. `tools/run-tests.sh`) and make BUILD a one-liner that invokes it: `sh tools/run-tests.sh`. Single-line `&&` chaining also works for short pipelines.
- **NEVER name a build-output directory `build/` next to a `BUILD` file on macOS/Windows.** HFS+/NTFS are case-insensitive — `build` and `BUILD` collide, so `rm -rf build` from inside the BUILD script deletes the script itself (mid-execution, with no way to recover except `git checkout HEAD -- BUILD`). Use `_build/`, `.cmake-out/`, `gradle-build/`, or any name whose case-insensitive folding doesn't match `BUILD`. This has now bitten the repo twice: Gradle (lesson #48) and CMake/C++ (mosaic-flux-qt cycle 8).

## Cross-platform & Windows BUILD_windows

- **`.venv/bin/python` does not exist on Windows; `.venv/Scripts/python` does.** In `BUILD_windows`, always use `.venv\Scripts\python` (BACKSLASHES — `cmd.exe` parses `/` as a switch and `.venv/Scripts/python` becomes command `.venv` with option `/Scripts/python`). Cross-platform alternative: `uv run --no-project python`.
- **`.[dev]` quoting on Windows.** `cmd /C` passes `"..."` literally to uv: `uv pip install -e ".[dev]"` fails with "not a valid editable requirement". Use unquoted `-e .[dev]` (with `-e`, no quotes). Dropping `-e` does a non-editable install which breaks `__file__`-based path walks (Windows site-packages depth differs from Linux).
- **`uv pip install -e ../dep -e .[dev]` can fail on Windows** (universal resolution looks at all extras and may try PyPI). Split into two commands: install local deps first with `--no-deps`, then `uv pip install -e .[dev]`. Also explicitly install pytest/ruff/mypy if needed.
- **No-runtime-dep Python packages on Windows** (e.g. grammar-tools): `uv venv --clear` creates the workspace-root venv; `uv run python -m pytest` re-syncs and removes pytest. Use `python -m venv .venv --clear` + `.venv\Scripts\pip install -e .[dev]` + `.venv\Scripts\python -m pytest`.
- **uv workspace membership on Windows** creates the venv at the workspace root, sharing it across parallel package builds (race condition wipes pytest). Don't add new packages to `[tool.uv.workspace]` members unless intentional. Fix unresolvable workspace deps by removing the offending member, not by adding the missing dep to the workspace.
- **Windows env-var syntax in BUILD_windows.** Use `set "VAR=value" && command` (defensive quoting handles `&|()` in paths/`%CD%`), NOT Unix-style `VAR=value command`. `if [ -f ]`, `elif`, `fi` all break — translate to CMD or skip on Windows.
- **`xcrun swift test` on macOS, `swift test` on Linux.** Bare `swift test` on macOS CI fails to find XCTest framework (lives in Xcode bundle). Make BUILD platform-aware: `if command -v xcrun >/dev/null 2>&1; then xcrun swift test; else swift test; fi`. Swift on Windows requires `winget install Swift.Toolchain` in the workflow.
- **Unix-only syscalls (`syscall.Stat_t`, `libc::getuid`, `libc::statvfs`) won't compile on Windows CI.** Go: split with `//go:build !windows` / `windows` and provide stubs. Rust: `#[cfg(unix)]` / `#[cfg(not(unix))]`.
- **Swift POSIX `bind` collides with `Sequence.bind`** inside closures. Wrap POSIX calls at module scope: `posixBind` → `Darwin.bind`/`Glibc.bind` via `#if canImport`. `SOCK_STREAM` is `Int32` on Darwin but a `__socket_type` enum on Glibc — use `Int32(SOCK_STREAM.rawValue)` under `#elseif canImport(Glibc)`.
- **Perl on Windows (Strawberry Perl)**: `cpanm --with-test` is a `cpm` flag, not cpanm — use `cpanm --installdeps --quiet .`. CI skips Perl on Windows entirely; provide a no-op `BUILD_windows` (`echo Perl testing not supported on Windows`) so the build tool doesn't fall back to BUILD.
- **Add `.gitattributes` `* text=auto eol=lf`** to force LF line endings everywhere. Otherwise Elixir heredoc tests, Python doctests, Ruby tests fail on Windows checkouts because `\r\n` ≠ `\n`.
- **Use body files for `gh pr` text containing Markdown backticks.** Inline backticks in `--body "..."` get evaluated by zsh as command substitution. Write to a tempfile with single-quoted heredoc and pass via `--body-file`.
- **`git worktree add` inherits HEAD unless you pin the base.** Always `git worktree add <path> -b <branch> origin/main`. Whenever the source checkout is shared or noisy, default to a fresh worktree from `origin/main` to avoid accidentally committing other agents' files or shared-manifest pollution.
- **`git worktree add` on this repo can exceed a 2-minute Bash timeout** (tens of thousands of tracked files across 4800+ packages) and gets killed mid-checkout, leaving 60k+ files showing as deleted in `git status` and a stale `.git/worktrees/<name>/index.lock`. Fix: confirm no real git process is running (`ps aux | grep git`), `rm -f` the stale `index.lock`, then re-run the checkout with a long timeout: `git checkout HEAD -- .` (pass `timeout: 480000` or similar to the Bash tool). Better: pass a generous timeout to the original `git worktree add` call itself rather than letting it hit the default. **A second symptom of the same root cause**: if the timeout kills a `for`-loop of several `git worktree add` calls mid-loop, the interrupted one shows up in `git worktree list` as `locked` with reason `"initializing"`, not just a stale lock file — `git worktree remove --force` refuses to touch a locked worktree. Fix: `git worktree unlock <path>` first, then `git worktree remove --force <path>`, delete the orphaned branch (`git branch -D <branch>`), and recreate it as its own separate command rather than looping several `git worktree add` calls together.

## Workspace & package metadata

- **Rust workspace `Cargo.toml` `members` must match what's pushed.** Listing a member whose dir hasn't been pushed breaks the entire workspace in CI (`failed to load manifest`). Crates with their own `[workspace]` (node-bridge, python-bridge, ruby-bridge) must be EXCLUDED from the parent — including them gives "multiple workspace roots". After merge conflicts on `members`, dedupe — modern CI rejects duplicate entries even though older Cargo tolerated them. Run `cargo build --workspace` to catch missing exports; expect platform-only crates (paint-vm-direct2d, paint-vm-gdi) to fail compile on the wrong OS — that's not a regression.
- **Keep the Rust toolchain current.** External deps adopting Edition 2024 require `rustup toolchain install stable` before declaring the workspace broken.
- **Don't put `@ file:../path` in Python `pyproject.toml` dependencies.** Hatchling rejects them, and even with `allow-direct-references = true`, uv resolves the relative path from a temp build dir. Use bare names + BUILD pre-installation + `[tool.uv.sources]` for local-path redirection.
- **Python downstream tests should not assert exact dependency versions.** Assert minimum-compatible (`__version__ >= "0.3.0"`) or capability — exact-version asserts fail when a foundational package bumps and downstream gets force-rebuilt.
- **TypeScript `package.json` must use `"main": "src/index.ts"`** (not `dist/index.js`) because Vitest resolves `file:` deps via `main` and we don't pre-compile. Also: `"type": "module"`, `@vitest/coverage-v8` in devDeps, run real coverage gate locally before pushing. Never commit `.js`/`.d.ts` transpile outputs alongside `.ts` sources.
- **Vite-based TS programs with `file:` deps must NOT use `tsc -b` in build script.** `tsc -b` follows imports into nested `node_modules` (npm copies, not symlinks on Windows) and fails on un-installed transitives. Use plain `vite build`; type-check via vitest.
- **Clean Vite deploys with source-level `file:` dependencies must install every local source package in dependency order.** Vite follows `main: src/index.ts` links back into the repository, so each package needs its own `node_modules` on a fresh runner. `npm install --install-links` is not a portable shortcut: npm 11 packed the recursive graph in a Windows repro, but GitHub's Node 20/npm 10 runner stopped at a nested `file:` dependency with ENOENT. Mirror the full production closure explicitly, then run the app's normal install and prove it on the deployment runner.
- **Do not label a forward-only runtime as a training compiler.** Use the real
  runtime for saved forward evidence, then name a new language-neutral
  backward/optimizer contract explicitly. Keep backward, gradient reduction,
  optimizer update, and zeroing as separate observable operations until the
  production runtime implements those contracts. Prove persistence with a
  nonzero incoming gradient buffer: reduce the current batch separately, add it
  to the prior buffer, and finite-difference only the current batch loss.
- **Backend parity needs distinct evidence levels.** A deterministic oracle,
  a native fixture test, and a live accelerator probe answer different
  questions. Treat an unavailable accelerator as an honest result, never label
  a CPU oracle as GPU execution, keep MatrixIR and byte payloads language-
  neutral, and test a Node-free Rust helper without forcing host-linked N-API
  symbols into standalone Windows binaries.
- **Haskell `cabal.project` must list every transitive local package.** Cabal does not discover sibling deps from a sibling's own `cabal.project`. Single-package validation: plain `cabal test` (NOT `cabal test all`, which builds the whole universe).
- **Haskell record accessors share the module's top-level namespace.** A field such as `requestBodyKind` creates a function with that exact name, so a private helper with the same spelling fails with `Multiple declarations`. Name decision helpers distinctly (`determineRequestBodyKind`) before compiling.
- **HTTP/1 parity must preserve fail-closed wire grammar, not legacy parser permissiveness.** Reject TE+CL ambiguity, conflicting duplicate lengths, non-final request chunking, whitespace before a field colon, variable start-line delimiters, and unbounded heads; response framing must receive HEAD/CONNECT request context, and parse errors must not retain raw targets or field values.
- **Add `gradle-build` directory override** in every Java/Kotlin `build.gradle.kts`: `layout.buildDirectory = file("gradle-build")` BEFORE the plugins block. Gradle's default `build/` collides with the `BUILD` file on case-insensitive filesystems (macOS/Windows) and explodes with `Could not create problems-report directory`. Also: don't pin `java { toolchain { languageVersion } }` — let Gradle use the running JDK so CI's `actions/setup-java` is honored.
- **JVM composite Gradle BUILDs need a shared lock** when multiple packages reuse the same included builds — parallel runs corrupt shared `gradle-build` class outputs. Use `--no-daemon --no-build-cache --max-workers=1` plus a repo-local file lock.
- **Lua rockspecs must pin immutable refs** (release tag or commit SHA) over `https://`, never moving branch tips. Patch flaky LuaRocks GitHub-archive URLs to the stable `archive/refs/tags/<tag>.tar.gz` form during CI install.

## Python

- **Use `.venv/bin/python -m pytest` in BUILD, never `python3.12` or `python3` directly.** System Python on CI has no deps installed.
- **`uv venv` must use `--no-project`** so it creates a package-local `.venv` instead of finding the workspace root. Pattern: `uv venv .venv --quiet --no-project` then `uv pip install --python .venv ... --quiet` then `uv run --no-project python -m pytest`.
- **Newer uv rejects quoted extras** `".[dev]"` — use unquoted `.[dev]` everywhere (`-e .[dev]`).
- **`uv pip install` on one line, no backslash continuations.** Backslash gets appended to a path producing `file:///path/%5C` and "Distribution not found" on Ubuntu CI.
- **Don't pass `--no-deps` when tests need `[dev]` extras** — that flag suppresses the optional groups too, leaving you without pytest at test time.
- **Python Enum rejects invalid values** (`MyEnum(99)` raises `ValueError`). For "not found"/"invalid" tests use `None` or sentinels, not arbitrary ints. Use `IntEnum` if you need int compatibility.
- **Reject negative indexes explicitly** in bytecode/constant-pool decoders. Python sequence indexing accepts negatives as offsets-from-end; `IndexError` alone won't catch malformed `operand=-1`.
- **Test imports for assertion-only types are required** — pytest doesn't pick up `LogicVar` from sibling tests; every isinstance/equality target needs its own `import`.
- **Run ruff** before assuming hand-sorted imports are correct, especially around similarly named symbols (`callo`, `callableo`, `clauseo`).
- **Parsing pyproject.toml with regex is brittle.** Comment lines containing `[` break naive `[^[]*?` cross-line patterns. Parse line-by-line, skip `#` lines, track section headers explicitly.
- **Mocked wrapper tests for native packages** — when native smoke tests skip on the wrong platform, the Python facade's wrapper logic still needs coverage from mocked tests, or coverage gates fail off-platform.
- **Compiler-generated data segments need source-stage byte caps.** AST-depth and source-size limits don't bound semantic frame plans or generated runtime images. Cap at the earliest stage that computes the size.
- **JVM multi-module: exported functions in dep modules need `extra_callable_labels`.** `_discover_callable_regions` builds callable names from `CALL` instructions; exported functions that are only called cross-module have no local callers, so they are silently omitted from the class file → `NoSuchMethodError` at runtime. Pass `module.program.module.exports` as `extra_callable_labels` in the JvmBackendConfig.
- **JVM multi-module: all `__ca_regs` references must use `_reg_owner`, not `self.config.class_name`.** Any `field_ref(self.config.class_name, "__ca_regs", ...)` inside helper methods (`__ca_syscall`, etc.) that was added before `external_runtime_class` was introduced must be updated to `_reg_field_ref(...)`. Two `getstatic` calls in `_build_syscall_method` (SYSCALL 1 write-byte, SYSCALL 10 exit) were missed and caused `NoSuchFieldError` at runtime in multi-module mode.
- **JVM cross-class invokestatic requires `ACC_PUBLIC`** — a method tagged `ACC_PRIVATE` on class A cannot be called by class B even via `invokestatic`. The JVM raises `NoSuchMethodError` at runtime (not a compile-time error). In multi-module mode set all callable methods to `ACC_PUBLIC | ACC_STATIC`.

## Ruby

- **Predicate methods use `?` suffix.** `contains?`, `empty?`, `valid?`, `halted?`, `idle?`. Tests calling `obj.contains("x")` raise `NoMethodError` — must be `obj.contains?("x")`.
- **`include` inside a method body raises NoMethodError** — it's a class-level operation. Either include at class scope, or use fully-qualified constants like `CodingAdventures::SystemBoard::PHASE_NAMES`.
- **Require ordering matters.** Ruby loads files in order — if a config file references `RomBios::BIOSConfig`, `require "coding_adventures_rom_bios"` must come BEFORE `require_relative` of your own modules in the entry point.
- **Module naming: `StarlarkVM` not `StarlarkVm`.** Verify the exact constant by reading the gem's entry-point file before referencing it.
- **`spec.add_dependency`** in `.gemspec` — block var must be `spec`, not `s` (the build-tool regex requires `spec.`).
- **`bundle install` requires `mise.toml`-managed Ruby** (project requires 3.4+; system Ruby is 2.6.10). Locally: rely on mise shims (no `mise exec --` prefix). Building Ruby 3.4 from source on macOS needs `brew install libyaml` and `RUBY_CONFIGURE_OPTS="--with-libyaml-dir=/opt/homebrew"` on Apple Silicon; mise's `ruby.compile=false` does not yet use precompiled binaries.

## Lua

- **Every Lua test file MUST set `package.path` before `require`** — even with rockspec installed:
  ```lua
  package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
  ```
  This is NOT optional, especially on Windows CI where rockspec install does not put modules into the default search path. Re-learned multiple times.
- **`^` returns float in Lua 5.4** — `2^24` is `16777216.0`, fails `math.type(x) == "integer"` checks. Use `1 << 24` (bitwise ops always return integers).
- **`\v` and `\f` aren't recognized inside character classes** in Lua's regex engine — they're matched literally. Lua lexers loading `.tokens` grammars must replace them with actual control chars before parsing: `content:gsub("\\v", "\x0B"):gsub("\\f", "\x0C")`.
- **Lua test BUILDs must install LuaRocks deps** declared in the rockspec (`luasocket`, etc.) before invoking busted. Native deps may fail to compile on Windows; gate with `BUILD_windows` no-op.
- **Lua sibling rocks: invoke their `BUILD`s, don't `luarocks make` them directly** — they may depend on other unpublished local rocks. For grammar-driven lexer tests, prepend sibling `src/` dirs to `package.path` so in-repo `.tokens` files resolve over installed rocks.
- **`--deps-mode=none` consistency.** If your BUILD bootstraps sibling rocks first, the final `luarocks make` should also use `--deps-mode=none`. Don't bootstrap rocks that your tests reach via `package.path` rather than declared rockspec deps.
- **Lua decoder hygiene.** After block-loop exits (`last_block == 1`), assert read cursor equals input length — silently ignoring trailing bytes hides truncation/concatenation bugs.

## Perl

- **`reverse @list, $extra` reverses BOTH** — Perl precedence parses it as `reverse(@list, $extra)`. Use explicit double parens: `((reverse @list), $extra)`.
- **Perl modules must `use lib '../sibling/lib'`** themselves, not just from test files. `prove -l` only adds local `lib/`, and `use lib` in tests doesn't help if the module compiles `use Sibling::Module` at compile time.
- **`unpack('C*', $buf)` amplifies memory ~56× per byte** (Perl scalar header). Always validate `length($buf)` against a hard cap (64 MB is a safe default) before unpacking caller-supplied data.
- **`~$x` is 64-bit on 64-bit Perl.** Always mask: `(~$x) & 0xFFFFFFFF` for 32-bit arithmetic (MD5, bitsets).
- **`>>` is not arithmetic right shift on negatives.** `(-1 >> 7)` is a huge positive. Use `floor($x / 128.0)` from `POSIX` for signed shifts (LEB128, etc.).
- **`Test2::V0` does not export `use_ok`.** Replace with `ok( eval { require Module::Name; 1 }, '...' )`.
- **JSON null comes back as `JsonValue::Null` blessed object, not `undef`.** Use `JsonSerializer::is_null($v)` to normalize. Tests asserting `$v == undef` fail.
- **VMs that swap programs at runtime must re-read `$vm->{_program}` each step** — capturing the original code list once causes calls to loop in the caller after a context handler switches programs.
- **The build-tool's validator requires perl BUILD files to textually reference every transitive local prerequisite's path** (perl is in `requiresExplicitPrereqs` in `validator.go`, alongside python/typescript) — it scans the literal command text in `BUILD` for `../relative/paths` and resolves them against known packages; `use lib` statements inside `.pl`/`.t` files (even via `FindBin`) are invisible to it. Symptom: CI fails with `missing prerequisite refs for standalone builds: perl/foo, perl/bar` even though the program runs fine locally and its tests pass. Fix: add matching `-I../../../packages/perl/foo/lib` flags directly to the `prove`/`perl` invocation in `BUILD`, mirroring the existing `paint-vm-ascii` package's own BUILD. Verify locally by building `code/programs/go/build-tool` and running `./build-tool -root . -diff-base origin/main -validate-build-files -detect-languages -emit-plan plan.json` — confirm your package no longer appears in the failure list (ignore pre-existing failures in unrelated packages).
- **A literal backslash as the last character before a single-quoted string's closing `'` is silently absorbed as an escaped quote**, not "backslash then end of string" — `'...)\/\'` does NOT end where it looks like it does; Perl keeps scanning for the real closing quote, silently swallowing subsequent lines into the string literal and producing a confusing downstream syntax error many lines later (not at the actual mistake). Any content ending in a real backslash needs `\\` before the closing quote: `'...)\/\\'`. Bit a cowsay `.cow`-art string embedded in a test file (`'            (__)\       )\/\'` needed to become `'...\\'`). When embedding cow-art or other backslash-heavy literal text in Perl single-quoted strings, check for a lone (un-doubled) `\` immediately before the closing `'`.

## Elixir

- **Reserved words can't be variables**: `after`, `rescue`, `catch`, `else`, `end`, `fn`, `do`, `when`, `cond`, `try`, `receive`. Rename when porting (`after` → `rest`, etc.).
- **Ranges `0..(n-1)` default to step `-1` when `n=0`** — iterates `[0, -1]`. Always use explicit step `0..(n-1)//1`. Ascending range `0..-1//1` is correctly empty.
- **`if` expressions return values that are silently discarded** if not bound. `if cond do compiler = ...; compiler end` discards the rebinding — wrap as `compiler = if cond do ... else compiler end`.
- **Coverage thresholds (80%) include delegate helpers and error branches.** Test those, not just the happy path, or coverage drops to low 70s.
- **Don't commit `cover/` HTML output** — every Elixir package needs `.gitignore` with `cover/`, `_build/`, `deps/`, `.elixir_ls/`. Stage explicitly.
- **NIF module names use the full Elixir atom format**: `b"Elixir.CodingAdventures.GF256Native\0"`, not `"gf256_native"`. Otherwise Erlang raises `:bad_lib`.
- **Don't use `:make` compiler in `mix.exs` when BUILD compiles the NIF externally.** Mix tries to load `Mix.Tasks.Compile.Make` before `elixir_make` is built from deps and exits non-zero on the very first `mix` command. Just `cargo build --release` from BUILD and copy `.so` into `priv/`.
- **GenericVM handlers must call `advance_pc`** at the end, or the VM loops forever. Exceptions: `HALT`, unconditional `JUMP` (uses `jump_to`), conditional jumps (advance OR jump, never both).

## Swift

- **Every Swift package must `.gitignore` `.build/` and `.swiftpm/` BEFORE the first `swift test`.** The directories contain thousands of deeply nested files that break Windows CI with "Filename too long".
- **`XCTestCase` (via `NSObject`) shadows module-level `load`.** Always qualify: `FontParser.load(...)` instead of `load(data)` inside test classes.
- **`GrammarLexer` emits `KEYWORD` for all keywords** with the actual word in `value`. Swift lexer wrappers must promote: map `KEYWORD` to `token.value.uppercased()` as the type.
- **Redeclaring a `let` binding** (e.g. when adding overflow-safe `multipliedReportingOverflow` for `bLen`) fails compile — remove the original.
- **F# interpolated strings break on quoted literals inside expressions.** Bind with `let` first, or switch to `sprintf` for dense XML/HTML attributes.

## C#

- **`CliBuilder.Parser`'s `argv` follows the C/Go convention where index 0 is the program name** (`Parser.Parse()` sets `program = argv[0]` and starts real token parsing at `index = 1`). C#'s top-level `args` array does **not** include the program name — passing it straight to `new Parser(specPath, args)` silently drops the first real CLI token (single-arg invocations parse zero positional arguments; multi-arg invocations lose the first one). Symptom is silent: no exception, just an empty/short result. Fix: prepend a placeholder before calling, `var argv = new List<string> { "<program-name>" }; argv.AddRange(args);`. Found while wiring the C# `cowsay` port to `cli-builder` (first C# consumer of `Parser` outside its own test suite) — verify with an end-to-end run (`dotnet run -- <realistic args>`), not just unit tests that call `Parser` with a hand-built `argv` list, since it's easy to hand-build the list "correctly" (with a leading program name) in a test and then get the real entry point wrong.

## Haskell

- **`CliBuilder.parseArgs`'s `argv` also follows the C/Go convention where index 0 is the program name** (`parseArgs (Parser spec) args` pattern-matches `program : argv` and errors `"argv must have at least one element (the program name)"` on an empty list). `System.Environment.getArgs` does **not** include the program name — pass `"cowsay" : args`, not `args` directly. Same pitfall as the C# port (see `## C#` above); confirmed independently by reading `CliBuilder.hs`'s `parseArgs` rather than assuming from Perl's precedent (Perl's `CliBuilder` is the one exception in this repo — its `parse` iterates the whole array from index 0, no placeholder needed).
- **`cabal test` fails locally in this sandbox** with `ghc-pkg-9.4.8.exe: ...package.conf.inplace\: openBinaryTempFileWithDefaultPermissions: invalid argument` — confirmed via `git stash` that this reproduces on unmodified/original package code too, so it's a pre-existing environment issue (likely OneDrive-sync interference with the deeply-nested `dist-newstyle` path), not a real code bug. `cabal build` (compile + link, including the test suite's own component) works fine; only the package-registration step for local "inplace" packages fails. Locally, rely on `cabal build cowsay:test:spec` (or the equivalent target) for type-check confidence, then run the built test `.exe` directly if you need to see it execute, or trust CI (which doesn't hit this OneDrive-path issue) for the official `cabal test` run.
- **`Prelude.readFile` is lazy and can leave the file handle open past the point the caller thinks it's done.** `loadCow`'s `contents <- readFile cowPath; pure (extractHeredocBody contents)` compiled and worked, but a test that immediately `removeDirectoryRecursive`s the temp dir the file lives in intermittently hit `PermissionDenied: ... DeleteFile ...: The process cannot access the file because it is being used by another process` on Windows — the handle hadn't been finalized/closed yet even though the returned `String` looked fully consumed. Fix: use `System.IO.readFile'` (the strict variant, base >=4.15/GHC 9.0+) instead of `readFile` for any file this repo's code reads once and expects to be free of afterward (matches the existing project's minimum bound of `base >=4.14`, so confirm the GHC version actually in use ships strict `readFile'` before relying on it elsewhere).
- **A test helper that walks up from `"."` looking for a sentinel file needs an ABSOLUTE starting directory, not the literal string `"."`** — `takeDirectory "."` returns `"."` again, so a loop like `findRepoRoot` that stops when `takeDirectory dir == dir` returns immediately without ever climbing real directories. The production entry point (`Main.hs`) got this right (`cwd <- getCurrentDirectory; findRepoRoot cwd`), but a test that shortcuts to `findRepoRoot "."` silently no-ops and then fails downstream with a confusing "file does not exist" instead of the real problem. Always resolve `getCurrentDirectory` first in tests too.
- **"Finite" is not "safely convertible to `Int`."** A five-round `/security-review` loop on `paint-vm-ascii`'s new `line`/`clip`/`glyph_run` support kept finding narrower variants of the same bug class: `toCell coordinate scale = round (coordinate / scale)` had no NaN/Infinity check at first (round 1: DoS via unbounded Bresenham recursion on `Infinity`), then gained per-instruction NaN/Infinity checks (round 2-3) and per-axis scene-size caps (round 3-4) — but an *ordinary, finite* `Double` around `6.6e35` (nowhere near `Double`'s own ~1.8e308 range limit) still rounds to exactly `minBound :: Int`, and that value survived every "is it NaN/Infinite" guard. It then broke a downstream invariant (`clMaxCol - 1` silently Int-wrapping from `minBound` to `maxBound`, un-clamping a nested shape's fill range) — the same DoS class reopened through integer wraparound instead of floating-point non-finiteness. The fix that actually closed it: make the *one* function that converts `Double` coordinates to `Int` cell indices (`toCell`) saturate its output to a fixed bound (`±1e9`, comment explains why that's both large enough for any legitimate scene and small enough that no downstream `±1`/`min`/`max` on it can approach `Int`'s real ~±9.2e18 range), instead of patching every call site that happened to feed it an extreme value. When a codebase converts an unbounded external `Double`/`Float` into an `Int` used for allocation, iteration bounds, or array indexing, validate finiteness AND clamp magnitude at the single conversion point, not per-caller — chasing individual "large value" call sites is how a security review loop keeps finding "one more" variant of the same root cause.
- **A shared sum type's constructors can be extended safely under `-Wall` without `-Werror`** — adding 5 new `PaintInstruction` constructors (`PaintGlyphRun`, `PaintLine`, `PaintGroup`, `PaintClip`, `PaintLayer`) to `paint-instructions` left every existing non-exhaustive `case`/pattern-match elsewhere in the repo (barcode/qr-code packages) compiling with only a missing-pattern *warning*, not an error. Still, grep every consumer for direct `PaintRect{...}`-style record construction (not just pattern matches) before extending a shared type — a producer that builds records positionally or partially would break, even though pattern-match consumers wouldn't.

## TypeScript / JavaScript

- **JS bitwise ops are signed 32-bit.** `1 << 32 === 1` (shifts are mod 32) — guard `bitWidth >= 32` separately. `0xFFFFFFFF & 0xFFFFFFFF === -1` — use `>>> 0` to convert to unsigned: `(value & mask) >>> 0`. Critical for register files / ALU / addressing.
- **Vitest stubbing `crypto`** must include `getRandomValues` and `subtle` from `node:crypto.webcrypto`, bound via arrow function (NOT `{...webcrypto}` — methods are on prototype and need internal-slot `this`):
  ```ts
  vi.stubGlobal("crypto", {
    randomUUID: () => "mock-uuid",
    getRandomValues: (b) => webcrypto.getRandomValues(b),
    subtle: webcrypto.subtle,
  });
  ```
- **Vitest coverage includes build scripts by default.** Add `"scripts/**"` to `coverage.exclude` alongside `dist/**`, `vite.config.ts`.
- **CI is ~25× slower than local for compute-heavy tests.** Vitest's 5s default times out on 200KB+ LZSS round-trips. Set explicit `30_000` ms timeout for tests that exercise large compression/LZ77 passes.
- **TSDoc `@example` blocks must not contain unescaped glob `**`** — esbuild errors on the `*` after `*/`. Use `"src/*.py"` or backtick code fences.
- **Hand-rolled parsers walking attacker-controlled keys MUST defend against prototype pollution.** `target[seg] = value` resolves `target["__proto__"]` to `Object.prototype` and the subsequent write pollutes the global prototype. Two-layer fix: (1) reject `__proto__`/`constructor`/`prototype` segments at the lex layer with an explicit denylist; (2) construct every internal table as `Object.create(null)` so even if the denylist is bypassed there's no prototype chain to walk. Caught in `forme-manifest` parser, retroactively applied to the JSON Schema validator in `forme-pipeline-config`. Test: `Object.keys(Object.prototype).length` before/after parsing must be equal.
- **`Object.prototype.hasOwnProperty.call(obj, key)` not `key in obj` when checking attacker-controlled keys.** `in` walks the prototype chain, so `"toString" in {}` is `true` (via `Object.prototype.toString`). In a JSON Schema validator this means `required: ["toString"]` passes vacuously and `additionalProperties: false` with empty `properties: {}` accepts `{ toString: "x" }`. Bypass found in `forme-pipeline-config`; fix uses a `hasOwn(obj, key)` helper at every `key in` call site that touches user data.
- **`String.prototype.replace` with a STRING second arg honours `$&`, `$1`, `$<name>`, `$$`.** A frontmatter slug like `"$&"` injected via `template.replace(/\{slug\}/g, slug)` expands to the regex match (`"{slug}"`) — unintended substitution. Fix: pass a function replacement (`() => slug`), immune to `$`-parsing. Found in `forme-router`, `forme-collect-chronological`, `forme-render-static` (same bug pattern across all three from copy-paste).
- **Recursive walkers on attacker-controlled input need a depth bound.** `walk()`/`deepEqual()` style functions on JSON Schemas / validator inputs / parsed TOML can crash with `RangeError` on adversarial 10k-deep nesting. Thread an explicit `depth` counter through the recursion and short-circuit at `MAX_WALK_DEPTH = 256` (well beyond any sane input). Never throw — push a synthetic violation/finding and return.
- **`new RegExp(userControlledPattern)` is a ReDoS vector.** Cap pattern length (`MAX_PATTERN_LENGTH = 1024` is generous for any real schema) before construction. Document the cap; treat oversize as a validation failure, not an exception. Found in `forme-pipeline-config`'s JSON Schema `pattern` keyword.
- **Per-spec security review BEFORE pushing catches things linters miss.** The `/security-review` skill spawns a sub-agent to audit every diff; ~50% of overnight PRs surface findings (most LOW/INFO, occasionally MEDIUM, rare CRITICAL) the main agent didn't see. Treat as mandatory pre-push, not optional.
- **A numerical fixture validator must pin its tolerance and validate derived finiteness.** A caller-controlled `absolute_tolerance` with only a `> 0` check can be raised until a dishonest oracle passes. Require the corpus's canonical tolerance in both schema and executable validator. Likewise, checking that inputs are finite is not enough: addition, multiplication, reductions, and finite differences can overflow into `Infinity` or `NaN`. Bound teaching inputs at the runtime boundary, verify arrays before invoking array methods, catch host-language numeric conversion overflow, and assert every derived output, gradient, score, and audit error remains finite. This was caught in the NN26 tensor-broadcasting pre-push review.
- **Python `math.isfinite(huge_int)` can raise `OverflowError` before a validator rejects the value.** Check an integer's absolute magnitude before asking a float-oriented finiteness helper to convert it. Keep the `isfinite` call for bounded floats. Caught by the NN29 hostile thousand-digit input regression.
- **Fixture document IDs and executable node IDs can require different grammars.** Corpus IDs commonly use descriptive hyphens (`nn27-dynamic-graph-and-saved-values`), while graph node IDs benefit from a tighter identifier grammar for portable map keys. Reusing the node-ID regex for the top-level lab ID rejected a valid checked-in fixture before execution. Validate each namespace according to its contract instead of sharing the strictest helper by convenience.
- **Testing Library button names include all descendant text unless an explicit accessible name is provided.** A scenario button containing a title and explanatory `<span>` is named `"Title explanation"`, so an exact `{ name: "Title" }` query fails even though the visible title is correct. Use a deliberate `aria-label` when the compact name is part of the UI contract, or use a sufficiently specific regex when the description should remain in the accessible name. Caught while adding NN28 gradient-buffer scenario tests.
- **`grid-template-columns` has no effect until the element is a grid container.** A new NN28 workbench inherited spacing from `.workspace` but not `display: grid`; computed columns looked correct in devtools while the stage and sidebar still stacked at full width. Declare `display: grid` on each standalone workspace variant and assert both computed display and element rectangles during desktop browser QA.
- **Windows path separators in `path.join` output break POSIX-only test assertions.** `f.endsWith("/foo.md")` against `path.join(root, "foo.md")` succeeds on Linux/macOS, fails on Windows where `\` is the separator. Fix: normalise the assertion with `.split(/[/\\]/).join("/")`. Pre-existing bugs in this pattern can hide if the package's BUILD never runs on Windows CI (the build-tool only runs BUILDs for *changed* packages, so dormant tests stay dormant).
- **`String.prototype.replace` regex flags: include `g` when replacing every occurrence.** Without `g`, only the first match is replaced — common foot-gun when the pattern looks deceptively "all-occurrences." Use `/pattern/g`.
- **Multi-pass validator pattern (collect all violations, throw once).** `validateConfig` / `validateManifest` / `validateStyleDocument` should aggregate every violation into a single error rather than throw on the first. Users want the full punch list; chasing errors one at a time is the slowest possible feedback loop. Pattern established in FM03 §2.4's `ConfigError` and adopted across FM02 (`ManifestError`), FM03 (`ConfigError`), FM04 (`StyleError`).
- **Repo-root `.gitignore` has `compile.*` (Erlang/Elixir BEAM artefact pattern, line 89) that silently eats TS files named `compile.ts` / `compile.test.ts`.** `git add` skips them without warning; `git commit` succeeds without the files; tests pass locally because the file exists on disk. Always `git ls-files` after a fresh-package commit to verify all source files are tracked. Workaround: rename to `orchestrator.ts` / `pipeline.ts` / `runner.ts` — avoid the `compile.*` collision entirely. Found in `forme-style-orchestrator`.

## Rust

- **Recursive local functions need a 2-step declaration.** Short assignment `addConstant := func(...)` can't reference itself. Use `var addConstant func(...)` then `addConstant = func(...)`. (Same pattern in Go.)
- **Validate caller-controlled lengths before `int` casts.** Binary parsers must explicit-bounds-check `u4`/`u8` lengths against host capacity; never recursively decode nested structures unless the format requires it.
- **Don't run `cargo fmt --all` for package-scoped work** — it reformats hundreds of unrelated crates and buries the feature diff. Use `cargo fmt -p <pkg>`.
- **`cargo fmt -p moslayout-compiler` is especially destructive: it rewrites the GENERATED `src/_grammar.rs` (600+ lines) AND explodes the hand-formatted compact `PRIMITIVES` array (several entries per line) into one-per-line.** Adding two UI35 primitives — a genuinely +15-line change — produced an 837-line diff across a generated file and a roster nobody asked me to reformat. Verify with `git diff --stat`: a purely additive registration should show insertions only. Fix: `git checkout --` both files and re-apply the addition by hand in the file's existing compact style, skipping `cargo fmt` for this crate entirely. Same family as the `adj-lang` generated-grammar lesson below and the `cargo fmt -p <pkg>` scope lesson.
- **`cargo fmt -p <pkg>` is still not safe — it reformats files *inside that package* you never touched, and `main` is not necessarily clean under YOUR local rustfmt.** Hit twice on `task-core`: adding a struct to `model.rs` and running `cargo fmt -p task-core` also rewrote `scheduler.rs` (~30 lines of match-guard/assert re-wrapping) because the local rustfmt version disagrees with whatever formatted main. That churn is unrelated to the change, invites a "why is scheduler.rs in this diff?" review, and risks conflicting with concurrent PRs. **Always `git diff --stat` right after `cargo fmt -p <pkg>` and `git checkout -- <file>` anything you didn't intend to touch**, then re-run the tests. Corollary: don't "fix" a fmt diff in a file your change doesn't own — CI does not run `cargo fmt --check` as a blocking gate, so leave main's formatting alone. (Same shape as the generated-file lesson below: fmt, then selectively revert.)
- **wasm-bindgen `JsValue::from_str` aborts on native test targets.** Gate behind `#[cfg(target_arch = "wasm32")]`; use `JsValue::NULL` placeholders for native error-path tests.
- **FFI input enums must be primitive ints, never `repr(C)` Rust enums.** Foreign callers can pass any bit pattern; observing an out-of-range Rust enum is UB before validation runs. Use `u32`/`c_int` in the ABI struct, then `TryFrom`.
- **Linux `epoll_event` is packed.** A plain `#[repr(C)]` mirror works for single events but corrupts/drops readiness when `epoll_wait` returns multiples. Always use the kernel's packed layout.

## Native extensions & FFI

- **Do not drive a Lua state from the test runner while native worker threads
  invoke callbacks on that same state.** A Rust mutex can serialize the worker
  callbacks with each other, but it cannot guard ordinary Lua execution in the
  parent test thread. The result is nondeterministic stack corruption and
  SIGSEGVs. Exercise a foreground native server in a dedicated Lua child process
  and drive it over TCP from the parent.

- **Ruby `QNIL = 0x04` on 64-bit Ruby (USE_FLONUM), not `0x08`.** The pre-FLONUM `0x08` causes Ruby to dereference it as an object pointer (klass at `+8` → SIGSEGV at `0x10`). Constants: `QFALSE=0x00, QNIL=0x04, QTRUE=0x14, QUNDEF=0x24`. Confirm against `ruby/internal/special_consts.h`. When a Ruby native ext SIGSEGVs at low addresses like `0x10`, suspect a special-constant bit-pattern bug.
- **Lua 5.4 `LUA_REGISTRYINDEX = -1_001_000`** (derived from `-LUAI_MAXSTACK - 1000`), NOT the Lua 5.1 value `-10000`. Using `-10000` in `luaL_ref` treats it as a regular negative stack index, landing 10000 slots below the frame and causing SIGBUS/SIGSEGV.
- **Lua userdata GC + raw `luaL_ref` integers**: integer slots aren't tracked by the GC. If Rust holds `i32` registry refs derived from a userdata's state, pin the userdata itself in the registry (extra `lua_pushvalue` + `luaL_ref`) and unref it only after all integer refs retire — otherwise Linux's aggressive incremental GC collects the parent and your slots become nil mid-flight.
- **Lua `__gc` metatable attachment**: do NOT `push_cstr("__gc")` before `lua_rawset_str_top` — the function supplies the key. Pattern: `luaL_newmetatable; lua_pushcclosure(gc_fn); lua_rawset_str_top(-2, "__gc\0"); lua_setmetatable(-2)`.
- **CPython type-slot numbers must match `Include/typeslots.h` exactly.** Wrong slots cause silent memory corruption / `UnicodeDecodeError` / access-violation crashes during module load. Numbers are NOT sequential per category; verify each. Examples: `Py_tp_hash=59`, `Py_tp_iter=62`, `Py_nb_and=8`, `Py_nb_or=31`.
- **Python C API `long` is `c_long`** — on Windows x64, `c_long == i32`, not `i64`. Always use `std::ffi::c_long` for `PyLong_AsLong`/`PyLong_FromLong`/`PyModule_AddIntConstant`. Hardcoding `i64` fails Windows compile.
- **OTP 26 Linux**: `enif_get_int64`/`enif_make_int64` are NOT reliably exported from `beam.smp` — declaring them gives `undefined symbol` at NIF load. Use `enif_get_long`/`enif_make_long` (always exported); on 64-bit POSIX they're equivalent.
- **OTP 25+ BEAM file format (`ir-to-beam` / `encode_beam`)**: The `ir-to-beam` encoder currently produces **pre-OTP-25 format** files that OTP 28 rejects with `"This BEAM file was compiled for an old version of the runtime system"`. Three things are required for OTP 25+ compatibility: (1) an `Attr` chunk (ETF-encoded `[]`), (2) a `CInf` chunk (ETF-encoded `[]`), (3) a `Meta` chunk (ETF-encoded `[{enabled_features, []}]` = bytes `<<131,108,0,0,0,1,104,2,119,16,...,106,106>>`), AND (4) `AtU8` chunk with a **negative count** as the first 4-byte field (e.g. count = -N → `0xFFFF_FFFB` for N=5) followed by compact-term-encoded atom lengths. `beam_lib.erl` distinguishes old format (positive count) from new long-atom format (negative count, `signed-integer`). Until `encode_beam` is fixed, any test that writes `.beam` and calls `erl` must be marked `#[ignore]`.
- **N-API cdylib link flags must come from the cdylib crate's own `build.rs`,** NOT from a bridge dep — `cargo:rustc-cdylib-link-arg` does not propagate. On macOS, every `.node`-producing crate needs its own `build.rs` emitting `-undefined dynamic_lookup`.
- **`napi_create_threadsafe_function`**: pass C `NULL` (`ptr::null_mut()`), not `napi_get_undefined()`, for `async_resource` — Node v25 checks `IsObject()` and JS undefined isn't an Object → `napi_invalid_arg`. When using a custom `call_js_cb`, also pass `func = NULL` and carry the JS function via the `context` pointer as an `napi_ref`.
- **WASI / WASM host-side bounds.** `iovs_len`, per-buffer length, total read/write bytes, `random_get` `buf_len`, function arity, data-segment sizes — all are guest-controlled and must be capped before allocation, slicing, or invoking host providers. Validate every length against remaining section bytes AND a package-level max.

## Compiler / VM / language pipeline

- **Compiler runtime specs need execution fuel, call-depth limits, frame-stack/heap byte caps, and explicit captured-environment lifetime rules** before implementing recursion, closures, thunks. Source-size and AST-depth limits alone are insufficient. Either reject escaping descriptors or heap-lift captured envs.
- **Runtime failure paths must unwind activation state.** Inside a procedure, an array-bounds or heap-exhaustion guard that just `RET`s skips frame/heap restoration normally done by the success path. Add cleanup symmetric with the success return.
- **Conservative call-by-name analysis must track lexical procedure shadowing.** A nested procedure shadowing a known read-only one can write through a by-name formal while the outer one stays marked read-only.
- **CALL_FUNCTION stack order: closure on top, args below.** Pop closure FIRST, then args via `unshift` (or equivalent). Reversing this dereferences integer arg values as heap addresses → KeyError.
- **`GenericVM.execute` must save and reset caller state** (pc, stack, call_stack, halted, vars, locals) for function calls, then restore after extracting the return value.
- **Fresh VM context per call.** Same applies in any VM where the outer loop reads pc/code from VM state — re-read both on each step if handlers can swap them.
- **Hand-written and grammar-driven parsers diverge.** Grammar-driven parsers pick up `python.grammar` updates automatically; hand-written ones (Perl python-parser) have hardcoded type checks. After token name changes, grep ALL parsers for the old name.
- **Skip-pattern ordering affects NEWLINE emission.** If `\n` is in a grammar's WHITESPACE skip pattern, no NEWLINE tokens will be emitted. Update downstream lexer-wrapper tests when changing the lexer's main loop.
- **Indentation-sensitive parsers need INDENT/DEDENT tokens.** `skip_newlines` must NOT skip DEDENT (block boundary). Use a separate `skip_whitespace` that drops NEWLINE+INDENT+DEDENT for contexts where indentation is noise.
- **GrammarLexer strips quotes from string capture groups.** `STRING = /"([^"\\]|\\.)*"/` makes the value `hello`, not `"hello"`. Fix tests.
- **Grammar-lexer test helper**: `_tok_type` normalizer is required — non-keyword tokens keep `TokenType` enum values, only promoted keywords use strings. Direct `t.type == "EOF"` comparison fails because `TokenType.EOF != "EOF"`.
- **Bracket-aware regex delimiter scanning** in `.tokens` parsers — `/` inside `[...]` is not the closing delim. Don't escape it as `[^\/]`; the parser handles it correctly.
- **New language frontends MUST wrap `GrammarLexer` / `GrammarParser`, not hand-write their own.** Every Twig/Lisp/whatever frontend in the repo (Python, Rust, etc.) is a thin shim that loads `code/grammars/<lang>.tokens` and `<lang>.grammar`. The wrapper pattern is the canonical approach — see `code/packages/rust/brainfuck/` for the reference. The standalone `lisp-lexer` / `lisp-parser` Rust crates are NOT a model — they predate the grammar-tools refactor. Hand-writing forks the grammar into a second implementation that drifts silently.
- **In the S language `_` is the assignment operator, so it cannot appear in any identifier** (the `NAME` pattern in `s.tokens` excludes it). Builtin names borrowed from R that contain an underscore — `seq_len`, `seq_along`, `is_null` — are therefore unwriteable in S: `seq_len(4)` lexes as `seq _ len(4)` (assign `len(4)` to `seq`) and the call silently does the wrong thing rather than erroring. Use dot-style names (`is.na`, `as.character` — dots ARE valid in S names) or drop the underscore form. Hit while adding the S v2 builtin library; the failure surfaced as a runtime "expected double" panic in a test, not a parse error.
- **Rust GrammarParser NAME-match collision fix** (parser/grammar_parser.rs `match_token_reference`): when the grammar expects literal `NAME`, reject tokens whose `type_name` is set (e.g. a `QUOTE` token whose `type_: Name` is just the enum-fallback). The original logic only excluded type_name'd tokens for non-NAME custom types, so a Twig `'foo` would lex `'` as `(type_=Name, type_name="QUOTE")` and then incorrectly match the `NAME` slot in `atom = ... | NAME`. Symmetric tightening: a custom Name-based type reference (e.g. `AT_KEYWORD`) requires `type_name == expected_type` — bare-Name tokens no longer cross-pollute custom types.
- **Custom token types (DIMENSION, HASH_COLOR, TOKEN_REF, …) do NOT get new `TokenType` enum variants.** The GrammarLexer maps custom-named regex tokens to `type_ = TokenType::Name` with `type_name = Some("DIMENSION")` etc. To detect them in a Rust compiler, use `t.type_name.as_deref() == Some("TOKEN_REF")`, NOT `t.type_ == TokenType::TokenRef` (which doesn't exist). Tests must check `t.type_ == TokenType::Name && t.type_name.as_deref() == Some("DIMENSION")`.
- **`GrammarParser::new` takes `Vec<Token>` and `ParserGrammar` by value.** Do NOT borrow: `GrammarParser::new(&tokens, &grammar)` fails. Use `GrammarParser::new(tokens, grammar)`. If you need the token list after parsing, clone before passing.
- **`TokenType::LBrace`, not `TokenType::Lbrace`.** The enum variant for `{` uses camelCase `LBrace` with capital B. Same applies to `LParen`, `RParen`, `RBrace` — always match the actual Rust declaration.
- **Rust format strings need `{{` / `}}` to emit literal braces.** `format!("{ ... }")` is a format-string error — `{` inside `"..."` with no closing `}` fails at compile time with "expected `}`, found `\"`". Use `format!("{{...}}")` to produce the literal string `{...}`.
- **Shorthand slot-binding props (`slot: label`) require an alternation in the grammar.** If a `.mll` grammar rule only has `prop = NAME COLON prop_value`, then `slot: label` fails at parse time because `slot` is a KEYWORD token, not a NAME. Add `| KEYWORD COLON NAME` as an alternative and handle it in the semantic analyzer. LL(1) is preserved because KEYWORD ≠ NAME are disjoint token types.
- **Symbolic-VM eager-simplification cascades break structural assertions.** When porting algebraic-rule phases between languages, a single new rule fires on every re-eval of every matching subterm — including transient subterms produced by *other* handlers. PR #3468's Phase 30 rule `exp(n·log(x)) → x^n` turned `D(x^x, x)` from `exp(x·log(x))·(log(x)+1)` (the prior handler-internal intermediate) into `x^x·(log(x)+1)` because the derivative handler emits `exp(x·log(x))` internally and that node is re-evaluated. Mathematically equivalent, but structural-match tests using `toEqual` / `assert_eq!` on the old form fail. Lesson: when adding a new global rule, grep for tests that hand-write the affected intermediate form and update both shape expectations and the CHANGELOG "regression note" — don't pin the structural test to either form, expect the simplest one.
- **TypeScript symbolic-ir uses `bigint` for IRInteger.value / IRRational.numer/denom**, not `number`. Fraction-arithmetic helpers in TS handlers (`fracGcd`, `fracMake`, `fracMod`) must take and return `bigint` throughout — mixing in `number` will silently truncate at the `Number.MAX_SAFE_INTEGER` boundary and corrupt the π-multiple lookup keys. Rust's analogous helpers use plain `i64` (sufficient for denominators ≤ 6).
- **Rust handler factories that recurse via `vm.eval` need `vm: &mut VM`, not `_vm: &mut VM`.** The convention in `symbolic-vm/src/handlers.rs` was that pure-numeric handlers ignored the VM (`_vm`); Phase 31+ symmetry rules (`sin(-x) → -sin(x)`) need a recursive `vm.eval(...)` call to re-simplify the wrapped result, so the underscore must come off. Forgetting this gives "unused variable" warnings the first time you compile, but the bigger trap is leaving an early-return path that constructs a `NEG` wrapping an *unevaluated* `Sin(x)` — works in isolation but breaks composition (`sin(-(-x))` doesn't fold to `sin(x)` because the inner `-(-x)` never re-enters the simplifier).
- **Neither TS nor Rust `symbolic-ir` exports a constant for the `Abs` head.** When implementing `Abs` rules in the Phase 29-33 port, references must use the string form (`sym("Abs")` in TS, `IRNode::Symbol("Abs".to_string())` in Rust). The asymmetry with `SIN`, `COS`, etc. — which do have head-symbol constants — is silent: imports succeed for the wrong-cased near-misses (`SQRT` is exported, `ABS` is not). Add an `ABS` constant to `symbolic-ir` when one of the languages first ships an Abs handler, or accept the string-form convention indefinitely.
- **Long-running Rust test files re-define identical helpers across phases.** `tests/test_vm.rs` already had `eval_at`, `contains_head`, `trapezoid` at the top of the Phase 26+ block by the time Phase 34 was added; appending another `fn eval_at(...)` produced `E0428: the name eval_at is defined multiple times` and the entire test binary failed to compile. Fix: prefix new helpers with the phase name (`phase34_eval_at`, `phase34_subst`, `phase34_numerical_derivative`) when their semantics differ — the Phase 34 evaluator routed through `SymbolicBackend::eval` to handle `Tan`/`Sqrt`/`Atan` correctly, which the existing hand-rolled `eval_at` did not. Same trap applies to TypeScript test files when the test count grows past one phase block.
- **Coupled version bumps across PRs need explicit numbering reservations.** When Phase 34 (TS) was being pushed before Phase 29-33 (TS) had merged, the natural `0.5.0 → 0.6.0` bump would have collided with PR #3468's `0.5.0 → 0.6.0`. Fix: bump Phase 34 to `0.7.0` directly (skipping 0.6.0) and call out the reservation in the CHANGELOG note ("leaves 0.6.0 for the in-flight Phase 29-33 port"). Rebase merge order — Phase 29-33 first, then Phase 34 — produces a clean 0.5.0 → 0.6.0 → 0.7.0 history without per-PR conflicts.
- **Numerical-derivative testing is the universal correctness check across Python/TS/Rust CAS ports.** Instead of asserting exact IR shapes (which vary with the surrounding simplifier passes), substitute `x ← x_val` into the returned closed form, evaluate through the full backend (`vm.eval` / `SymbolicBackend.eval`), and central-difference at several sample points. Compare against the original integrand at the same samples. Tolerance of `1e-4` with step `h = 1e-5` is enough headroom for f64 round-trips through `Sin`/`Cos`/`Tan`/`Atan`/`Sqrt`. This single pattern carried Phases 26, 27, 28, and 34 across three languages with zero per-language correctness divergence.
- **`git rebase` from a noisy-working-tree branch on Windows.** Switching branches in the coding-adventures checkout always surfaces a long list of `D code/programs/kotlin/.../.gradle/...` deletions (Gradle build outputs untracked in some branches, tracked in others). Plain `git rebase origin/main` errors with `cannot rebase: You have unstaged changes`. Fix: `git rebase --autostash origin/main`. Autostash also drops the changes silently if they don't apply cleanly to the rebased branch.
- **SQL query planner: `Project` must be the OUTERMOST (last) step in `planSelect`.** The correct 8-step pipeline is `Scan → Filter → Aggregate → Having → Distinct → Sort → Limit → Project`. Building `Project` before `Distinct`/`Sort`/`Limit` produces the wrong tree shape — e.g. `Sort(Project(Scan))` instead of `Project(Sort(Scan))`. Tests C5 (ORDER BY), C6 (LIMIT), C7 (DISTINCT), and Struct stacking all fail if `Project` is wrapped too early. This bug appeared identically in Java, Kotlin, and Haskell `planSelect` implementations (PR #7045, #7047, #7048). Fix: move `Project` construction to the last step after all sorting/pagination nodes are wrapped.
- **Haskell: always export data type constructors with `TypeName(..)` in the module export list.** `TypeName` alone exports the TYPE but not its constructors — `LitInt`/`LitText`/etc. from `LiteralVal` become invisible to callers including the package's own test suite. Symptom: `Data constructor not in scope: LitInt :: t0 -> SqlPlanner.LiteralVal`. On Windows CI tests silently pass as "skipped" (cabal not found), masking the error. Fix: `LiteralVal(..)` in the export list. Applies to every custom ADT whose constructors are referenced by callers or tests.
- **Java 21: unnamed variables/patterns (`_`) are a preview feature, disabled by default.** `case Foo _` and `case Foo(_, var x)` trigger `unnamed variables are a preview feature and are disabled by default` on Java 21 (JEP 443 — finalized only in Java 22). Fix: replace `_` with a named binding: `ignored`, `op2`, `nm`, `pat2`, etc. Record patterns with named bindings (`case Foo(var x, var y)`) and type patterns with names (`case Foo bar`) ARE finalized in Java 21.

## Cryptography & security review

- **Decompressors must cap declared output sizes** from untrusted headers BEFORE allocation. Expose an override for trusted callers; fail closed by default.
- **Backreference validators**: every LZ77/LZSS-style decoder needs `offset > 0` AND `offset <= output.length` checks before indexing into the decoded prefix. Throw `FormatException` on malformed/truncated streams.
- **Reject both undersized AND padded payloads** in fixed-width binary deserializers. Fail-closed on incomplete headers, trailing bytes, length mismatches.
- **Zero-length decoders must validate the full canonical empty encoding** — don't early-return on declared length 0; check trailing bytes and end-of-block markers.
- **Reject negative indexes explicitly** in pool/table-indexed decoders (Python in particular — negatives are valid offsets-from-end).
- **Public recursive comparison helpers need cycle tracking.** A `DeepEqual` that walks dicts/enumerables/properties without tracking visited reference pairs explodes on cyclic graphs. Always assume hostile input.
- **Recursive parsers (markdown, etc.) need depth + input-size caps at every recursive entry point**, not just the public API. Inline parsers that retry delimiter parsing char-by-char also need bounded unmatched-delimiter scans, or quadratic work on hostile input becomes a DoS.
- **Stateful TCP servers must cap per-connection buffered-input size.** Partial-frame buffers (RESP, HTTP body) without a max let a slow-stream attacker exhaust heap. On overflow, clear, send protocol error if possible, close.
- **Security fixes that change error messages break tests.** After unifying messages (e.g., generic "Invalid PKCS#7 padding"), `grep -r "old message" */test* */t/` for stale assertions.
- **Variable-length integer encoding with a format-marker byte: marker MUST come first** on the wire, regardless of host endianness. Zstd seq_count: `(count >> 8) | 0x80` BEFORE `count & 0xFF`. Round-trip tests on a self-consistent broken codec are blind to byte-order bugs — always include integration tests with values in each form whose low byte is < 128.
- **F# unsigned `count - 1u` underflows when `count = 0u`.** Always guard before writing `0u .. count - 1u`. Same: cap header counts to remaining-payload bytes before looping.
- **Typed import boundaries: `Some("")` ≠ epsilon.** If the runtime uses an empty-string sentinel internally for epsilon transitions, the typed contract uses `None` — reject `Some("")` at imports so malformed defs can't smuggle free moves past the validator.

## Testing & coverage

- **Every new source file needs a corresponding test file in the same commit.** Pytest-cov `fail_under=80` and similar gates trip on uncovered new code. Plan tests alongside implementation.
- **Rust has no built-in coverage** — install `cargo-tarpaulin`. `cargo tarpaulin -p <name> --out stdout`; sum the per-file lines for your package's `src/`. Always report a real number, never "n/a".
- **Tests requiring an external CLI must run a probe** (`git --version`, etc.) and skip if it errors. `exec.LookPath("git")` only proves the binary exists, not that it works.
- **.NET coverlet must be filtered to the package under test**: `/p:Include=[CodingAdventures.PaintInstructions]*`. Otherwise referenced assemblies' coverage drags down the threshold.
- **.NET parallel test runs need isolated artifacts**: `dotnet test --artifacts-path .artifacts`. On Linux, ALSO set `HOME="$PWD/.dotnet"`, `DOTNET_CLI_HOME="$PWD/.dotnet"`, AND `TMPDIR="$PWD/.dotnet/tmp"` — the CLI's first-run `NuGet-Migrations` mutex uses `/tmp/.dotnet/shm` shared state that races otherwise.
- **C# package referencing a type with the same name as its namespace** needs an explicit alias: `using FieldMath = CodingAdventures.Gf256.Gf256;`. Otherwise `Gf256.*` binds to the namespace.
- **C# tests using `BinaryPrimitives` need `using System.Buffers.Binary;` explicitly** — implicit usings don't cover it.
- **F# `dict [...] :> IReadOnlyDictionary<string, obj>`** infers an intermediate `IDictionary<string, objnull>` that fails strict upcasts. Build a concrete `Dictionary<string, obj>` first, then upcast.
- **Cloning a .NET package skeleton** requires renaming `.csproj` files and setting explicit `AssemblyName`/`RootNamespace`, not just changing `PackageId`. MSBuild treats same-filename copies as the same project identity in `.artifacts`.
- **Reactor / async / socket tests must tolerate cross-poll latency.** Don't assert that two independent readiness sources appear in the same `poll()` batch — accumulate observations across iterations. Don't assume a single `write_ready()` step makes the other side immediately readable. For nonblocking accept, try `accept()` first and only wait for readiness on `WouldBlock`.
- **Music/score fixtures**: derive event/sample counts from the parser's duration table, not by guessing.
- **`adjudication-tsa-demo` audit trail's `ir_nodes[].payload` does NOT contain term trees** — only `id`, `kind`, `modality`, `polarity`. A walker scanning the audit trail for `quantity(...)` compounds will get 100% false-negatives. The full term JSON is emitted as a SEPARATE block in stdout under the marker `--- LLM-extracted IR (raw decompose_text output) ---` (when `ADJ_DEMO_AUDIT=1`). When the LLM goes through clarification reprompts the term tree also ends up embedded as a string inside `dialogue[*].response.text` and `dialogue[*].question_text`; in the no-clarification case those don't exist and the marker block is the only source. Always parse BOTH blocks.
- **`decompose_text` silently falls back to a hand-built fixture when the LLM emits unterminated JSON.** Logged as `decompose_text FAILED: ... EOF while parsing` then `fallback: hand-built TSA fixture` on stderr-mixed stdout. Any bench that infers "model emitted no quantities" from an empty term-tree will misclassify these as model failures when they're really gateway/output-budget failures. Capture the failure-mode line explicitly.

## CI & GitHub Actions

- **Setup-job failures (action archive download, `Failed to download archive`)** are infrastructure flakiness, not code failures. Inspect the log before changing code; rerun. The same applies to `Prepare all required actions` failures before checkout.
- **Wait for full CI and explicit user sign-off before merging.** Even trivial PRs catch real regressions.
- **Always merge `origin/main` first** before reasoning about CI failures — the CI already merges your branch into main before building, so local reasoning about "what crates exist" is wrong if main moved.
- **Verify all agent-written files are staged.** Parallel agents may write after the initial `git add`. Run `git status --short` and `git diff --name-only` before committing.
- **Don't leak local machine state in commits or PR descriptions.** Translate "this failed because my workstation has X" into a portable engineering rule before committing.
- **Don't pin tool versions to `latest`.** `astral-sh/setup-uv@v4` with `version: latest` resolved to a release missing `aarch64-apple-darwin`. Use a known-good version range like `"0.6.x"`.
- **CI workflow classifier must recognize helper shell lines** in toolchain-scoped hunks of `.github/workflows/ci.yml`. Adding `sed`/`rm`/etc. to a Lua-only setup hunk without updating `internal/gitdiff/ci_workflow_test.go` makes the build tool fall back to a full monorepo rebuild.
- **CI detect outputs must use `steps.toolchains` (not `steps.detect`).** Adding a new language to CI requires THREE places: `allLanguages` in `main.go`, the detect job `outputs:`, AND `steps.toolchains` normalization (BOTH the `is_main=true` and `else` branches).
- **CodeQL flags `int64 → int` downcasts of CLI input** as `go/incorrect-integer-conversion`. Add explicit platform-sized bounds checks first; for `float64`, reject NaN/Inf/non-integral before the cast.
- **Miri timeout grows with code, not with test count.** `lang-runtime-safety.yml` had `timeout-minutes: 30`; PR 5 (closures) tripled `twig-vm` Miri wallclock and one of two parallel runs failed at 30:15 from runner variance, not a real bug. Bump generously (90 min) and shard by crate when wallclock crosses 60 min. Locally, `MIRIFLAGS="-Zmiri-ignore-leaks" cargo +nightly miri test -p twig-vm` is the canonical pre-push smoke check; don't trust the timeout to catch slowdown.
- **Miri belongs on unsafe code, not the integration seam — and not on every PR.** PR 7 moved `twig-vm` Miri off the per-PR critical path entirely.  PRs only run Miri on `lang-runtime-core` + `dynval-runtime` (where the unsafe is); both run in ~5 min total, blocking.  `twig-vm` Miri runs only on **post-merge to main** + a nightly cron, both via `lang-runtime-safety-deep.yml`, and **never gates anything** (`continue-on-error: true`) because twig-vm has zero unsafe — a Miri failure there is an integration-seam regression worth investigating, not a "main is broken" signal.  Engineers run `code/scripts/miri-twig-vm.sh` locally before pushing twig-vm changes — that's the canonical verification.  Lessons: (a) when Miri wallclock exceeds the per-PR budget, split by **where the unsafe lives**, not by tightening the cap further; (b) intensive checks belong on **main, not PRs** — fast PR iteration matters more than 100% per-PR coverage; (c) for crates without unsafe, even main-side Miri stays non-blocking — the workflow run history is the regression marker, not a status-badge red X.
- **Clippy is a blocking CI gate via the build tool's `-clippy` flag, not a `cargo clippy --workspace` job.** `cargo clippy --workspace` CANNOT run in this repo: platform-gated crates (`paint-metal`/`metal-compute`/`objc-bridge` need macOS, `paint-vm-direct2d`/`paint-vm-gdi` need Windows) `compile_error!` off their platform, so a whole-workspace clippy always fails somewhere. Instead, `build-tool -clippy` runs `cargo clippy --all-targets -- -D warnings` PER affected Rust package, from the crate dir, before its BUILD commands. `clippyStepFor` mirrors each BUILD's own platform guard: unconditional `cargo …` → lint unconditionally; `if [ "$(uname)" = "Darwin" ]; then cargo …` → reuse the condition; pure `echo SKIP` (no cargo) → no clippy. Diff-based on PRs, full on main. Setup gotchas, all learned the hard way wiring this gate:
  (1) `dtolnay/rust-toolchain` installs the **minimal** profile (no clippy) — add `with: components: clippy`.
  (2) **Match the clippy version you verify against to CI's `@stable`.** A stale local toolchain (mise's cached "stable" was 1.94 while CI stable was 1.97) hid ~65 lints that only failed in CI (`manual_checked_ops`, stricter `collapsible_match`/`while_let_loop`/`question_mark`/`unnecessary_sort_by`). Before pushing: `rustup update stable` then `cargo +stable clippy`.
  (3) **Do NOT pin the toolchain to a version for determinism.** `dtolnay/rust-toolchain@1.97.0` makes the *default* toolchain a version, not the `stable` **alias** — which breaks every BUILD that runs `rustup run stable …`/`rustup target add …` (wasm packages, embedded `thumbv7em` firmware): `rustup target add` adds to the default but `rustup run stable` uses a different, target-less toolchain → `E0463: can't find crate for std/core`. Clippy drift on `@stable` is cheaper to maintain (fix new lints when a stable bump surfaces them) than pinning.
  (4) **Clippy lints are platform-conditional; a macOS-only local run misses Linux-only lints.** Crates whose real path is `#[cfg(target_vendor="apple")]` leave dead code / unused imports on Linux (`barcode-layout-1d` `let_unit_value` — Linux font stub returns `()`; `paint-metal`/`text-native-coretext`/`window-appkit` dead_code). The gate runs on ubuntu AND macOS, so clean BOTH. Reproduce Linux lints locally without a Linux box by **cross-checking** (clippy checks, doesn't link): `rustup target add x86_64-unknown-linux-gnu` then `cargo clippy --target x86_64-unknown-linux-gnu …`. Fix with `#![cfg_attr(not(target_vendor = "apple"), allow(...))]` — enforced where the code is live, allowed where it's inactive.
  (4a) **Gate an import to the platforms that USE it, not the platforms where it EXISTS.** `twig-aot/tests/macos_arm64_smoke.rs` had `#[cfg(unix)] use std::os::unix::fs::PermissionsExt;` — that gate answers "does this module exist here?", not "is it used here?". Both `set_mode` sites sat inside `#[cfg(all(target_os = "macos", target_arch = "aarch64"))]` tests, so on Linux (which IS unix) the import survived with zero use sites and `-D warnings` rejected it; `std::path::PathBuf` and `std::process::Command` were ungated and failed identically. Fix: find EVERY use site and its cfg, then write a gate covering precisely that set. Keep the `target_arch` half — on an Intel Mac every use site is still cfg'd out, so gating on `target_os` alone relocates the error instead of fixing it. The same bug in `lang-aot/tests/end_to_end_smoke.rs` went the other way: gating `use std::io::Write;` to `target_os = "windows"` fixed macOS and broke Linux, because a second, Linux-gated set of `writeln!` callers had been missed — the correct gate was `any(target_os = "windows", target_os = "linux")`. A whole-file `#![cfg(...)]` (as in `linux_x86_64_smoke.rs` / `windows_x86_64_smoke.rs`) is structurally immune, but is only available when NO test in the file must run on other hosts. Never reach for `#[allow(unused_imports)]` — it hides the same class of rot the gate exists to prevent.
  (4b) **The cross-check in (4) does NOT work for a crate whose `build.rs` compiles C via the `cc` crate.** Clippy skips linking, not build scripts: `cargo clippy -p twig-aot --all-targets --target x86_64-pc-windows-gnu` dies in build.rs with `error occurred in cc-rs: failed to find tool "x86_64-w64-mingw32-gcc"` before a single lint runs. Substitute, needing no cross toolchain: temporarily rewrite every `target_os = "macos"` in the file to `target_os = "linux"` and compile on the macOS host — all those cfgs then evaluate false, so the surviving item set is identical to a non-macOS host's and unused-import diagnostics match exactly. (Rewrite to a *real* target_os; a made-up value like `"nonesuch"` trips the `unexpected_cfgs` lint instead.) **Validate the simulation with a control**: apply the same rewrite to the PRE-fix file and confirm it reproduces the real CI errors. A check that cannot fail proves nothing — in PR #11264 the control reproduced all three errors exactly, which is the only thing that made the clean run meaningful.
  (5) **The gate runs on EVERY affected Rust package the build tool knows, not just the `code/packages/rust` workspace.** `code/programs/rust/*` are separate cargo projects and wasm/rust packages too; a `--workspace` clippy in `packages/rust` misses them all. It also surfaces pre-existing host-un-buildable crates (`os-kernel`: `#![no_std]` + crates.io `uefi` dep with `panic_handler` → `cargo test` can never link on a std host) — guard such a crate's BUILD to skip on hosts (it targets `x86_64-unknown-uefi`).
  (6) **`clippyStepFor` only inspected `buildCommands[0]`, so a BUILD *preamble* switched the whole gate off for that package — silently, on every platform.** `readLines` strips `#!/bin/sh` and `#` comments but NOT `set -e`, `export VAR=…`, `cd …`, or `echo "[pkg] Building…"`. sql-codegen's BUILD is `set -e` / `export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld` / `cargo test --package …`; command zero is `set -e`, which contains no `cargo`, so the package got NO clippy step at all and nobody noticed — `coding-adventures-sql-planner` carried a live `manual_is_multiple_of` error on `main` for exactly this reason. 22 packages were dark this way (12 chief-of-staff-* — harmless, their BUILDs run their own `cargo clippy` line — plus the 4 sql-*, mini-sqlite, smart-home-enphase-envoy-integration, and 4 `packages/wasm/*` whose BUILD opens with `rustup target add`). Fix: scan EVERY command, prefer an unconditional cargo over a platform-guarded one. **And the mirror-image trap on the way**: `strings.Contains(cmd, "cargo ")` treats a *mention* as an invocation — compile-only crates print the command they cannot run (`echo "  To build: cargo build -p font-parser-node --release"`), so scanning all lines naively would attach a clippy step to a package whose BUILD deliberately builds nothing. Erase single- and double-quoted spans first, then look for a bare `cargo` word: that keeps `RUSTDOCFLAGS="-D warnings" cargo doc` and `cd "$WORKSPACE" && cargo test` while rejecting the echo. **General rule: when a gate is derived by pattern-matching a build script, the failure mode is silence — a package that matches nothing is indistinguishable from a package that passed.** Assert the derived set is non-empty, and diff old-vs-new coverage before landing a change to the derivation.
  (7) **A crate that `compile_error!`s off Windows is linted by NOBODY.** Its default `BUILD` is a bare `echo SKIP`, so (correctly) no clippy step on the Linux/macOS legs; its real commands live in `BUILD_windows`, which never runs because CI skips the build step on the Windows leg (see the `#![cfg(target_os = "windows")]` lesson below). `paint-vm-direct2d` accumulated 8 `-D warnings` errors in that blind spot, and because **clippy lints path dependencies** (the `-- -D warnings` args reach every crate clippy-driver compiles, via `CLIPPY_ARGS`, not just the primary package), those 8 broke `cargo clippy --all-targets -- -D warnings` for every crate depending on it — while the crate's own gate step reported success. `paint-vm-gdi` had 3 of its own. Fix (PR #12125): a `Clippy Windows-only Rust crates` step on the existing windows-latest PR leg, with the crate list **derived from the platform gate in the source** (`grep` for `#[cfg(not(target_os = "windows"))]` … `compile_error!`) rather than hand-maintained, and a hard failure if the derived set is ever empty. Note it runs on PRs only — the main-merge matrix is ubuntu-only, so there is no Windows leg to attach to there.
- **Getting a large workspace to zero clippy warnings: `cargo clippy --fix` first, but it stops at the first deny-by-default hard error.** `absurd_extreme_comparisons`/`approx_constant`/`not_unsafe_ptr_arg_deref`/`never_loop` are deny-by-default, and a deny error aborts that crate's compile so `--fix` can't touch its other (machine-applicable) warnings. Clear/allow the hard errors first, then re-run `--fix`; crates that previously failed to compile now get auto-fixed. `--fix` only applies `MachineApplicable` suggestions — `approx_constant` (replacing `3.14159` with `PI` changes the value) is `MaybeIncorrect`, so it is NEVER auto-fixed; resolve those with a scoped `#[allow(clippy::approx_constant)]` + justification, never by editing the literal (it's usually test data / codegen input / an intentional hand-written constant). FFI crates that expose raw-pointer C ABIs (`node-bridge`, `ruby-bridge`) get a crate-level `#![allow(clippy::not_unsafe_ptr_arg_deref)]` with a comment rather than ~80 per-fn annotations.
- **A `#![cfg(target_os = "windows")]` test target is never executed by CI — the windows leg does not build Rust, on PRs or on main.** Two independent reasons, both verified in `.github/workflows/ci.yml`: on a PR the "Build and test affected packages" step (the one that runs `-language all`) is guarded by `if: needs.detect.outputs.is_main != 'true' && (runner.os != 'Windows' || needs.detect.outputs.needs_swift == 'true')` (ci.yml:1529), so on Windows it runs ONLY when the PR touches Swift — a Rust-only PR skips it entirely and the leg still reports green; and on main the `detect` job builds every matrix entry as `"os": "ubuntu-latest"`, so there is no windows leg to skip in the first place. The "Full build on main merge" step has no OS guard, which makes it look like it covers Windows — it does not, because the matrix never puts it there. Consequence: do not count a Windows-gated suite as watched, and do not read a green `build (windows-latest)` as evidence that Rust compiled there. Confirm by grepping the job log for the package name — the macOS leg logs `rust/<pkg>  BUILT`, the windows leg never mentions it.
- **A `gh pr checks` "FAILED" line can mean "cancelled by the platform," not a real error — check the job `conclusion`, not just its display status.** PR #10017 (a pure state-JSON change) showed `build (windows-latest)` and the downstream `CI gate` as failed after ~15 min. `gh api repos/<owner>/<repo>/actions/jobs/<job-id> --jq '{status,conclusion}'` showed `conclusion: "cancelled"` (not `"failure"`) — `ubuntu-latest`/`macos-latest` had already built the identical commit successfully, so the content was never at fault. Fix: `gh run rerun <run-id> --failed` (not a code change, not a new commit) — it passed clean on rerun. Rule: before touching code in response to a CI red X, fetch the job's actual `conclusion` field; `cancelled`/`skipped`/`abandoned` mean "rerun," only `failure` with real log output means "investigate the diff."
- **A brand-new branch/PR can show "no checks reported" (zero workflow runs, not merely queued) for hours** during an account-wide GitHub Actions backlog (many concurrent branches pushing at once saturates the concurrent-job ceiling). Distinguish "queued behind others" from "never triggered" with `gh api repos/<owner>/<repo>/actions/workflows/<workflow-id>/runs?branch=<branch> --jq '.total_count'` — `0` means the push/PR event never even created a run; a nonzero count with `status: queued/pending` means it's just waiting its turn. Watch `gh api repos/<owner>/<repo>/actions/runs --jq '.workflow_runs[] | select(.status != "completed") | .status' | sort | uniq -c` for the account-wide backlog size — once it drains (roughly, once `in_progress` count is >0 and the queued count is dropping), new branches start getting picked up on their own. An empty retrigger commit (`git commit --allow-empty`) does not skip the queue; only waiting does.
- **Changing a shared frontend compiler runs DOWNSTREAM consumers' tests in CI — run them locally first.** Editing `twig-ir-compiler`'s lowering (LANG-FULL TW2: value-defines → typed locals) passed `cargo test -p twig-ir-compiler` and the lang-aot matrix locally, but CI's affected-package detection also rebuilt `twig-vm`, whose `dispatch.rs` test compiled a real Twig program and asserted on the *exact* emitted ops (`(define x 5)` → a `global_set` writing `x` to the host global table).  The new lowering dropped that `global_set`, so the downstream test's expectation broke (the result was still correct).  This is NOT a latent bug to defer — it's a test that legitimately tracks the compiler output you changed, so update it in the same PR.  Rule: when a PR touches a shared `*-ir-compiler` (or any crate many others depend on), enumerate consumers with `grep -rln '<crate>' */Cargo.toml` and run **each** consumer's tests (`cargo test -p <consumer>`) before pushing — not just the changed crate + the integration matrix.  Preserve a downstream test's *intent* when updating it (here: keep exercising `global_set` by switching to a lambda-**captured** define, which still hits the global table) rather than deleting the assertion.

## QR / format-marker / file-format specifics

- **QR format-info bit ordering is MSB-first across row 8 cols 0–5, LSB-first down col 8 rows 0–5** (copy 1); copy 2 mirrors it. Always verify with `zbarimg` or another standard decoder immediately after implementation — BCH check is ground truth.
- **`kern` Format 0**: subtable format is in the HIGH byte of `coverage` — `coverage >> 8 == 0`. `coverage & 0xFF == 0` checks flags, not format, and skips all valid Format 0 subtables (horizontal flag = bit 0 sets the low byte to 1).
- **OpenType `head` table is exactly 54 bytes.** Missing the `xMin/yMin/xMax/yMax` quartet (8 bytes) makes it 46, mis-aligning every subsequent table offset.
- **QOI encoder seen-table updates the CURRENT pixel** after emitting any non-INDEX op, not the previous one. Lag-one strategies diverge from the decoder.
- **Intel 4004 has no AND instruction.** `AND_IMM vR, vR, 15` and `AND_IMM vR, vR, 255` are no-ops on a 4-bit machine; emit a comment, not an opcode. Other masks would need a RAM lookup table.
- **Intel 4004 R1 corruption.** When `_emit_add_imm`'s source virtual register maps to physical R1, don't clobber R1 as scratch — use R14. Special-case `k=0` as a pure copy: `LD Rsrc; XCH Rdst`.
- **Intel 4004 simulator halt**: emit `HLT` (opcode 0x01) — `JUN $` self-loop is not detected as halt and runs out of `max_steps`.
- **IBM 704 index-register family** (LXA/LXD/SXA/SXD/PAX/PDX/PXA): the tag selects the source/destination register only; the address field is used directly with NO `(Y - C(T))` subtraction. "Store IRA at Y" must not shift Y by IRA. Always test register-family ops with a non-zero index value to catch this; tag=1 with IRA=0 is silently correct either way.

## Editing human-language LESSON files breaks the language-ladder APP's tests, not just the data package's

- **A content change under `code/learning/human-languages/*/lessons/` crosses two packages.** `code/packages/typescript/human-language-data` parses the lessons, but `code/programs/typescript/language-ladder` ALSO loads them at build time via `import.meta.glob` and pins facts about them. Running only the data-package suite is not enough, and CI will catch what you skipped.
- Concretely (HL-C18A, PR #9982): splitting fifteen over-budget Spanish lessons into thirty-three micro-lessons moved the per-chapter lesson counts hardcoded in `language-ladder/tests/bookhashes.test.ts` (ch3 12→14, ch4 13→15, ch6 7→9). The data package was green, `npm run check:books` was clean, and the generated hash manifest was regenerated correctly — the only stale thing was the app-side pin. Same shape bit `modality.test.ts` and `integration.test.ts` on other lesson-adding PRs.
- **Rule: after ANY change to lesson files, script data, or `core/*.json`, run BOTH suites.**
  ```
  cd code/packages/typescript/human-language-data && npx vitest run
  cd code/programs/typescript/language-ladder && npm install && npx vitest run
  ```
  The app suite takes ~85s and needs the `file:` dep installed first.
- When a pinned corpus count legitimately moves, **update the pin with a comment saying why** — never delete or loosen the assertion. The surrounding assertions (hash matches the browser-loaded AST, chapter reports `synced`) are the real gate and must stay untouched.
- Related trap on the same PRs: a wall-clock performance assertion (`expect(Date.now() - started).toBeLessThan(2_000)`) failed at 10,677 ms on a contended runner while the implementation was correctly linear — 561 ms locally for the same input. See the existing "CI is ~25× slower than local" entry. Pick a threshold that separates the algorithmic classes you care about (linear vs quadratic), not one that measures runner load.

## A level claim goes stale when OTHER PRs move material INTO the level

**What happened (#13061).** A branch drove Spanish's A1 reinforcement residue to
zero and asserted `attained: A1`. It then sat unmerged for a few hours. In that
window #13132 and #13144 moved DELE A1 verbs **down from A2 into A1**. Those
verbs' atoms carried their own reinforcement debt, and it landed at A1 because
that is where the verbs now lived. Rebasing turned the branch's headline
assertion false: ten atoms at or below A1, revisited fewer than twice.

The reflex is to think a level claim is threatened by PRs that ADD lessons to
that level. It is equally threatened by PRs that MOVE existing material into it,
and those are easier to miss because the corpus gained no lessons at all.

**Rule:** after rebasing any branch that asserts a level has been attained,
re-run the gate before trusting the branch's own numbers. `attained` is a
statement about a whole corpus, and a merge is a corpus change.

## A "not yet measurable" edge is a bomb with someone else's finger on the pin

Same PR. It documented two atoms as unmeasured-because-nothing-follows-them, and
wrote that `ES-LEX-GRITAR` "becomes measurable the moment it stops being last."
It then stopped being last, and the declared known-open edge became a live
blocker on the very PR that declared it.

Declaring an edge open is honest and worth doing. It is not the same as being
safe from it. If the thing that makes an edge measurable is *anyone adding
content*, in a repo landing dozens of PRs a day, expect to inherit it yourself.

## Four rules a new human-language lesson has to satisfy that no single gate names

Authoring seven review lessons hit all four in one pass, each from a different
test:

1. **Every activity id must begin with its lesson id plus a hyphen.**
   `integration.test.ts`, not the activity compiler.
2. **A block's `hl-knowledge: assesses=[...]` must list every atom its
   activities assess.** Declaring the atom on the activity alone fails with
   "assesses X outside block Y".
3. **A lesson beyond the one realizing its path segment's spine node needs an
   extension node** -- "is local support but belongs to no extension node". Add
   it to an existing extension's `lessons`, or create one and list it in the
   path segment's `inline`.
4. **Transitive prerequisites must actually introduce every atom in `requires`
   and `practises`.** A review of four verbs needs a prerequisite chain reaching
   all four, not just the nearest lesson.

Also: `answer` and every `accepted` variant must be distinct after
normalization, which lowercases -- so `"english"` and `"English"` collide. And
a table with **four or more columns** is refused by the narrator and counts
against a corpus-wide refusal pin; three columns are speakable, so put the
fourth column's content in prose.

## A concept_tag matching /(^|-)VERB-/ registers as a verb even on a review

`ES-VERB-ETYMOLOGY-CERTAINTY-REVIEW` on a review lesson that introduces no verb
pushed Spanish's namespaced-verb `extras` count 43 -> 44 and failed
`verbs.test.ts`. The tag is read by `verbCoverage`, not just by humans. Name a
review's concept tag for what it reviews, without the `VERB-` infix.

## Repo policy / workflow reminders

- **Always pull `origin/main` first** (`git fetch origin && git merge origin/main`) before starting work — the repo moves fast.
- **Default to a fresh `git worktree`** from `origin/main` whenever the source checkout is shared, noisy, or has other agents active. Treat it as the default, not an exception.
- **Add new lessons to this file IMMEDIATELY** when a CI failure or mistake recurs. Don't wait until later. Keep entries short — read this file before starting any work.
- **STOP and generalise when you find yourself writing the Nth variant of the same helper.** During the cas-summation work an agent generated **74 open PRs + 27 already-merged PRs** that added a hand-written grid of `N-Sqrt × M-Log × polynomial` helper functions — one per `(N, M)` pair, up to N=64. The bodies were identical modulo two hardcoded counts. A single generic `_log_sqrt_poly_effective_x2_generic(node, k)` that *counts* factors instead of hardcoding them handles every `(N, M, K)` combination, including cases beyond the grid that silently failed. Symptoms to watch for: functions whose names embed a small integer (`_two_sqrt_six_log_poly_*`), CHANGELOGs listing `Phase N — N-Family`, version bumps far past semver-meaningful (`2.373.0`), tests that just instantiate the same template N times. Whenever a "family" pattern emerges, ask "can the count be a `for` loop?" before writing helper N+1. Cleanup PR for this specific incident: #4545 (Phase 86 — generic log×sqrt×poly recogniser).
- **`flock` is Linux-only — use `mkdir` spin-lock for cross-platform BUILD serialization.** `flock /tmp/name.lock sh -c "..."` works on Linux but fails with `sh: flock: command not found` on macOS runners. Replace with: `while ! mkdir /tmp/name.lock 2>/dev/null; do sleep 1; done; (cmd); EC=$?; rmdir /tmp/name.lock 2>/dev/null; exit $EC`. The `mkdir` call is atomic on all POSIX filesystems; the subshell captures the exit code so the lock is always released and the correct status propagates.
- **Generated BUILD scripts must be POSIX-compatible.** The repo's build-tool runs `BUILD` files via `sh`, which on Ubuntu CI is dash. Dash rejects `set -o pipefail` ("Illegal option -o pipefail"). Don't emit `#!/usr/bin/env bash` shebangs or `set -euo pipefail` from any code-generator's BUILD template. Plain commands match the convention of `cli-builder`, `state-machine`, etc.
- **Don't let downstream tools re-parse `.tokens` / `.grammar` files at runtime.** Those files are build-time-only artifacts: `<lang>-lexer/build.rs` and `<lang>-parser/build.rs` already compile them into Rust source via `grammar_tools::compiler`, baking the parsed `TokenGrammar` / `ParserGrammar` into the lexer/parser rlibs as struct literals. Tools that need keyword lists, brackets, or grammar rules should pull from those compiled artifacts (e.g. `twig_token_grammar_spec()`, `twig_grammar()`) — not re-parse the source files. The `<lang>-spec-dump` binary in each language's parser crate is the canonical exit point: it serialises the embedded grammars to a `LanguageSpec` JSON document for editor tooling. Same source of truth, no drift.
- **grammar-tools codegen for `-compiler` packages** (not `-lexer`/`-parser`): `generate-rust-compiled-grammars` only auto-discovers `*-lexer` and `*-parser` packages. For `*-compiler` packages that embed both token + parser grammars in one `_grammar.rs`, run separately: `grammar-tools -f compile-tokens <file>.tokens` and `grammar-tools compile-grammar <file>.grammar`, then combine the two function bodies under one header with both import groups. The `-f` flag is needed for `.tokens` files that use `escapes: standard` (the validator rejects it, but the compiler correctly emits `escapes: Some("standard")` in the struct).
- **Never hand-edit `_grammar.rs` files.** Edit the `.tokens` or `.grammar` source files in `code/grammars/` and re-run the grammar-tools pipeline to regenerate. Hand-edits diverge from the grammar source of truth and break the single-source-of-truth invariant.
- **Autonomous loops MUST use CronCreate, not ScheduleWakeup.** During a multi-cycle scaffolding job, an agent used `ScheduleWakeup` to chain cycles together (each wakeup fired the prompt for the next cycle). When auto-mode exited between cycles, the agent treated the wakeup-fired prompt as a passive notification and responded with "No response requested" instead of executing the work — the loop stalled silently. `CronCreate` prompts are always treated as actionable (the agent reliably acts on them — every babysit-PR cron has proven this), and the cron survives session restarts and auto-mode toggles. **Pattern for autonomous loops**: (1) write a state file at `.claude/<job>-state.json` recording the work queue and per-item status (`pending` / `in-progress` / `pr-open` / `merged`); (2) `CronCreate` ONE recurring job (3 min interval — not 5, the user is impatient and so should you be) with a prompt that opens with literal directive language ("AUTONOMOUS LOOP DRIVER — execute the steps below without asking for confirmation. This cron-fired prompt IS the directive; do not defer to the user."); (3) the prompt reads the state file, transitions states, babysits open PRs (handling CI failures and conflicts), starts new cycles when prior ones merge, and `CronDelete`s itself when all entries reach `merged`. NEVER use `ScheduleWakeup` for autonomous work — only for one-shot follow-ups the user expects.
- **Gradle's `build/` directory collides with the repo-required `BUILD` script on case-insensitive macOS HFS+.** Two failure modes: (1) `gradle test` fails to create `build/reports/problems/` because the filesystem already has `BUILD` at the same name; (2) `rm -rf build` (cleaning gradle's output) ALSO deletes the `BUILD` script — silent data loss. **Fix in `build.gradle.kts`**: `layout.buildDirectory.set(file(".gradle-out"))`. Add `.gradle-out/` to the package's `.gitignore` alongside `.gradle/`. Document the redirect in the README so other contributors don't undo it. Applies to every new Kotlin/Gradle package added under `code/packages/kotlin/`. Same logic would apply to any other Gradle-using language we add (Java, Scala, etc.) if their convention is also lowercase `build/`. **NEVER `rm -rf build` in a repo with a `BUILD` script** — use `rm -rf .gradle-out` (or whatever you redirected to).
- **Plan autonomous work for maximum parallelism: smallest unblocker first, then fan out.** When the work queue contains N independent items (e.g., 8 runtime libraries, none of which depend on each other), DO NOT serialize them through the loop one PR at a time. The right pattern: identify the smallest piece of work that unblocks parallel streams, push it as a quick PR, and **while it's in review/CI, open the rest as parallel PRs** — each on its own branch, each with its own babysit. The shared cron then juggles them all: it advances any stream whose PR merged, fixes any stream whose CI broke, rebases any that conflicts. Concretely: for the Phase-2 runtime libraries, the first PR (`mosaic-flux-react`) was the reference implementation that established the API surface; once it was in CI, the next three (`html`, `webcomponent`, `swiftui`) could have been opened in parallel rather than waiting on the chain. Recognizing parallelism is a state-file design choice — list items as `{ id, depends_on: [...], status }` so the driver can pick ALL items whose `depends_on` are merged.

- **A subagent sees the Agent tool's `description`/title, not only its `prompt` — never leak the answer there.** In a blind-evaluation experiment (ADJ52: a domain-blind ingester that must classify a clinical case without knowing the diagnosis), an `Agent` call's `description` was "Domain-blind ingester on McArdle case". The subagent picked up "McArdle" from the description and referenced it in its reasoning, contaminating the supposedly blind run — even though the prompt itself was scrubbed. Rule: **every field that reaches a subagent (description AND prompt) must be scrubbed of the ground truth / answer / case name.** Keep descriptions generic ("Ingest problem statement into IR"). The same leak applies to file paths handed to a sandboxed agent — a path like `.../cases/mcardle-pmr/...` names the answer; pass inputs inline, not by revealing the path. Discard and re-run any blind output produced after such a leak.

## Mosaic compiler pipeline

- **Grammar alternation order matters: more specific alternatives must come first.** In `slot_type`, `list_type` must appear before `KEYWORD`; otherwise `list<text>` is lexed as just the keyword `list` and the `<` causes a parse error. The rule of thumb: try the longest / most specific match first in any alternation. This applies in both `.grammar` files and the generated `_grammar.rs`.
- **When updating a grammar alternation order:** edit both the `.grammar` source file in `code/grammars/` AND the corresponding `_grammar.rs` (the embedded Rust representation). They must stay in sync; the CI grammar-tools pipeline validates the `.grammar` but the Rust parser uses `_grammar.rs`.
- **Web component `when`/`each` blocks must emit JavaScript ternaries / `.map()`, not `<template>` HTML tags.** The `<template data-when>` approach requires a client-side runtime to interpret. Since Custom Elements use a self-contained `_render()` that writes to `shadowRoot.innerHTML`, the when/each control flow must be JS expressions: `${this._show ? \`...\` : ''}` and `${this._items.map(item => \`...\`).join('')}`.
- **Custom Elements need backing fields for ALL slot types, not just attribute-observed ones.** Scalar primitive slots (text, number, bool, image, color) appear in `observedAttributes` and get string backing fields. List and node slots must NOT appear in `observedAttributes` (the browser would stringify them); instead, expose them through explicit JavaScript property setters and initialize their backing fields as `[]` or `null` in the constructor.
- **Add new primitives to `is_primitive_node()` in `mosaic-analyzer`.** When a new layout element (e.g. `Grid`) is added to the Mosaic spec, it must be included in the `is_primitive_node()` function in `mosaic-analyzer/src/lib.rs` so that `is_primitive = true` is set correctly. Missing this causes the VM to emit it as a custom element tag (e.g. `<grid>`) instead of the intended HTML element.
- **`_grammar.rs` hand-edit caveat:** the general rule is "never hand-edit"; however for embedded grammar reordering (alternation order fix) it is acceptable when `grammar-tools` is not available in the current environment. Always note the edit prominently and regenerate properly before the next full CI run.
- **`mosaic-compile pkg --output X` treats `X` as a DIRECTORY and writes `X/react/<Component>.tsx` (+ `index.ts`, `.lattice`) — passing a file path silently creates a directory literally named `Foo.tsx`.** This is true with *and* without `--emit-project`; the flag only adds the Vite shell side-files (`package.json`, `vite.config.ts`, `index.html`, `README.md`, `src/main.tsx`) next to the component. So `--output "$WEB/src/TaskApp.tsx"` produced `.../src/TaskApp.tsx/react/TaskApp.tsx`. To land a single component in a hand-written host: emit to a scratch dir, copy `<scratch>/react/<Component>.tsx` to its destination, and delete the scratch dir. The emitted component is self-contained (imports only `react`, exports `<Component>` + `<Component>Event`), so copying just that one file is sufficient.
- **Don't build a real app by overlaying host files onto `--emit-project` output — the generated `package.json` is banner-stamped `"AUTO-GENERATED by mosaic-compile --emit-project. Edits will be overwritten on next emit."`, so app dependencies cannot live there.** The overlay model works only for a zero-dependency demo. The moment the host needs a real dep (a storage layer, a router), make the host a **committed npm package** that owns `package.json`/`vite.config.ts`/`tsconfig.json`/`index.html`/`BUILD`/tests, and have the build emit *only* the component into its `src/` (see the previous lesson). Done for `task-app/host/web`. Consequence to remember: such a host sits several levels below `code/`, and BUILD lines run with cwd reset to the package dir **for every line** (`cmd.Dir = pkg.Path` in `build-tool/internal/executor`), so every `cd ../../../../../packages/typescript/<dep>` uses the *same* depth — do not write cumulative `cd`s.

- **N-API `napi_unwrap` is type-agnostic — when an addon registers more than one `napi_wrap`-ed class, each unwrap helper MUST check a type tag before casting.** Discovered in matrix-rust-napi Phase 2b: the `Graph` and `Runtime` classes both went through `napi_wrap`, and `unwrap_graph` only checked `napi_unwrap`'s `status == NAPI_OK` + non-null pointer. A JS caller could pass a `Runtime` instance where a `Graph` was expected (`rt.run(rt, [])` or `g.toJson.call(rt)`); the bare unwrap returned the `Box<WrappedRuntime>` pointer, which `unwrap_graph` cast to `&Graph`, causing immediate UB on the first `graph.tensors.len()` read. **Fix pattern**: prefix every wrapped payload with `#[repr(C)] struct Wrapped<T> { tag: [u64; 2], inner: T }` using a class-specific 128-bit constant for `tag`, and validate the tag in every unwrap helper before dereferencing the rest. The "right" long-term answer is `napi_type_tag_object` / `napi_check_object_type_tag` (N-API v8+) — defer until node-bridge grows those bindings. Single-wrap-class addons like `font-parser-node` are not a model for this: with only one class, the bug doesn't exist. The lesson applies the moment a second wrapped class joins.

## mosaic-emit-xaml — running the generated output on Windows

- **`HostDialog` cannot lower inside a `<UserControl>` root — the XAML root must BE the `<ContentDialog>` and the partial class must extend `ContentDialog`.** Discovered while making the first end-to-end Mosaic→XAML→on-screen-dialog demo run (`code/programs/csharp/hello-dialog-xaml/`). WinUI 3's `ContentDialog` is a top-layer primitive that becomes the visual root when shown; wrapping it as `<UserControl><ContentDialog>...</ContentDialog></UserControl>` parents the dialog to the UserControl, then `await ShowAsync()` fails with `ArgumentException: Value does not fall within the expected range` or a native `0xc000027b` heap-corruption crash in `CoreMessagingXP.dll`, depending on timing. **Fix pattern**: when the moslayout root is `HostDialog`, `emit_xaml` must emit `<ContentDialog x:Class="...">` as the root and the matching `.xaml.cs` partial class must be `: ContentDialog`. Non-HostDialog cases keep the existing `<UserControl>` lowering. Generalises to any future top-layer-popup primitive (HostFlyout, HostMenuFlyout, etc.) — same "root promotion" rule.

- **Every emitted `xmlns:` prefix must be declared on the root, or XAML fails to parse.** Same demo found `emit_host_dialog` writing `mos:Dialog.IsOpen="{Binding ...}"` without ever declaring `xmlns:mos="..."`. The XAML compiler accepted the source but the runtime XAML loader rejected the binding, manifesting as an opaque "could not be started" error dialog ahead of any other crash. **Fix pattern**: either reuse PR-5's `used_xmlns` mechanism to declare every prefix referenced by an emitter, or drop the attribute. Recommend the latter for `mos:Dialog.IsOpen` — the existing "host owns the lifecycle" contract (host code-behind calls `ShowAsync()/Hide()`) is sufficient.

- **Use `{x:Bind ...}` not `{Binding ...}` — the emitter never sets DataContext.** `emit_host_dialog`'s `Title` binding was inconsistent with every other emitter (all using `{x:Bind}`). `{Binding}` resolves through `DataContext`; since nothing sets it, the binding silently fails and the property renders blank. `{x:Bind}` is compile-time-typed against the partial class itself, which is what the rest of the generator relies on.

- **Slot names colliding with the base class's properties must be aliased.** A `slot title : text` on a component that extends `ContentDialog` shadows the inherited `ContentDialog.Title` property, making `{x:Bind Title}` ambiguous and the heading rendering blank. **Fix pattern**: when the emitter picks a non-`UserControl` base class (HostDialog → ContentDialog), each slot whose PascalCased name appears on the base class needs renaming to `<BaseName>{Slot}` (e.g. `DialogTitle`). Either rename only on collision, or always namespace.

- **`BoolToVisibilityConverter` is referenced but never emitted.** `emit_if` writes `{StaticResource BoolToVisibilityConverter}` into the XAML but no C# `IValueConverter` class is generated — first `If`-using consumer hits a XAML parse failure at runtime. **Fix pattern**: when `ctx.needs_bool_to_vis` is set, also emit a `BoolToVisibilityConverter.cs` alongside the per-component triple (3-line C# `IValueConverter` with `ConverterParameter="invert"` support).

- **WinUI 3 unpackaged `dotnet build` requires Visual Studio's AppxPackage MSBuild tasks for the final packaging step — the .dll and .exe build cleanly anyway.** On a bare .NET 9 SDK install (no Visual Studio), `dotnet build` of any WindowsAppSDK 1.5+ project ends with `MSB4062` errors loading `Microsoft.Build.AppxPackage.RemovePayloadDuplicates`. The HelloDialog.dll + HelloDialog.exe + every runtime dependency in `bin/Debug/.../` are present and functional — the failure is in a post-compile packaging-cleanup target. Project-file `<Target Name="...">` overrides don't help because the package's transitive targets load AFTER the project file. **Workaround**: ignore the MSBuild error; trust the binaries that are present; manually copy `runtimes/win-x64/native/*.dll` next to the .exe (those don't get auto-flattened by `dotnet build`, only `dotnet publish`). The emitted `.csproj` should add a post-build target to do the native-DLL copy.

- **WinUI 3 ContentDialog `ShowAsync()` from code requires an explicit `XamlRoot` — but use a button's `XamlRoot`, not the Window's `Content.XamlRoot`.** The latter is sometimes null/invalid even after `Activated`, while a button's `XamlRoot` is guaranteed valid at click time because the button is in the visible tree. **Fix pattern in emitted MainWindow boilerplate**: `dlg.XamlRoot = (sender as FrameworkElement)?.XamlRoot;` inside the host's Click handler.

- **The unpackaged WinUI 3 bootstrap rejects the app with a system error dialog if the Windows App Runtime isn't installed on the machine.** The dialog reads "This application requires the Windows App Runtime Version X.Y (MSIX package version >= ...)". Two ways out: (a) document `winget install Microsoft.WindowsAppRuntime.1.7` in the emitted README, OR (b) set `<WindowsAppSDKSelfContained>true</WindowsAppSDKSelfContained>` in the emitted `.csproj` to bundle the runtime. (b) avoids the install dance but increases output size to ~50MB.

- **The catalogue of issues from the first end-to-end demo lives at `code/programs/csharp/hello-dialog-xaml/ISSUES.md`** — read that BEFORE starting any "make the Mosaic XAML output actually run" work. Each entry there is a follow-up PR. Once they all land, `mosaic-compile --backend xaml --emit-project` should produce a project that runs end-to-end with only the user's business logic added.

- **N-API: `napi_value` is a local handle; storing it across calls requires `napi_ref`.** Surfaced when matrix-rust-napi's class-based API (Phase 2b's `Graph` / `Runtime` classes with `Graph.fromJson` / `Runtime.create` static methods) silently returned `undefined` from every static-method call. Root cause: the class constructors were stored in an `AtomicUsize` as raw `napi_value`s captured from `napi_define_class` inside `napi_register_module_v1`. By the time a later JS-triggered callback (`Graph.fromJson(...)`) loaded the stored value and passed it to `napi_new_instance`, the local handle had been invalidated when its handle scope ended, and `napi_new_instance` returned `napi_invalid_arg` (status 1). **Fix pattern**: immediately after `napi_define_class`, wrap the returned `napi_value` in a persistent `napi_ref` via `napi_create_reference(env, class, /* refcount */ 1, &mut ref)`. Store `napi_ref` in the AtomicUsize. In each static-method callback, call `napi_get_reference_value(env, stored_ref, &mut value)` to get a scope-bound `napi_value` for that callback, then pass it to `napi_new_instance`. `napi_ref` is also pointer-sized so the AtomicUsize approach still works for Worker-thread safety. `font-parser-node` has the same latent bug in `FONT_FILE_CTOR` but no existing test reaches `napi_new_instance` (every input rejects earlier via `fp::load`), so it's never fired in practice — file a follow-up to fix it before the next consumer of that addon hits the same wall. **Always throw a precise JS error** (not silently return `undefined`) on every napi-call failure inside static-method shims — silent returns mask exactly this kind of bug.

- **Ruby parser: `method_call_no_paren` can mis-eat a `def` body's tail expression.** Surfaced in Phase 6m when `def myor(a, b)\n  a || b\nend` started parsing as `method_call_no_paren("def", expression="myor(a,b)")` at the top level — the inner def_statement was lost and the body's `a || b` ended up as a separate statement after, leaving the literal `end` as a third statement. Root cause: `method_call_no_paren = (NAME|KEYWORD) expression { COMMA expression }`. When the parser tries `def_statement` first and its body's `statement` rule fails on `a || b` (because the alternation backtracking for `NAME` then-`||` doesn't fully unwind), the framework backtracks to `method_call_no_paren` at the top level, which happily consumes `def` (as KEYWORD callee) plus `myor(a, b)` (as expression argument). **Workaround**: wrap the tail expression in parens — `def myor(a, b)\n  (a || b)\nend` — which forces the expression through the `LPAREN expression RPAREN` atom in `factor`, bypassing the ambiguity. **Affected phases**: 6m logical operators (`&&`/`||`/`and`/`or`/`!`/`not`), likely 6n/6o (range, ternary), 6p (compound assignment). Plan a follow-up to either (a) drop `method_call_no_paren` from `statement` alternation in favour of an explicit `command_call` shape that only fires when a `(NAME|KEYWORD)` is followed by something other than a binary operator, or (b) reorder so `expression_stmt` is tried before `method_call_no_paren`.

## Mosaic emitters — language-specific coercion traps

- **JavaScript loose equality makes `0 == ""` true — so a code-generator that defaults numeric slots to `""` produces a subtle false-positive bug in every comparison against an unset value.** Surfaced in PR #4999 (`mosaic-emit-webcomponent`). The Web Component emitter generated `const editRow = this.getAttribute("edit-row") ?? "";` uniformly for every typed slot. In the visicalc demo's per-cell editing predicate `r == editRow`, leaving `edit-row` unset rendered the editor at cell (0, 0) by default — because `0 == ""` (via JS coercing `""` to `0`). The host had to explicitly set `edit-row="-1"` as a workaround, but the right fix is in the emitter. **Fix pattern (by slot type)**: `Number` → `attr !== null ? Number(attr) : NaN` (NaN's `==` is always false, even against itself — exactly the "no value" sentinel semantics every numeric comparison needs); `Bool` → `attr === "true"` (strict string equality, absent attribute reads `false`, matches HTML boolean-attribute conventions); `List` → `JSON.parse(attr ?? "[]")` (already existed pre-fix); `Text`/`Image`/`Color` → keep `attr ?? ""` (these are never compared numerically and `${var}` template interpolation strings-coerces uniformly). **Scope check**: this trap is specific to JS-emitting custom-element backends. HTML emitter renders server-side without `getAttribute`; React uses typed props; SwiftUI/Compose/Qt/Flutter/XAML all have static type systems that catch the mismatch at compile time. So the fix is only needed in `mosaic-emit-webcomponent`.

- **Swift and Kotlin's `==` operator rejects `Int == Double` — when a For-index (`Int`) is compared against a Number-typed slot (`Double`), the emitter must rename-and-cast the index.** Surfaced first in PR #4987 (SwiftUI) and again in PR #4994 (Compose). When `Grid.mll` writes `is-editing: ( r == editRow )` with `editRow: number`, the SwiftUI/Compose backends emit a `ForEach`/`forEachIndexed` whose index variable is `Int`. The naive translation `r == editRow` then fails to compile in both languages because `Int == Double` requires explicit conversion. **Fix pattern**: rename the for-index in the emitter (`_swiftIdx<idx>` in SwiftUI; `_kotlinIdx<idx>` in Compose), then introduce a shadow binding `let r = Double(_swiftIdx0)` / `val r = _kotlinIdx0.toDouble()` so the user-visible identifier (`r`) keeps the same name across the layout but is now `Double` everywhere it's used in expressions. The shadow rename is necessary because the layout's bound name (`as: r`) cannot collide with the emitter's index variable in the host-language scope. **Scope check**: this trap is Swift/Kotlin-specific. Dart's `==` accepts `int == double` (returns true when numerically equal — `1 == 1.0` is `true`). C# implicitly promotes `int` to `double` for `==`. Qt/QML uses JS-style coercion. So no analogous fix is needed in `mosaic-emit-flutter`, `mosaic-emit-xaml`, or `mosaic-emit-qt`.

- **The taxonomy of emitter coercion bugs maps cleanly onto host-language semantics — survey by language family, not by backend.** Three classes show up:
  1. **JS loose-equality + empty-string-default trap** — only JS-emitting custom-element backends (`mosaic-emit-webcomponent`). HTML/React use different rendering pipelines that don't go through `getAttribute`.
  2. **Strict-static `Int != Double` rejection** — only Swift/Kotlin (`mosaic-emit-swiftui`, `mosaic-emit-compose`). C#/Dart/QML coerce naturally.
  3. **No-coercion-needed languages** — Flutter (Dart), Qt (QML), XAML (C#), HTML (static markup). When a new emitter-coercion bug class is discovered, classify by host-language semantics first; the backend list to audit falls out automatically.

## Mosaic emitters — the synthetic root wrapper must carry the host framework's sizing contract

- **A generated component's root wrapper has to publish whatever "how big am I?" signal its host framework's layout system reads — and QML's `Item` is the one container that publishes nothing.** `mosaic-emit-qt` wrapped every component in `Item { id: mosaicRoot; … }` with no `implicitWidth`/`implicitHeight`. QML's `Item` has **no intrinsic size**: it reports `0 x 0` and does *not* grow to fit its children (they just paint outside it). Since `implicitWidth`/`implicitHeight` is exactly what `RowLayout`/`ColumnLayout`/`GridLayout` read to size a child, every generated component collapsed in any layout — four toolkit `Button`s in a `RowLayout` measured `48x0` total (the spacings alone), overlapping at the same coordinate with labels clipped to one character. **Fix**: `implicitWidth: childrenRect.x + childrenRect.width` (and `.y`/`.height`) on the root. **The pitfall to check before reaching for `childrenRect`**: it is a binding loop whenever a child anchors back to the parent (`parent.width` ← `implicitWidth` ← `childrenRect.width` ← `child.width` ← `parent.width`). So it is right for `Box` (children are independent) and wrong for `Stack` (children get `anchors.fill: parent`). Note QML does *not* always warn — a styled `Box` whose `Text` fills it silently **converges** to the margin size (`6x6`) instead of looping, which looks like a fix and isn't.

- **Why this class of bug hides for a long time: the in-tree host anchored the component.** `VentureChrome { anchors.fill: parent }` sizes from the anchor and never consults `implicitWidth`, so every existing test and demo passed. The bug only appears the first time someone puts a generated component in a *layout*. **When auditing a code generator, exercise the generated artifact in the composition mode consumers will actually use, not just the one the in-tree demo uses.**

- **Scope check by host framework — only backends that inject a *synthetic* root wrapper can have this bug.** Compose, Flutter, SwiftUI, and XAML all emit the layout root node directly (`Row`/`Column`/`Box`, `HStack`/`VStack`, `StackPanel`/`Border`), and every one of those is a real measuring container; Qt was the only backend adding a wrapper, and it picked the one QML type with no content-derived size. **Generalisation**: when a backend wraps generated output in a synthetic host-framework node, check that node against the framework's sizing contract specifically — the wrapper is exactly where the framework's defaults stop applying. XAML's analogous trap would be emitting `<Canvas>` (measures children with infinity, reports `0x0`) instead of `<Grid>`.

## Mosaic Qt output — component names collide with `QtQuick.Controls`, and the module type wins

- **`Button.mil` emits `Button.qml`, whose type name collides with `QtQuick.Controls.Button` — and with both in scope the imported module wins, not the same-directory file.** Verified on Qt 6.8.1: a consumer writing `import QtQuick.Controls; import "."` then a bare `Button { variant: "primary" }` gets the *platform* button and fails with `Cannot assign to non-existent property "variant"`. If it only sets properties the two types share, the wrong component renders with no error at all. Same collision for `CheckBox`, `RadioButton`, `TextField`, `Popup`. **Fix for consumers**: always use a namespaced directory import — `import "." as Mosaic; Mosaic.Button { … }`. **Do not "fix" it by renaming the emitted type** (`MosaicButton`): the MIL component name is the cross-backend contract that React/SwiftUI/Compose/Flutter/XAML all emit verbatim, so a Qt-only rename breaks that correspondence and every existing consumer. The structural fix is emitting a `qmldir` so the package becomes a real QML module. **Bonus consequence of the same precedence rule**: it is what keeps the emitter's own output working — `Button.qml` contains a `Button { … }` element intended as `QtQuick.Controls.Button`, and it resolves outward rather than recursing into itself.

## A downstream consumer's pinned generated-artifact assertions can drift for a long time with nobody noticing, then all surface at once when an unrelated upstream PR drags the consumer back into the diff-affected set

- **`mosaic/programs/engram-app`'s `native_project_shells_expose_engram_host_contract` test (`tests/package_compiles.rs`) had six independently-stale pinned string assertions**, spanning Qt QML event handlers (missing the `function(...) { ... }` wrapper `mosaic-emit-qt` added at some point — its own tests already expected the wrapped form), a CMakeLists.txt Qt6 version pin (`6.7` vs. actual `6.8`) and missing `QuickControls2` in two places, a native-library copy loop missing entries added for a later `venture_browser_qt` dependency, and an XAML `ContentControl`/`StackPanel` block missing `Mode=OneWay` at a nesting depth that had grown deeper since the assertion was written. None of these six drifts were related to each other or to the PR that surfaced them (#12026, a security fix narrowing a `*.dll` glob in `mosaic-emit-xaml`'s generated `.csproj`) — they'd each landed separately, upstream, over time, and `engram-app`'s own test suite was never rebuilt/retested by any of those PRs because the diff-based build tool only rebuilds packages the diff actually touches transitively. This PR touched `mosaic-emit-xaml` (a transitive dep of `engram-app`), which put `engram-app` back in the diff-affected set for the first time in a while — and its tests, run for real, failed on all six long-accumulated drifts at once, one `assert!` panic at a time (each fix only reveals the *next* failure, since `assert_contains` aborts the function on first mismatch).
- **Before assuming a surfaced failure is caused by your own change, check it against a clean `origin/main` worktree** (`git worktree add /tmp/check origin/main --detach`, run the same test there). If it fails identically on main, it's pre-existing drift your diff merely unmasked — confirmed here in under a minute and saved a wrong diagnosis.
- **To fix several drifted `assert_contains` string-literal assertions in one long test function efficiently**: don't iterate one `cargo test` cycle (25–50s here) per assertion. Temporarily insert `fs::write("C:/scratch/name.txt", &content).ok()` right after each `let content = fs::read_to_string(...)` in the test, run once, let it fail as far as it gets (later reads whose line was never reached still won't have dumped — the panic aborts the function), inspect what actually got dumped, fix those assertions, re-run to reach further, repeat. Remove all the dump lines before committing.
- **`--diff-base origin/main --dry-run`-style local runs of the repo's own build tool are the authoritative way to check "is package X actually in this PR's CI-affected set"** — don't guess from a raw `grep` of who imports what. Confirmed here: `mosaic/programs/engram-app` genuinely was `WOULD-BUILD` for this diff (it and `typescript/programs/engram-app` both depend transitively on `mosaic-emit-xaml`), which meant this fix was required to unblock CI, not optional scope creep.

## sql-execution-engine — grammar drift between a shared grammar and its hand-written tree-walker

- **A hand-written AST tree-walker that switches on rule names will silently rot when the shared grammar grows new precedence layers — and the failure mode is `nil`, not a parse error.** `sql-execution-engine` (Go) walks the `sql-parser` AST with a `switch node.RuleName` in `evalExpr`. Over many PRs (#4055–#4164) `code/grammars/sql.grammar` adopted SQLite's full operator-precedence ladder, inserting two new rules — `collated` (`COLLATE` postfix) and `bitwise` (`& | << >>`) — *between* `comparison` and `additive`, and rewrote `comparison` to take `collated` operands. The evaluator had no `case "collated"`/`case "bitwise"`, so every expression fell through to `default: return nil` *before reaching* `column_ref`. Result: `SELECT id, name` returned `<nil>` for every cell, `WHERE` matched zero rows, and `TestWhereNumericComparison` panicked on an empty slice. `SELECT *` kept working because it reads the row map directly and never touches `evalExpr` — a misleading "half the package works" signal. Separately, `limit_clause` changed from bare `NUMBER` tokens to `signed_number` child nodes (plus negative-LIMIT-means-unbounded and MySQL `LIMIT m, n`), so `executeLimit` found no NUMBER tokens and ignored LIMIT/OFFSET entirely. **Why it lay hidden**: the Go build tool only rebuilds/tests packages whose diff touches them or a transitive dep, so the break stayed dormant until an unrelated `go/lexer` change re-triggered the engine's tests on a `main` CodeQL build. **Fix pattern**: when you add a precedence layer to a shared `.grammar`, grep every consumer that walks the parse tree by rule name (`grep -rl 'RuleName' code/packages/*/sql-*`) and add the passthrough/eval case in the same PR. A new wrapper rule needs at minimum a passthrough case (`evalCollated` just evaluates its inner node); operator layers need real evaluation (`evalBitwise`, `||` concat, unary `~`/`+`). **Prevention idea**: give the tree-walker a `default:` that returns a sentinel error instead of `nil`, so an unhandled rule surfaces as a loud `EvaluationError` rather than silent NULLs — file a follow-up to do this across the grammar-driven Go engines.

## Nib `shift_expr` grammar addition — a shared-grammar precedence-layer change must be greped across EVERY language port, not just the one PR touched

- **Adding a rule to a shared `.grammar` file breaks every language's downstream consumers that read that grammar at runtime, not just the language the PR happened to touch — and CI can go red on all 5 build shards simultaneously from one merged PR.** PR #11257 ("lower shift expressions") inserted a new `shift_expr` precedence level into `code/grammars/nib/nib.grammar`, changing the cascade from `add_expr → mul_expr → ...` to `add_expr → shift_expr → mul_expr → ...`. The PR updated only the Rust consumers (`nib-lexer`, `nib-parser`, `nib-iir-compiler`, `nib-type-checker`) because Rust — like Go and Haskell — embeds its own generated copy of the grammar (`_grammar.rs` / `grammar_data.go` / `Generated/ParserGrammar.hs`), so those three languages were each independently safe *by construction* (Go/Haskell simply don't support shift expressions yet — a feature gap, not a regression, since their embedded copies were never regenerated either). But TypeScript, Python, Elixir, Lua, Perl, and Ruby's `nib-parser` packages all read the shared `nib.grammar` file at runtime (`readFileSync`/`File.read`/etc.) as their single source of truth — so all six started emitting an extra `shift_expr` wrapper node around every `add_expr` operand immediately, even for plain `a + b`, with zero code changes on their end. Their `nib-type-checker`/`nib-ir-compiler` packages each hardcode an expression-rule allowlist (`EXPRESSION_RULES` Set in TS, `@expression_rules`/`%_EXPRESSION_RULE`/`expression_rules` table in Elixir/Perl/Lua, an inline `%w[...]` array in Ruby) that didn't list `shift_expr`, so the operand got silently filtered out of `add_expr`'s children — same failure shape as the sql-execution-engine lesson above (`nil`/`undef`/zero-operands, not a parse error). This is exactly the same bug class recurring across *six* independent hand-rolled tree-walkers at once, because they all copy-pasted the same allowlist pattern originally (there's even prior art baked into the comments: `mul_expr` had to be added to every one of these same lists in an earlier PR, #5677/#7378, for the identical reason). **Why some failures were louder than others**: TS `nib-formatter` crashed outright (`Malformed add_expr: expected at least one operand` — its printer required ≥1 operand); TS/Elixir/Lua/Perl/Ruby `nib-type-checker`s degraded silently, returning `nil`/`None`/no type-error instead of crashing, so a couple of them (Lua, Ruby) had type-mismatch bugs (e.g. `u4 + u8` silently accepted) that never showed up as a hard test failure in CI at all — only found by writing a *new* targeted regression test, not by re-running the existing suite. **Fix pattern, extending the sql-execution-engine lesson**: when a shared `.grammar` file gains a rule, `grep -rn "add_expr\|<old-neighbor-rule>"` across `code/packages/*/nib-*` (all languages, not just the one the PR's diff touched) to find every hand-rolled allowlist/dispatch that needs the new rule name added — and check whether each language's own lexer/parser reads the shared grammar file at runtime (affected) or embeds a generated/hand-written copy (safe until that copy is regenerated, at which point it becomes affected too — track as a separate future task, don't conflate "doesn't support the feature yet" with "broken"). A single Rust-only PR is not "done" for a repo that ports the same language to 6+ runtime-grammar-reading implementations. **Verification note**: local `npm test`/`pytest`/`mix test`/`busted`/etc. across 6 languages needed Node 22 (local Node 20.11.1 too old for vitest 4/vite 8 — installed via `brew install node@22`, ran with `PATH="/opt/homebrew/opt/node@22/bin:$PATH"` rather than touching the system default) and each package's exact BUILD-file install order (`file:`/`-e`/`npm ci` chains) — the fix could not be trusted from code-reading alone; every claimed "unaffected" (Go, Haskell, TS `nib-ir-compiler`, Python both packages, Perl `nib-ir-compiler`) was confirmed by actually running that package's tests, not by inspection.

## closure compiler clone (closurec) — pass scheduler `changed=true` infinite-loop hazard

- **Under `IterationPolicy::FixedPoint`, a pass that reports `changed = true` while returning an unchanged program causes the pipeline scheduler to re-run that pass forever.** Surfaced first as a security-review catch on CLOC13.E (PR #4766, `closure-pass-remove-unused-vars`). The pattern that triggers it: a pass body wires up new analysis (scope-analyzer use-count, alias scan, shape candidates, etc.), identifies candidates, but *defers the actual program mutation* to a follow-up PR. The intuitive draft is `changed = !candidates.is_empty()` — "I found work, so something changed." But the scheduler keys `FixedPoint` on `changed`: each iteration re-runs the pass, the pass re-finds the same candidates, claims a change, returns the same program, and the loop spins. **Fix pattern (now codified across CLOC13.A..E)**: until step 3 (the apply step) actually mutates the program, hard-pin `changed = false` as a struct literal in the returned `PassOutput { .. changed: false, .. }`. Don't derive it. Don't condition it. Add inline comment AND CHANGELOG note pointing at this lesson so the next contributor doesn't reintroduce the bug when they wire the apply step. The same discipline applies to `OneShot` passes (rename, CLOC13.A) even though the literal infinite loop doesn't fire — pipeline consumers may key off `changed` for cache invalidation or to skip downstream serialization, and reporting `true` without mutation forces unnecessary work. **Defence in depth**: the candidate vec gets bound to `_alias_candidates` / `_shape_candidates` / etc. so the work-survey is preserved for the apply-step PR without triggering an unused-var warning, and so the `_` prefix telegraphs "this is deliberately unused until step 3 lands."

- **Open at most one shared-crate PR at a time — sibling merges cascade-rebase every other shared-crate PR.** Surfaced when PR #4752 (CLOC12.15 `BigIntLiteral` in `javascript-ast`) needed *four* rebases as orthogonal sibling PRs touching `javascript-ast` landed one after another. Each sibling merge re-derived the file hash, GitHub flipped the PR to `CONFLICTING`, and the local worktree needed a rebase + force-push-with-lease cycle. Burned ~30 min of wall-clock per cycle, plus re-running tests + re-running `/security-review` per rebase. **Fix pattern**: before opening a shared-crate PR, check `gh pr list --state open` for any in-flight PR touching the same crate. If one exists, defer the new PR; pick truly orthogonal work instead (a pass-body PR that consumes the shared crate via Cargo dep doesn't *modify* it and so doesn't cascade). The CLOC13 5-stream parallel set worked specifically because each PR modified a different pass crate and only *consumed* the shared `closure-scope-analyzer` via path dep — no rebase cascades. **Detect early**: a PR transitioning `CLEAN → CONFLICTING` immediately after a sibling merge is the cascade firing. Don't bisect; rebase, re-test, re-review, re-push.

- **Prep-locally-while-blocked: when a shared-crate PR blocks the next move, draft the follow-up in its own worktree without pushing — verify build + tests + security review locally, then push the moment the blocker merges.** Surfaced during the CLOC13 cascade: PR #4778 (CLOC13.B inline body, a pass-body PR) sat in CI for ~6 minutes; the natural next move was CLOC13.0 (analyzer body activation), which was a *shared-crate* PR and therefore blocked by the standing "one shared-crate PR at a time" rule. Idling 6 minutes is wasted wall-clock. **Pattern**: `git worktree add .claude/worktrees/<next-task>-tree -b <branch> origin/main`, then write the full implementation + tests + CHANGELOG + version bump + run the local cargo test suite + run /security-review via Agent — all *without* pushing. When the blocker's `mergedAt` flips non-null: `git fetch origin && git merge origin/main` in the prepped worktree (fast-forward in the common case), re-run tests against the now-current main, and push. **Numbers from CLOC13.0**: prep took ~4 minutes (read analyzer source, write 119-line body, write 7 new tests, run cargo test, run security review). When #4778 merged, time from "merged" to "PR open" was ~25 seconds (rebase + retest + push + gh pr create). Without the prep, that gap would have been the full 4 minutes. **When the rule doesn't apply**: if the worktree's diff would conflict with the blocker's diff (e.g., both modify the same lines of the same file), the rebase after merge fails and the prep is wasted. Detect by checking `git diff origin/main...<blocker-branch>` against your intended changes before starting prep. **Why this beats "just wait"**: the blocker's merge is itself a signal that the system is ready for the next layer. Prep lets you ride that wave instead of starting from cold context every time. Especially valuable in autonomous-loop mode where idle minutes are dead minutes.

- **closurec version bumps are a 3-file change, not a 2-file change.** Bumping `code/programs/rust/closurec/Cargo.toml` requires synchronised bumps in **both** `code/programs/rust/closurec/cli.spec.json` (the version cli-builder reads at startup to populate `ParserOutput::Version(v).version`, which the `--version` banner prints) **and** `code/programs/rust/closurec/tests/diff/help-markdown/expected.stdout` (the golden line `Version: X.Y.Z` for the `--help_markdown` byte-exact integration test). Surfaced in CLOC14 (PR #4929): bumped Cargo.toml 0.43.0 → 0.44.0, missed both downstream spots, `tests::version_string_matches_crate_version` (which asserts `--version` output contains `env!("CARGO_PKG_VERSION")`) failed on macos-latest CI. Required a follow-up fix commit. **Mitigation**: before pushing a version bump, `grep -rn "$OLD_VERSION" code/programs/rust/closurec/` and confirm zero hits. The unit test that catches the spec drift is in the main binary's `mod tests` (`cargo test --bin closurec`), not in `tests/`, so a partial test run via `--test diff_minify` won't catch it.

- **`cargo test` STOPS at the first failing test TARGET — so a unit-test failure hides every integration-fixture failure behind it. After regenerating goldens, re-run the whole suite with `--no-fail-fast` before declaring it green.** Surfaced in CLOC #214 (emitter 0.55.0, the "terminate only the last program item" change, PR #8644). The emitter change drifted goldens in three places: the emitter's own tests, the pass crates' tests, and closurec's `tests/diff/*` fixtures. The first full `cargo test --manifest-path .../closurec/Cargo.toml` reported `675 passed; 11 failed`, all 11 in the main binary's `run::tests::*` — and I fixed exactly those. What I did not notice is that cargo **never ran the `tests/*.rs` integration binaries at all**, because it aborts remaining targets once one target fails. My follow-up runs were filtered (`cargo test ... run::`), which re-confirmed the unit tests and still never touched the integration targets. Result: `tests/diff/advanced-bigpass/expected.stdout` kept the stale `function f(x){return x*10};report(...)` and CI went red on three of four platforms after the PR was already open. **Mitigation**: any change that can move emitted bytes (emitter, any pass, the printer) must end with ONE unfiltered `cargo test --no-fail-fast` per affected workspace — `code/packages/rust` AND each `code/programs/rust/*` project, which are separate cargo projects. Treat a filtered or fail-fast run as "not yet verified", never as green. **Tell**: a suite summary that shows a single `test result:` line when the crate has several `tests/*.rs` files means the other targets were skipped, not that they passed — count the `test result:` lines against the number of test binaries. Companion to the closurec-3-file-version-bump lesson above: both are "the run you did was narrower than the blast radius you created".

- **A bare `}` (or `{`) inside a Rust `assert!`/`panic!` message is parsed as a FORMAT BRACE — `error: invalid format string: unmatched \`}\` found`. Never paste emitted-code snippets straight into an assertion message.** Bit me TWICE in the same arc (CLOC #214, emitter 0.55.0): first writing ``"lone class declaration terminates with `};`"`` in a closure-emitter upstream test, then again in six closurec integration tests (``"class declaration must terminate with `};` ..."``), the second time breaking the compile of all six test targets *in the commit that was fixing the first problem*. The trap is specific to macros that take a format string: `assert!(cond, "…{actual}")` runs the message through `format_args!`, so `}` must be escaped `}}` — but the emitted-JS snippets this repo asserts on (`class C{…};`, `function f(){}g()`) are *full of* braces, so the temptation to quote them verbatim is constant. **Mitigations, in order of preference**: (1) describe the shape in prose instead of quoting it — "must terminate with a semicolon as the last program item" beats ``"must end with `};`"``; (2) if you must show the bytes, put them in a `{:?}`-formatted argument (`"unexpected tail: {tail:?}"`), never inline in the literal; (3) only as a last resort, double the braces (`}}`), which is unreadable next to real JS. **Tell**: the error points at the *string literal column*, not the brace, so it reads like a mysterious macro failure. If a test file that only had its assertion text edited suddenly fails to compile, this is why. Note that a plain `assert_eq!(a, b)` with NO message is immune — the danger only appears once you add the explanatory message, i.e. exactly when you're being a good citizen.

- **Writing a `\u`-followed-by-four-hex-digits sequence into a tool call silently injects a REAL control byte into the source file, because the tool payload is JSON and JSON unescapes `\uXXXX` before the file is ever written.** Hit three times in the CLOC #204 control-char arc, twice in a doc comment and once in test comments, each time while writing prose that *described* the escape (`the oracle emits \u001b`). The file then contains an actual ESC/BEL/DEL byte where the text should read backslash-u-0-0-1-b. It compiles, it usually passes tests, and it is invisible in normal `cat`/editor output -- I only caught it with `cat -v`, which renders the bytes as `^[`, `^G`, `^?`. A pre-existing instance from an earlier PR was still sitting in `closure-emitter/CHANGELOG.md` (a raw BEL inside `(`U+0007` -> `\x07`)`), which is how these survive: nothing flags them. **Rules**: (1) never type a bare `\uXXXX` into a tool call -- build the string in a heredoc'd script where you control the bytes (`B=chr(92)` then `"%su001b" % B`), or write it split (`backslash-u-001b`) in prose; (2) after ANY edit whose text discusses escapes, run a contamination check -- `python3 -c "s=open(F).read(); print(sum(1 for c in s if (ord(c)<0x20 and c not in chr(10)+chr(9)) or ord(c)==0x7f))"` -- and expect 0; `grep` with a bracket range is NOT reliable here (it silently matched nothing while `cat -v` showed the bytes plainly). (3) The same trap applies to `\x41`-style and `\0` sequences in any JSON-transported payload. This is a *documentation* hazard, not a code hazard: Rust source escapes like `'\u{1b}'` are fine because the brace form is not valid JSON escape syntax and passes through literally.

- **`cargo clippy` without `-- -D warnings` EXITS 0 ON WARNINGS, so a clean local clippy run does not reproduce CI.** CI lints with warnings denied; a bare `cargo clippy -p <crate> --all-targets` prints the warning and still returns 0, so the pre-push gate silently passes. Burned on PR #8722 (CLOC #204): `clippy::type_complexity` fired on `[(&str, fn(&str) -> String); 3]` -- an array of function pointers I introduced while WIDENING A TEST to address a security-review coverage nit. So the hardening itself turned CI red, on a PR whose code was otherwise fully verified. The local run had reported `CLIPPY_EMITTER=0` and I took that as the gate passing. **Rule**: every pre-push clippy invocation must be `cargo clippy -p <crate> --all-targets --manifest-path <ws>/Cargo.toml -- -D warnings`, run for EVERY affected crate in EVERY affected workspace (`code/packages/rust` AND each `code/programs/rust/*`, which are separate cargo projects). **Fix pattern for type_complexity**: hoist the type behind an alias (`type Escaper = fn(&str) -> String;`) rather than restructuring the code. **Family resemblance**: this is the same failure shape as the `--no-fail-fast` lesson above -- in both cases the command I ran was WEAKER than the command CI runs, reported success, and I trusted it. When a gate passes suspiciously easily, check that the local invocation actually matches CI's flags, not just CI's tool.

- **Pull-from-origin-first failure mode: a multi-hour-stale main worktree silently hides recently-landed files, leading to redundant PR work that has to be closed.** Surfaced when I drafted PR #4784 (a CLOC13 spec) at `code/specs/CLOC13-scope-analyzer-and-pass-bodies.md` without realising `code/specs/CLOC13-scope-analyzer.md` had already landed on main (in a commit I never pulled). My local main worktree had been on commit `69dbb4119` for the entire session; main had advanced past that with the CLOC13 spec + ADJ14 + LP19e + others. The redundant PR ran CI, sat in review queue, then had to be closed with a self-correcting comment. CLAUDE.md working principle #1 ("Pull from origin/main first") exists precisely for this. **Mitigation**: at the start of every loop iteration that does any work in the main worktree (not just feature worktrees), run `git fetch origin && git merge origin/main` BEFORE any Read/Edit/Write. Feature worktrees forked from `origin/main` are immune to this specific bug because they're forked fresh — but the main worktree is sticky-stateful and silently rots. **Detection**: if `code/...` paths returned from `git ls-tree -r origin/main` don't appear in `ls code/...`, the worktree is stale. Treat that mismatch as a stop-the-line signal. **Cost**: ~40 minutes of wall-clock to draft, push, and then close #4784 — plus the contributor-trust hit from a self-acknowledged "this was redundant" comment.

- **The macOS native-executable path can't link the runtime archive's external helpers (`__twig_alloc_bytes`, `__twig_putchar`, …); native AOT smoke tests are Linux/Windows-gated for exactly this reason.** Surfaced in McCarthy L3b when adding heap cons-cell support to the native backends. The aarch64 backend correctly emits `BL __twig_alloc_bytes` for the new `alloc` op (verified: the object compiles and `ld` is reached), but `compile_file_to_macos_executable` then fails with `ld: Undefined symbols: "__twig_alloc_bytes"`. **It is NOT a codegen bug** — a plain Brainfuck program (whose tape uses `alloc_bytes`) fails identically on the macOS-exe path (`__twig_alloc_bytes` AND `__twig_putchar` both undefined). The macOS runtime archive embedded by `twig-aot/build.rs` does not resolve these C helpers (symbol-prefix / archive-member granularity), and twig-aot's own `macos_arm64_smoke` tests only exercise *scalar* programs (no externals), so the gap was never caught. **Mitigation**: gate any native-executable e2e test that uses a runtime helper (`alloc_bytes`/`putchar`/`print_string`/cons cells) to `#[cfg(target_os = "linux")]` + `#[cfg(target_os = "windows")]` — matching the existing Nib/Brainfuck/BASIC smoke tests — and rely on CI-ubuntu (x86_64-linux) for the real end-to-end check. Verify backend codegen host-independently with `compile`-returns-bytes unit tests (hand-built CIR) in the backend crate. Fixing the macOS archive (so `alloc_bytes` etc. link there) is a separate twig-aot build-system task, out of scope for a frontend/backend-op PR.

## ADJ benchmarks — output format leaks the arm and can invert a "blind" LLM-judge result

- **When two arms of a benchmark differ in output *format*, a "blind" LLM judge is not blind — style leaks the condition, and a rubric that rewards that style measures the format, not the property you care about. The failure is not noise; it can systematically *invert* the conclusion.** Surfaced when re-examining ADJ99 (HLE-100, 4 arms × 100 items). ADJ99 scored "defensibility" 0–5 with a blind Opus judge, but the rubric's top scores were "nearly every claim traceable to a cited source" — i.e. it operationalized **citation/traceability density**, not the thesis definition of defensibility ("the load-bearing premise is surfaced and flagged as fallible so a reviewer can override it and re-derive"). The `fw-*` arms emit a literal `RETRIEVED FACTS (CAS): … REASONING CHAIN … [cites: n]` structure; the `plain-*` arms emit prose. A **one-line regex** on `{RETRIEVED FACTS, REASONING CHAIN, [cites, (src:}` separated fw-vs-plain with **100% accuracy** (197/197 plain clean, 197/197 fw tagged), so the judge could read the arm off the style — and **70.3%** of the def≥4 answers were actually *wrong*, because the rubric graded whether claims were *attributed*, not whether the pivot was *true or flagged fallible*. **What the correction showed**: re-judging all 395 non-error cells with a construct-valid rubric (does the trace expose its load-bearing premise and flag it as fallible? — booleans for `premise_named` / `premise_flagged_fallible` / `would_flip`), after **format-normalizing** every trace into one `REASONING:/CONCLUSION:` envelope with all citation chrome stripped, did not merely temper the numbers — it **reversed ADJ99's headline**. The fw−plain gap *widened* (haiku +0.54→+0.81; opus **−0.11→+0.45**, a sign flip), and ADJ99's H2 ("framework helps Opus defensibility"), which ADJ99 had declared FALSIFIED, became TRUE. The bad rubric had been **masking** the framework's real effect (it ~doubles the rate of flagging the pivot as fallible at both model scales), not inflating it. **Fix pattern**: (1) before trusting any arm-vs-arm judge delta, run a *deterministic* leak check — a regex/string classifier that tries to predict the arm from raw output; if it beats chance, the judge isn't blind, so normalize format and re-judge. (2) Name the **construct** you actually mean (locus-exposure / fallibility-flagging) and keep it distinct from the convenient **proxy** (citation count); traceability ≠ defensibility, and they diverge exactly on confidently-wrong-with-citations. (3) Keep the metric correctness-decoupled on purpose — under the corrected rubric 84% of def≥4 cells are *wrong*, which is the intended behavior (a defensible-but-wrong answer should score high). **Caveat that still applies to the correction itself**: it is single-judge (n=1/cell). A corrected number isn't load-bearing until a second, ideally non-Opus, judge over the same normalized traces reproduces the direction (cf. ADJ95's single-judge-noise warning). Full writeup + reproducible pipeline: `code/specs/data/adj99-hle100-run/analysis/rescore/` (PR #5261).

- **Workflow fan-out of LLM judges hits a hard rate-limit wall around ~50 calls per window; throttle with small *sequential* `parallel()` batches, not one big `parallel()`.** Surfaced running the ADJ99 rescore (395 blind Opus judges via the Workflow tool). A single `parallel()` over all 395 (or even 60) let the first ~50 succeed, then every later agent failed with "subagent completed without calling StructuredOutput (after 2 nudges)" — the rate-limit error surfaces as a missing tool call, not an obvious 429. The workflow concurrency cap (`min(16, cores-2)`) is *not* the lever; sustained tokens/min is. **Fix pattern**: process the index list in sequential batches (`for i … i += BATCH { await parallel(batch) }`) with `BATCH = 10` — the per-batch barrier waits for stragglers, which adds idle time and drops sustained throughput to ~24/min, under the ceiling. Bigger batches (30) self-pace to ~36/min and still wall. Accumulate verdicts across runs by idx, recompute the missing set, and mop up stragglers with one more small fresh-window pass. Also: **`args` passed to a workflow can arrive JSON-stringified** — always `Array.isArray(args) ? args : JSON.parse(args)`.

## adj-constraint-solver — solver math over user input must use CHECKED arithmetic (silent i64 wrap flips verdicts)

- **A fixed-width rational (`cas-solve::Frac`, i64-backed) used inside a constraint solver wraps silently on coefficient blow-up, and the wrap can FLIP a feasibility verdict — guard the operands BEFORE the op, not the already-wrapped result.** Surfaced in security review of ADJ constraints C1 (PR #5424, Fourier–Motzkin QF_LRA). The first cut ran FM elimination over `Frac` and tried to bound blow-up with a post-hoc `within_bounds()` check (cap 10^15) on the *result* of `scale().add_scaled()`. Two bugs: (1) `Frac`'s final `(n/g) as i64` cast wraps silently (no panic, no checked cast), so a positive numerator could become negative — and in FM the *sign* of a coefficient decides upper-vs-lower bound and the sign of a residual constant decides Unsat-vs-feasible, so a wrapped sign flips the answer; (2) the guard inspected the already-wrapped value, and 10^15 was far too loose (10^15 × 10^15 = 10^30, and gcd doesn't guarantee reduction below i64::MAX ≈ 9.2·10^18). A crafted `.adj` with ~9-decimal coprime-denominator bounds across a few coupled variables reaches the wrap in one elimination step. **Fix pattern**: replace the fixed-width rational on the user-input math path with a *self-contained checked* rational — `Rat { num: i128, den: i128 }` where every op (`add`/`sub`/`mul`/`div`/`neg`/`new`) returns `Option`, returning `None` on `i128::checked_*` overflow OR past an explicit magnitude cap (`RAT_CAP = 10^18`). Pick the cap so that even the *comparison* cross-products (`a.num * b.den`, used by `Ord`) stay within i128: 10^18 × 10^18 = 10^36 < i128::MAX ≈ 1.7·10^38. Then thread `Option`/`?` through every combinator (`LinForm::scale`/`add_scaled`, `linearize`, the FM combine step, witness back-substitution, and the witness re-check) and collapse any `None` to an `Unknown` outcome — never a wrapped value. **Decision-vs-witness split that limits blast radius**: compute the Sat/Unsat verdict *before* reconstructing a witness, so an overflow during witness construction only drops the witness (return Sat with an empty assignment) while keeping the exact verdict — a feasibility answer is never corrupted by witness math. **Lock it with contract tests**: assert `Rat::new(i128::MAX, 1).is_none()`, `big.mul(big).is_none()`, exact ordering of `1/3 < 1/2 < 2/3`, and an end-to-end "overflowing constraint set is Unsat-or-Unknown, never a fabricated Sat". **General rule**: any solver/CAS arithmetic that runs on attacker- or model-controlled magnitudes (constraint coefficients, LP tableaus, polynomial coeffs) must be overflow-checked at a trust boundary; "exact rational" is only exact until the backing integer wraps. Also: keep the JSON witness rendering through the existing finite-guarding `jnum` (maps non-finite f64 → `null`) so a degenerate witness can't emit `Infinity`/`NaN` and break the JSON.

## adj-lang grammar changes — regenerate the embedded _parser_grammar.rs with the in-crate bin, and never fmt generated files

- **A `.grammar`/`.tokens` edit is not live — the parser is the checked-in `src/_parser_grammar.rs` / `src/_lexer_grammar.rs`, embedded Rust data structures compiled from the grammar. You MUST regenerate them, and each grammar-driven Rust crate ships its own regen binary.** Surfaced adding `optimize_decl` (`minimize`/`maximize`) to `adj_lang.grammar` for ADJ constraints C2. Three traps: (1) **wrong tool/language** — the *Go* `grammar-tools` (`code/programs/go/grammar-tools/`) `compile-grammar` emits *Go* (`gt.RuleReference{…}`); the Rust crates need the *Rust* emitter in the `grammar-tools` **Rust** crate (`grammar_tools::compiler::compile_parser_grammar`). Running the Go tool clobbered the Rust file with Go syntax. (2) **wrong output path** — `cargo run --example` / `--bin` runs with CWD = the crate root, so a relative `src/_parser_grammar.rs` writes into *that crate's* src, not the target crate's. The canonical fix: `adj-lang` already ships `src/bin/regen_grammars.rs`, which reads `code/grammars/adj_lang.{tokens,grammar}` and writes BOTH `src/_lexer_grammar.rs` and `src/_parser_grammar.rs` to the right place — just `cd code/packages/rust/adj-lang && cargo run --bin regen_grammars`. Look for an existing `regen_*` bin before hand-rolling regeneration. (3) **don't `cargo fmt` generated files** — `cargo fmt -p adj-lang` reformats `_lexer_grammar.rs`/`_parser_grammar.rs` (e.g. single-line `keywords: vec![…]` → multi-line) and the regen bin itself, producing churn unrelated to your change. The convention is generated files stay in the compiler's native emission style: after `cargo fmt`, `git checkout` the generated files + the regen bin, then re-run the regen bin so only the intended rule diff remains. **New-keyword note**: structural keywords matched as grammar literals (`"minimize"`, `"solve"`, `"check"`, `"let"`, `"symbol"`) do NOT need a `keywords:` block entry in the `.tokens` file — `"x"` matches an IDENT token by value, so a word matching `[a-z_][a-z0-9_]*` works as a literal with zero tokens-file change. Only add to `keywords:` if you need lexer-level reservation. **Verify**: after regen, `grep <new_rule> src/_parser_grammar.rs` must hit, and a round-trip parse test (`compile("minimize x")`) must produce the new AST node. No CI check regenerates-and-diffs grammars today, but a faithful regen (vs hand-edit) keeps the file byte-identical to what the next contributor's regen produces.

## closurec WHITESPACE_ONLY paren elision — `**` forbids an unparenthesised unary LEFT operand (`(-a)**b` must keep its parens)

- **When stripping a redundant grouping paren around a binary operator's LEFT operand (`(a)+b` → `a+b`), the `**` (exponentiation) case is special: `**` requires its left side to be an `UpdateExpression`, NOT a `UnaryExpression`, so `-a**b` is a `SyntaxError`. `(-a)**b`, `(!a)**b`, `(~a)**b`, `(+a)**b`, `(typeof a)**b`, … MUST keep their parens even though the operand is otherwise "safe" to unwrap.** Surfaced implementing gap-077 (CLOC12.88, left-operand paren elision in `src/whitespace_only.rs`). The first cut mirrored gap-075/078 (right-operand): anchor on a grouping `(` whose matching `)` is followed by a binary operator, check the span with `is_safe_unary_paren_operand`, drop both parens. But `is_safe_unary_paren_operand` accepts a *leading prefix-unary chain* (`-a`, `!a`) as a valid atomic operand — correct for the RIGHT side (`a**-b` is legal) but WRONG for the LEFT side of `**`. So `(-a)**b` got stripped to `-a**b`, which is invalid JS. The byte-identity fixture `minify_exp_of_unary` (`var x=(-a)**b;`, expected to round-trip unchanged) caught it — the unit suite was green (489/489) but the harness `diff_minify_all_fixtures` diverged on exactly that fixture. **Fix pattern**: in the left-operand pre-pass, before dropping the parens, add an `exp_unary_hazard` guard — if the operator immediately after `)` is `**` AND the parenthesised span's FIRST token is a prefix unary (`is_structural_punct` `-`/`+`/`!`/`~` OR a word-like `typeof`/`void`/`delete`/`await`), skip the strip. Only `**` needs this; every other binary operator (`+ - * / % == < && || & << …`) accepts an unparenthesised unary left operand. A plain `(a)**b` (atomic non-unary operand) still strips to `a**b`. **General rule**: the RIGHT and LEFT operands of an operator do NOT have symmetric parenthesisation rules — `**` is the asymmetric case (unary OK on the right, not on the left). When mirroring a right-operand transform to the left, re-derive the safety guard against the JS grammar rather than assuming symmetry, and lean on a JAR-captured byte-identity fixture (`(-a)**b`) to catch the asymmetry. The unit tests alone passed; the golden fixture is what flagged the invalid output.

## git checkout -- <file> restores from the INDEX, not HEAD — a staged change survives the "undo" and silently ships

- **`git checkout -- <file>` (and `git restore <file>` without `--source`) restores the working tree from the *staging area*, NOT from HEAD. If a change was already `git add`-ed, this "undo" does NOT revert it — the staged content stays and gets committed.** Surfaced in the closurec byte-identity loop (CLOC14.54, PR #5553). I appended a wrong gap-112 spec entry and a wrong `("for_await_of", "gap-112…")` IGNORE_FIXTURES entry, then ran `git add -A` (just to inspect `git status`), discovered the mistake, and tried to undo the two files with `git checkout -- code/specs/CLOC12-gaps.md` and `git checkout -- tests/diff_minify.rs`. Because the bad edits were already staged, `checkout --` restored them *from the index* — they survived, shipped in #5553, and (a) wrongly marked the **passing** `minify_for_await_of` fixture as an ignored gap, and (b) left a duplicate `### gap-112` spec entry. The diff_minify walk-test stayed green (ignoring a passing fixture just skips it — no failure), so nothing caught it until the *next* PR's rebase surfaced the duplicates as a conflict. **Fix pattern**: to truly discard a change after a `git add`, use `git restore --source=HEAD --staged --worktree <file>` (or `git checkout HEAD -- <file>`, which is explicit about the source being HEAD). The bare `git checkout -- <file>` form only works as an "undo" when the change was never staged. **General rule**: after any "revert this edit" step that *follows* a `git add`, re-run `git diff --cached <file>` (and `git diff <file>`) to confirm the change is actually gone before committing — an "undo" that reads from the index is a no-op for staged hunks. Especially dangerous for IGNORE-list / allowlist edits, where wrongly ignoring a passing test produces no failure signal.

## `git stash` is ONE list shared across every worktree in this repo — never `pop`, and re-identify by SHA before dropping

- **All worktrees checked out from this repo share a single stash stack.** I needed to temporarily shelve in-progress changes (to test whether a failing test was pre-existing on a clean tree) and ran `git stash push -u`. `git stash list` immediately after showed 100+ entries from unrelated concurrent sessions/branches going back months — my push landed at `stash@{0}`, but that index is not stable: any other agent stashing concurrently in a different worktree shifts every index. **`git stash pop` is unsafe here** — if the index has shifted since you pushed, pop applies (and drops) someone else's entry instead of yours, exactly the failure mode the pre-existing `feedback_git_stash_is_shared_across_worktrees` lesson warns about (seen previously as an "accidentally popped by concurrent worktree agent" recovery entry sitting in this same stack). **Safe pattern**: immediately after `git stash push`, capture the SHA with `git rev-parse stash@{0}`; when ready to restore, use `git stash apply <that-SHA>` (not `pop`, and not the index) so you're unambiguously restoring your own entry regardless of what else has been pushed/popped since; only after `apply` succeeds, re-check `git rev-parse stash@{0}` still equals your captured SHA before `git stash drop stash@{0}` — if it doesn't match, the stack has moved and you must locate your entry by SHA/message in `git stash list` instead of dropping blindly.

## Rebasing N sibling PRs that each append a near-identical function at the same insertion point produces conflict markers that split MID-FUNCTION, not between functions — never trust a "resolved" file without building it

- **Context:** a 9-architecture-expansion campaign landed 9 sibling PRs, each adding one new `pub fn compile_file_to_<arch>_bin(...)` function (plus an `EmitMode` variant, a `parse_emit_value` match arm, and a `dispatch()` `if` block) to the same shared file (`lang-aot/src/lib.rs`, `bin/lang_aot.rs`). Rebasing PR N+1 onto a base that already had PRs 1..N merged produced conflict markers as expected — but for the two files where each new function's doc-comment/body was structurally near-identical to its neighbors' (differing only in architecture name/types), git's diff3 did NOT cleanly bracket "HEAD's functions" vs "incoming's function" as two whole blocks. It found long stretches of *textually similar* boilerplate (doc-comment headers like `/// # Errors`, the `let source = ...; let stem = ...;` prologue, the `.map_err(...)?;` tail) as "common context" and drew the `<<<<<<</=======/>>>>>>>` markers **inside** individual function bodies and even inside function signatures — e.g. `pub fn compile_file_to_arm1_bin(` from one side directly followed by `/// lang-aot foo.twig --emit=mips-r2000 ...` from the other, or a `Mos6502Bin` dispatch arm's `.map_err(...)` call and closing `}` silently deleted because the *next* arch's arm's opening comment landed inside the same hunk.
- **Why "resolve by concatenating both sides" (safe for genuinely additive hunks — new enum variants, new match arms with no shared trailing code) actively corrupts this case:** concatenating ours-then-theirs when the split point is mid-function produces syntactically-plausible-looking Rust (it has balanced-looking indentation) that is actually missing a `.map_err(...);` + `}` for one function and has a duplicate/orphaned function signature for another. This is NOT caught by re-reading the diff hunk in isolation — it only surfaces as `error: this file contains an unclosed delimiter` / `mismatched closing delimiter` pointing at a totally unrelated line number dozens of lines later, because the parser only discovers the imbalance once it runs out of file.
- **How I caught it and the safe procedure that worked:** for every hunk touching one of these repeated per-architecture functions, first `Read` a wide enough window (100+ lines) around each conflict marker to check whether the marker falls *between* two complete `pub fn ... { ... }` blocks or *inside* one. If inside, do NOT hand-edit around the markers — instead extract each side's function as ground truth directly from its source ref (`git show <ref>:<path> | awk '/^\/\/\/ doc-comment-start-pattern/{flag=1} flag{print} /^pub fn the_fn_name/{fn=1} fn && /^}/{exit}'`, one `awk` per function per ref), concatenate the extracted, verified-complete functions in the right order, and use Python to splice that block in as a literal line-range replacement (`lines[:start] + replacement + lines[end:]`) rather than editing the conflict markers in place. For the shorter, genuinely-non-overlapping hunks (enum variant lists, `LangAotError` variant + `Display` arm, `Cargo.toml` member lists), a scripted "delete markers, keep both sides in order" pass is safe and fast — the risk is specific to hunks spanning function bodies with similar boilerplate.
- **The one invariant that makes this reliable regardless of how careful the manual resolution was: `cargo build -p <every-newly-touched-crate>` (or the equivalent for the language) after EVERY conflict resolution, before `git add`/`rebase --continue`/push.** Across 6 rebased PRs in this campaign, the build caught every single mis-split immediately and precisely (`unclosed delimiter`, `mismatched closing delimiter`) — never once passed with silently-wrong code. Don't trust "no conflict markers remain" as sufficient; markers-resolved and syntactically-valid are different guarantees, and only a real build checks the second one. `git push --force-with-lease` only after the build (and ideally `cargo test`) is green.

## closurec number printer — emit over the f64 VALUE with shortest-round-trip, and don't globally round the integer (it corrupts the scientific form)

When testing variable-length codecs, the round-trip test parameter set MUST include at least one value in each form whose low byte is < 128 (e.g. 256, 300, 515, 768) — otherwise a low-byte-first regression silently passes. Pure round-trip tests on a self-consistent broken codec are blind to byte-order bugs by construction. The integration test that catches it reliably is "≥ 200 KB of repetitive text → ≥ 128 sequences in a single block" — that input distribution naturally produces counts spanning both halves of the 2-byte range.

- **JS `Number`s are IEEE-754 f64; Closure's number printer emits the shortest STRING (decimal / uppercase-`E` scientific / lowercase `0x` hex) that round-trips to the f64 VALUE, not to the exact source integer. When adding a new candidate form (gap-114: large int → hex), round to f64 ONLY for that candidate — do NOT round the shared integer `n` globally, because the existing `scientific_form_of(n)` depends on `n` being the exact `m×10^e`.** Surfaced closing gap-114 in `normalize_number_value` (`whitespace_only.rs`). The hex case is genuinely f64-sensitive: `123456789012345678` exceeds 2^53, so its runtime value is the nearest double `123456789012345680`, and upstream emits `0x1b69b4ba630f350` (hex of the double) — NOT `0x…34e` (hex of the exact u128). So the hex candidate MUST be `format!("0x{:x}", (n as f64) as u128)`. The trap: I first "fixed" this by rounding `n` globally (`let n = (n as f64) as u128;`) before computing decimal/scientific/hex — which made the gap-114 fixture pass but **regressed `minify_num_exp_23`**: `100000000000000000000000` (10^23) round-trips through f64 to `99999999999999991611392`, which is no longer a clean power of ten, so `scientific_form_of` returned `None`, the `1E23` candidate vanished, and hex (`0x152d02c7e14af6000000`, 22 chars) wrongly won over the correct `1E23` (4 chars). The unit suite stayed green (625 passed) — only the `diff_minify` golden fixture caught it. **Fix pattern**: keep decimal/scientific over the EXACT integer (upstream's shortest-round-trip decimal reproduces `1E23` for a clean power), and round to f64 for the hex candidate alone. **General rule**: the byte-exact match for `> 2^53` numbers requires shortest-round-trip (Grisu/Ryu) formatting over the double; that's deferred. Until then, only add narrowly-scoped f64-aware candidates and verify against the JAR across the 2^53 boundary AND clean powers of ten (`1E23`, `1E18`). And — as with gap-077 — a green unit suite is not enough for number/formatting changes; the JAR-captured `diff_minify` golden is the gate that catches the cross-form regression.

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

---

## Lesson 97 — `code/packages/haskell/zstd` inherited the same repo-wide FSE sequences-codec + FHD checksum-bit bugs as java/rust, confirmed via real `zstd` CLI interop (TC-9)

**Date:** 2026-08-03

**What happened:** A sibling effort (PR #9780, `java/zstd`; see its `lessons.md` entries once merged, titled "ZStd FHD Content_Checksum_Flag is bit 2, not bit 4" and "ZStd sequences-section FSE codec had THREE compounding bugs") found and fixed a cluster of RFC 8878 non-conformance bugs in the sequences-section FSE codec, and confirmed the same bugs in `code/packages/rust/zstd` (the shared reference the ports were designed against). `code/packages/haskell/zstd` was added independently, months after that reference (PR #8712) — so it was audited from scratch rather than assumed guilty or innocent. It turned out to have inherited the identical bug pattern:

1. `spreadSymbols`'s table-spread used a fabricated two-pass split (all `count > 1` symbols in ascending order, then all `count == 1` symbols in ascending order) instead of the real single-pass algorithm (`FSE_buildDTable_internal`: one pass over symbols `0..maxSymbolValue`, placing each symbol's full count immediately).
2. `decodeSequences`/`encodeOne` fused FSE symbol-peek and state-update into one step, in the wrong order (LL, OF, ML) and *before* reading any extra bits, instead of: peek LL/ML/OF (free), read extras OF/ML/LL, then update states LL/ML/OF. The one-time initial-state read at the top of a compressed block was also wrong (read LL, ML, OF instead of the RFC's asymmetric LL, OF, ML).
3. The state-transition update was performed unconditionally for every sequence, including the last one in a block, where a real decoder never performs it (no "next" sequence needs a prepared state) and a real encoder cannot produce it via a normal bit-flushing transition (no corresponding decode-side read exists to consume it) — it needs a direct-formula init (`FSE_initCState2`) that writes zero bits.
4. Frame Header Descriptor `Content_Checksum_Flag` was read from bit 4 (`Unused_bit`) instead of bit 2, and the "reserved bits" check treated bits 2+3 as jointly reserved — rejecting every real checksummed frame while never detecting a checksum trailer on any frame (same root cause as Lesson 94's trailing-bytes check: the two bugs mask each other until both a strict trailing-bytes check AND a correct checksum-flag bit are in place simultaneously).

**Why it survived undetected:** Exactly Lesson 94's shape — bugs 1-3 are self-cancelling as long as encode and decode agree on the same wrong convention, and this package's own encoder/decoder pair always did. All 16 pre-existing unit tests passed throughout, including a hard-coded "established cross-language compressed vector" fixture — because that fixture had been generated by the *same* buggy encoder, so it was internally consistent with itself, not with real zstd. There was no CLI-interop test in this package (or any other language's `zstd` port) before this — confirmed nothing under `code/packages/*/zstd` shelled out to `subprocess`/`ProcessBuilder`/`Command::new`/ the real `zstd` binary prior to PR #9780.

**Rule:**
- Ported/parallel-developed code should never be assumed correct-by-association *or* buggy-by-association — audit it. Here the "maybe it's fine, it's a different author/era" hope was wrong; the "assume it's broken like the others" shortcut would have skipped the actual verification step (running the real interop test) that this task required regardless.
- Fixed by porting the corrected Java algorithm (`gh pr diff 9780`) into idiomatic Haskell: `spreadSymbols` reduced to a single ascending pass; `decodeSequences` restructured into explicit peek/extras/conditional-update phases; a new `encodeInitState` function (mirrors `FSE_initCState2`) added for the reverse-encode loop's first (semantically last) sequence; the FHD checksum-flag/reserved-bit masks corrected from `0x10`/`0x0C` to `0x04`/`0x08`.
- Verified via a new `ZstdCliInteropSpec` (TC-9): `compress` here / `zstd -d` there, and `zstd -c` there / `decompress` here, both directions, plus a high-sequence-count case crossing the 1-byte→2-byte sequence-count wire-format boundary. Also spot-checked ad hoc against a 6-case fuzz corpus (periodic patterns, pseudo-random bytes, prose, mixed, repeat-distance) and a real `zstd -c`/`--no-check` checksum-bit probe, all outside the committed test suite, before writing the permanent test. The pre-fix code failed the real CLI with `Decoding error (36): Data corruption detected` on every input that produced more than a trivial number of LZ77 sequences, exactly as in the java/rust findings — confirming this is a systemic bug in the shared design, not a language-specific porting mistake.
- Two of the 16 pre-existing unit tests needed their hard-coded byte fixtures regenerated post-fix (the cross-language compressed vector, and the hand-crafted checksum-frame descriptor byte) — a expected consequence of fixing a bug that both the implementation *and* its fixtures were built around.

See also `gh pr diff 9780` (java/zstd fix) and PR #9774 for the same audit's discovery trail across languages.

---

## Lesson 98 — `code/packages/c/zstd`'s decoder needed real Repeated-Offset (R1/R2/R3) support to pass TC-9 in both directions, even though this repo's ZStd ports deliberately never EMIT repeat-offset codes

**Date:** 2026-08-05

**What happened:** While implementing the new `c/zstd` port (CMP07), TC-9's fixed prose corpus ("the quick brown fox..." × 25) passed real `zstd` CLI interop cleanly in both directions on the first try — the FSE codec was transcribed directly from the already-corrected `code/packages/rust/zstd` reference (Lesson 96), so none of that bug class reappeared. To gain more confidence than one fixed corpus provides, an ad hoc 200-trial fuzz harness (random/periodic/constant/ramp byte patterns, sizes up to 5000 bytes) was run against the real `zstd` CLI in both directions before pushing. Trial 2 — 4713 bytes of a single repeated byte (`'Z'`) — failed: `our decompress() failed on real zstd output`. Manually decoding the captured `.zst` frame showed real `zstd` chose a **Compressed** block (not the RLE block type this port's own encoder would pick for constant data) containing exactly one sequence: 2 literal bytes ("ZZ") + one match with `Offset_Value = 1`. Per RFC 8878, `Offset_Value <= 3` is a **repeat-offset reference** (reuse one of three tracked recent offsets, R1/R2/R3, default `1/4/8`), not a literal `Offset_Value - 3` computation — and this port's decoder (like the Rust reference it was transcribed from) only implemented the explicit-offset path, computing `offset = of_raw - 3` unconditionally. For `of_raw = 1` that underflows to a huge bogus offset, which the existing offset-bounds check correctly rejected as malformed — but the frame was actually valid, encoded using a mechanism the decoder didn't understand.

**Why it went unnoticed:** This repo's ZStd ports all implement the "no repeat-offset shortcuts" educational simplification on the ENCODER side — `encode_sequences_section` never emits an offset code `< 2`, since the minimum possible LZ77 match offset is 1, giving `raw_off = offset + 3 >= 4` always. So a port's own `compress()`/`decompress()` round trip — and TC-9's one fixed prose corpus, which apparently never happened to produce a real-`zstd`-encoded sequence with `Offset_Value <= 3` — never exercises the repeat-offset DECODE path at all. But the real `zstd` CLI's encoder uses repeat offsets constantly (they're one of its principal entropy wins, especially for periodic or highly repetitive data — exactly the kind of input a compression test suite is most likely to include), so any decoder that only understands explicit offset codes will systematically fail to decode a meaningful fraction of real-world `.zst` files. This is a different bug from Lesson 96's FSE-codec class (which was about the ENCODE/DECODE pair disagreeing with the real format symmetrically); this one is a decode-only FEATURE GAP that a self-consistent codec — and even the one prescribed TC-9 corpus — can fail to surface, because "spec-compliant against the one test input" and "spec-compliant against arbitrary real-world input" are different claims.

**Rule:**
- When a spec's own interop test corpus is a single fixed input, treat it as necessary, not sufficient, evidence — especially for a decoder that must accept the real ecosystem's full output space, not just its own encoder's output space. Before trusting a "TC-9 passes" result, fuzz the same interop check (compress-here/decode-there, compress-there/decode-here) across varied inputs (random, periodic, constant-byte, ramps, several sizes) — cheap to write, and it catches exactly this class of "narrow test corpus never exercised this code path" gap that a single hand-picked example cannot.
- An "educational subset" simplification stated as "we don't emit X" must not be silently read as "we don't need to understand X on decode" when the decoder's job includes accepting output from a fuller, independent encoder (the real `zstd` CLI) that DOES emit X. Decode-side feature scope and encode-side feature scope are separate decisions — the former is bounded by what real-world producers emit, not by what this port's own encoder produces.
- Fixed in `code/packages/c/zstd/src/zstd.c` (`decompress_block`, `zstd_decompress`): implemented full Repeated_Offset (R1/R2/R3) decode support per RFC 8878 §3.1.1.3.2.1.1, cross-checked against both the RFC prose and the literal reference C source (`ZSTD_decodeSequence` in `zstd_decompress_block.c`, fetched directly rather than recalled from memory, per the Lesson-96 playbook) — including the "when Literals_Length is 0, repeated offsets are shifted by 1" special case. The three registers are frame-scoped (default `1/4/8` "for the first block", threaded unmodified through Raw/RLE blocks, updated after every Compressed block's sequences, explicit-offset or repeat-offset alike) — NOT block-scoped or reset per Compressed block. The port's own encoder is intentionally left unchanged (still never emits repeat-offset codes; this is a decode-only fix). Re-verified with the original 200-trial fuzz harness (now passing) plus the existing fixed TC-1..TC-10 suite (all 89 checks, unaffected, since this port's own round trip never touches the new code path) and ASan/UBSan clean, then run again at 1500 trials.
- Any other language port in this repo that implements ZStd decode should be treated as suspect of the same gap until it is specifically checked against varied real-world `zstd`-CLI-encoded input, not just its own fixed TC-9 corpus — flagged as a plausible cross-language follow-up, not verified here (out of scope for this PR, which only touches `c/zstd`).

---

## WEB09 Java Conduit / JNI cross-thread callbacks — five gotchas

**Date:** 2026-04-27

Porting Conduit to Java over `web-core` via `jni-bridge` surfaced a cluster of JNI-specific traps. All five recur for any future JVM↔Rust callback port (Kotlin, etc.):

1. **`JNIEnv*` is thread-local; the JavaVM is not.** web-core dispatches on Rust I/O threads the JVM never created. You cannot reuse the registration-call `env`. Capture the `JavaVM*` once via `GetJavaVM` (offset 219 on the JNIEnv table) on a JVM thread, then on each I/O thread call `AttachCurrentThreadAsDaemon` (offset 7 on the *JavaVM invocation* table — a different table, `JavaVM = *const *const c_void`). Daemon attach is idempotent, needs no `DetachCurrentThread`, and doesn't block JVM shutdown. Plain `AttachCurrentThread` requires a matching detach before the thread exits or the JVM aborts.

2. **Local refs leak on a self-attached thread.** Local references are normally freed when a native method returns to Java — but an I/O thread that attached itself and loops forever never returns, so every `NewObjectA`/`NewStringUTF`/`CallObjectMethod` local ref accumulates = unbounded leak per request. Bracket each dispatch with `PushLocalFrame(env, n)` / `PopLocalFrame(env, null)` (offsets 19/20) and copy everything you need into owned Rust values before popping.

3. **Handler objects must be promoted to global refs.** A Java lambda passed to `addRoute` is a local ref, dead after the native call returns. `NewGlobalRef` (offset 21) it at registration; `DeleteGlobalRef` (22) on dispose. `jclass` from `FindClass` is also local — pin the classes you cache as globals too, and resolve all method IDs on a JVM thread (FindClass from a native thread uses the system classloader, which can't see app classes).

4. **Disjoint closure capture drops the Send+Sync wrapper.** web-core closures must be `Send + Sync`. Wrapping a raw `jobject` in a `struct Obj(jobject); unsafe impl Send/Sync` is not enough if the closure writes `obj.0` — Rust 2021 captures only the inner `*mut c_void` field (not Send). Add a `fn get(&self) -> jobject { self.0 }` and call `obj.get()` so the whole wrapper is captured. (Same lesson as the Node port's `ThreadSafePtr::get`.)

5. **A Rust panic must never unwind across `extern "C"`.** A poisoned `Mutex` makes `.lock().unwrap()` panic; if that happens inside a JNI entrypoint called from a JVM thread, the unwind crosses the FFI boundary = UB (usually a JVM abort). Use `.lock().unwrap_or_else(|e| e.into_inner())` (poison-tolerant), and null-check every peer `jlong` before `Box::from_raw`/deref so a use-after-close lifecycle bug degrades to a safe no-op instead of dereferencing a dangling pointer. Route handler *errors* as data (an `Outcome` enum), never as panics.

Also: the security sub-agent flagged a `pct_decode` "off-by-one" (`i + 2 < len`) as HIGH — it was a false positive (`i+2 < len` ⟺ `i+2` is a valid index; a trailing `%XX` decodes fine). Always settle a claimed boundary bug with a targeted unit test before "fixing" it; here the fix would have introduced the bug.

---

## WEB11 Perl Conduit — the crash was `newSVpv(ptr, 0)`, NOT a threading wall

**Date:** 2026-06-14

**What happened:** The Perl Conduit cdylib SIGSEGV'd on the first request dispatch. I initially (wrongly) blamed non-threaded Perl + web-core worker threads. The real cause was a one-line memory bug, and the threading model is actually fine.

**Two corrected findings:**

1. **`newSVpv(ptr, len)` treats `len == 0` as "call `strlen(ptr)`".** Perl's `newSVpv` uses the C string length when you pass 0. An empty Rust `&str` (`""`) has a non-NUL-terminated pointer, so `strlen` reads out of bounds → segfault. The very first empty field (an empty `QUERY_STRING`) crashed it. **Fix: use `newSVpvn` (explicit length, never strlens) for ALL Rust→Perl string conversions** where the value can be empty. The crash reproduced single-threaded, which is what unmasked the misdiagnosis.

2. **The embeddable-http-server serve path is single-threaded inline — it does NOT spawn.** `HttpServer::bind` builds a single `TcpRuntime` (not the `ShardedTcpRuntime`); `TcpRuntime::serve()` → `StreamReactor::serve()` runs the event loop on the *calling* thread and dispatches handlers inline. So foreground `serve()` runs handlers on the calling thread — perfect for a single-interpreter language. (The `ShardedTcpRuntime` that DOES `thread::spawn` per worker is a separate, unused-by-this-path API.) The only spawn was in my own cdylib's `serve_background`; that one path is unsafe for non-threaded Perl, so the Perl port serves in the foreground and tests fork a client process.

**Rule:** When a foreign-language port over web-core crashes on dispatch, reproduce it **single-threaded** (foreground serve in a standalone process + curl) before blaming threads. And from Rust always cross the boundary with the explicit-length string constructor (`newSVpvn`, not `newSVpv`); the strlen-on-zero footgun is silent until an empty value hits it. For a single-interpreter language, prefer foreground `serve()` (the inline reactor runs handlers on the calling thread) and fork a client for concurrent E2E tests.

---

## WEB12 Swift Conduit — three Swift/SPM gotchas + the reusable C ABI pattern

**Date:** 2026-06-14

Built `conduit-capi` (a reusable Rust C ABI for the whole Conduit framework, to be
shared by WEB12–WEB18) and the Swift port on top of it. Three gotchas worth
remembering:

1. **`"\r\n"` is ONE extended grapheme cluster in Swift.** A CRLF check written as
   `location.contains("\r")` (Character-based) returns FALSE for a string
   containing `\r\n`, because Swift treats CRLF as a single `Character`. Scan
   **unicode scalars** instead: `location.unicodeScalars.contains { $0 == "\r" || $0 == "\n" }`.
   The response-splitting guard silently passed until a test caught it.

2. **A library's relative `-L` linker path breaks for downstream packages.** SPM
   runs the linker from the *root* package being built. So `Sources/CConduit` in
   the Conduit library's `linkerSettings` resolves correctly for Conduit's own
   tests but NOT when a demo/executable in another package links it — you get
   `library 'conduit_capi' not found`. Fix: the downstream package re-adds its own
   `-L <relative-path-to-the-staged-lib>` in its target's `linkerSettings`. (The
   library's wrong path then just emits a harmless `search path not found` warning.)

3. **Edition-2021 disjoint closure capture defeats a `Send + Sync` wrapper.** A
   closure that touches `cb.ctx` (a raw pointer field) captures *that field*, not
   the whole `Send + Sync` struct, so the closure isn't `Send`/`Sync`. Call a
   `&self` method (`cb.call(...)`) instead — a method borrows the whole struct, so
   the closure captures the wrapper and inherits its `Send + Sync`.

**Reusable C ABI pattern:** the seven C-ABI-capable ports (Swift/C++/Go/C#/F#/Dart/
Haskell) share ONE `conduit-capi` crate instead of re-wrapping the facade each
time. The trust boundary (header_safe, status clamp, UTF-8 validation, panic
isolation) is audited once. Handlers cross as a C function pointer + opaque `ctx`
+ a `ctx_free` destructor; the host boxes its closure. `crate-type = ["staticlib",
"cdylib", "lib"]` serves compile-time linkers, FFI loaders, and `cargo test`.
---

## Swift IRC binding — POSIX `close`/`send` shadow inside a socket class

**Date:** 2026-06-14

The Swift IRC port (`IrcServerNative`, on the reusable `irc-server-capi` C ABI —
the same pattern as `conduit-capi`) reuses Conduit's raw POSIX-socket test client.
Gotcha: a class with a method named `close()` (or `send()`) **shadows the global C
`close`/`send`** inside its own body — `close(fd)` resolves to the instance method
and fails to compile (`use of 'close' refers to instance method rather than global
function`). Even inside `init`, before the method exists conceptually, the name
resolves to the member. Fix: wrap the C calls in free functions
(`private func cclose(_ fd: Int32) -> Int32 { close(fd) }`) and call those from the
class. Same `SOCK_STREAM` enum-vs-macro normalization as Conduit applies.

This completed the all-Rust IRC engine port to **all 8 targets** (Rust engine +
Python/Ruby/Node/Java/Elixir/Perl/Swift). The native-binding effort surfaced four
real engine bugs, each fixed engine-wide with a regression test: panic containment
(`catch_unwind` + poison-tolerant locks), RST survival (close-now instead of `?`
propagation), the stop-before-serve race (don't reset the stop flag at loop entry),
and an Elixir concurrent-resource data race (`Mutex` around the inner handle).
---

## stream-reactor — never reset a cancellation flag inside the run loop's own entry

**Date:** 2026-06-14

**`StreamReactor::serve()` must NOT reset the stop flag — a `stop()` racing serve startup gets silently swallowed.** The reactor's `serve()` used to do `self.stop_flag.store(false)` at the top. The flag is already `false` at construction, so the reset was redundant on first serve — but it created a race: an FFI binding (JNI/N-API/etc.) that flips a "running" flag and lets the caller request `stop()` *before* the background serve thread enters the loop would have its stop erased, so `serve()` runs forever and `join()` hangs. This was invisible in Python/Ruby because their tests wait for the server to actually be listening before stopping (and Python/Ruby `join` with a timeout); it surfaced in the JVM binding, whose JUnit tests flip running→stop synchronously in one thread, and whose native `join()` has no timeout. **Rule:** a `stop()` request must never be lost — don't reset stop/cancellation flags inside the run loop's own entry. Reset (re-arm) only via an explicit separate operation if a consumer truly needs to re-run. Reproduce FFI hangs single-threaded in pure Rust (spawn serve, `stop()` immediately, `join()`) before blaming the binding; use `sample <pid>`/jstack to see the native thread stuck in `kevent`.

---

## WEB13 C++ Conduit — four C++ gotchas linking the reusable C ABI

**Date:** 2026-06-14

Second consumer of the `conduit-capi` C ABI (after Swift). Header-only C++ wrapper.
Four things worth remembering:

1. **A reused HTTP-client result struct must be RESET each request.** The E2E
   client appended parsed headers with `emplace_back` into a caller-provided
   `out` struct reused across requests, so `headerValue("content-type")` returned
   the FIRST match (from an earlier route) — the echo content-type assertion
   failed even though the server was correct. Always `out = Result{};` at the top
   of the request function. (The server/port was never wrong; the test client was.)

2. **A joinable `std::thread` destroyed during stack unwinding calls
   `std::terminate`.** A failing assertion in the E2E threw, which destroyed the
   still-joinable watchdog `std::thread` before the harness could catch it →
   `libc++abi: terminating`, masking the real failure. Wrap the body in
   `try { ... } catch (...) { join_watchdog(); throw; }` so the thread is always
   joined first and the real error surfaces.

3. **`extern "C" inline` trampolines, not C++ ones.** Passing a C++ function
   pointer where the C ABI typedef expects a C-linkage pointer warns under
   `-Wpedantic -Werror` (and is technically a language-linkage mismatch). Declare
   the trampolines `extern "C"` (and `inline` so the single header stays
   include-safe across TUs).

4. **Delete the copy ctor → you must add a move ctor for return-by-value.** A
   `make_app()` factory returning `Application` by value fails to compile if the
   copy ctor is deleted and no move ctor exists — NRVO is not guaranteed for a
   named local. Add `Application(Application&&) noexcept` that transfers the handle
   and marks the source consumed.

**Link flags:** a Rust staticlib needs its platform `native-static-libs` (e.g.
`-liconv -lSystem -lc -lm` on macOS, `-lpthread -ldl -lrt -lm` on Linux). Don't
hardcode them — query `cargo rustc --release --crate-type staticlib -- --print
native-static-libs` and pass the result to the C++ link line.

**Build-tool note:** C++ packages are discovered as "unknown" language (the
build-tool's language list has no "cpp"), so the undeclared-local-ref validator
skips them — but declare `# build-tool: deps=rust/conduit-capi` anyway so the
build graph pulls the Rust crate in and the runner gets `cargo`.

## WEB14 Go Conduit — cgo + conduit-capi gotchas

**conduit.Application.GetSetting is invalid after Bind().** `conduit_server_bind`
moves (and frees) the native `ConduitApp*` into the server handle on BOTH
success and failure. Any Go closure registered as a handler, before-filter, or
after-hook that calls `app.GetSetting()` at request time will read from a freed
pointer. Fix: pre-capture settings into local strings BEFORE registering the
hooks. Surfaced in conduit-hello's after-hook stamping `x-served-by`.

**cgo Rule 6 / uintptr-as-void*: use cgo.Handle, not raw casts.** Passing a
`uintptr` value of a Go pointer directly to C as `void*` triggers `go vet` Rule
6 warnings (the GC may move the pointer between the cast and the store). The
correct pattern: `cgo.NewHandle(fn)` returns a GC-safe integer handle; cast
`C.uintptr_t(handle)` to `void*` in a C shim (where the cast is invisible to
`go vet`). The trampoline recovers the value with `cgo.Handle(uintptr(ctx)).Value()`.

**Static link the Rust .a by full path, not -lconduit_capi.** On Linux `ld`
resolves `-lconduit_capi` to the sibling `.so` cdylib, not the `.a` staticlib.
Resulting binary fails at runtime (`libconduit_capi.so.0: not found`). Use the
full path in `#cgo LDFLAGS: ${SRCDIR}/.../libconduit_capi.a` instead.

**cgo LDFLAGS native deps differ by OS.** macOS needs `-liconv`; Linux needs
`-lpthread -ldl -lm -lrt -lutil`. The full list comes from
`cargo rustc --release --crate-type staticlib -- --print native-static-libs`.

**Go BUILD files that link a Rust static lib must be self-sufficient (build
the Rust lib themselves).** The `# build-tool: deps=rust/foo` directive ensures
ordering in the *normal* build workflow (which uses the build-tool directly),
but the CodeQL workflow generates a `build-plan.json` of 300+ packages and
processes them in plan order — which may reach `go/conduit` before
`rust/conduit-capi`. Bare `go test` then fails: `libconduit_capi.a: No such
file or directory`. Fix: add a `tools/run-tests.sh` that runs `cargo build
--release` for the Rust crate first (mirroring the C++ pattern), and call
`sh tools/run-tests.sh` from the BUILD file. The deps= hint still fires in
the normal build (making the cargo step a fast no-op); CodeQL gets the Rust
lib on demand. Surfaced in PR #5739.

## tcp-runtime — cross-thread reactor wakeup + running ThreadSanitizer locally

**Date:** 2026-06-14

Multi-core PR2: a `Send + Sync` `WakeHandle` lets an off-reactor mailbox interrupt
the reactor's `poll` immediately instead of waiting the 10 ms poll timeout. Two
durable lessons:

1. **Decouple the cross-thread trigger from the `&mut self` platform.** The
   platform's `wake(&mut self)` is owned by the reactor thread; producers are on
   other threads. The fix is a handle that owns a *duplicated* OS primitive
   (`Kqueue::try_clone` of the kqueue fd; `OwnedFd::try_clone` of the `eventfd`)
   and re-issues the trigger via a `&self` syscall (`kevent`/`write`, both
   thread-safe). Give the trait a **default `wake_handle` returning
   `Unsupported`** so a backend that can't share its primitive (IOCP, today) keeps
   working — callers just fall back to the poll timeout. Never make the whole
   reactor depend on a capability one platform lacks.

2. **Running ThreadSanitizer on this repo needs `-Zbuild-std`.** Plain
   `RUSTFLAGS="-Zsanitizer=thread" cargo +nightly test` fails with "mixing
   `-Zsanitizer` will cause an ABI mismatch" because the prebuilt std/deps weren't
   compiled with the sanitizer. Rebuild everything from source:
   ```
   rustup +nightly component add rust-src
   RUSTFLAGS="-Zsanitizer=thread" cargo +nightly test -Zbuild-std \
     -p stream-reactor --lib --target aarch64-apple-darwin <test-filter>
   ```
   The `--target` is required (build-std needs an explicit target). A clean TSan
   run on the wake stress test (8 threads firing `wake()` while the reactor
   drains) is the canonical check for the new cross-thread `unsafe` fd sharing.

## multi-core — SO_REUSEPORT does NOT load-balance on macOS/BSD (only Linux)

**Date:** 2026-06-14

The `sharded-echo-bench` (multi-core PR4) measured the `ShardedTcpRuntime` at
1/2/4/8 reactor shards and the per-shard accept-balance column revealed a hard
truth: on **macOS** every connection lands on a **single** shard (`[0% 0% 0% 100%]`),
so throughput is flat (8-shard ≈ 0.97× single-shard). Plain `SO_REUSEPORT` only
load-balances on **Linux** (kernel hash across the reuseport group); macOS/*BSD
permit the multi-bind but deliver all connections to one socket (in practice the
last bound), and FreeBSD needs the separate `SO_REUSEPORT_LB` option.

Consequences:
- The N-reactor + SO_REUSEPORT design scales on **Linux** (the deployment target
  and where CI runs) but gives **no** accept distribution on a macOS dev box.
  Don't read a flat local (macOS) scaling curve as "the runtime doesn't scale" —
  check the shard-balance column first; it's a kernel policy, not a runtime bug.
- True multi-core accept distribution on macOS/BSD needs an explicit fan-out (a
  single accept loop that round-robins accepted fds to per-core reactors, or
  `SO_REUSEPORT_LB`), not reliance on the kernel balancing plain `SO_REUSEPORT`.
- Lesson for benchmarks: always surface the *distribution*, not just the average.
  A single throughput number would have hidden this; the per-shard column made it
  obvious in one glance.

## multi-core — accept fan-out fixes macOS distribution; even ≠ throughput scaling

**Date:** 2026-06-14

Closing the macOS `SO_REUSEPORT` gap (multi-core PR5): on macOS/BSD a
`ShardedTcpRuntime` now uses an explicit accept fan-out — one acceptor owns the
client-facing listener and round-robins each accepted socket to a worker reactor
via `adopt_stream` (transport-platform) + `StreamMailbox::adopt_connection`
(stream-reactor). The workers bind throwaway loopback listeners and only *serve*
adopted connections. Linux keeps the kernel `SO_REUSEPORT` balancing.

Two durable lessons:

1. **Even distribution is necessary but NOT sufficient for throughput scaling.**
   The fan-out fixed the shard balance (`[0% … 100%]` → `[13% × 8]`) and made
   `conns/s` scale (connection setup parallelizes). But steady-state `req/s` for a
   trivial **echo** stayed flat — echo on loopback is latency-bound, not
   CPU-bound, so spreading connections across cores adds no throughput when the
   per-request work is near-zero. Don't read "added cores, req/s didn't move" as
   "the fan-out didn't work" — check the shard-balance column (it did) and
   remember scaling only shows up once each connection's work can saturate a core
   (real parsing, TLS, compute). A benchmark must measure a CPU-bound load to
   demonstrate throughput scaling; a benchmark must measure *distribution* to
   demonstrate the fan-out.

2. **Cross-thread fd handoff is the crux and must be TSan-clean.** The acceptor
   creates an `OwnedFd` on its thread and hands it to a worker reactor through the
   mailbox; the worker adopts it into its own kqueue. `OwnedFd` is `Send`, the fd
   is linear (consumed exactly once on every path), and the whole chain
   (acceptor → mailbox → adopt_stream) passes under ThreadSanitizer. Run the
   distribution test under `-Zsanitizer=thread -Zbuild-std` whenever this path
   changes.

## haskell FFI — GHC 9.4.8 int-conversion + runtime dylib path (WEB18)

**Date:** 2026-06-14

The Haskell Conduit port (WEB18, the first Haskell package using GHC C FFI) was
green locally on GHC 9.14.1 but red on CI, which pins GHC 9.4.8. Two distinct,
environment-specific failures that a newer local GHC masked:

1. **macOS compile error — `-Wint-conversion`.** GHC < 9.6 generates libffi
   adjustor C stubs for every `foreign import ccall "wrapper"` that assign a
   `void*` (`HsPtr`) to an `ffi_arg` (`unsigned long`): `*(ffi_arg*)resp = cret;`.
   Clang 15+ (macOS CI) promotes `-Wint-conversion` from a warning to a hard
   error, so `gcc' failed in phase C Compiler`. Fix: add
   `-optc-Wno-error=int-conversion` to the **library** `ghc-options` (the stub is
   emitted when compiling the module that declares the `"wrapper"` imports). GHC
   9.6+ fixed the codegen, which is why a modern local GHC never sees it.

2. **Linux runtime error — `libconduit_capi.so: cannot open shared object file`.**
   `cabal test --extra-lib-dirs=DIR` only affects **link** time. At **run** time
   the test binary loads the cdylib through the OS dynamic loader, which does not
   consult `--extra-lib-dirs`. Fix: `export LD_LIBRARY_PATH` (Linux) and
   `DYLD_LIBRARY_PATH` (macOS) to the release dir (and `…/deps`) in run-tests.sh
   before invoking `cabal test`. Locally this was masked because I had been
   exporting `DYLD_LIBRARY_PATH` by hand in every manual run.

Durable lessons:
- **Match the CI toolchain version locally before declaring green.** CI's
  `haskell-actions/setup` pins `ghc-version: 9.4`; a newer local GHC hid a
  codegen bug. For any FFI-heavy package, test on the CI-pinned compiler.
- **Link path ≠ run path for cdylibs.** Any port that dynamically links
  conduit-capi (vs. static) must set the loader path at run time, not just the
  linker path at build time. Sibling dynamic-load ports (C#) sidestep this with a
  custom `CONDUIT_CAPI_PATH` resolver; GHC FFI has no such hook, so the OS loader
  env vars are mandatory.

## LANG-FULL N6 (Nib u8 wrap) — the exit-code matrix can't prove a u8 wrap

**Date:** 2026-06-15

Attempted N6 (make `200u8 + 100u8 = 44` run cross-backend). Three findings that
block a HONEST proof — recorded so the next attempt doesn't ship a false positive:

1. **`Expect::Exit` is `& 0xFF` — it CANNOT distinguish a u8 wrap from no wrap.**
   `lang_matrix.rs` reads the process exit code, which the OS/C-runtime truncates
   to 8 bits (documented at the top of the file: "process exit code (`& 0xFF`)").
   `200 + 100 = 300`; `300 & 0xFF = 44` **whether or not the backend masks**. So a
   Nib `fn main() -> u8 { return 200 + 100; }` "passes" `Expect::Exit(44)` even when
   the arithmetic is plain i64 with no E2 mask. For u8 the exit-code truncation IS
   the u8 mask, so the test proves nothing. u16/u32 are worse — their masked values
   (4464, …) exceed 255 and can't ride an exit code at all. **A real N6 proof needs
   the value at full width**: either a stdout print (`Expect::Stdout`, but Nib has
   no `print`/`out` — Oct does, via O-OUT) or a return-value-at-width harness like
   `iir-to-wasm/tests/width_wrap.rs` (which calls `load_and_run` and reads the raw
   i64 result, NOT an exit code). Decide the proof mechanism BEFORE writing N6.

2. **The Nib `type_hint` lookup was a silent no-op.** `arith_result_hint` /
   `lookup_node_type` consult `types: HashMap<usize, NibType>` keyed by AST-node
   pointer address. Tagging the `add` op with the inferred `u8` produced `i64`
   anyway (unit-tested: the emitted `add` carried `"i64"`, not `"u8"`). Either the
   checker doesn't type the `add_expr` node, or the pointer keys don't match the
   nodes the compiler walks (the checker types one AST, `compile_typed` may walk a
   moved/rebuilt one). Verify the lookup actually returns `Some(U8)` (a direct unit
   test on the emitted hint) BEFORE relying on it — the i64 collapse hid this for
   every prior Nib item because everything was i64 regardless.

3. **JVM + CLR E2 mask legs were only structurally tested and don't fire for the
   real shape.** Their executed proof was explicitly deferred to "the integration
   PR." With the universal shape (i64 slots + narrow hint on the op — see
   `iir-to-llvm`'s `e2_*` tests), a u8 add over `long` operands returns the unmasked
   value on JVM (`iir_type_to_jvm("u8") = Int` → `IADD`, but operands ride `long`
   slots; the `iand` mask path doesn't match). Native/LLVM/WASM/VM/JIT mask
   correctly; JVM/CLR need a fix for the i64-operand case. These are two real
   remaining E2 backend legs, not done.

Net: N6 is a multi-part item (proof-mechanism design + Nib type-lookup fix + JVM/CLR
backend fixes), NOT a one-line frontend wiring. The native-AOT E2 leg (PR #5887,
merged) IS real — its `aarch64-backend` proof installs and *calls* the generated
code and reads the raw u64 (44), so it is not exit-code-confounded.

## Identity must be a SUBSET of fields when a rich struct is a map/graph key (spreadsheet-core fill PR)

`CellAddress` carries `{row, col, absolute_row, absolute_col}` and derives
`Eq`/`Hash` over **all four** fields. But a cell's *identity* is its position
(`row, col`) only — the `$` markers just steer copy/fill shifting. The engine
stored cells (and dependency-graph nodes) keyed by the bare `CellAddress`, yet
the evaluator looked them up with the address taken **straight from the formula's
`Ref`**, flags and all. So `=$A$1` built a lookup key `{1,1,true,true}` that never
matched the relatively-stored `{1,1,false,false}` cell → it read as empty (**0**),
and editing `A1` never recomputed a dependent that referenced it absolutely.

Nobody had ever written a test that *evaluated* an absolute reference, so the bug
sat latent until the fill feature (whose whole point is "`$A$1` stays pinned")
surfaced it. Fix: a `without_absolute()` normaliser applied at the two key
boundaries (the evaluator's `lookup` closure and `collect_refs`).

Lesson: when a struct with "decorative" fields is used as a `HashMap` key or graph
node, either (a) don't derive `Hash`/`Eq` over the decorative fields, or (b)
normalise to the identity subset at **every** key boundary — and add a test that
exercises the decorated form through the real lookup path, not just the AST. A
derive that silently widens identity is a landmine.

## stream-reactor `defer_read` REPLAYS the chunk — it is not "pause output" (WEB01b-1a, PR #6047)

`MailboxHttpServer` (the new mailbox/deferred-response HTTP server) framed a
request, submitted it to the worker pool, and returned
`TcpHandlerResult::defer_read()` intending "pause this connection's reads until
the response is written." The test passed locally on macOS but failed on the
Linux CI runner: the client received a spurious `400 Bad Request` written
*before* the real `200 OK` on the same connection.

Root cause: in `code/packages/rust/stream-reactor`, `defer_read` does **not**
mean "pause output." It means **"I did NOT consume these bytes — buffer this
chunk and *replay* it (re-invoke the handler with the same bytes) when reads
resume."** The mailbox response-router calls `resume_all_reads()` for *every*
connection's response, so a deferred chunk gets replayed (`progress_reads_with_state`
→ `apply_read_chunk`, stream-reactor/src/lib.rs:651-668,720-724). Because the
handler had already *consumed* the bytes (drained its buffer + submitted the
job), the replay re-fed the chunk. On macOS the whole request arrived in one TCP
segment, so the replay merely double-submitted (the leading `200` still satisfied
the assertion). On Linux under load the request was TCP-segmented, so the
replayed **trailing fragment** (`ection: close\r\n\r\n`) parsed as a malformed
head → `pop_request` returned `Err` → a `400` was queued before the real `200`.

Fixes:
1. After a successful consume+submit, return `TcpHandlerResult::default()`
   (keep reading) — **never** `defer_read()`. Only return `defer_read` when you
   genuinely did NOT consume the bytes and want them replayed (e.g. the
   QueueFull backpressure case in embeddable-tcp-server). There is currently no
   "pause-without-replay" primitive; a per-connection in-flight gate needs a
   reorder buffer (deferred to WEB01b-1b).
2. Drain **every** complete request a read delivered by looping `pop_request`
   until `Ok(None)` — one TCP read can carry multiple coalesced/pipelined
   requests; popping once strands the extras in the buffer and hangs a client
   that sent them together and then waited. (Caught by the security review.)

Lesson: a handler-result flag named for an *intent* ("defer") may be implemented
as a *mechanism* ("replay") — read the reactor's drain path before reusing it,
and never trust a same-host test to expose a TCP-segmentation-dependent bug
(loopback coalescing hides it; the Linux runner under parallel load splits the
read and surfaces it).

## `mv file.bak file` restores OLD mtime → cargo skips the rebuild (false "race")

While building WEB01b-1b I did a "does this test actually prove anything" check:
`sed -i.bak 's/ordered_responses: true/false/' lib.rs` (build+run → correctly
FAILED), then `mv lib.rs.bak lib.rs` to restore. The restored file had
`ordered_responses: true` again — but every subsequent `cargo test` kept FAILING
as if it were still `false`. Adding ANY `eprintln!` "fixed" it, which screamed
"timing race." It was not a race.

`mv` preserves the SOURCE file's mtime. The `.bak` was created at the moment of
the `sed` (before the false-build), so restoring it stamped `lib.rs` with an
mtime OLDER than the compiled artifact from the false build. Cargo's
mtime-based staleness check then judged `lib.rs` "older than the build" and
skipped recompiling — so the tests ran against the stale `ordered_responses:
false` binary. Adding an `eprintln` edited the file (fresh mtime) and forced a
real rebuild, which is why it "passed."

Lessons:
- To revert a quick experiment, restore from git (`git checkout -- file`) or
  `touch file` after a `cp`/`mv` — never trust `mv file.bak file` to trigger a
  rebuild; it can move the mtime backwards.
- A Heisenbug that disappears the instant you add a print, where the print is
  AFTER the observed effect, is almost never a real race — suspect a stale build
  artifact (or caching) first. Confirm by `touch`-ing the source and re-running
  clean BEFORE hunting for a concurrency bug.

## clang `@llvm.floor.f64`/math intrinsics need `-lm` on Linux (E8 PR-2, #6584)

A RUN-verified iir-to-llvm test that compiled a program using `@llvm.floor.f64`
passed locally on macOS but failed `build (ubuntu-latest)` with `clang: error:
linker command failed`. Cause: at `-O0` (the default for `clang -x ir`) the
`@llvm.floor.f64`/`@llvm.trunc.f64` intrinsics lower to libm `floor`/`trunc`
*calls*, which on Linux must be linked with `-lm`. macOS libm lives in libSystem
(linked by default), so the gap is invisible locally. Fix: add `.arg("-lm")` to
the clang invocation. LESSON: any LLVM program using a floating-point math
intrinsic (floor/trunc/sin/sqrt/…) must link `-lm`; test it on Linux CI, and the
real entier matrix pipeline (E8 PR-7) must link `-lm` too.
## Autonomous loop must self-heal: a single ScheduleWakeup is not durable

The SIR-completion loop used `ScheduleWakeup(180s)` to babysit each PR. When the
session went idle/closed, the wakeup chain stopped and PR #6561 (M3) sat
unbabysat for ~3 days (the user had to nudge it back). LESSON: a session-scoped
ScheduleWakeup loop does NOT survive a closed session — for multi-day autonomy,
prefer a durable cloud schedule (the `/schedule` skill / CronCreate), and on
EVERY wake re-derive state from `git fetch` + `gh pr list` rather than assuming
the previous turn's plan still holds. Also: a merged PR with no open follow-up
means "pick up the next item," not "idle" — check `mergedAt`, mark the task
done, and immediately start the next backlog item in the same turn.

## Lazy synthetic-function emission breaks exact module-function-count assertions

BA2 (BASIC multi-item PRINT) appends two synthetic helper functions
(`__basic_print_uint`/`__basic_print_int`) to the module whenever a `PRINT`
renders a value. A pre-existing test (`compiles_def_fn_into_sibling_function`)
compiled a program that PRINTs and asserted `m.functions.len() == 2` (main +
the DEF FN sibling) — which silently became 4. It passed on a stale local test
binary but failed `build (ubuntu-latest)` on fresh CI (cf. the closurec
stale-binary lesson). LESSON: when a frontend change adds module-level functions
conditionally, grep the whole crate for `functions.len()` / `functions.iter().count()`
assertions and any downstream consumer (e.g. `basic-dap`) before pushing; assert
the *named* functions you care about (`functions[0].name == "main"`,
`.iter().find(|f| f.name == "FNS")`) rather than a brittle total count. Always
re-run the FULL crate test suite (not `--lib` alone, and not a possibly-cached
binary) after changing module assembly.

## A tail expression holding a MutexGuard temporary compiles locally but fails CI (E0597)

`String::from_utf8(buf.lock().unwrap().clone()).expect(...)` as the **final
expression** of a function holds the `MutexGuard` temporary until the end of the
block — i.e. it is dropped *after* the `Arc` it borrows goes out of scope. A
newer local rustc accepted this; the CI toolchain rejected it as E0597
("`buf` does not live long enough ... dropped here while still borrowed"). BA2's
JIT-capture helpers hit this and turned a green local run into a red CI build.
LESSON: the local rustc can be MORE lenient than CI's — a clean local build is
not proof CI compiles. Bind the cloned value to a `let` first
(`let bytes = buf.lock().unwrap().clone();`) so the guard drops at that
statement. When a CI build error can't be reproduced locally, suspect a
toolchain-version difference (temporary-lifetime/edition rules, lint levels)
rather than assuming the local result holds.

## NullBackend stubs FullyTyped functions to no-ops — breaks programs that call them

`jit_smoke.rs` used `jit_core::NullBackend`, whose `compile()` returns
`Some(sentinel)` and `run()` returns `Value::Null` — i.e. it "compiles" every
function to a **no-op binary**. Before BA2 this was harmless: BASIC's `main` was
the only function and `execute_with_jit` Phase-2 interprets `main` directly. BA2
made `PRINT` lower to a `call` of FullyTyped helper functions
(`__basic_print_int`/`__basic_print_uint`), which Phase-1 **eagerly compiles** —
with NullBackend they became no-op binaries, so `PRINT 42` emitted only the
trailing newline (`"\n"`, no digits). It passed locally but failed on CI
(environment-dependent whether the cached no-op was consulted). The sibling test
`jit_real_backend.rs` (BasicCirJit, whose `compile` returns `None` for the
helpers' `call`/`putchar` → interpreter runs them) PASSED on CI — proving the
interpreter executes the recursive helpers correctly and that `compile → None`
is the working pattern. LESSON: `NullBackend` is only safe for programs whose
output doesn't depend on a *called* function's side effects; once a frontend
emits cross-function `call`s, use a backend whose `compile` returns `None`
(defer-to-interpreter) so nothing is stubbed. When a JIT test passes locally but
fails on CI with missing output, suspect eager-compilation of FullyTyped
functions to no-op binaries.

## Gradle / JVM

### Gradle optimizer packages need `includeBuild` for the build-tool's dep graph

When a Gradle package (Java or Kotlin) depends on a sibling package via a file-based JAR
(`implementation(files("../sql-planner/gradle-build/libs/..."))`), the build tool's
`parseGradleDeps` function reads `includeBuild(...)` lines from `settings.gradle.kts` to
construct the dependency graph. Without this, the validator raises:

  `<lang>/sql-optimizer (BUILD): undeclared local package refs: <lang>/sql-planner`

Fix: Add `includeBuild("../sql-planner")` to the optimizer's `settings.gradle.kts` BEFORE
the `rootProject.name` line. This applies to Java and Kotlin optimizer packages (and
any future Gradle packages that depend on siblings via file deps).

Discovered: 2026-06-30 during Java sql-optimizer CI (PR #7073 fix commit d31296a48).

## Adding a match arm to a deeply-recursive fn can overflow the stack (macOS CI only)

Symptom: `build (macos-latest)` failed while `windows`/`ubuntu` passed —
`logic-engine` test `deeply_nested_expression_is_a_clean_error_not_a_stack_overflow`
aborted with "has overflowed its stack, fatal runtime error: stack overflow"
(PR #7299, adding `ComputeOp::Abs`).

Cause: `logic-engine/src/compute.rs::eval` recurses up to `MAX_EVAL_DEPTH` (256)
levels. In **debug builds** (which `cargo test` / CI use) the compiler reserves
stack for ALL match arms' locals in the function's single frame — it does not
scope stack slots per-arm. Adding a new `ComputeExpr::Unary` arm with several
locals (`operand, dim, exact, result, …`) enlarged every one of the up-to-256
recursive `eval` frames. 256 × a fatter frame overflowed the macOS test thread's
~2 MB stack BEFORE the depth guard could return a clean `TooDeep`. Windows/ubuntu
have more headroom so they passed, and the local `cargo test` on macOS passed too
(margin is razor-thin and runner-dependent) — so it only surfaced on CI macOS.

Fix: move the new arm's body into a separate `#[inline(never)]` helper (e.g.
`eval_unary`) so its locals live in their own frame instead of bloating every
recursive `eval` frame — restoring `eval` to ~its pre-change size. Behavior is
identical; the deep-nesting test's guarantee ("clean `TooDeep`, never a stack
overflow") is preserved.

Rule: when adding a match arm to a function that RECURSES up to a large fixed
depth (`eval`, tree walkers, parsers with a depth cap), keep the arm's body in an
`#[inline(never)]` helper rather than inline — otherwise its locals multiply
across every recursive frame and can overflow a small (macOS 2 MB) test-thread
stack in debug builds. `cargo test -p <crate>` locally is NOT sufficient to catch
it (margin is runner-dependent); the guard is the deep-nesting stack test on CI
macOS.

Recurrence (PR #7343, adding binary `Min2`/`Max2`): the same overflow re-appeared,
and the FIRST two fixes made it WORSE — a cautionary tale about the mechanism:
  1. Adding a separate `if op == Min2 || Max2 { let result_dim…; let (x,y)…;
     let result…; let exact…; }` block BEFORE the general path DUPLICATED those
     locals (the general path has the identical set), so the frame grew by a full
     extra copy → still overflowed.
  2. Extracting the whole `Bin` arm into `#[inline(never)] fn eval_binary`
     (mirroring `eval_unary`) made it WORSE, not better: the `deeply_nested` test
     nests `Bin(Add, …)` 306 deep, so the recursion is `eval → eval_binary → eval
     → eval_binary → …` — **two** stack frames per nesting level instead of one.
     Even though each `eval` frame shrank, the total (`eval`+`eval_binary`) × 256
     exceeded the single inline frame × 256. Extraction only helps when the
     extracted helper is NOT on the deep-recursion path (e.g. `eval_unary` is fine
     because the test doesn't nest unary ops 256-deep).
The fix that WORKED: keep the `Bin` arm inline and FOLD min/max into the EXISTING
`result`/`exact` `match op { … }` arms (and map them through `dim_op` like
addition), adding ZERO new persistent locals — match arms share the frame's slots,
they don't each get their own. Frame(after) ≈ frame(main), so CI behaviour matches
the passing baseline.
Corrected rule: to add an op to a depth-capped recursive `eval`, FOLD it into the
existing match arms (reuse the shared locals); do NOT add a parallel `if`-block
(duplicates locals) and do NOT extract the recursive arm into a helper that then
sits ON the recursion path (doubles frame COUNT). `cargo test` locally cannot
verify the margin — local debug frames are so fat that even `main` overflows at
`RUST_MIN_STACK=5MB`, yet passes on CI at 2MB; the only reliable check is the
delta-vs-main (no new locals) plus the CI macOS run itself.

Discovered: 2026-07-02 during logic-engine abs-value CI (PR #7299 fix commit adf710c3).

---

### Java mini-sqlite Level 1 graduation — plan tree normalization required

`SqlPlanner.planSelect()` produces `Project(Sort(Limit(Distinct(core))))` — Project is the
outermost (last) node.  `SqlCodegen.compilePlan()` expects Sort/Limit/Distinct to be
OUTERMOST so it can peel them in its while-loop and then call `compileCore(Project(core))`.
If Project is outermost and Sort is inside it, `compileScanBody(Sort(...))` throws
"Unsupported plan node in scan body: Sort".

**Fix**: normalize the plan before calling `SqlCodegen.compile()`:

1. Peel Sort/Limit/Distinct from under Project (in the order they appear in the plan,
   i.e., outermost-first via `addLast`).
2. Rebuild: apply wrappers LAST-to-FIRST so Sort is outermost:
   `Project(Limit(Sort(core)))` → `Sort(Limit(Project(core)))`.
3. When Sort key columns are NOT in the SELECT list (e.g. `SELECT label FROM t ORDER BY rank`),
   inject them as extra hidden OutputColumn.Expr entries into the Project so SortResult can
   find them by name; strip those extra columns from the final QueryResult.

Additional mini-sqlite Level 1 lessons:

- **SqlPlanner passes `OutputColumn.Star` through unchanged.** `SqlCodegen.emitProjectColumns`
  silently skips Star columns ("the planner should have resolved them"), producing an empty
  result schema. Fix: expand `*` to explicit column references before planning by querying
  `backend.columns(table)` for each table in the FROM/JOIN list.
- **`INSERT INTO table VALUES (...)` with no column list** sends an empty columns list to
  `InsertRow`, causing the instruction to pop 0 values and store an empty row.  Fix: expand
  the column list from `backend.columns(table)` before planning.
- **`SqlCodegen.compileCore` only handles `Project(Aggregate)` directly.** If `Having` wraps
  the Aggregate (`Project(Having(Aggregate(...)))`), it falls through to
  `compileScanBody(Having(Aggregate))` which strips Having then throws "Unsupported: Aggregate".
  Fix: strip Having from between Project and Aggregate during normalization, save the Having
  predicate, and post-filter result rows using a simple expression evaluator after execution.
- **`NullOrder` default in SqlTextParser must be NULLS_FIRST for ASC** to match SqlVm's
  sort semantics (null rank = 0 = lowest, which makes NULLs sort first in ASC).  Using
  NULLS_LAST for both ASC and DESC (the initial incorrect default) puts NULLs last in ASC,
  violating the VM's sort-null-first-by-default contract.
- **Jacoco `excludes` on violationRules** does NOT exclude classes from measurement — it only
  applies to class-level rules.  To exclude old/unrelated classes from the COVEREDRATIO
  check, use `classDirectories.setFrom(files(...).map { fileTree(it) { exclude(...) }})` on
  the `jacocoTestCoverageVerification` task.

Discovered: 2026-07-01 during Java mini-sqlite Level 1 graduation (PR #7153).

## Mosaic .msl part names must match the layout/package part names exactly (silent style drop)

A stylesheet (`.msl`) styles named `part`s; the emitter matches each `part X {...}`
block to a part of the same name in the layout (`.mll`) / resolved package composition.
If a part name doesn't match, the emitter **silently drops** those styles — the element
renders with NO styling, no error.

`Grid.light.msl` was authored against the legacy monolithic-Grid naming
(`sheet/cell`, `sheet/header-cell`, …) removed in U29-X1, while the shipped
`mosaic-pkg-grid` composition (and `Grid.dark.msl`) use flat names (`cell`,
`header-cell`, `header-row`, `data-row`). Result: the light-theme grid cells had
no borders/background → **no gridlines**. It stayed hidden because the light theme
was **dead code** (rendered by no demo) until the web dark/light switcher (PR #7338)
finally exercised it.

Lessons:
- When you light up a previously-dead variant/theme, VERIFY IT ACTUALLY RENDERS
  (drive it), don't assume parity with the working variant — the parallel file may
  have drifted.
- Keep sibling stylesheets' `part` names identical to each other and to the
  layout/package parts. A quick guard: `diff <(grep '^\s*part ' X.dark.msl) <(grep '^\s*part ' X.light.msl)`.
- Silent-drop-on-mismatch is a sharp edge in the emitter; a warning for unmatched
  `part` blocks would have caught this at build time (possible follow-up).

Discovered: 2026-07-02, light-theme grid had no gridlines after PR #7338 shipped the switcher.

## A frontend-emitted SIR builtin must exist in EVERY backend runtime, or `case` panics at runtime

Driving a real Ruby `case`/`when` program through the SIR pipeline to all
backends revealed that `case`/`when` (and `case`/`in`) **panicked at runtime on
Go, Rust, AND JavaScript** with `unknown builtin: case_eq` — it only worked on
Python/TypeScript. Root cause: the Ruby→SIR frontend lowers every `when` arm to
`BuiltinCall("case_eq", [pattern, scrutinee])` (Ruby's `===`), but three of the
five backend runtimes never implemented that builtin. Because an arbitrary
`BuiltinCall` name is *open-ended* (backends fall through to a
`call_builtin_by_name` dispatcher whose floor is `panic!("unknown builtin")`),
there is **no compile-time gate** — the feature manifest can't reject it, so the
gap only surfaces when the emitted program actually runs.

Why it went unnoticed: the per-backend `compile_and_run_*` tests **hand-build
SIR modules** exercising one feature each; none had ever hand-built a `case_eq`
call, and no single test drove real Ruby *source* → Go/Rust/JS end-to-end. The
Python/TS backends implement `case_eq` via their runtime *packages*, so those
arms passed and masked the gap on the inline-runtime backends.

**Lessons:**
- When the frontend starts emitting a new builtin name, grep EVERY backend
  runtime (`semantic-ir-to-{go,rust,javascript}` inline runtimes + the
  `sir-runtime-*` packages) for it before assuming cross-backend support.
- A per-backend exec test that hand-builds SIR proves only the feature it builds;
  it cannot catch a builtin the frontend emits but the test never constructs.
  A Ruby-source → all-5-backends golden suite is the real guard (still a TODO).
- `case_eq` = Ruby `===`, keyed to the *pattern* type (Range→membership,
  Regexp→match, else `==`); `when SomeClass` is lowered to `.is_a?` at the
  frontend, so class patterns never reach `case_eq`. Backends with no
  Range/Regexp value type implement it as plain structural equality.

## The same open-ended-dispatch trap applies to native RUNTIME SYMBOLS, not just SIR builtins

Same shape as the `case_eq` lesson above, one layer down. When the precise-GC
track taught `x86_64-backend` to emit `call __twig_gc_write_barrier` after every
`field_store` / `array_set`, `x86-simulator`'s `Simulator::host_call` dispatch
table was not extended to match — its floor is
`Trap::UnresolvedExternal(...)`, so **every** matrix program doing a heap store
trapped. Three LANG-FULL cells sat red on main until an unrelated near-full-repo
rebuild surfaced them (PR #12164).

**Why it stayed hidden for so long**: `x86-simulator`'s BUILD only re-runs when a
diff touches its dependency cone, and the backend crates that emit the symbol
(`x86_64-backend`, `aarch64-backend`) have **no BUILD file at all** — so neither
the emitter's own relocation-assertion tests nor the consumer's execution tests
ran when the emitter changed. A green CI on the PR that added the barrier proved
nothing about either side.

**Lessons:**
- When a backend starts emitting a NEW external runtime symbol, grep every
  *consumer* of that symbol — simulators, runtime shims, linker stubs — before
  assuming it resolves. The emitter-side test (assert the relocation exists) and
  the consumer-side test (assert the symbol resolves) are different tests, and
  passing the first tells you nothing about the second.
- `x86-simulator` is currently the repo's only host-runtime dispatch table (the
  sole `fn host_call` / `Trap::UnresolvedExternal` site) — so it is the one place
  to update, but also the one place with no sibling to cross-check against.
- A **no-op** is sometimes the correct implementation of a runtime hook, not a
  stub: the simulator's bump allocator never collects, so a generational write
  barrier has no remembered set to notify. Say *why* in a comment, or the next
  reader will "finish" it.
- Barrier/GC regression tests are notoriously **vacuous** (see the aarch64 and
  `iir-to-llvm` entries elsewhere in this file — both passed with the barrier
  deleted). Always prove a new guard is load-bearing by temporarily reverting the
  fix and confirming the test goes red.

Discovered: 2026-07-02, while adding trailing-`case` implicit return; a `case`
program printed correctly on Python but panicked on Go/Rust/JS.

## The conformance harness immediately found three latent cross-backend gaps

Expanding the `sir-conformance` corpus (6 → 11 Ruby programs) surfaced three
real bugs the moment broader features were exercised — exactly what the harness
is for. Each is a `case_eq`-style "works on some backends" gap, caught only
because the harness compares each backend to the **reference**, not to
backend-consensus (so backends *agreeing on a wrong answer* still fails).

1. **Frontend: array/hash index (`a[i]`, `h[k]`) is under-baked.**
   - `puts a[1]` (paren-less) **mis-parses** as `(puts a)[1]` — all four backends
     faithfully print the whole array then index, agreeing on the wrong answer.
   - `puts(a[1])` **fails to parse** outright ("Expected … got `[`").
   - `x = a[1]` parses but **fails SIR validation**: `var-ref scope=local
     references unknown name 'a'` — the index-base variable is mis-scoped during
     lowering. `x = a[1] + 0` (index inside arithmetic) *does* lower.
   These are Ruby-frontend parser/lowering bugs, not backend gaps.

2. **JavaScript backend: Ruby string method names aren't all renamed.** The JS
   backend translates Ruby method names to native JS **at emit time**
   (`upcase` → `toUpperCase`), unlike Python/Go/Rust which dispatch Ruby names in
   a runtime catalog. The rename table is missing the case pair, so `"x".upcase`
   raises `NoMethodError` on JS while the other three succeed. `.length` (spelled
   identically) works everywhere.

3. **JavaScript (and possibly Go/Rust) runtime: no `or`/`and` builtin.** A
   multi-value `when 1, 2, 3` lowers to `case_eq(...) or case_eq(...)` as a
   `BuiltinCall("or", …)`; the JS runtime's builtin table has no `or`, so it
   throws `unknown builtin: or` — the same shape as the `case_eq` gap. (Go/Rust
   were not reached before the JS failure, so they may share it.)

**Lesson:** each of these passed every per-backend unit test and only a
Ruby-source→all-backends run against a reference caught them. Keep growing the
corpus; every added feature is a chance to surface the next one. Programs that
hit an *unfixed* gap are kept OUT of the corpus (with a comment pointing here)
until the gap is fixed, so the suite stays green and the gaps stay visible.

Discovered: 2026-07-02, expanding the conformance corpus (PR after the harness landed).

## Ruby assignments are sequential (`let*`), not parallel (`let`) — `x = a` was broken

Investigating the conformance "array index" gap (`x = a[1]` fails SIR
validation) revealed the real bug is NOT about indexing: **any first-sighting
`newvar = <expr that reads an earlier local>` failed to compile on every
backend.** `a = 5\nb = a + 1` was rejected with `var-ref scope=local references
unknown name 'a'`.

Root cause: the SIR distinguishes `LetBinding` (parallel `let` — all RHSs
evaluate BEFORE any name binds) from `LetStarBinding` (sequential `let*` — each
RHS sees prior bindings). The validator (`validator::check_stmt_seq`) correctly
treats a *run of consecutive `LetBinding`s* as one parallel-`let` group. But the
Ruby frontend lowered EVERY first-sighting assignment to a parallel `LetBinding`,
even though Ruby assignments are sequential. So `[LetBinding(a); LetBinding(b =
a+1)]` put `b`'s RHS in a scope that didn't yet include `a`.

Fix (frontend, `sequentialize_let_bindings` post-pass): after a block's
statements are lowered, rewrite a `LetBinding` to a `LetStarBinding` (identical
fields) exactly when its value reads a name bound by an EARLIER statement in the
block. `let*` binds immediately and breaks the parallel run, so the reference
resolves. Independent bindings stay `LetBinding` (minimal churn). Both variants
lower to the same sequential declaration on every backend — behaviour-preserving.

**Lessons:**
- Shape-only frontend tests (asserting `Stmt::LetBinding {…}`) do NOT run the
  SIR validator, so they happily locked in a shape the validator would reject.
  ~16 such tests asserted the buggy parallel-`let` form; they were updated to
  accept either variant. When adding a frontend feature, run a program through
  `semantic_ir::validate` (or the conformance harness), not just a shape assert.
- Don't "fix" this in the validator by making `LetBinding` sequential — that
  would break genuine parallel-`let` frontends (Lisp `let`). The frontend that
  emits the wrong binding kind is what's wrong.
- The array/hash INDEX-READ parser bug (`a[1]` → `a` + `[1]`) is separate and
  still open (grammar precedence); only the assignment-scoping half is fixed.

Discovered: 2026-07-03, expanding the conformance corpus toward array indexing.

---

## Reimplementing a third-party numeric crate: dump the oracle, never estimate; disambiguate the `-p` spec

Context: Phase B of the Engram zero-dep program replaced the third-party `fsrs`
crate (FSRS-6 spaced-repetition scheduler; pulls in the `burn` tensor framework)
with a from-scratch zero-dep `code/packages/rust/fsrs`.

**Lessons:**
- **Never hand-estimate frozen numeric snapshots.** The scheduler math is exact
  scalar `f32`, but my mental estimates for the expected `next_states` /
  `memory_state_from_sm2` outputs were off by large margins (e.g. guessed
  difficulty 5.68, real 6.91; guessed stability 18.06, real 15.45). The reliable
  path: run the REAL crate (engram-core already depended on it) via a throwaway
  `#[test] … println!` to dump ground-truth for the exact snapshot inputs, then
  paste those values. Estimating would have shipped a wrong "oracle".
- **Cross-verify against the LIVE crate before deleting it.** Add the upstream
  crate as an aliased `[dev-dependencies]` (`upstream_fsrs = { package = "fsrs",
  version = "…" }`), assert your impl matches across a randomized grid (5,900+
  comparisons here), THEN remove the throwaway test + dev-dep. Same interop-gate
  discipline as the protobuf-vs-prost byte check.
- **Naming a repo crate the same as a crates.io crate makes `cargo -p` ambiguous
  while both are in the graph** (during the dev-dep cross-check). Error: "multiple
  `fsrs` packages … specification `fsrs` is ambiguous." Disambiguate with the
  version: `cargo test -p fsrs@0.1.0 …`. After the upstream dep is dropped the
  bare `-p fsrs` is unambiguous again. Naming the repo crate identically is still
  worth it: the consumer swap becomes a one-line path change with zero `use`
  edits.
- **Transcribe the exact operation order**, not just the formulas — same order of
  `+`/`*`/`.exp()`/`.powf()` gives bit-for-bit `f32` agreement. Clippy will flag
  snapshot literals with `excessive_precision`; `cargo clippy --fix` trims them
  to the f32-canonical form (still within any sane test tolerance).
- Upstream `fsrs` needs `burn` only for TRAINING; the scheduling/inference path
  we consume is pure scalar arithmetic. Reimplementing only the forward half
  dropped the entire `burn` subtree. Check whether a heavy dep's weight is in a
  code path you actually use before assuming it's unavoidable.

Discovered: 2026-07-03, Engram zero-dep Phase B (drop `fsrs`).

## The conformance oracle can expose a *deliberate* semantics conflict, not just a bug

Building the SIR21 integer/division reference oracle (`sir-conformance::oracle`)
and probing division across backends surfaced that Ruby's integer `/` (which
*floors*: `-7 / 2 == -4`) is implemented **four different wrong ways**: Python's
runtime `div` truncates (`int(a/b)` → `-3`) and is *deliberately* documented +
unit-tested that way ("to match SIR semantics"); JavaScript true-divides (`7 / 2`
→ `3.5`); Go/Rust crash on the negative path. So the oracle didn't just find
bugs — it found a **genuine design conflict** between a pre-existing, tested,
documented runtime decision (truncate) and the newer reference authority (Ruby
floor).

Lesson: when the reference model disagrees with a deliberately-chosen, tested,
documented behaviour in shared/published code, do **not** unilaterally flip it in
an autonomous loop — that overrides someone's decision and risks breaking the
tests that pin it. Capture the divergence as an `#[ignore]`d, oracle-judged
frontier test (so it's tracked and flips green when resolved), record the
conflict, and surface the *decision* (here: div_floor/div_trunc split vs. flip,
plus the `Integer#/` floors / `Float#/` true-divides polymorphism) to the human.
The split is exactly what SIR21 §E3 prescribes precisely to avoid one overloaded
`/` that different sources read differently.

Discovered: 2026-07-04, SIR21 division frontier (sir-conformance 0.10.0).

---

## Generating Unicode/data tables for a zero-dep crate; reading a branch diff when main has advanced

Context: Phase C1 of the Engram zero-dep program replaced `unicode-normalization`
with a from-scratch zero-dep `code/packages/rust/unicode-normalize` (NFD/NFC +
`is_combining_mark`, Unicode 17.0.0).

**Lessons:**
- **Generate large data tables from the real crate, don't transcribe by hand.**
  A throwaway `#[ignore]` test with the upstream crate as a `[dev-dependencies]`
  enumerated every scalar value (`0..=0x10FFFF`) and emitted a compact
  `src/tables.rs` (CCC / recursive decomposition / composition / Mark ranges).
  Then a second throwaway test cross-checked this crate vs the live upstream one
  across ALL code points + 200k random strings (zero mismatches) before deleting
  the generator, the cross-check, and the dev-dep. Same interop-gate discipline
  as protobuf/fsrs. Composition pairs were captured by probing upstream
  `compose(a,b)` over candidate (starter, combiner) sets drawn from the decomp
  table — no need to parse UCD files or hit the network.
- **Two traits with the same method name (`nfd`/`nfc`) on the same receiver make
  calls ambiguous** in the cross-check (both `unicode_normalize` and
  `unicode_normalization` define them). Call via fully-qualified syntax
  `Trait::method(x)` in the gate test.
- **A blanket `impl<I: Iterator<Item=char>> Trait for I` collides with
  `impl Trait for &str`** (coherence can't prove `&str: !Iterator`). Mirror the
  upstream crate: impl for the concrete receivers actually used (`&str` and
  `str::Chars`), not a blanket.
- **`git diff --cached origin/main` on a feature branch shows the OTHER merged
  PRs as spurious "deletions" once origin/main has advanced past your branch
  point** — it compares your index to the newer main, so their newer additions
  read as removed-on-your-side. This looked alarmingly like the stale-worktree
  revert bug. Distinguish with `git status --short` and `git diff --name-only HEAD`
  (working tree vs HEAD) — if those show ONLY your files, you're clean. Fix the
  scary diff by `git rebase origin/main`; afterwards `git diff --name-only
  origin/main` shows exactly your files. Always rebase before pushing so the PR
  diff is only yours.
- **Hangul is algorithmic** (UAX #15 §16) — decompose/compose by arithmetic on
  code points, saving ~11,000 table entries. Only the 11172 syllables; the jamo
  compose via the same arithmetic.

Discovered: 2026-07-03, Engram zero-dep Phase C1 (drop `unicode-normalization`).

---

## Don't hand-emulate a specific regex's backtracking — reach for the engine

Context: Phase C2 of the Engram zero-dep program aimed to hand-write scanners
replacing the two HTML-tag regexes in `engram-core` search-text rendering.

**Lessons:**
- **A hand scanner can replace a *simple* regex, but not a backtracking one.**
  The tag-strip `(?is)<[^>]+>` has no alternation/backreference/overlap, so a
  two-pointer scanner reproduces it byte-for-byte (verified vs live `regex` on
  300k random strings). But the media pattern
  `<(?:img|…)[^>]*(?:src|data)\s*=\s*(?:"([^"]+)"|'([^']+)'|([^ >]+))[^>]*>`
  hides layered backtracking that a fuzz cross-check exposed one corner at a
  time: (1) a **quoted value `[^"]+` crosses `>`**, so the tag end isn't the
  first `>`; (2) when the quoted alt yields no trailing `>`, the regex
  **backtracks through the alternation** to the bare `[^ >]+`; (3) `\s*` and the
  value class `[^ >]` **overlap** (tab ∈ both), so greedy `\s*` gives a tab back
  to the value. Each fix surfaced the next corner — a sign you're reimplementing
  a regex engine badly.
- **When you catch yourself hand-emulating quantifier/alternation backtracking,
  stop and build/adopt the general engine.** The right move was to descope: ship
  only the trivially-exact tag-strip scanner now, and move the media pattern (plus
  glob/whole-word/`re:`) to the planned zero-dep regex *engine*, where correct
  semantics give byte-exactness for free instead of per-pattern emulation.
- **The randomized cross-check earned its keep**: every divergence was a
  malformed-HTML input a human would never think to unit-test. Fuzz vs the real
  library before trusting a reimplementation — and read the *first* divergence
  carefully; it usually reveals a structural misunderstanding, not an off-by-one.

Discovered: 2026-07-03, Engram zero-dep Phase C2 (HTML tag scanners).

---

## Building a regex engine: Pike VM for DoS-safety; is_match is far easier than match-extent

Context: Phase D of the Engram zero-dep program replaces the third-party `regex`
crate with a from-scratch engine. D0 shipped the `is_match` core.

**Lessons:**
- **Use a Pike VM (Thompson NFA), not backtracking.** Engram matches
  *user-supplied* `re:` patterns; a backtracker is exponential on inputs like
  `(a*)*b` — a DoS. The Pike VM advances all threads in lockstep: O(pattern ×
  input), immune. It also matches the `regex` crate's leftmost-first + greedy
  semantics if you order Split's targets by priority (greedy tries "take it"
  first).
- **Make the epsilon-closure iterative, not recursive.** A long chain of
  epsilon transitions (`a?a?a?…`) recursed would overflow the stack — another
  DoS. An explicit stack that pushes Split's second target before its first
  preserves DFS priority order without recursion.
- **Cap the compiled program size.** `{0,10000000}` expands to millions of
  instructions; reject over a cap (as `regex` does) to bound memory + match time.
- **`is_match` is dramatically easier than `find`/`captures`/`replace_all`.**
  Boolean existence doesn't care about *which* match or its extent, so the basic
  Pike VM nails it (cross-verified vs `regex` `(?-u)` on 100k+ random pairs, zero
  divergences). But exact match **extent** requires solving the **nullable-loop**
  priority problem: `(b??)+` on `"b"` — `regex` returns the empty match `(0,0)`
  (the greedy outer loop must not take a zero-progress iteration), but a naive
  Pike VM consumes the `b` → `(0,1)`. This is a known-hard sub-problem (RE2
  handles it with empty-width tracking). So: **ship `is_match` first**, and give
  match-extents their own PR. Most of engram's regex use is boolean anyway.
- **Force the reference into ASCII mode for a fair cross-check.** The `regex`
  crate's `\w\d\s\b` are Unicode by default; prefix the reference pattern with
  `(?-u)` so the corpus compares against ASCII-class behaviour while the engine's
  Unicode classes are still a follow-up.

Discovered: 2026-07-03, Engram zero-dep Phase D0 (regex engine `is_match` core).
## A stale-branch merge can silently REVERT already-merged work → red main from two green PRs

**Symptom:** `main` stopped compiling. `spreadsheet-core-wasm` 0.15 (SSIO PR4
#7701) called `spreadsheet_io::load_json`, but `spreadsheet-io` on `main` was
back at 0.3.0 with the whole JSON codec gone. `cargo build -p spreadsheet-wasm
--target wasm32-unknown-unknown --release` failed with `cannot find function
load_json in crate spreadsheet_io`.

**Root cause:** PR #7682 ("feat(sir): sir-hashmethods-go … **rebased onto
main**") was rebased on a **stale, pre-#7693 main**. Its merge silently reverted
the ENTIRE #7693 changeset (13 files across spreadsheet-io + json-parser +
json-lexer) as collateral — #7682 was the only post-#7693 commit touching any of
them, and it dragged them back to their old state. #7682's CI was green (its
snapshot compiled); #7701's CI was green (its base still had `load_json`).
Neither CI saw the other, so **two individually-green PRs merged into a red
main**. This also re-opened a CRITICAL DoS (the json-parser recursion depth cap
went with the revert).

**How to detect fast:** when a symbol "vanishes" that you know was merged,
`git log --oneline <good-merge>..origin/main -- <file>` shows the culprit, and
`git show origin/main:<file> | grep <symbol>` confirms it's gone. A reverted
crate's **version number rolls backward** (0.4.0 → 0.3.0) — a dead giveaway.

**Fix:** restore only the affected files to their last-good commit —
`git checkout <good-sha> -- <the exact file list>` — never a broad revert (that
would clobber the stale PR's *legitimate* work). Verify the previously-failing
build command actually passes now. The restore is byte-identical to
already-reviewed code, so no fresh security review is needed.

**Prevention / takeaways:**
- **"Rebased onto main" in a PR title is a smell** — verify its file list is only
  what it claims. A SIR/Go PR touching `spreadsheet-io`/`json-*` is a red flag.
- Green CI on *both* PRs does NOT mean their *combination* is green. A shared
  crate that one PR removes and another PR starts calling is invisible to both
  pre-merge checks. (See also: "Diff branch full file list vs origin/main before
  push; stale worktrees revert files.")
- When you inherit a broken main mid-loop, **fixing main is the priority** — a
  focused hotfix PR unblocks the whole repo, not just your feature.

Discovered: 2026-07-06, SSIO PR5 setup (found `main` non-compiling; hotfix #7709).

## Adding match arms to a hot recursive fn can overflow a calibrated deep-recursion guard (CLOC12.171)
When a new AST `Expression` variant forces new arms into a *recursive* dispatch fn
(`constant-fold`'s `fold_expression`), building the new node structs INLINE in the
match enlarges that fn's per-level stack frame in debug builds (each arm gets its own
slots). `constant-fold` folds a 20 000-deep binary chain on a fixed 64 MiB worker
(~3.2 KiB/frame budget); the enlarged frame overflowed it — a test green on origin/main
went red on the branch. FIX: delegate new arms to `#[inline(never)]` helpers (mirroring
the existing `fold_member`/`fold_call` delegation) so their locals live in the helper
frame, entered only when that node is actually hit — never on the hot binary path.
LESSON: after adding arms to a recursive dispatch fn, run the deep-nesting DoS-guard
tests; prefer delegating heavy arms out-of-line rather than inlining struct construction.

## `grep -c "test result: FAILED"` is 0 for a crate that FAILED TO COMPILE (CLOC12.173 PR1)
When verifying "do all these crates pass?", a **compile error emits NO `test result:` line at all** —
so `cargo test -p X 2>&1 | grep -c "test result: FAILED"` returns `0` for a crate that never
even built. I read that `0` as "green" and concluded only 2 of ~10 pass crates needed a new
`ClassExpression` match arm; CI's per-crate build-tool then failed 6+ passes with `E0004:
non-exhaustive patterns`. A second flawed check — `cargo build -p X 2>&1 | grep -B1 "not covered"`
piped into a regex that didn't match rustc's `-->` location format — returned empty and I again
read empty as "no errors". LESSON: never infer compile success from the ABSENCE of a grepped
error string. Check the command's EXIT CODE (`if cargo build -p X >/dev/null 2>&1; then OK else
FAIL`), or grep for the POSITIVE signal (`Finished`/`test result: ok`) and require it. A new
`Expression`/exhaustive-enum variant breaks EVERY crate that matches it without a catch-all —
enumerate them by exit code, not by a fragile error-string grep.

## Enum-variant additions: grep the WHOLE repo for matchers, not just the crates you plan to touch (CLOC12.175)

Adding `ClassMember::Field` to javascript-ast broke `javascript-parser`'s **test**
build (`let ClassMember::Method(m) = &c.body[0];` → refutable) — a crate I never
built locally because I only tested the 9 pass crates + emitter I edited. CI's
affected-package graph builds ALL transitive consumers of the changed shared crate,
so it caught it on macos ("Build and test affected packages"). Fix: before pushing
a shared-enum change, `grep -rl --include="*.rs" "EnumName::" code/` to find EVERY
consumer (production AND test code — refutable-let sites live in tests too), and
build each. A per-crate build of only the crates you edited is NOT the affected set.
Cross-refs feedback_run_downstream_consumer_tests, feedback_affected_package_latent_bugs.

## Lesson: `const T[N][M]` array-of-array params fail on real gcc but not Apple clang

**Context:** The c/des and c/aes ports built clean locally (macOS `gcc` = Apple
clang) but FAILED on ubuntu CI's real gcc with:
`error: invalid use of pointers to arrays with different qualifiers in ISO C
before C2X [-Wpedantic]`.

**Cause:** In ISO C before C23, a `T (*)[N]` does NOT implicitly convert to
`const T (*)[N]` — `const`-ness does not compose through the array-of-array
pointer. So passing a non-const `uint8_t subkeys[16][6]` / `uint8_t state[4][4]`
local as an argument to a parameter declared `const uint8_t [16][6]` /
`const uint8_t [4][4]` is a constraint violation. (This is unlike 1-D
pointer-to-scalar, where `int* -> const int*` is fine.) Apple clang accepts it;
real gcc with `-pedantic-errors` rejects it.

**Fix:** Drop `const` on 2-D (and higher) array parameters of internal helpers
that receive a non-const array — leave the read-only intent to a comment. Grep
before pushing: `grep -rnE "const [a-z0-9_]+ [a-z_]+\[[0-9]*\]\[" src/`.

**Blind spot:** Apple clang (the macOS `gcc`) is more permissive than ubuntu
gcc on several pedantic ISO-C rules. "Verified under gcc/clang locally" does not
cover the ubuntu gcc arm — the pure-ISO guarantee is only firm once CI's gcc has
run. Watch babysit CI to green, don't assume from a local build.

## Lesson: new grammar frontends must opt into the parser's recursion depth cap

**Context:** Adding `IF` to the COBOL runtime introduced the first *nestable*
construct. A security review found that deeply-nested `IF … IF … IF …` (one per
80-column card, so a statement flows across cards and the nest is real)
overflowed the native stack — an uncatchable `SIGSEGV`/abort that a
`Result`-returning entry point cannot report.

**Cause:** `parser::GrammarParser::new(...)` defaults `max_depth` to
`usize::MAX` — the recursion-depth guard is **opt-in**. `cobol-parser` built the
parser with `GrammarParser::new(...)` and never chained `.with_max_depth(...)`,
so every deep nest recursed once per level through `parse_rule` with no ceiling.
The shared parser already HAS the fix (`DEFAULT_MAX_RULE_DEPTH = 128`, below the
~192–224 overflow point on a 2 MB thread); the frontend just wasn't using it.

**Fix:** Every frontend that constructs a `GrammarParser` must chain
`.with_max_depth(DEFAULT_MAX_RULE_DEPTH)` on *all* construction paths (both the
panicking and the `try_` entry points). Deep nesting then returns a clean
"input nests deeper than the supported limit" parse error. This transitively
bounds any *runtime* tree-walk (e.g. a recursive `exec_stmt`) too, since the CST
can no longer be built deeper than the cap.

**Blind spot / grep before pushing a new frontend:**
`grep -rn "GrammarParser::new" code/packages/rust/<lang>-parser/src/` — every hit
that lacks a following `.with_max_depth(` is an unguarded native-stack DoS. This
is the same class as the closurec/json-parser deep-recursion DoS
(project_closurec_deep_recursion_dos): a nestable construct is the trigger, and
the mitigation belongs at the parser layer, not the runtime.

## Lesson: grammar `{ }` repetition width is NOT bounded by the parser depth cap

**Context:** After hardening `cobol-parser` with `.with_max_depth(DEFAULT_MAX_RULE_DEPTH)`
(which stops deeply-*nested* input like `IF … IF …` or `((((…))))` from
overflowing the native stack), the COMPUTE runtime added an expression evaluator.
A security review found a *second*, distinct stack-overflow DoS that the depth
cap does **not** catch: a flat operator chain `COMPUTE R = A + A + A + … + A`
with tens of thousands of terms.

**Cause:** The depth cap counts *rule recursion* (`parse_rule` re-entry). But a
grammar repetition `arith_expr = arith_term { ("+"|"-") arith_term }` is a **flat
loop** in the parser — it appends children without recursing, so N terms produce
ONE `arith_expr` CST node with 2N−1 children at *bounded* parse depth, for
arbitrarily large N (there is no token/source-size cap). The consumer then folded
those children into an N-deep left-nested `Expr` tree, and both the tree-walking
`eval_expr` and the **recursive `Drop` of `Box<Expr>`** recursed N frames deep →
uncatchable `SIGSEGV` on a few hundred KB of source.

**Fix:** Any reader that folds a grammar *repetition* into a *recursive* tree
(binary-operator chains, list-of-list structures, etc.) must bound the operand
count itself — the parser's depth cap won't. Here: thread a single
`MAX_EXPR_OPERANDS` (1024) budget through every expression reader, charged once
per primary (across parenthesised levels too, so nested parens can't multiply it),
returning a clean `RuntimeError` when exhausted. That bounds the folded tree
height, hence both eval and Drop recursion.

**Blind spot / how to catch it:** when reviewing a new tree-walking evaluator,
ask two *separate* questions — (1) is nesting depth bounded? (parser cap) and
(2) is repetition *width* bounded? (needs its own budget). They are different
axes; the depth cap only answers the first. Cross-refs the depth-cap lesson above
and [[project_closurec_deep_recursion_dos]].

## ISO C (pre-C23) forbids implicit `T[N][M]` → `const T[N][M]`; clang allows it, gcc `-pedantic-errors` rejects it

**Symptom:** A pure-ISO C package built clean on macOS (Apple clang) but failed
on ubuntu CI (real gcc) with `-pedantic-errors`:
`invalid use of pointers to arrays with different qualifiers in ISO C before C2X
[-Wpedantic]` at every call site passing a **non-const** 2-D array to a function
whose parameter is `const double m[3][3]`.

**Cause:** For a plain pointer, `T*` → `const T*` is a permitted implicit
conversion. For a pointer to an *array*, `T(*)[N]` → `const T(*)[N]` is **not**
permitted in ISO C before C23 (C11/C17 6.3.2.3 only qualifies the immediately
pointed-to type, and a 2-D array parameter decays to `double(*)[3]`, so the const
lands one level too deep). C23/C2X finally allows it. **Clang does not diagnose
this even under `-pedantic-errors`; real gcc does.** So a Mac-only local build
(where `gcc` is an Apple-clang shim) cannot catch it — only ubuntu CI will.

**Fix (caller side, minimal):** make every *input-only* array `const` at its
definition so the call passes `const → const` (identical qualifiers, always
clean); for a genuinely non-const array that is also read as input (e.g. an
out-param from an earlier call reused as input), add an explicit
`(const double (*)[3])` cast at the call. Do **not** drop the `const` from the
library parameter — that just flips the error onto the const fixtures (`ID`,
`SWAP`) instead.

**Blind spot / how to catch it:** any C API that takes a multi-dimensional array
by `const` parameter is a portability trap you cannot verify on a clang-only
machine. When writing such tests, declare read-only matrix fixtures `const` from
the start. Treat ubuntu-gcc CI as the source of truth for pure-ISO C conformance,
not the local build. Cross-ref [[feedback_no_third_party_ffi]] and the C/C++
lane's strict-ISO intent.

## gcc `-Werror=format-truncation`: `snprintf("...%s", buf)` where `buf` is a known-size array can overflow the destination

**Symptom:** A pure-ISO C package built clean on macOS (Apple clang) but failed
on ubuntu CI (real gcc) with:
`error: '%s' directive output may be truncated writing up to 127 bytes into a
region of size 101 [-Werror=format-truncation=]`
for `snprintf(err->message, sizeof err->message, "CompileError: Parse error: %s", perr.message);`
where both `err->message` and `perr.message` are `char[128]`.

**Why:** When the `%s` argument is a *fixed-size array* (e.g. `char[128]`), gcc
knows its maximum length (127) and can prove that `prefix + 127` may exceed the
destination buffer, so `-Werror=format-truncation` fires. It does NOT fire when
the argument is a bare `const char *` of unknown length (gcc can't bound it) —
which is why sibling `snprintf(... "%s", value)` calls with `char *` args were
not flagged. Apple clang doesn't implement this warning, so it only surfaces on
ubuntu CI.

**Fix:** Bound the embedded string with a precision so the total provably fits:
`"CompileError: Parse error: %.100s"` (128 − 27-char prefix − NUL = 100 usable).
Truncating an over-long nested message inside a fixed error buffer is correct
behaviour anyway. Do NOT silence the warning; cap the field.

## An agent isolated in a `.claude/worktrees/<id>` worktree must NEVER `cd` to the repo's top-level absolute path — it silently lands in a DIFFERENT sibling checkout

**Context:** Started a task already inside a dedicated worktree at
`/Users/.../coding-adventures/.claude/worktrees/agent-<id>` (Bash's default cwd,
confirmed by an early `pwd`). Ran a later command as
`cd /Users/.../coding-adventures && git checkout -b <branch>` out of habit —
`/Users/.../coding-adventures` LOOKS like "the repo root" and IS a valid git
checkout, but it is the **main/shared checkout**, a sibling of the worktree, not
an ancestor or the same directory. The `git checkout -b` succeeded there with no
error (it's a completely valid repo), silently switching the SHARED checkout's
active branch and leaving the actual worktree untouched and still on its
original branch.

**How it was caught:** The Write tool refused a subsequent absolute-path write
under `/Users/.../coding-adventures/code/...` with: "This agent is isolated in
the worktree .../.claude/worktrees/agent-<id>. Edit the worktree copy of this
file instead of the shared-checkout path." — i.e. the write guard, not the git
command, is what flags the mistake. `git`/`find`/`grep`/`Read` all succeed
silently against the wrong checkout because it's a real, valid repo with
(usually) identical file content at the same commit — there is no error to
notice until a Write/Edit call is rejected, or until `git status`/`git log` is
inspected side-by-side in both directories.

**Fix:** In an isolated-worktree session, never `cd` to (or hardcode absolute
paths rooted at) the plain top-level clone path — always use the cwd Bash
already defaults to (the actual worktree path), or explicitly prefix every
absolute path with the worktree's own directory. If a stray `cd` to the wrong
checkout already happened: check `git branch --show-current` and `git status
--short` in BOTH directories, restore the shared checkout's original branch
(`git checkout main` or whatever it was), and delete any stray branch created
there (safe with `git branch -d` if it has no unique commits — verify with `git
log <stray> --oneline` first). Do all actual work (branch, commits, file
writes) only inside the real worktree path. Cross-refs the existing
`git worktree add inherits HEAD` and "default to a fresh worktree" lessons
above — this is the write-side counterpart: even with a correctly-created
worktree, a single absolute-path `cd` habit can still misdirect an entire
session.

## Flat repetition chains fold into a deep tree → stack-overflow DoS (parser depth cap does NOT catch them)

**Context:** COBOL compound conditions (`A AND B AND C AND …`). The parser's
`with_max_depth` rule-depth cap bounds *rule-reference nesting* (e.g. parenthesised
`((((…))))`), so I assumed any crafted-deep condition was already refused before
the AST was built. It is not.

**Mistake:** A grammar `{ op tail }` repetition (a flat `AND`/`OR` chain) parses as
N flat *sibling* children at CONSTANT rule-depth — the depth cap never fires; it's
bounded only by source length. I then folded those N siblings into a left-leaning
binary tree (`And(Box, And(Box, …))`) and evaluated it with recursion
(`eval_cond(l) && eval_cond(r)`). A crafted `IF A=1 AND A=1 AND … (thousands)`
recurses N frames deep → uncatchable stack-overflow `abort` (and the recursive
`Drop` of the deep `Box` tree overflows too). The security review caught it; my
"parser caps depth" reasoning was the bug.

**Fix / rule:** Represent repetition-folded operators as a **flat n-ary list**
(`And(Vec<Cond>)` / `Or(Vec<Cond>)`), collected iteratively and evaluated with a
loop (`for part in parts { … }`, short-circuiting). Then recursion depth = only the
genuinely-nested (parenthesised) structure the parser *does* cap; a flat chain is
O(1) stack. Dropping a flat `Vec` is iterative too. Cross-ref
`feedback_depth_guards_dont_compose` and `feedback_verify_dos_guards_adversarially`:
when a construct can repeat unboundedly, add an ADVERSARIAL test (here a 5000-term
chain) that would overflow the naive version, and confirm it evaluates by
iteration. The compiler was already safe because it *emitted* the fold iteratively
(a flat loop over children) rather than building a tree — mirror that shape in the
interpreter.

## A rank-0 SIR22 `NDArray`'s shape doesn't identify which frontend produced it — gate display conventions on `source_language`, not value shape

**Context:** Fixing `semantic-ir-to-javascript`'s three APL monadic-scalar-atom
bugs (`neg` printing ASCII `-5` instead of APL's high-minus `¯5`; `neg` on a
rank ≥ 1 array giving `NaN`; `sign`/`recip`/`ceil`/`floor` crashing outright) —
bugs found by `apl-to-semantic-ir/tests/oracle.rs`. The task's own suggested
fix (verified before implementing, per its own explicit instruction) was: "make
`neg` check whether its operand IS a genuine NDArray, and if so, preserve the
box (any rank including 0) so `formatSeen` routes it through the high-minus
`ArrayRt.display` path; only fall back to the old behavior for a non-NDArray
operand."

**Mistake almost made:** That fix looks locally correct and is exactly what a
"generalize from the bug report" pass would produce — but a rank-0 `{shape: [],
data}` NDArray is genuinely NOT unique to APL. `matlab-to-semantic-ir`'s `^`/
`.^` unconditionally lower to `ElementwiseOp::Pow` even for two literals (no
scalar fast path for power), so a plain MATLAB `2 ^ 2` reaches the byte-for-byte
identical rank-0 representation an APL scalar does by the time it reaches
`neg`. `matlab-to-semantic-ir/tests/oracle.rs`'s own `unary_minus_on_power` case
(`-2 ^ 2` must print ASCII `-4`) already exercises exactly this shape. Applying
the suggested fix (box-preserve ANY rank, including 0) would have flipped that
MATLAB case to high-minus `¯4`, silently breaking an existing, currently-green
downstream oracle test — a real regression that a purely value-shape-driven
fix cannot avoid, because the two languages' scalars are representationally
indistinguishable at that point.

**How it was caught:** Traced the actual generated JS for `-2 ^ 2` (a scratch
`cargo test`-based dump of the real compiled output, not just reading source)
*before* writing the fix, and separately traced the actual generated JS for a
bare APL `-5` (also via a scratch dump) — which revealed the operand there is
a BARE `IntLit`-shaped number, never an NDArray at all, meaning the "box
preservation" idea wouldn't even fix the literal bug-report example. Both
traces disproved a plausible-sounding, not-yet-tested design before it was
implemented.

**Fix / rule:** Only the SOURCE LANGUAGE (`module.metadata.source_language`),
never the runtime value's own shape, can decide a *display convention* when
two unrelated frontends can legally produce the identical runtime
representation. The existing precedent for this is `SIR_DISPLAY_RUBY` (a
per-module boolean baked into the emitted JS via a `__SIR_DISPLAY_RUBY__`
placeholder substitution, gating `true`/`false` vs `#t`/`#f`) — the fix here
added a second, analogous `SIR_DISPLAY_APL_HIGH_MINUS` flag rather than
inventing a new mechanism. Concretely: keep VALUE-computing functions
(`neg`/`sign`/`recip`/`ceil`/`floor`) unaware of source language entirely — a
rank-0 operand always unwraps to a bare number exactly as before, for every
language — and make ONLY the actual glyph decision, inside `formatSeen`
(the shared `print`/`puts`/`format` display path), consult the new flag. This
cleanly separates "is the VALUE correct" (source-language-independent, safe to
fix unconditionally — e.g. the rank ≥ 1 NDArray branch, which no frontend
besides APL ever prints raw anyway) from "is the DISPLAY SPELLING correct"
(source-language-dependent, needs the flag). Before trusting a "box-preserve
the value to fix the display" pattern for a value that flows through a SHARED
backend, check whether every consumer that can construct that same runtime
shape agrees on how it should be displayed — grep for other frontends
lowering to the same builtin/node and read their own oracle/e2e test
expectations, don't assume the bug report's one example generalizes.

## Inserting a function above another orphans its doc comment (clippy `-D warnings` failure)

Bit me twice in one session, in two different crates. When you insert a new
function immediately *before* an existing one, you land between that function
and its `///` doc comment:

```rust
/// Shared helper: build the `style="..."` attribute …   <- now documents nothing
                                                          <- blank line
/// UI35 — lower a `HostDraggable` …                      <- your new fn
fn emit_host_draggable(…) { … }

fn build_style_attr(…) { … }                              <- lost its docs
```

`cargo test` passes. `cargo clippy -- -D warnings` fails with `empty line after
doc comment`, so it only shows up in CI unless you lint locally — which is
exactly why the repo rule is to run clippy in BOTH feature configs before
pushing. Insert *after* the preceding function's body instead, anchoring on its
closing brace rather than on the next function's signature; or if you do anchor
on a signature, move the doc block down with the insertion. Also worth knowing:
the clippy message names the function it thinks you meant to document, which is
your *new* function, not the one that actually lost its docs — read the line
number, not the name.

## Rewriting a CRLF file with Python can break a compiler — and `grep -c $'\r'` lies about it

Editing `TaskApp.light.msl` (CRLF in the working tree, `eol=lf` in `.gitattributes`)
by reading + rewriting it in Python produced a file the mosstyle lexer rejected:

```
mosstyle tokenization failed: LexerError at 8:1: Unexpected sequence '\r'
```

Two traps, one after the other:

1. **Mixed endings.** Python's text mode translates on read *and* write, so a
   partial rewrite can leave the original CRLF lines alongside freshly written LF
   ones. Some hand-written lexers (mosstyle's included) accept a consistent file
   but choke on the mix.
2. **The obvious check gives a false negative.** `grep -c $'\r' file` reported
   **0** under Git Bash while `od -c file | grep -c '\r'` reported **238**. I
   nearly concluded the file was clean. Verify with `od -c`, or
   `python -c "print(open('f','rb').read().count(b'\r'))"` — not `grep`.

Fix: normalize to LF on disk, which is what git stores and what CI sees anyway
(`git check-attr text eol -- <file>` to confirm). When scripting an edit to a
tracked file, read and write **binary** (`open(p,'rb')` / `'wb'`) so you never
silently re-encode line endings you weren't asked to touch.

## Two OOP backend bugs the per-backend tests missed because the tests sidestepped the exercised path

The `sir-conformance` matrix (real Ruby source → every backend) caught two latent
backend bugs that every per-backend unit test passed over — both because the unit
test happened to avoid the exact construct that breaks:

1. **Ruby backend: `Foo.new` never ran `initialize`.** A `def initialize` is
   registered like every method under the reserved `sir_um_` prefix as
   `sir_um_initialize` — a name Ruby's own `Class#new` never calls. The `__new__`
   emitter emitted a native `Foo.new`, so the constructor body (its `@ivar`
   initialisers) never ran; `@n` stayed nil and `@n + 1` raised. The two existing
   ivar e2e tests used an explicit `start`/`set` method (`c = Counter.new; c.start;
   c.inc`), calling the initialiser BY HAND — so they never exercised construction-
   time init. Fix: `__new__` → a `sir_new` runtime helper that `allocate`s then
   invokes `sir_um_initialize` (mirroring Go/C/Rust). semantic-ir-to-ruby 0.18.1.

2. **C backend: `raise ArgumentError, "boom"` looked up a nonexistent constant.**
   The frontend lowers it to `raise(VarRef(Const "ArgumentError"), "boom")`; the C
   `raise` emitter used only arg 0 and let the `Const` fall through to
   `_sir_const_get("ArgumentError")` — but the C runtime registers no builtin
   exception-class CONSTANTS, so it raised `NameError: uninitialized constant
   ArgumentError` (and dropped the message). Every existing C exception test raised
   a bare STRING (`raise "boom"` → RuntimeError), never a named class — so the
   class-name path was never emitted. Fix: intercept a `Const` first arg as a class
   name → `_sir_raise(_sir_error("ArgumentError", <msg>))`, on BOTH the simple and
   the compound (non-simple message) emit paths. semantic-ir-to-c 0.17.1.

**The pattern:** a unit test that reaches the same *observable outcome* by a
different *code path* (hand-calling the initialiser; raising a string not a class)
gives false confidence. When a backend has two ways to express a feature, cover the
one the FRONTEND actually emits — a real-source conformance oracle is what surfaces
the gap. (Also: both were pre-existing failures on green main, each in a DIFFERENT
subsystem; when the whole suite is red, don't assume one root cause — bisect each
failing `(program, backend)` cell independently.)

**Third occurrence (moslayout-compiler), with a new symptom.** Inserting a test before
an existing one splits that test's `#[test]` from its `fn` too, giving:

```rust
/// G3-4: Index access …
#[test]                      // <- orphaned from its fn
/// A string literal …
#[test]
fn my_new_test() { … }       // <- now carries TWO #[test] attributes
```

`cargo test` still passes — it just silently runs `my_new_test` **twice** (the count
went 82 → 86 instead of 85, which is the tell). Only `clippy -D warnings` fails, as
`duplicate_macro_attributes`. Same root cause, so the same rule applies and is worth
restating as a hard habit: **anchor an insertion on the previous item's closing brace,
never on the next item's signature** — attributes and doc comments both live above the
signature, and anchoring there always cuts through them.

## A brand-new `concept_tag` fails CI even with perfect content — register it in `concepts/taxonomy.json` first

Adding lessons with a fresh `concept_tag` (e.g. `COURTESY-SORRY`, introducing a new
courtesy concept alongside existing `COURTESY-PLEASE`/`COURTESY-THANKS`) failed both
`human-language-data` integration tests — "every concept id is canonical or
namespaced" and "has zero validation errors" — with `concept_tag 'COURTESY-SORRY' is
neither canonical nor namespaced`. `COURTESY-PLEASE` needed no such step because it
was already a registered concept from an earlier PR; a genuinely new concept has no
such precedent.

`code/learning/human-languages/concepts/taxonomy.json` is the canonical concept
registry (`code/packages/typescript/human-language-data/src/loader.ts` loads it;
`validate.ts` rejects any `concept_tag` that is neither `in taxonomy.concepts` nor
matching the language-local `NAMESPACED_TAG` pattern like `TA-NUMBERS-1-5`). Before
writing the first lesson for a brand-new cross-language concept, add its entry to
`taxonomy.json` (`family`, `gloss`, `core`, optional `retires`/`notes` — copy the
shape of a neighboring entry, e.g. `COURTESY-PLEASE`) and validate it's still parseable
JSON, THEN write the lessons. Re-run both suites
(`code/packages/typescript/human-language-data` + the app suite) after any new
concept_tag lands — a green run before is not evidence the new tag is registered.

## Accumulator-PR pattern: GitHub auto-deletes the head branch on merge — a push right after merge silently recreates an empty-history branch with no PR

Running a "keep adding commits to one PR" loop (per explicit user instruction, see
`feedback_consolidate_pr_when_reviewer_away.md`), a background push to
`content/dravidian-please` succeeded with `git push` printing `[new branch]` instead
of a normal fast-forward update — the tell that something was wrong. The repo has
"automatically delete head branches" enabled, so the moment the user merged PR
#9210, GitHub deleted `content/dravidian-please` on the remote. My next `git push`
to that same branch name didn't fail or warn — it just recreated the ref from
scratch, accepting local history that was still based on the *old* pre-merge
`main`. The commit landed on a branch with no PR, silently orphaned, invisible
until `gh pr list --head <branch>` came up empty.

**The check that catches it:** after ANY push in an accumulator-PR loop, don't
just verify `git rev-parse HEAD == origin/<branch>` (that was true here — the
push "succeeded"). Also check `git push`'s own output for `[new branch]` on a
branch you believe already has history/a PR, and independently confirm the PR
still exists and is still open (`gh pr view <N> --json state,mergedAt`) *before*
building the next slice, not just before pushing it. A merge can land at any
point mid-loop, not only when you happen to check.

**The fix, once caught:** `git fetch origin --quiet`, diff origin/main against
the orphaned branch to see which commits are already merged (`git ls-tree
origin/main -- <path>` per file, or compare `git log --oneline` lines) vs. still
new, then `git switch -c <fresh-name> origin/main` and `git cherry-pick` only the
truly-new commits onto a clean base — don't just re-push the stale branch. Delete
the orphaned remote branch afterward (`git push origin --delete <name>`) so it
doesn't linger with no PR pointing at it.

## A recursion depth cap sized for an 8 MB stack still overflows Windows' 1 MB stack

The C backend guards cyclic-structure display (`_sir_fmt`) and equality
(`_sir_value_eq_d`) with a depth cap — past the cap it prints `[...]` / assumes
equal, so `h[0] = h` terminates instead of recursing forever. The cap was `5000`,
commented "comfortably under the stack limit." It was: on Linux/macOS (8 MB
stack) 5000 frames (~875 KB on the display path) fit fine, so CI and the sibling
cyclic-**sequence** test both passed. But Windows' default stack is **1 MB**, and
5000 frames overran it **before** the cap could trip — so the very guard meant to
prevent the overflow was unreachable there. Symptom: `cyclic_map_does_not_stack_
overflow` failed ONLY on Windows, exit `0xC00000FD` (STACK_OVERFLOW / -1073741571).

Lessons:
1. **A depth/recursion cap must be sized for the SMALLEST stack the artifact can
   run on, not the dev box.** Windows 1 MB is the floor for emitted C/Rust; pick
   caps (here lowered to `500`, ~90 KB) with margin for an *unoptimised* build
   (debug frames are 2–3× larger than the `-O` frames you might estimate from).
2. **"Passes in CI" can mean "CI's stack is bigger," not "correct."** A
   platform-dependent resource limit (stack, fd count, path length, default int
   width) makes a bug invisible on the CI OS. When a guard is about a resource
   limit, test against the tightest limit — or assert the guard *fires* (reaches
   the `[...]` branch), not just that the program happens not to crash.
3. `0xC00000FD` / exit `-1073741571` on Windows == stack overflow; suspect
   unbounded (or insufficiently-bounded) recursion.

## Prefer PowerShell filtering over nested jq quoting in PowerShell commands

A parity-loop pre-push check embedded a jq expression containing spaces and
quoted regular expressions in one PowerShell command. PowerShell split the
expression before `gh` received it, so `gh pr list` rejected part of the jq
program as an unknown argument. The following independent `git push` still ran,
which made the noisy read-only failure easy to miss.

Lessons:

1. On PowerShell, request JSON from `gh`, pipe it through `ConvertFrom-Json`,
   and filter with `Where-Object` when the jq program needs nested quoting.
2. Keep a required precondition check separate from the external write it is
   meant to guard. A failed check must prevent the push rather than merely share
   a command invocation with it.

## Verify pinned-tool assertions against the pinned CLI before publishing CI

The OCaml toolchain workflow pinned opam `2.5.2` but tried to attest its
repository with `opam repository get-url default`. That subcommand does not
exist in opam 2.5, so all three runners finished compiler setup and then failed
the same preflight before dependency installation.

Lessons:

1. Pinning a tool version also pins its command surface. Run every scripted
   assertion against that exact version, rather than relying on a plausible
   subcommand name.
2. Do not assume a materialized `repo/<name>` checkout survives setup. The
   setup action may populate opam's repository cache and then remove unused
   mirrors, as it does on Windows. Query opam's own color-disabled repository
   report, assert the exact configured name set, and require the reviewed
   commit-qualified URL exactly once.
3. A cross-platform failure at the same post-setup step is a shared contract
   defect until logs prove otherwise; wait for the matrix logs and fix the one
   common assertion.
4. Exact machine comparisons must explicitly disable color. A CI action can
   export `CLICOLOR_FORCE=1`, causing a correct version such as `1.9.0` to
   arrive wrapped in ANSI escapes unless the probe passes `--color=never`.

## Authenticate a WebSocket control plane in application state, not in client claims

A portable WebSocket server cannot assume Unix-socket peer credentials, and the
RFC 6455 handshake layer may intentionally expose no HTTP headers to its
application handler. The clean boundary is a connection-local unauthenticated
state whose first successful application request exchanges one bounded opaque
credential through an injected authenticator. Store only the opaque authority
returned by that adapter, drop it with the connection, and ask the adapter again
for every operation; never treat a request field such as `principal` or `role`
as authority.

Two less obvious bounds matter at this seam:

1. A 64 KiB JSON limit applied *after* WebSocket assembly still lets the runtime
   allocate its larger default message limit. Clamp both frame and assembled
   message limits when binding the API, then retain the pre-parse check as
   defense in depth.
2. Recursive duplicate-key rejection implemented by scanning every prior key is
   quadratic. An attacker can spend the whole request budget on object keys, so
   use a per-object `HashSet` and keep the parser recursion-depth capped.

Encode revisions and nanosecond timestamps as decimal strings in JSON. They are
opaque or 64-bit values; passing them through an IEEE-754-backed JSON consumer
can otherwise silently destroy the exact compare-and-swap or health evidence.

The matching client codec belongs beside the server codec, not inside the CLI.
A hand-built CLI request can look correct while omitting response-version checks,
accepting the wrong request ID, or interpreting a malformed error envelope. A
typed sequential client should generate IDs, require an exact response ID, apply
the same size/depth/duplicate-key bounds in the reverse direction, and return
only a validated result or stable remote error. Then the terminal layer owns
only argument parsing, secure credential acquisition, and human rendering.

---

## Lesson 95 — ZStd FHD Content_Checksum_Flag is bit 2, not bit 4 (repo-wide mistake in spec + Go + Rust)

**Date:** 2026-08-03

**What happened:** Rescuing `java/zstd` (CMP07) from a stale branch and running it against the real `zstd` CLI for TC-9 interop testing surfaced that `code/specs/CMP07-zstd.md`, `code/packages/go/zstd/zstd.go`, and `code/packages/rust/zstd/src/lib.rs` all document/parse the Frame Header Descriptor's `Content_Checksum_Flag` at **bit 4**. That is wrong. Verified empirically: `zstd -c file.txt` (checksum on by default) emits FHD byte `0x64`; `zstd -c --no-check file.txt` emits FHD byte `0x60` — the differing bit is **bit 2**, and the checksummed output is exactly 4 bytes longer (the trailing xxHash64). RFC 8878 §3.1.1.1 agrees: bit 4 is `Unused_bit`, bit 2 is `Content_Checksum_Flag`.

**Why it went unnoticed:** Go and Rust's decoders read the (wrong) bit 4, silently discard the value (`_ = (fhd >> 4) & 1` / `let _checksum_flag = ...`), and never actually skip checksum bytes or reject trailing data after the last block — so a real checksummed `.zst` frame still "worked" by accident (the trailing 4 bytes were just ignored, per the anti-pattern in Lesson 94). The bug only becomes fatal once a decoder correctly implements the Lesson-94 "reject trailing bytes after the last block" check without ALSO fixing the checksum-flag bit position — that combination throws a false "trailing data" error on every real-world checksummed frame, breaking TC-9 CLI interop in the CLI→ours direction.

**Rule:** `Content_Checksum_Flag` is FHD bit 2 (`(fhd >> 2) & 1`), not bit 4. When adding/verifying a Lesson-94-style trailing-bytes check to any ZStd port, the checksum-flag bit MUST be fixed first (or simultaneously), and the check must skip 4 bytes when it's set, before comparing final `pos` to `data.length`. Fixed in `java/zstd` (PR landing this lesson); Go and Rust still have the wrong bit documented/parsed as of this writing — low urgency there only because neither enforces the trailing-bytes check yet, but both should be corrected the next time either package is touched, to avoid the same trap resurfacing if the trailing-bytes check is ever added.

---

## Lesson 96 — ZStd sequences-section FSE codec had THREE compounding bugs, none catchable by internal round-trip tests; all found via real `zstd` CLI interop (TC-9)

**Date:** 2026-08-03

**What happened:** After fixing the seq-count byte-order bug (Lesson 250) and the FHD checksum-bit bug (Lesson 95), `java/zstd`'s TC-9 CLI interop test (compress with ours, decompress with real `zstd -d`) STILL failed with `Decoding error (36): Data corruption detected` on any input that produced more than a trivial number of LZ77 matches. All prior tests — including 17 unit tests and an isolated "encode/decode two sequences" FSE codec test — passed, because they only ever round-tripped through our OWN encoder/decoder pair. Bisecting confirmed Raw and RLE blocks interoperated fine; only Compressed (FSE) blocks failed, and the SAME minimal repro (`compress("ababababab"*3)`, one sequence: ll=2, ml=28, offset=2) also failed when compressed with the **Rust reference implementation** — i.e. this was not a Java-specific porting mistake, it was inherited from the shared design both ports followed.

Three independent, compounding bugs were found by comparing against the real RFC 8878 text and the actual zstd C reference source (`ZSTD_decodeSequence`, `FSE_encodeSymbol`, `FSE_initCState2`, `FSE_buildDTable_internal` — fetched directly from `github.com/facebook/zstd`, not recalled from memory, after two independent RFC-text fetches gave answers too imprecise to trust on their own):

1. **FSE table-spread algorithm used a fabricated two-pass split.** `buildDecodeTable`/`buildEncodeTable` spread symbols into table slots using "first all symbols with count>1, then all symbols with count==1" (both in ascending symbol order) — a plausible-looking but entirely invented convention. The real algorithm (`FSE_buildDTable_internal`'s low-probability branch) is a SINGLE pass over symbols 0..maxSymbolValue, placing each symbol's full count immediately when encountered. The two-pass version produces a completely different (but internally self-consistent) table layout.

2. **Per-sequence field order was wrong in two ways.** RFC 8878 §3.1.1.3.2.1.2 (confirmed against `ZSTD_decodeSequence`'s literal C code): a decoder PEEKS all three symbols from current state (free — the state IS the table index, no bits consumed), THEN reads extra bits in order **OF, ML, LL**, THEN (see bug 3) updates states in order **LL, ML, OF**. The pre-fix code combined peek-and-update into one step and got both the extras/updates relative order AND the OF/ML sub-order wrong.

3. **The state-transition "update" is skipped entirely for the last sequence in a block** (`if (!isLastSeq) { update LL; update ML; update OF; }` in the real decoder) — there is no "next" sequence to prepare a state for. The encoder side must mirror this: the first symbol processed in the reverse encode loop (which is the semantically LAST real sequence) cannot get its starting state via a normal bit-flushing transition (there is no corresponding decode-side bit-read to consume it) — it must be computed directly via `FSE_initCState2`'s formula (no bits written at all). The pre-fix encoder always flushed a transition for every sequence uniformly, writing bits a real decoder would never read, shifting the bit-alignment of everything that followed.

**Why it went unnoticed:** All three bugs are self-cancelling as long as encode and decode use the SAME (wrong) convention — every internal round-trip test, including a dedicated low-level "build tables, encode two sequences by hand, decode them, check `(ll,ml,off)` match" unit test, passes regardless of which of the 3 bugs are present, because both sides of the comparison are wrong in the identical way. Round-trip testing against yourself can never catch a systematic, symmetric protocol deviation — only testing against an INDEPENDENT, spec-conformant implementation (the real `zstd` CLI, via TC-9) can.

**Rule:**
- For any binary codec claiming wire-format compatibility with an external spec, a same-language/same-codebase round-trip test is necessary but never sufficient. Budget real interop testing (TC-9-style: compress-here/decompress-there AND the reverse, against the actual reference tool) as early as possible, not as an afterthought — three bugs of this severity survived undetected through this package's entire history because the interop test was simply never written (confirmed: it doesn't exist in Go, Rust, Python, TypeScript, or any other language's `zstd` port in this repo either — grep for `subprocess`/`ProcessBuilder`/`Command::new`/`zstd -d` across all `code/packages/*/zstd` turned up nothing before this PR).
- When re-deriving a "should be self-evident" wire-format detail (bit order, field order, table-construction algorithm) from memory or from a paraphrased RFC fetch, and a test still fails after the fix, DON'T iterate on more memory-based guesses — fetch the actual reference C source (`fse.h`, `fse_decompress.c`, `zstd_decompress_block.c` from `github.com/facebook/zstd`) and quote the literal code. English-prose descriptions of bit-level algorithms (even official RFC text, even fetched twice independently) are lossy in exactly the ways that matter here; C code is not.
- This bug class is NOT Java-specific — reproduced identically against the Rust reference (`code/packages/rust/zstd`) with the same minimal repro. Every other language port's `compress()` output is very likely equally non-conformant to real zstd for any input producing more than a handful of LZ77 sequences. Flagged as a follow-up task rather than fixed in this PR (scope: `java/zstd` only) — see the spawned task for cross-language remediation.

Fixed in `java/zstd`: `code/packages/java/zstd/src/main/java/com/codingadventures/zstd/Zstd.java` (`buildDecodeTable`, `buildEncodeTable`, `encodeSequencesSection`, `decompressBlock`, new `fseInitState` method). Verified against the real `zstd` CLI across an 82-case fuzz corpus (varying periodic patterns, semi-random run-length data, pure random data, and prose at multiple repeat counts) in both directions, plus the two dedicated JUnit interop tests (`tc9CliInterop`, `rtCliInteropHighSequenceCount`).

## Adjudicate oracle differences against the manifest contract

A full Haskell-versus-Go Rust graph comparison initially showed two Haskell-only
edges. Deleting them to match the reference engine would have hidden valid Cargo
dependencies: each entry used a local source alias plus an authoritative
`package = "..."` published-name override. A reference engine is evidence, not
the specification. When parity comparison finds an extra edge, inspect the real
manifest and shared behavior contract before classifying it as false. If the
oracle is incomplete, add a language-neutral fixture and repair every affected
engine in the same dependency-shaped slice.

## Adding shared build-tool fixtures must update the pinned corpus-summary tests

Adding three valid conformance cases changed `validate-corpus` from 38 to 41,
but the fixture-specific validation command still passed because it reports the
new count rather than asserting it. CI later failed two
`test_build_tool_conformance_runner.py` assertions that deliberately pin the
checked-in corpus size. Whenever a shared case is added or removed, update both
the direct `validate_corpus` summary assertion and the CLI machine-readable
summary assertion, then run the full conformance-runner test module—not only
`build_tool_conformance.py validate-corpus`.

## Shared TypeScript config paths are anchored to the declaring config

An extending `tsconfig.json` does not rebase an inherited relative `rootDir` or
`outDir` to the child package. TypeScript resolves those paths from the shared
config that declared them, so `rootDir: "src"` in a lane-level base config can
produce TS6059 for every child package and direct output into the lane-level
tree. For shared compiler configs, use TypeScript 5.5 or newer and declare
package-local paths with `${configDir}/src` and `${configDir}/dist`; then audit
every locked compiler and verify every inheritor with `tsc --showConfig`.

Also remember that a failed `tsc` invocation may still emit JavaScript,
declarations, and maps when `noEmitOnError` is not enabled. After reproducing a
compiler-path failure, inspect the source tree for generated artifacts and
remove only the exact verified outputs before continuing.

## Lesson 101 — `dart/deflate`'s wire format is not RFC 1951; `dart/zip` (and every sibling `zip` port) must not depend on the language's `deflate` package

**Date:** 2026-08-05

**What happened:** The task brief for implementing `dart/zip` (CMP09) assumed `dart/deflate` (CMP05) was ready to reuse directly for the reader's dynamic-Huffman DEFLATE decode — "this is exactly what `dart/deflate` should provide, so USE it rather than reimplementing DEFLATE." Reading `code/packages/dart/deflate/lib/src/deflate.dart` before wiring it in showed this is false: its `compress`/`decompress` pair uses a **private, self-designed wire format** — an explicit header carrying `(originalLength, llEntryCount, distEntryCount)` followed by `(symbol, codeLength)` triples for its own LL/distance tables — not a standard RFC 1951 raw DEFLATE bit-stream (no BFINAL/BTYPE block header, no fixed-Huffman option, no RLE'd code-length transmission per §3.2.7). This directly contradicts the current CMP05 spec (`code/specs/CMP05-deflate.md` §"Wire Format — standard RFC 1951": *"`compress` emits a standard RFC 1951 raw DEFLATE stream ... the exact bytes a ZIP entry or gzip body carries"*) — `dart/deflate` (PR #858, version 0.1.0) predates that spec revision and was never updated to match it.

Checking every other language's existing `zip` package (`code/packages/{python,rust,go,ruby,typescript,elixir,lua,swift,perl}/zip`) confirmed this is not a Dart-specific gap: **all of them** depend only on the sibling `lzss` package (for LZ77 match-finding) and implement RFC 1951 framing — bit I/O, fixed Huffman tables, length/distance tables, `deflateCompress`/`inflate` — directly inside the `zip` package itself. None of them declare a dependency on the language's `deflate` package, despite the CMP09 spec's "Dependencies" section claiming they do. `rust/zip` additionally proves *why* a self-contained reader must decode dynamic Huffman (not just the fixed-Huffman blocks its own writer emits): its test suite carries a real Python-`zipfile`-produced fixture that uses a dynamic-Huffman block, because that's what real-world producers (`zip`(1), Python, Java, Microsoft Office) actually emit.

**Rule:**
- Before wiring a new package to an existing sibling package "because the task brief says to," read the sibling's actual source and compare it against its own spec's stated wire format — a stale/non-conformant dependency is a real, checkable fact, not a matter of trusting the brief. `dart/deflate`'s docstring even says "educational CMP05 wire format," which in hindsight was the tell that it diverged from the "standard RFC 1951" the current spec promises.
- When N-1 existing ports of the same spec entry (here, 9 language `zip` packages) all made the same architectural choice that contradicts what the spec document says, the spec document is the stale one — update it to match the repo's actual, working precedent rather than either blindly following the stale text or silently diverging from precedent with no note. Updated `code/specs/CMP09-zip.md`'s Dependencies paragraph accordingly.
- A DEFLATE (or any RFC-wire-format) package's decoder used inside a container format (ZIP, gzip, PNG) needs full spec-range decode capability, not just enough to read its own encoder's output — e.g. RFC 1951's length symbol 285 (length 258, 0 extra bits) is outside the 257–284 range that a writer capped at 255-byte matches ever produces, but a real-world encoder (`zip`(1) itself, immediately, in the very first CLI-interop test run) emits it freely. A hand-copied length/distance table sized to your own writer's output range will silently index-out-of-bounds the first time it reads someone else's real stream — caught here only because TC-10 shells out to the actual `zip`/`unzip` CLI rather than only round-tripping through the package's own encoder.

Implemented in `code/packages/dart/zip/lib/coding_adventures_zip.dart` (self-contained CRC-32, RFC 1951 bit I/O, canonical Huffman decoder, fixed-Huffman writer, full stored/fixed/dynamic reader, ZipWriter/ZipReader). Verified against a real Python-`zipfile` dynamic-Huffman fixture and against the system `zip`/`unzip` CLI in both directions (TC-10, via `dart:io Process`, skips gracefully when Info-ZIP isn't on `PATH` — same pattern as `dart/zstd`'s TC-9).

---

## `cpp/zstd` (CMP07): std::span isn't C++17, and the Rust reference itself skips a spec-required check

**Date:** 2026-08-05

**What happened:** Implementing the first C++ port of `zstd` (CMP07), two
small but worth-recording gotchas surfaced while translating the corrected
`code/packages/rust/zstd` reference into pure ISO C++17:

1. The task brief's suggested public API signature used
   `std::span<const std::uint8_t>` as the parameter type. `std::span` is a
   **C++20** addition — this repo's `iso-harness` compiles everything with
   `-std=c++17 -pedantic-errors`, so `std::span` doesn't exist under that
   standard and would fail to compile (not just warn). Used
   `const std::vector<std::uint8_t>&` instead, matching `cpp/lzss`'s
   existing convention. When a task description's example code and a repo's
   actual pinned language standard disagree, the pinned standard wins —
   check `ISO_CXXSTD`/the harness's `-std=` flag before assuming a
   "reasonable-sounding" modern-C++ type is actually available.
2. `code/specs/CMP07-zstd.md`'s Security Considerations and `lessons.md`
   Lesson 94 both require a ZStd decoder to reject trailing bytes after the
   frame's end (past an optional content checksum) rather than silently
   ignoring them. The `code/packages/rust/zstd` reference this port was
   translated from — despite being the corrected, post-audit reference for
   the FSE codec bugs (Lessons 95-97) — does **not** actually implement this
   check (its `decompress()` just returns after optionally skipping the
   checksum bytes, with no `pos == data.len()` assertion). The C++ port
   implements the check anyway, since both the spec and Lesson 94
   explicitly require it and it doesn't affect interop with the real `zstd`
   CLI's own output (a single CLI invocation always produces exactly one
   frame with nothing trailing). **Rule:** a reference implementation
   flagged as "already corrected" for one specific bug class is not
   automatically correct against every requirement in the spec/lessons.md —
   cross-check the actual requirements text, not just the reference's
   current behavior, especially for checks that are cheap to add and
   explicitly called out as security-relevant.

**Verification that the FSE bug class itself (Lessons 95-97) was avoided:**
implemented directly against the corrected reference rather than
re-deriving the algorithm from the RFC text or memory, then verified via
real `zstd` CLI interop (TC-9, both directions) plus a high-sequence-count
regression test at the `Number_of_Sequences` 1-byte/2-byte wire-encoding
boundary — 115 checks passed under both g++ and clang++ with no `zstd` CLI
skip (the binary was available in the dev environment), confirming actual
RFC 8878 wire-format conformance rather than just internal self-consistency.

---

## `dart/zstd` (CMP07): missing Repeated-Offset (R1/R2/R3) decode is a repo-wide `zstd`-port gap, not a Dart-specific bug

**Date:** 2026-08-05

**What happened:** While implementing the first `c/zstd` port (PR #9941),
that agent found — independently of the Lesson 95-97 FSE codec bugs — that
`ZSTD_decodeSequence`'s **Repeated-Offset (R1/R2/R3) mechanism** (RFC 8878
§3.1.1.3.2.1.1) was never implemented in this repo's `zstd` decoders. The
sequence Offset_Value on the wire is not always a literal distance: values
1, 2, and 3 are reserved as references into a 3-slot history of
recently-used offsets (R1/R2/R3, defaulting to `{1, 4, 8}` at the start of
a frame, threaded across every Compressed block in the frame — not reset
per block); only Offset_Value >= 4 is `actual_offset + 3`. Real `zstd`
encoders use repeat offsets constantly (one of the format's main entropy
wins), but every port in this repo's own `compress()` always emits an
explicit +3-biased offset and so never *emits* Offset_Value 1/2/3 itself —
meaning no in-process round trip, including every prior CLI-fuzz corpus in
Lessons 96/97 (which only ever fuzzed the ours→`zstd -d` direction), could
ever exercise this decode path. `code/packages/dart/zstd` inherited the
identical gap: `_decompressBlock` computed every offset as flat
`of_raw - 3` and threw `FormatException: decoded offset underflow` the
instant a real `zstd`-CLI-produced frame's sequence had Offset_Value 1, 2,
or 3 — confirmed with a minimal repro (4713 bytes of one repeated byte,
compressed by the real CLI: a single Compressed block whose one sequence
has Offset_Value=1, i.e. "reuse the default R1") and with the CMP07 spec's
own TC-8 fixture (`pattern + (b"X" * 128 + pattern) * 10`), both of which
failed identically before the fix and passed after it.

**Rule:**
- A decoder's job is to understand every valid wire form a *real* encoder
  emits, not just the subset this repo's own (deliberately simplified)
  encoder happens to produce. "Our encoder never emits X" is a valid reason
  to skip implementing X in the encoder; it is never a valid reason to skip
  understanding X in the decoder, if the format's real-world encoders use X
  routinely. Repeat offsets are exactly this shape of gap: cheap for the
  encoder to skip, but the single highest-value feature to get right for
  the decoder, since real periodic/repetitive data (the case `zstd` is
  optimized for) triggers it on nearly every sequence.
- Cross-check the algorithm against more than one source before trusting
  it: the exact selector mapping (which of R1/R2/R3 each of Offset_Value
  1/2/3 selects, the `Literals_Length == 0` special case that shifts the
  mapping by one, and the post-sequence history-rotation shape) is easy to
  get subtly wrong from memory or a single paraphrase. `c/zstd`'s PR #9941
  fix (`decompress_block`'s `_resolveOffset`-equivalent) was cross-checked
  against both the RFC 8878 prose and the reference decoder
  (`ZSTD_decodeSequence` in `facebook/zstd`'s `zstd_decompress_block.c`,
  fetched live) and independently fuzz-tested (1500 trials, ASan/UBSan
  clean) before this Dart fix reused the identical, now-doubly-verified
  formula rather than re-deriving it from scratch a third time.
- This is a **repo-wide** `zstd`-port gap, not a Dart-specific bug — every
  language's `zstd` decoder that computes offset as flat `code - 3` without
  a 3-slot offset-history state machine has the identical hole and will
  fail to decode real-world `.zst` files that use repeated offsets (i.e.
  most of them). Each port should be audited and fixed the same way: add a
  frame-scoped `[R1, R2, R3]` list threaded through every Compressed block,
  resolve Offset_Value 1/2/3 against it per the selector table above, leave
  the encoder unchanged, and add a real-CLI-interop regression test — an
  in-process round trip can never catch this class of bug on its own. See
  Lesson 98 above (`code/packages/c/zstd`'s original fix) for the
  cross-checked selector formula this entry's Dart fix reused verbatim.
## Lesson 105 — `java/zstd`'s decoder never implemented Repeated-Offset (R1/R2/R3) sequence decoding — a decode-only feature gap, not the FSE-codec bug class, invisible to every internal round-trip test

**Date:** 2026-08-06

**What happened:** Implementing the first `c/zstd` port (PR #9941) transcribed its sequences-section FSE codec directly from the already-corrected `code/packages/rust/zstd` reference (Lessons 95-97), avoiding that bug class entirely — but a 200-trial ad hoc fuzz sweep against the real `zstd` CLI still found a failure: 4713 bytes of a single repeated byte, real-`zstd`-compressed, failed to decode. The cause was RFC 8878 §3.1.1.3.2.1.1's Repeated-Offset (R1/R2/R3) mechanism — `Offset_Value <= 3` is a reference into a 3-entry offset history (defaulting to `1/4/8`, threaded frame-scoped across blocks), not a literal `Offset_Value - 3` computation. `c/zstd`'s decoder (like the Rust reference it copied) only implemented the explicit-offset path; for `Offset_Value = 1` that underflows, which its offset-bounds check correctly rejected as malformed — except the frame was valid, using a mechanism the decoder didn't understand. `java/zstd` — the ORIGINAL fix site for Lessons 95-97, already independently audited and CLI-interop-tested for the FSE codec class — turned out to have the identical gap: `decompressBlock` computed `matchOffset = ofRaw - 3` unconditionally for every sequence, with a same-shaped "decoded offset underflow" guard for `ofRaw < 3` that (mis)classified valid repeat-offset frames as malformed.

**Why it went unnoticed even after the FSE-codec audit:** This port's own encoder (`encodeSequencesSection`) never emits an offset code `< 2` — the minimum possible LZ77 match offset is 1, so `raw_off = offset + 3 >= 4` always — an intentional educational simplification, not a bug. So `compress()`/`decompress()` self-round-trip, and every unit test built on top of it (including the TC-9 CLI-interop tests added for Lessons 95-97, which only exercised `compress()`-here/`zstd -d`-there and `zstd -c`-there/`decompress()`-here on ONE fixed prose corpus that never happened to produce a repeat-offset sequence), never touched this code path. This is a fundamentally different failure shape from Lessons 95-97: those were an encode/decode *disagreement* masked by both sides sharing the same wrong convention; this is a decode-only *feature gap* — correct behavior on every input the self-consistent round trip (and the one fixed CLI-interop corpus) could produce, wrong on a class of valid input the encoder simply never generates. "Passes against itself" and "passes against every fixed test corpus" are not the same claim as "implements the full format," and a prior audit that fixed one bug class doesn't imply immunity to an unrelated one.

**Rule:**
- Real `zstd`'s encoder uses repeat offsets constantly — they are one of its principal entropy wins, especially for periodic or highly repetitive data (exactly the shape a compression test suite is likely to include) — so any decoder that only understands explicit offset codes will systematically fail to decode a meaningful fraction of real-world `.zst` files, independent of whether its own encoder is spec-complete.
- Fixed in `code/packages/java/zstd/src/main/java/com/codingadventures/zstd/Zstd.java` (`decompressBlock`, `decompress`): implemented full Repeated_Offset (R1/R2/R3) decode support per RFC 8878 §3.1.1.3.2.1.1, cross-checked against both the RFC prose and the literal reference C source (`ZSTD_decodeSequence` in `zstd_decompress_block.c`, fetched directly rather than recalled from memory) AND PR #9941's independently-verified `c/zstd` fix — not derived from memory alone, per the Lesson-96 playbook of not trusting any single source. Includes the "when `Literals_Length == 0`, the repeat-offset selector shifts by 1" special case. The three registers are FRAME-scoped (default `1/4/8` for the first block, threaded unmodified through Raw/RLE blocks, updated after every Compressed block's sequences — explicit-offset sequences update `Repeated_Offset1` too, not just repeat-offset ones) — NOT block-scoped or reset per Compressed block. The encoder is intentionally left unchanged (still never emits repeat-offset codes; this is a decode-only fix).
- Verified with a new `rtCliInteropRepeatedOffset` test reproducing the exact 4713-byte repro (confirmed failing against the pre-fix decoder with `IOException: decoded offset underflow: of_raw=1`, passing after the fix) plus a broader `rtCliInteropRepeatOffsetFuzz` sweep (5 periodic cycle patterns × 4 sizes) and the full existing 23-test suite (all pass, unaffected — this port's own round trip never touches the new code path), with JaCoCo LINE coverage staying above the package's 80% gate.
- Flagged for every other language's `zstd` port in this repo (Go, Haskell, Perl, C++, C#, Elixir, Kotlin, Lua, etc.) — all share the same encoder-side "no repeat-offset shortcuts" simplification, so all are suspects for this exact decoder gap until individually audited. `rust/zstd` already has the fix (predates this audit); `c/zstd` (PR #9941) and `java/zstd` (this entry) are confirmed fixed; the rest are unverified.

See also `gh pr diff 9941` (the `c/zstd` port that independently found this gap first) and `gh pr diff 9780` (this package's original FSE-codec fix, Lessons 95-97, which this gap survived).

---

## Lesson 102 — `code/packages/perl/zstd` inherited the same Repeated-Offset (R1/R2/R3) decode gap discovered in `c/zstd` (Lesson 98), confirmed via real `zstd` CLI interop

**Date:** 2026-08-05

**What happened:** While implementing the new `c/zstd` port (CMP07, PR
#9941), fuzzing against the real `zstd` CLI found that its decoder never
implemented Repeated-Offset (R1/R2/R3) sequence decoding (RFC 8878
§3.1.1.3.2.1.1) — RFC 8878 reserves sequence `Offset_Value` 1..3 for
"repeat offsets" (a reference into a three-entry offset history, default
`1/4/8`, frame-scoped), not a literal `Offset_Value - 3` computation. That
PR's Lesson 98 flagged this as likely repo-wide, since every port in this
repo shares the same "encoder never emits repeat-offset codes" educational
simplification (`raw_offset = offset + 3` always, so `of_code >= 2`
always), meaning no port's own compress()/decompress() round trip — nor
TC-9's one fixed prose corpus — ever exercises the decoder's repeat-offset
path.

Auditing `code/packages/perl/zstd/lib/CodingAdventures/Zstd.pm` confirmed
the same gap: `_decompress_block` computed `offset = of_raw - 3`
unconditionally and `die`d with `"offset underflow (of_raw=$of_raw)"` for
any `of_raw < 3` — i.e. every repeat-offset sequence. Reproduced with the
exact same minimal repro as the C port (4713 bytes of a single repeated
byte — real `zstd` picks a Compressed block with one sequence,
`Offset_Value=1`, reusing `Repeated_Offset1` which starts at its default
value of 1): `decompress_block: offset underflow (of_raw=1)`.

**Fix:** `_decompress_block` and `decompress` now thread a frame-scoped
`[rep1, rep2, rep3]` offset-history array (default `[1, 4, 8]`) through
every Compressed block in a frame, mutated in place. For offset code `>= 2`
(explicit offset), `offset = of_raw - 3`, then the history rotates in the
new offset (`rep3 <- rep2 <- rep1 <- offset`). For offset code `<= 1`
(`of_raw` in `{1, 2, 3}`, a repeat-offset reference), the actual register
used depends on both `of_raw` and whether `Literals_Length == 0` for this
sequence (`selector = ll_is_zero + of_raw - 1`, RFC 8878's "when
Literals_Length is 0, repeated offsets are shifted by 1" rule):
`selector 0` reuses `rep1` unchanged (no rotation), `selector 1` swaps in
`rep2`, `selector 2` rotates in `rep3`, and `selector 3` rotates in
`rep1 - 1`. `ll_is_zero` is knowable from the PEEKED LL code alone (LL code
0 is the only code with baseline 0 and 0 extra bits), before any extra bits
are read — matching the reference decoder's evaluation order. Algorithm
cross-checked against both the RFC 8878 prose and the reference C source
(`ZSTD_decodeSequence` in `zstd_decompress_block.c`, per the Lesson-96
playbook of not trusting either alone), mirroring the `c/zstd` fix exactly.
The encoder is unchanged (decode-only fix).

**Verification:** the original constant-byte repro now decodes correctly;
a 60-trial fuzz sweep (periodic/constant/ramp/low-entropy-random byte
patterns, real `zstd` CLI → `decompress()`, sizes 500–4500 bytes) passed
with zero failures; all 28 pre-existing tests in `t/zstd.t` still pass
unaffected, since this port's own round trip never touches the new code
path. New `RT-12` regression test added covering both the constant-byte
repro and a cyclic-pattern case that forces back-to-back repeat-offset
sequences within one block.

**Rule:** when a sibling language port documents a decode-only feature gap
that stems from a *shared, repo-wide* encoder-side simplification (as
opposed to a port-specific translation mistake), audit every other port
implementing the same spec for the identical gap — the shared design
choice that hid the bug from that port's own tests hides it identically
everywhere else the same choice was made.

---

## Lesson 100 — `code/packages/go/zstd` had the same missing Repeated-Offset (R1/R2/R3) decode support found in `c/zstd` (PR #9941); confirmed independently, fixed, with a deterministic low-level regression test added

**Date:** 2026-08-05

**What happened:** While `c/zstd` (PR #9941, still open as of this writing) was being implemented, ad hoc fuzzing against the real `zstd` CLI turned up a decode-only feature gap independent of the FSE-codec bug class in Lessons 95-97: this repo's `zstd` decoders never implemented RFC 8878 §3.1.1.3.2.1.1's Repeated-Offset (R1/R2/R3) mechanism, where a sequence's `Offset_Value` of 1, 2, or 3 means "reuse one of the three most-recently-used match offsets" rather than "the literal distance is `Offset_Value - 3`". This repo's own `zstd` encoders (by design, across every language port — an explicit "no repeat-offset shortcuts" educational simplification) never emit an offset code below 4, so no port's self-consistency round trip, and not even a fixed single-corpus CLI-interop test (TC-9/TC-11), ever exercised the decode path. But real `zstd`'s encoder uses repeat offsets constantly, so any decoder that only understood explicit offsets systematically fails on a meaningful slice of real-world `.zst` files.

Auditing `code/packages/go/zstd/zstd.go` for the same gap (task explicitly scoped to Go, cross-checking against both PR #9941's diff and RFC 8878 directly rather than re-deriving the algorithm from memory — per the Lesson 96 playbook) confirmed it independently: `decodeSequencesSection` computed `off: ofRaw - 3` unconditionally and rejected `ofRaw < 3` as "decoded offset underflow" — exactly the failure mode PR #9941 documented, reproduced here byte-for-byte with the same minimal repro (4713 repeated `'Z'` bytes; real `zstd` picks a **Compressed** block over RLE for this size, containing one sequence with `Offset_Value = 1`).

**Rule:**
- A gap found and fixed in one language port of a shared-design package (here, `zstd`) should be checked against every sibling port explicitly — "the reference implementation was already corrected for X" does not mean every OTHER port that copied the same original design also received that fix. This is the same shape as Lesson 97 (haskell/zstd inheriting java/rust's FSE bugs), just for a decode-only feature gap instead of a symmetric codec bug.
- Fixed in `code/packages/go/zstd/zstd.go`: `decodeSequencesSection` and `decompressBlock` now take `rep1, rep2, rep3 *uint32` — the three Repeated_Offset registers — which `Decompress` initializes once per frame to the RFC-mandated default `1, 4, 8` and threads unmodified across every Compressed block in that frame. The selector algorithm (including the "`Literals_Length == 0` shifts the interpretation by one slot" special case) was cross-checked against both RFC 8878 prose and the literal reference C source (`ZSTD_decodeSequence`), matching PR #9941's already-verified fix.
- Verified via: (a) the exact CLI repro reproduced and re-checked (fails pre-fix with `decoded offset underflow: ofRaw=1`, passes post-fix); (b) new `TestTC12CliInteropRepeatOffset`/`TestTC12CliInteropRepeatOffsetFuzz` — real `zstd` CLI compresses, this package decodes, byte-exact — covering long constant runs and multi-cycle periodic patterns; (c) all pre-existing TC-1..TC-11 tests unaffected, since this port's own encoder never emits repeat-offset codes.
- **Because this port's own encoder can never produce a bitstream that exercises the repeat-offset decode branches**, the real-CLI interop tests alone left `decodeSequencesSection`'s new code at only ~70% statement coverage (some selector branches — particularly "reuse rep1 unchanged" and "use rep2", which real `zstd` happens not to select for the tested input shapes — were never hit). Added `TestRepeatOffsetSelectors` and `TestRepeatOffsetExplicitOffsetUpdatesRegisters`, which hand-construct FSE bitstreams via a small test-only helper (`encodeSingleSeqForTest`, bypassing the production encoder's "always explicit offset" restriction) to deterministically exercise all four selector branches plus the "explicit offsets also update the register history" rule — raising coverage to 93% for that function without depending on real `zstd` happening to produce a specific selector on a specific input. **When a decoder-only branch is structurally unreachable through your own encoder AND unreliable to reach via an external reference tool's input-dependent choices, a hand-constructed low-level bitstream test is the only way to get deterministic coverage of it — don't settle for "the CLI interop test passes" as proof all branches work.**

See `gh pr diff 9941` for the `c/zstd` reference fix this was cross-checked against, and Lesson 98 above for that original finding.

---

## Lesson 99 — `fsharp/zstd` inherited the same Repeated-Offset (R1/R2/R3) decode gap as `c/zstd` (Lesson 98), confirmed via real `zstd` CLI interop

**Date:** 2026-08-05

**What happened:** While implementing `c/zstd` (CMP07, PR #9941), an ad hoc
fuzz sweep against the real `zstd` CLI found that its decoder never
implemented Repeated-Offset (R1/R2/R3) sequence decoding (RFC 8878
§3.1.1.3.2.1.1) — an `Offset_Value` of 1, 2, or 3 is a reference into a
three-entry offset history, not a literal `Offset_Value - 3` computation.
That PR flagged every other language's `zstd` port as "suspect of the same
gap until specifically checked against varied real-world `zstd`-CLI-encoded
input" (documented as that PR's own Lesson 98). Auditing
`code/packages/fsharp/zstd/Zstd.fs`'s `DecompressBlock` confirmed it: the
line `let matchOffset = rawOffset - 3` computed the actual match offset
unconditionally, with no repeat-offset interpretation for `rawOffset` values
1-3, and no offset-history registers threaded through the frame at all.

A regression test (added to `ZstdTests.fs` BEFORE the fix, to prove the gap
rather than assume it): compressing 4713 bytes of a single repeated byte
`'Z'` with the real `zstd` CLI (no `--no-check`, exercising the checksum
trailer too) and decompressing with this package's `Zstd.Decompress`
raised `System.IO.InvalidDataException: match offset exceeds decoded
output` — real `zstd` chose a Compressed block whose one sequence is 2
literal bytes ("ZZ") + a match with `Offset_Value=1` (i.e. "reuse
`Repeated_Offset1`", which starts at its RFC-mandated default of 1, an
unmistakable RLE-via-repeat-offset pattern); the pre-fix decoder computed
`rawOffset - 3 = 1 - 3` which underflows a `let` binding typed as `int` to a
large negative-then-implicitly-huge value once used as an array/offset
computation, correctly rejected by the existing offset-bounds check as
malformed — even though the frame was perfectly valid.

**Why it went unnoticed:** Identical shape to the `c/zstd` finding. This
package's own `EncodeSequences` is, by design, incapable of emitting
`Offset_Value <= 3` (the minimum LZSS match offset is 1, so
`rawOffset = offset + 3 >= 4` always), so a self round-trip — and every
pre-existing TC-9 CLI-interop test in this package (added in 0.1.1 for the
Lesson 96 FSE-codec bugs), whose fixed prose corpus never happened to
produce a real-`zstd`-encoded sequence with `Offset_Value <= 3` — never
exercised this decode path.

**Rule (same as Lesson 98, reconfirmed cross-language):**
- Decode-side feature scope and encode-side feature scope are separate
  decisions. An "educational subset" simplification stated as "we don't
  emit repeat-offset sequences" must not be silently read as "we don't need
  to decode them," when the decoder's job is to accept output from the real,
  independent `zstd` CLI ecosystem — which uses repeat offsets constantly,
  as one of its principal entropy wins.
- A single fixed TC-9 corpus is necessary but not sufficient evidence of
  decoder conformance. Before trusting "TC-9 passes," fuzz the same
  interop check across varied inputs (constant-byte runs are the cheapest,
  most reliable way to force real zstd into a repeat-offset-heavy encoding).
- Fixed in `code/packages/fsharp/zstd/Zstd.fs`: `DecompressBlock` now takes
  `rep1`/`rep2`/`rep3` as `byref<int>` (frame-scoped, threaded from
  `Decompress` through every Compressed block, default 1/4/8 for the first
  block per RFC 8878), and implements the full peek-then-select-then-rotate
  mechanism for `Offset_Value` 1-3 — including the "when `Literals_Length`
  is 0, repeated offsets are shifted by 1" special case, using the peeked
  (not-yet-extra-bit-read) literal-length code to know whether the eventual
  literal length is zero. Ported from, and cross-checked against, both the
  literal reference C source (`ZSTD_decodeSequence` in
  `zstd_decompress_block.c`, github.com/facebook/zstd) transcribed in
  `c/zstd`'s fix (`gh pr diff 9941`) and an independent RFC 8878 §3.1.1.3.2.1.1
  fetch — not re-derived from memory. The encoder is unchanged (still never
  emits repeat-offset codes; decode-only fix). semantic package version
  bumped 0.1.1 -> 0.1.2.
- Verified via two new TC-9 regression tests (the 4713-byte constant-run
  repro above, plus an independent periodic-6-byte-cycle repro not
  dependent on the constant-byte-specific heuristic) — all 26 existing +
  new tests pass, line coverage 91.16% (threshold 80%) — plus an ad hoc
  42-case fuzz sweep against the real `zstd` CLI (constant, periodic at
  several cycle lengths, ramp, random, and prose patterns, 16 bytes to
  20 KB), all byte-exact.
- Every other language's `zstd` port in this repo remains suspect of the
  same gap until specifically audited — this PR only covers `fsharp/zstd`.

See also `gh pr diff 9941` (`c/zstd`, Lesson 98) for the original finding
and reference fix this port's fix was cross-checked against.

---

## Lesson 103 — `haskell/zstd` had the same Repeated-Offset (R1/R2/R3) decode-only gap PR #9941 found in `c/zstd`; every `zstd` port needs an independent audit, not just the one that found the bug class

**Date:** 2026-08-05

**What happened:** PR #9941 (implementing the first `c/zstd` port) found, while fuzzing against the real `zstd` CLI, that its decoder — despite being transcribed from the already-corrected `rust/zstd` reference (Lessons 95-97) — only understood *explicit* offset codes (`Offset_Value - 3`), not the Repeated-Offset (R1/R2/R3) mechanism RFC 8878 §3.1.1.3.2.1.1 uses for `Offset_Value` in `{1, 2, 3}`. That PR's own note flagged "other language ports may share the same gap." Auditing `code/packages/haskell/zstd/src/Zstd.hs` confirmed it: `decodeSequences` computed `matchOffset = rawOffset - 3` unconditionally, for every offset code, with no offset-history state at all. This is the exact same decode-only feature gap, independently present, in a codebase that was NOT copy-derived from `c/zstd` (Haskell's `zstd` predates it and has its own from-scratch FSE implementation, corrected separately for Lessons 95-97).

**Proof before fixing:** compressed 4713 bytes of a single repeated byte with the real `zstd` CLI (the same minimal repro from PR #9941 — real `zstd` picks a single Compressed block whose one sequence has `Offset_Value=1`, i.e. "reuse `Repeated_Offset1`", not an RLE block or explicit offset) and fed it to the pre-fix `decompress`: failed with `match offset exceeds decoded output` (the `rawOffset - 3` underflow, correctly rejected by the existing bounds check, but for a stream that was actually valid).

**Fix:** threaded a frame-scoped `RepOffsets` register triple `(Repeated_Offset1, Repeated_Offset2, Repeated_Offset3)`, defaulting to `(1, 4, 8)`, through `decompress` → `decodeBlocks` → `decompressBlock` → `decodeSequences`. Raw and RLE blocks pass it through unmodified (RFC 8878: the registers are frame-scoped, not block-scoped); each Compressed block's sequences both consult and update it. `decodeSequences` now maps `Offset_Value` (`offsetCode <= 1` ⟺ `rawOffset ∈ {1,2,3}`) to a selector `(if literals_length==0 then 1 else 0) + rawOffset - 1 ∈ [0,3]`, each of which both resolves the actual offset and rotates the three-entry history — cross-checked against BOTH the RFC 8878 prose (fetched directly) AND the literal `ZSTD_decodeSequence` reference C source in `zstd_decompress_block.c` (fetched directly, per the Lesson-96 "don't trust either alone" playbook), which independently confirmed the same selector shape (`ll0 + repCode` collapsing to the same four cases). This package's own encoder is unchanged — it never emits `Offset_Value <= 3` (`sequenceOffset >= 1` always forces `rawOffset >= 4` in `encodeOne`), so this remains, like `c/zstd`, a decode-only fix with no encode-side symmetry to break.

**Why it went unnoticed:** identical shape to the `c/zstd` finding — this package's own `compress()`/`decompress()` round trip, and every existing test including the Lesson-95/96/97 `ZstdCliInteropSpec` (TC-9) cases, never produces or needs an `Offset_Value <= 3`, so the gap was invisible to 18 passing tests (16 unit + 2 CLI-interop) until real `zstd`-compressed data using repeat-offsets was fed to `decompress` directly.

**A note on constructing a regression test:** the *first* attempt at a "multi-sequence" repeat-offset test (an 8-byte anchor + differently-seeded pseudo-random filler per repetition, repeated 80×) accidentally proved nothing — it passed even against the deliberately-reintroduced bug, because real `zstd`'s optimal parser collapsed the whole repeating unit into a single giant match needing only one (explicit) offset. A second attempt reusing the *same* filler every time failed for an unrelated reason (real `zstd` chose Huffman-compressed literals for that block, which this educational decoder doesn't support at all — a different, pre-existing, intentional scope limit, not this bug). The version that actually worked: an anchor recurring at a fixed distance with an almost-constant filler carrying only a one-byte "salt" (the repetition index) — low-entropy enough to keep real `zstd` on raw literals + predefined FSE tables (this decoder's only supported modes), while the salt still stops the LZ77 matcher from merging repetitions into one match. **Rule:** a regression test for a real-CLI-only decode gap must be verified to actually fail against the bug (temporarily reintroduce it and confirm red, exactly as for any other regression test) — a plausible-looking "multi-sequence repeat-offset" input can pass for reasons that have nothing to do with the fix, because the real encoder's optimal parsing and mode-selection heuristics are opaque and easy to guess wrong about.

Fixed in `code/packages/haskell/zstd/src/Zstd.hs` (`RepOffsets`, `decodeBlocks`, `decompressBlock`, `decodeSequences`). Verified via two new `ZstdCliInteropSpec` cases (a long single-byte run, and the salted-filler periodic pattern above, both real `zstd`-CLI-compressed, both confirmed to fail before this fix and pass after it) plus the full existing 18-case suite, all genuinely executed against the real `zstd` CLI (not skipped — the binary was on `PATH`). See PR #9941 for the `c/zstd` port where this class of gap was first found. `swift/zstd` (PR #9944) and `fsharp/zstd` (Lesson 99, above) were independently fixed for the same gap in parallel, merged into `origin/main` while this audit was in progress — confirms the "other ports may share this gap" prediction was correct at least twice already; the rest of the repo-wide `zstd` set (java, rust, go, python, ruby, typescript, kotlin, perl, elixir, lua, dart, csharp) has not all been individually re-audited for this specific gap and should be treated as suspect until checked.

---

## Lesson 104 — `cpp/zip`: a spec byte-offset table bug, and "trim to declared size" silently defeats an aggregate decompression-bomb budget

**Date:** 2026-08-05

**What happened, part 1 (spec bug):** `code/specs/CMP09-zip.md`'s Local File
Header and Central Directory Header wire-format tables mis-sized
`Last_Mod_File_Time`/`Last_Mod_File_Date` as 4 bytes each instead of 2,
cascading a 4-byte offset error through every field after them (the table
claimed a 34-byte fixed Local Header with CRC-32 at offset 18; the correct,
universally-implemented layout — confirmed against `rust/zip` and
`python/zip`'s actual `struct.pack`/byte-offset code, both of which use
2-byte `mod_time`/`mod_date` fields — is a 30-byte fixed Local Header with
CRC-32 at offset 14, and a 46-byte fixed Central Directory Header). Every
implementation in the repo, including the reference Rust port, had always
written and read the CORRECT layout; only this one table's prose was wrong,
undetected because no port's tests round-trip against the written spec text
byte-by-byte — they round-trip against each other's code. Fixed the table
and its two downstream mentions (a corrupted-CRC-byte test-vector comment
still said "bytes 18–21" after the first pass fixed the table but not that
comment — caught in a later security-review round, not the same pass that
fixed the table; check EVERY prose reference to a byte offset, not just the
table itself, when correcting one).

**What happened, part 2 (security-review finding, the more important one):**
`ZipReader::read`'s original design silently TRIMMED an over-large
decompressed buffer down to the Central Directory's declared
`Uncompressed_Size` field before returning — matching the Rust reference's
own `if decompressed.len() > entry.size { decompressed.truncate(entry.size) }`
comment ("guards against a decompressor over-read"). `zip::unzip()`'s
aggregate decompression-bomb budget (a configurable total across every entry
it decompresses, on top of the existing per-entry `ca::deflate::inflate` cap)
then accounted the RETURNED (trimmed) size against that budget. Since
`Uncompressed_Size` is an attacker-controlled Central Directory field, a
crafted entry can declare `Uncompressed_Size = 0` while its real DEFLATE
stream still costs the FULL per-entry cap (256 MB) of genuine CPU/memory work
to decompress — the trim happens AFTER that work is done, so the caller-side
budget sees a 0-byte result and never grows, letting arbitrarily many such
entries each smuggle real decompression work past an "aggregate" cap that
never appeared to be approaching its limit. This is the identical bug class
independently found and fixed three separate times in `haskell/zip`
(Lessons in that package's own CHANGELOG — "the aggregate budget counted the
post-truncation size, not the actual decode work performed") and is a strong
signal that **every** `zip`/decompression-container port in this repo that
enforces an aggregate budget by trimming-then-measuring, rather than
rejecting a declared/actual size mismatch outright, is suspect until
individually audited.

**Rule:**
- A decompression-bomb budget that measures its input AFTER a lossy
  trim/truncate step is measuring the wrong thing. The trim itself must be
  replaced with a hard rejection (throw/error) whenever the actual
  decompressed size disagrees with a declared size sourced from untrusted
  input — for any HONESTLY-produced archive the two are always exactly
  equal by construction (a writer that compressed N bytes always declares
  `Uncompressed_Size = N`, and a correct DEFLATE stream decoding that entry
  always reproduces exactly N bytes), so rejecting a mismatch cannot break
  legitimate files, only crafted ones.
- Bounds-check arithmetic that combines an attacker-controlled field with a
  small constant (`local_offset + 6`, `local_offset + 26`) must be done in a
  width wide enough that the ADDITION ITSELF cannot wrap — not just the
  eventual `offset > buffer.size()` comparison. Doing the comparison safely
  in `uint64_t` while the offset argument was already computed via a
  narrower (`size_t`) addition one call-site up still leaves the wraparound
  where it always was, just one line earlier; the fix has to move with the
  addition, not just the check. In `cpp/zip` this meant changing
  `read_u16`/`read_u32`'s parameter type from `size_t` to `uint64_t`
  specifically so every call site is forced to widen the base value BEFORE
  adding to it, and having `ZipReader::read` widen `entry.local_offset` to a
  `uint64_t` exactly once rather than re-deriving a `size_t` copy at each of
  three call sites.
- The same "silent truncation into a structurally-misleading result" failure
  mode applies symmetrically on the WRITE side of any binary container
  format with fixed-width wire fields: an oversized single value (entry
  name/data past a 16-bit/32-bit field), a cumulative running total (Local
  Header offsets, Central Directory offset/size past 4 GiB), and a count
  field (more entries than a 16-bit count can represent) are three distinct
  places the same bug can hide, found across three separate review rounds
  on this package (`NameTooLong`/`DataTooLarge`, then `ArchiveTooLarge`, then
  `TooManyEntries`) — auditing only the first one found is not sufficient;
  grep every `static_cast<uintN_t>(...)` in a writer for the ones narrowing
  an unbounded (or merely uncapped) `size()`/count, not just the one the
  first pass happened to touch.

Fixed in `code/packages/cpp/zip/include/zip.hpp` (`ZipError::DeclaredSizeMismatch`,
`detail::require_fits_u32`, `read_u16`/`read_u32` widened to `uint64_t`
offsets, `NameTooLong`/`DataTooLarge`/`ArchiveTooLarge`/`TooManyEntries`).
Verified via dedicated regression tests for each finding
(`test_declared_size_mismatch_rejected`, `test_writer_name_too_long_rejected`,
`test_writer_too_many_entries_rejected`, `test_extreme_local_offset_rejected`)
plus the full 12-TC spec suite, real CLI-interop against the system
`zip`/`unzip`, and a real dynamic-Huffman fixture — 90 checks total, all
passing under GCC and Clang with `-pedantic-errors -Wall -Wextra -Werror`.
Every other `zip`/archive-container port in this repo that enforces an
aggregate decompression-bomb budget should be audited for the same
trim-then-measure pattern, not just `cpp/zip` and the three `haskell/zip`
rounds that found it independently first.

## Adding a same-shaped `if` block to a shared recursive dispatcher can overflow the stack on ONE platform only, even with correct logic and green local tests

`adj-lang`'s `expand_rec` (`code/packages/rust/adj-lang/src/lower.rs`) recurses
one native stack frame per AST level, guarded by `FORMULA_MAX_NODE_DEPTH`
(descend-then-check, so recursion never exceeds the cap). Its own doc comment
already flagged the margin as tight: "a *few hundred* debug-build frames
already approach the default ~2 MiB worker-thread stack." Adding FL-11's
`min(a, b)`/`max(a, b)` built-ins as two more `if name == "max" { .. let a =
..; let b = ..; }` / `if name == "min" { .. }` blocks — each an exact structural
copy of the pre-existing `mod` block, just with different names and a different
wrapped node — compiled clean, passed `cargo clippy`, and passed all 252 unit
tests locally (Windows). It still made macOS CI's `build` job fail for real:
`deep_operator_spine_trips_the_nesting_guard_not_the_stack` (a test asserting
the depth guard fires *before* the native stack does, on a 400-level spine)
aborted with `SIGABRT: process abort signal` / "has overflowed its stack" — the
guard never got a chance to fire, because 96 recursion frames of the new,
slightly larger `expand_rec` no longer fit in the runner's ~2 MiB thread stack.
Linux and Windows builds passed; only macOS's `build` job failed, and it was a
REAL failure (`gh api .../jobs/<id> --jq '{status,conclusion}'` showed
`"conclusion": "failure"`, not `"cancelled"`), so [[Check cancelled vs failed CI
jobs before debugging]] correctly routed to log-reading instead of a rerun.

Root cause: in an unoptimized (`cargo test`/debug) build, rustc/LLVM does not
reliably coalesce stack slots across sibling `if`-blocks within one function —
each block's locals (here, two `ExprAst` bindings) can each claim their own
space in the function's frame, so three near-identical blocks (`mod`, `max`,
`min`) cost roughly 3× one block's worth of frame size, not 1×. Fix: merge
same-shaped dispatch blocks that recognize different built-in NAMES but expand
identically (exactly two args, expand both, wrap in the built-in's own node)
into ONE `if name == "mod" || name == "max" || name == "min" { .. }` block with
ONE shared pair of locals, dispatching on `name` only at the point of
constructing the result node. This restored the exact frame footprint `mod`
alone had before the change — no logic change, same test outcomes, macOS green
on the next CI run.

Lessons:
1. **A function that recurses through itself (a walker/interpreter/expander)
   has a stack-frame-size budget shared by ALL its branches, not just the one
   your change touches.** Adding a new `if`/`match` arm with its own locals to
   such a function is not "adding code," it's "growing every future call's
   frame" — measure against the SAME kind of margin a recursion-depth cap
   documents (see the "8 MB vs 1 MB stack" lesson above; this is that lesson's
   sibling for "more local variables per frame" instead of "more frames").
2. **When several `if`/`match` arms share an identical shape (same arity check,
   same recurse-both-children pattern, different only in which node they
   construct), merge them into one arm with a final dispatch on the
   discriminant.** This is not just DRY — in a debug build it can be the
   difference between a stack-safety margin holding and not.
3. **A green local test suite does not confirm a recursion-adjacent change is
   stack-safe on every target platform.** Different OS default thread stack
   sizes (and different compiler stack-slot-reuse behavior per platform) mean
   the same source can pass on Linux/Windows and abort on macOS (or vice
   versa) purely from a stack-frame-size change, with zero logic difference.
   If a crate has an existing depth/recursion guard whose doc comment already
   calls out a tight stack margin, treat any change to the guarded walker's
   own function as touching that margin, and re-run (or at least reason
   about) its dedicated stack-overflow regression test specifically.
4. `SIGABRT` / "has overflowed its stack" from a `#[test]`-harness thread
   (Rust's default per-test-thread stack is ~2 MiB unless `RUST_MIN_STACK` or
   an explicit `Builder::stack_size` overrides it) is the same failure class as
   Windows' `0xC00000FD` — a real stack overflow, not a flaky/cancelled CI job.

## A conservative-stack-scan GC test's "freed" count is not deterministic in a debug build — assert on a non-conservative signal instead (gc-core-capi, AOT00-T8)

Writing a capi-level test for the new `__gc_collect_minor_precise` entry (AOT00-T8,
adaptive safepoint scheduling), I copied the exact pattern every other stack-scan
smoke test in `gc-core-capi/src/stack_scan.rs` already uses: allocate a `kept` object,
root it in a local; allocate a second object and immediately discard the pointer
(`let _ = __gc_alloc(16);`); call the collect entry; `assert!(freed >= 1, ...)`. It
failed **deterministically** (not flaky-sometimes — every run, `freed == 0`) even
though the underlying algorithm was already proven correct by a controlled
gc-core-only unit test with exact root slots. Swapping in the pre-existing,
already-shipped `__gc_collect_precise()` in the *exact same test function* reproduced
the identical `freed == 0` failure — proving the bug was not in my new code at all,
but in the test's own shape.

Root cause is the sibling of the "same-shaped `if` block" lesson directly above, one
layer down: in an unoptimized debug build, a discarded temporary's value (the dead
object's return value from `__gc_alloc`) is not guaranteed to be scrubbed from the
stack slot/register it transiently occupied — it can keep sitting there, byte-for-byte
identical to a real heap address, for the rest of the function's lifetime. A
conservative stack scan (`__gc_collect`/`__gc_collect_precise`/`__gc_collect_minor_precise`
with no stack maps registered — the exact case every existing unit test exercises)
reads *every* word in the scanned region as a *candidate* root, so that stale word
retains the "dead" object regardless of whether any live Rust binding still names it.
Whether a given test happens to dodge this is pure happenstance of that function's
specific local-variable layout and register allocation, not a property of the GC
algorithm — mine tripped it, several pre-existing tests apparently do not (this time).

**Fix:** don't assert `freed >= N` (or any positive lower bound) as the *sole* signal
in a new stack-scan integration test — it's inherently non-deterministic and doing so
just adds a coin-flip to CI without telling you anything actionable. If the property
under test has a **non-conservative, exact** way to check it (here: `__gc_kind_of`,
which does a real `find_header` lookup — no stack scanning involved — so it reports
"still live" vs "reclaimed" precisely), assert on that instead. Reserve the
`freed >= 1` pattern (as the *existing* precise/compacting smoke tests already do) for
cases with no better signal available, and don't add new tests that rely on it as the
primary proof of a specific new behavior.

**Update, having gone back to actually fix the 4 pre-existing flaky tests this entry
originally just characterized (not fixed):** `__gc_kind_of(dead)` is *not* a valid fix
for "prove this ONE specific unreferenced object was reclaimed" — it's a genuine
Catch-22. To read `dead`'s address back *after* the collect call, `dead` must still be
a live Rust local spanning that call — which means the compiler keeps its value in some
register or stack slot the conservative scan can see, and a conservative scan *correctly*
treats any value that looks like a live object's address as a possible root. So keeping
`dead` around long enough to check it is exactly what makes the scanner (correctly!)
retain it — confirmed empirically: adding the `__gc_kind_of` check turned an
occasionally-flaky `freed >= 1` into a *deterministically failing* "still alive" for
every one of the four affected tests. This is not a bug in the collector; it's what
conservative scanning is supposed to do.

## A "make it unreachable in a callee, then collect in the caller" GC differential is not reliable on `aarch64-backend` — the callee's vacated frame sits inside the always-conservative `[sp, start_fp)` gap (AOT00-T8, write-barrier follow-up)

Building a real compiled-and-executed proof for `aarch64-backend`'s new `field_store`
write-barrier emission (mirroring the LLVM-backend sibling test, which *does* work —
see `iir-to-llvm`'s changelog), I tried: `main` allocates `parent`, minor-collects it
to tenure it old, calls `helper(parent)` (a separate `IIRFunction`) which allocates
`child` and `field_store`s it into `parent`, then `main` minor-collects again and
checks `child`'s survival via `gc_live_bytes()`. The idea: once `helper` returns,
`child`'s only named local is gone from every currently-live frame, so its survival
should depend purely on the barrier's remembered-set edge.

It passed — with the barrier call *and* with it temporarily removed (a `TEMP-REVERT-
CHECK` comment, `git diff` restored immediately after). A vacuous pass, confirmed via
a full `cargo clean -p aarch64-backend -p twig-aot` rebuild both times (ruling out
stale-binary caching, a mistake this session already hit once with the LLVM sibling
test — see that lesson if it exists). Diagnostic instrumentation (returning
`gc_live_bytes()`/`freed` at each stage as the exit code) showed the object was never
even freed by the **already-shipped, already-well-tested** `__gc_collect_precise` in
the identical shape — with *no* barrier or `parent`/`field_store` involved at all,
just "allocate `child` in a helper, never reference it again, minor/precise-collect in
the caller." So this was never about the barrier, or about my new code; it's a
property of the collector's own stack walk.

Root cause: `gc-core-capi/src/precise_walk.rs`'s own module doc names it explicitly —
`[sp, start_fp)`, "the collector's own frames, below the first walked frame," is
**always** scanned conservatively, with no stack map, regardless of what used to be
there. `start_fp` is the first frame-pointer-mapped frame reached by walking up from
the collector's own entry — i.e. the caller (`main`, in this test). Since the stack
grows down, *every* function `main` has ever called and returned from — including
`helper`, long after it returned — occupies addresses strictly below `main`'s own
frame pointer, i.e. **inside** this exact gap. The collector cannot distinguish "my
own internal call frames" from "some long-since-returned user frame that happens to
sit in the same address range" — both get the identical bias-to-leak conservative
scan. So `child`'s stale address, sitting untouched in whatever stack slot `helper`
last wrote it to, is conservatively rediscovered on every subsequent collect call made
from `main`, no matter how many frames deep or how long ago `helper` returned — the
"once popped, a callee's locals are gone" intuition a heap-allocated-object test
naturally reaches for **does not hold** across this specific gap.

**This is not a bug** — it's the same deliberate bias-to-leak / never-under-mark
design every collect entry in this codebase already documents, and changing it (e.g.
zeroing callee frames on return) would be a real, unrelated engineering project with
its own costs, not a quick fix. It also is not specific to the new minor-collect
entry: the already-shipped `__gc_collect_precise` reproduces it identically, in a
shape with zero write-barrier involvement, proving the *test's* premise was wrong, not
the code under test (the exact same "swap in the pre-existing, already-shipped
collector and reproduce the identical failure" diagnostic move as the lesson directly
above — it is worth repeating as a default first step whenever a *new* GC entry point
looks broken).

**Fix (this session): don't chase a real-execution differential for this backend
across a callee-return boundary.** Fall back to the unit-level relocation-symbol
assertion (`aarch64-backend`'s own `compile_with_relocs` — assert the expected
`BL __twig_gc_write_barrier` relocation is emitted, confirmed load-bearing via a
revert-check at that level) as the primary evidence, exactly as `array_ref_tracing.rs`
concluded for a different but structurally similar reason ("the actual, reliable
regression proof lives at the `gc-core` level," not the compiled pipeline) — and say
so plainly in the PR: this GC-codegen change has strictly weaker end-to-end
verification than its LLVM-backend sibling, which *does* have a working real-execution
differential (LLVM's own SSA-value liveness/register allocation, not this backend's
per-function whole-lifetime stack-map declaration plus this specific conservative
gap, is presumably why that one works) — a genuine, structural asymmetry between the
two backends' testability, not a gap this session chose to leave open.

**If a real-execution differential across this exact boundary is ever needed again:**
the gap is a property of *where the collect call sits relative to the first mapped
frame*, not of the object itself — a design that kept the collect call and the
target object at a *shallower* call depth than any already-returned frame (so the
returned frame's memory is genuinely below, not overlapping, the conservative-gap
scan) or that deliberately clobbered the vacated stack region before collecting might
work, but is fragile by construction and wasn't pursued here given the unit-level
signal was already sufficient and reliable.

**Actual fix:** allocate a **batch** of dead objects (`DEAD_BATCH = 64`), none ever
bound to a Rust local that outlives its own loop iteration — so no *specific* address
needs to survive the collect call for verification. An unoptimized debug build's stray
stack/register garbage can only accidentally retain a small, bounded number of stale
addresses (bounded by how many registers/stack slots a call site can spill — this file's
own `spill_and_sp` bounds a maximal spill at 18 words), so among 64 independent,
never-named allocations, the overwhelming majority are certain to have no stale
reference anywhere on the stack. `freed >= DEAD_BATCH - STRAY_TOLERANCE` (tolerance 8)
is then a deterministic, generous bound — confirmed via 20 consecutive local runs, all
clean, after being flaky often enough in CI to get flagged as a known issue in the first
place. Applied to `gc-core-capi/src/stack_scan.rs`'s four affected tests.

## `iir-to-llvm`'s own real-execution write-barrier differential is NOT automatically portable from `field_store` to `array_set` — a within-one-frame vacuous pass, no callee-return boundary needed (AOT00-T8 follow-up)

Adding `array_set`'s generational write-barrier emission (mirroring `field_store`'s —
see `iir-to-llvm/src/lib.rs`'s `lower_array_set`/`lower_field_store` — and the
aarch64/x86_64 siblings' identical fix), I built a real-execution differential mirroring
`lang-aot/tests/llvm_gc_write_barrier.rs` (`field_store`'s own, which genuinely works —
confirmed via a `TEMP-REVERT-CHECK` that correctly turns the assertion red): allocate
`parent` (a 1-element `array<i64>`), minor-collect to tenure it old, allocate `child`,
`array_set parent, 0, child`, minor-collect again, assert `gc_live_bytes() == 32`
(both survived) via the barrier's remembered-set edge, with neither `parent` nor
`child` referenced by any local past the store — same shape as `field_store`'s test,
same reasoning for why nothing else should root either object into the second collect.

It passed — with the barrier call present, **and** with it fully deleted (no barrier
call, no address computed at all, i.e. behaving exactly like the code before this PR).
Unlike the aarch64 lesson directly above, this is **not** a callee-return-boundary
gap — everything happens in one `main` frame, no calls into a separate `IIRFunction`.
Root cause (probable, not fully isolated): `array_set`'s codegen is structurally
heavier than `field_store`'s — it emits `emit_bounds_check`'s conditional branch
(`icmp uge` + `br` to a trap block) in addition to the element `getelementptr`+`store`,
where `field_store` is straight-line with no branch at all. A branch forces `%handle`
to be available on both edges, which is exactly the kind of live-range shape an
unoptimized (`-O0`, no `opt` passes ever run on this hand-written IR) codegen path
spills to a stack slot rather than a register — and unlike a register, a stack slot
that nothing later reuses keeps its stale bit pattern for the rest of the frame,
conservatively rediscoverable by any later collect regardless of the write barrier.
Since `parent`'s own address is what leaks, and a **directly root-reachable** old
object is traced into on every collect regardless of remembered-set membership (see
`code/specs/AOT00-T9-moving-minor-collector.md` §3's identical point about
root/region-reachable old objects), `child` — sitting in `parent`'s own payload —
is found either way. This was verified by *removing* every barrier-adjacent
instruction (not just the call), so it is not an artifact of leftover dead-code
computed for the barrier itself; the pre-existing, unfixed `array_set` would be
equally vacuous under this exact test shape.

**Fix: don't ship the differential.** Deleted it rather than keep a passing-either-way
test in the suite (a false-positive regression guard is worse than no guard — it would
silently stop catching a reverted fix). Relied instead on: `iir-to-llvm`'s own
IR-string unit tests (`array_set_calls_the_generational_write_barrier` et al. in
`tests/test_backend.rs` — these DID catch a real bug during development: an earlier
draft passed `handle` itself as the barrier's `parent`, which is `raw_payload + 8`
in this backend's `alloc_array` lowering, not the true base `write_barrier` needs);
`field_store`'s own already-working differential, which proves the barrier
*mechanism* end-to-end for a call shape where it demonstrably matters; and
`gc-core`'s own generic, already-reviewed `write_barrier` tests. Exactly the same
call the aarch64 PR (lesson above) and `array_ref_tracing.rs` (`lang-aot` — "This
file does NOT attempt to prove the reclamation bug end-to-end… The actual, reliable
regression proof lives at the gc-core level instead") already made for structurally
similar reasons: **don't chase a real-execution differential across a scan gap this
codebase already knows is conservative-biased; the unit-level proof is the reliable
one.** If a real-execution proof for this specific op is ever needed again: forcing
`%handle`'s stack slot to be reused before the second collect (e.g. by doing enough
unrelated allocation/branching work in between) is the same "make it churn" approach
`gc-core-capi/src/stack_scan.rs`'s `DEAD_BATCH` fix above took, but wasn't pursued
here given the unit-level signal was already sufficient.

## Registering a `{0,8}`-style movable GC kind for `vm-core`'s generic `gc_alloc` is NOT a safe drop-in, unlike native-AOT's `__dyn_cons`/records — vm-core's fields are tagged words, not always-boxed (AOT00-T9 PR-5 follow-up scouting)

While scoping the next GC work item after AOT00-T9 PR-5 (moving-minor pacing) landed, I
looked at `vm-core`'s own "not yet load-bearing for relocation" limitation that PR-5's
changelog documents: every `vm-core` `gc_alloc` registers kind `0` (opaque/conservative),
so `collect_compacting`/`collect_minor_compacting` never actually relocate anything when
driven by vm-core — they degrade to non-moving behavior every time, safely but with zero
payoff. The obvious-looking fix: mirror native-AOT's own `__dyn_cons`, which lazily
registers a `{0,8}` kind (both 8-byte fields are reference slots) via
`__gc_register_kind` and allocates through `__gc_alloc_kind` instead of the opaque
`__gc_alloc` — `FlatHeap::register_kind`/`alloc(n, kind)` are already `pub`, so wiring
this into `vm-core::handle_gc_alloc` looks like a small, mechanical change.

**It is not sound, and I did not implement it.** Native-AOT's `{0,8}` cons/record kind
is safe only because native's own object model guarantees every field of a
kind-registered allocation is **boxed** — a genuine heap reference, never a raw scalar
(see the "records precise + movable" PR: "Record fields are boxed (constructor params
typed `any`) → `{0,8}` sound"). `vm-core`'s object model is different: `handle_gc_field_store`
(`vm-core/src/dispatch.rs`) stores a **tagged word** per field — `Value::HeapRef` gets one
tag (`FIELD_TAG_HEAP_REF`, `0b111`), `Value::Int` gets a different one (shifted, no `0b111`
low bits) — so a field vm-core allocates via `gc_alloc` can legitimately hold a raw integer,
not just a reference. This is true even restricted to cons cells specifically: `(cons 1 2)`
routinely stores bare integers as car/cdr, so "just register cons kind, not the generic
`alloc`" doesn't dodge the problem — cons is precisely the case where a field is sometimes a
ref and sometimes not.

Traced why this matters for *compaction* specifically (not just marking, which already
tolerates this fine): `mark_word` (`gc-core/src/flat_heap.rs`) tag-strips before checking
`find_header`, so a raw tagged int is simply never found as a live block — safe, standard
over-approximation-is-fine marking. But `fixup_ref_fields`'s `forwarded()` helper — the
function that decides whether to *rewrite* a precise field's bits during a moving
collection — looks the word (raw or tag-stripped) up as a **key in the `forward`
HashMap** (the set of addresses that were *actually* relocated this cycle) and only
rewrites on a hit. A raw int field is therefore rewritten **only if its bit pattern
happens to exactly equal some unrelated object's old base address** — astronomically
unlikely in practice, but a real, wrong-direction correctness bug if it ever fired (an
int's value silently corrupted to a stale pointer bit pattern), not a "safe
over-approximation" the way conservative-scan false positives are. This is a
categorically different risk from the pin-when-unsure/bias-to-leak arguments that justify
every *other* accepted probabilistic-collision case in this codebase (all of which retain
*too much*, never rewrite something wrongly).

**Not pursued further this session** — fixing this for real needs either (a) type-directed
field maps so vm-core can tell gc-core which specific field offsets are *always* references
for a given allocation site (a real new feature: field-level type tracking doesn't exist in
vm-core's IIR-op interface today), or (b) an explicit, reviewed decision to accept the
collision risk with a written soundness argument bounding it (this codebase's security
reviews have not been asked to accept this exact class of risk before, unlike the
already-reviewed conservative-scan collision cases). Either is a real design decision, not
a quick follow-up PR — flagging it here so a future session doesn't rediscover the same
trap by implementing the "obvious" mechanical version.

**Resolved in AOT00-T10** (2026-08-11, following an explicit owner directive that vm-core,
being unreleased, is free to change design rather than work around it: "We shouldn't have
10 different GC implementations"). Option (a) above — type-directed field maps — turned out
not to be the only way to give gc-core "which words are references" ground truth: vm-core
already computes that ground truth *dynamically*, once per store, via the tag bits this exact
lesson describes (`FIELD_TAG_HEAP_REF` vs. everything else). `gc-core` gained a second,
**tagged** kind-registration mode (`FlatHeap::register_tagged_kind`) that trusts a slot's own
tag bits at scan time instead of assuming every slot is always a reference — closing the
`forwarded()` collision risk exactly, not just bounding it. See
`code/specs/AOT00-T10-tagged-field-kinds.md` for the full design and `vm-core`'s
`VMCore::pair_kind` for the wiring.

## A blanket `--testTimeout` override during local verification hides timeout failures

**Context:** `human-language-data`, PR #10043 (second core-verb tranche, +24 lessons).

**What happened:** I verified locally with `npx vitest run --testTimeout=180000` and
reported "368 tests pass". CI then failed on `tests/cli.test.ts` — *"Test timed out in
5000ms"*. The override I passed to give slow corpus-walking tests room had silently
raised the ceiling for **every** test in the run, including the one that had no explicit
timeout and was relying on vitest's 5,000 ms default. I had made the very failure mode
invisible to the check that was supposed to catch it.

**Why it bit here:** `runCurriculumGapReport` builds the entire gap report **twice** (once
`--format json`, once `--format text`) over the whole corpus, and the report had just
gained a continuity section on top of modality, levels, verbs, chapters and ramp. At 1,249
lessons it runs ~5.08 s locally — already *over* the line on CI's slower runner. It was not
specific to this PR: the next content PR to land would have broken it instead, because the
corpus only ever grows.

**Fix (two parts):**
1. Give a test that legitimately needs more than the default an **explicit per-test**
   timeout — `it("...", { timeout: 60_000 }, () => {...})` — so the budget lives next to
   the test that needs it and travels with it, rather than being supplied by whoever
   happens to run the suite.
2. **Verify at default timeouts.** Run the suite once with no `--testTimeout` flag before
   claiming it passes. Use a per-file override only when actively iterating on one slow
   test, never as the final check. A local run whose flags differ from CI's is not
   evidence about CI.

**Generalisation:** any CLI flag that loosens a threshold globally (`--testTimeout`,
`--bail=0`, `--maxWorkers`, coverage thresholds) makes a local run *less* like CI, not
more. If you pass one to get a green run, the green means less than it looks like — and
the final verification pass should always be the one with no flags at all.

## A number that means one thing, read as meaning another — and the fix reproducing the bug (human-language-data, HL09)

The gap report said Spanish "reached A2". It had not: 178 words against the
~1,000–1,500 A2 asks for, fourteen lessons all realizing one spine node, present
tense only. **Nothing in the code lied.** `TrackLevelCoverage.reach` is documented
as *"the highest level this track has any lesson at"*, and that was accurate. One
lesson pointing at one A2 node moves it. The failure was that a number meaning
**touches** was read, for the life of the project, as meaning **attains**.

Then the gate written to fix that committed the identical error inside itself. Its
first version measured **whole-track** vocabulary (Spanish 138) against a
**per-level cumulative** target, and applied the atom-budget and reinforcement
criteria track-wide — so one over-budget Hindi lesson sitting *above* pre-A1 blocked
pre-A1, making that criterion unfalsifiable at the bottom of the ladder for every
track. Correctly scoped, Spanish's pre-A1 vocabulary is **44, not 138**.

**The general rule: when a criterion says "at or below level X", the measurement
must be filtered to level X.** A whole-corpus figure compared against a per-slice
threshold is not a stricter version of the right check — it is a different check
that happens to return a number. Three sibling bugs in the same module, all found by
the pre-push security review:

- criterion read "never revisited" where the spec said "fewer than twice", hiding 51
  of 141 failures;
- vocabulary counted every lesson type, so drill titles became words — `(practice)`,
  `qu-`, `fact or wish?`, 25 of 138;
- a level with **no authored nodes passed** its node criterion, because "no node is
  unrealized" was implemented as "every node is realized" — the same touches-vs-means
  error one level up, and live for B1–C2, which have zero nodes.

Corollaries worth keeping:

1. **Name the population in the field name or the doc.** `vocabulary` alone invites
   the misreading; `vocabulary` documented as *"at ANY level — context, not the
   criterion"* does not.
2. **Report the shortfall, not a boolean.** `false` moves the argument; *"teaches 44
   distinct headwords at or below pre-A1, against 300"* settles it.
3. **Absent ≠ zero.** "Not measured" and "attained nothing" are opposite facts; the
   section is `undefined` when its inputs were not supplied, and a test pins that.
4. A related discovery in the same PR: `report-cli` had never passed `curricula`/`spine`,
   so the whole `levels` section had been **silently absent from every CLI run** since
   it shipped — implemented, tested, and invisible to anyone reading the output. A
   feature with tests but no rendered output is not shipped.

## `assert old in s` is not enough for a scripted replace — assert it is UNIQUE (human-language-data)

A Python patch script filled placeholder pins with real numbers. It guarded every
replacement with `assert old in s`, per the existing lesson about scripted edits. It
still corrupted the file: `expect(report.summary.missedByWindow.R2).toBe(0)` appeared
**twice** — once as a corpus pin awaiting its value, once as a legitimate unit-test
assertion that no window is judged on a short track. `str.replace(old, new, 1)` took
the first, which was the unit test.

An earlier no-op guard in the same session produced `expect(report.summary.).toBe(1)`
by replacing a substring with the empty string, which the parser caught only because
it was syntactically invalid.

**Use `assert s.count(old) == 1`, or match on enough surrounding context to be
unique.** Presence proves the target exists; it does not prove you are editing the one
you meant. Tests caught both here — but a non-unique replace that lands on a *valid*
line produces no error at all.

## Backticks in `git commit -m "..."` are shell command substitution (any repo)

A commit message written with `-m` inside double quotes contained `` `review` `` and
`` `practises.knowledge` `` as inline code. zsh executed them:

    (eval):22: command not found: review
    (eval):22: command not found: practises.knowledge

The commit succeeded with those words **silently deleted** from the message —
"interleave a  lesson every three lessons" — and nothing failed. Found only by reading
the message back with `git log -1 --format=%B`.

**Write commit messages with a heredoc** (`git commit -F - <<'MSG'` … `MSG`), quoting
the delimiter so nothing expands. This applies to `gh pr create --body` too, which is
already conventionally written that way in this repo.

## A `practises` entry with no block-level `assesses` is rejected, and that gate is the point (human-language-data)

Closing spaced-retrieval windows looked like a frontmatter edit: add the atom to
`practises:` → `knowledge: [...]` and the measurement closes. The validator refused:

    ERROR [schema-v2-block-assessment-missing] ES-C05-hasta-luego:
      practised atom 'ES-LEX-ADIOS' is not assessed by any body block

That rule enforces the honesty principle HL09 §7.2 states in prose: **you cannot claim
practice without pointing at where it happens.** The frontmatter-only edit would have
closed the metric while helping no learner — a hollow claim that reads as progress.

The real work is two edits per atom: the frontmatter list **and** the
`assesses=[...]` of the specific `<!-- hl-knowledge: -->` directive on the block whose
prose actually exercises it. Of 58 open windows in Spanish chapters 3–6, only **17**
had prose to point at; the other 41 were genuine absence and were left open. A low hit
rate is the honest result, not a failure of the pass.


## A deletion regex must be verified on its OUTPUT, not on the token's absence

Removing nine cross-volume lesson ids (`TE-C29`, `AR-C27`, …) from prose, I checked the
only thing that was easy to check: that no id remained. Zero. Every generated artifact
regenerated, every `--check` passed, 392 tests green.

The regex had also eaten the words **around** each id. Thirteen sites, six of them
printed in the PDF and read aloud in the audio:

- `shares's spectacular PIE root?` — a recall question whose answer object vanished
- `treat it the same cautious way treated its own equivalent finding` — no subject
- `the same hedge already applied to Kannada's *aparāhna* in: thin evidence` — dangling
- `as Spanish fui, taught in), and war/waren` — dangling inside a parenthesis
- `a distinction this arc cares about, per's own finding` — `per's`

The absence of the token was never in question. What the sentence read like afterwards
was, and no automated check I had could see it. A security-review subagent reading the
prose caught all thirteen.

**Rule:** when a regex deletes a token from human-facing text, the deletion changes a
*sentence*, not a *string*. Diff the prose and read it. Grep the result for the
signatures of a bad splice — `'s` with no owner, `( `, ` )`, `in:`, `,,`, doubled
spaces, a verb with no subject. And prefer a replacement that names what the token
named (`TE-C29` → `Telugu's`) over one that deletes it, so the sentence keeps its
grammar by construction.

**Corollary, same session:** `git checkout -- <file>` to undo a *test* mutation threw
away that file's real, uncommitted fix too — it restores from the index, and nothing
was staged. Mutate a **copy** in the scratchpad, or stage the real work first.


## Replacing a pointer writes a new sentence, and a new sentence can be false

Removing "you met this in the Spanish track" means composing something in its place.
Three of my replacements asserted things the originals never had:

- *"the one European blue that is not a Germanic loan at all"* — Spanish's *azul* is
  Arabic too, and the corpus says so two files away.
- *"Latin's own vocabulary family for age and vigour"* — Latin has no such family.
  My *second* attempt, *"narrowed to force alone"*, was also wrong and contradicted
  the same lesson's body three lines down. The body already said it: the two words
  split toward different senses. **Read the rest of the file before rewriting a
  claim in its frontmatter.**
- *"the PIE-root/nox comparison is not independently attested for them"* — Kannada's
  and Telugu's *rātri* is the identical Sanskrit word, so it is attested exactly as
  well. The gap was in the CURRICULUM, not in the scholarship.

Two more turned up in a later review pass — a widening attributed to Malayalam when
the corpus names Malayalam as the counter-example, and a word said to *survive* in
its own ancestor language. All five have the same shape: a statement about **the books** rewritten as a
statement about **the world**. The pointer was the only thing that made the original
true, so deleting it silently widened the claim.

**Rule:** when the fix is a rewrite rather than a deletion, check the new sentence
against the corpus and against fact — not just against grammar. And prefer the
narrower true claim ("a second, separate road to blue") over the tidier absolute
("the one European blue"), because the absolute is the one that will be wrong.

**Corollary on guards:** a guard reading `canonicalLessonSource` is reading JSON, not
prose. Line breaks are the escape `\n`, so `\s+` cannot cross a wrap and `[^.?!\n]`
bounds nothing. Both of my newline-aware protections were inert on the surface I had
just added, and a real defect was sitting in the blind spot. Un-escape before matching
— and when a guard is extended to a new surface, re-prove it on THAT surface.


## A hedge you delete is a claim you widen

Removing "in this course" from *"Every European language in this course splits the
year into four seasons"* leaves a claim about European languages. The three words
were not decoration — they were the scope.

Two of my rewrites became false that way, in one pass, after I had already written
the lesson above about exactly this:

- *"Across Europe and North India, weekdays are named for planet-gods"* — Portuguese
  counts them (*segunda-feira*), as do Greek and every Slavic language. The original
  said "every language **so far in this course**": **two** hedges, and I dropped both
  at once. It also broke the lesson's own punchline, that Arabic is the odd one out
  for counting — Portuguese counts too.
- *"Most languages say 'my name is' with a word for my and a word for name"* — false
  for all of Romance, German, and Chinese.

**Rule:** when a scope-limiting phrase is deleted, the sentence needs a NEW scope
chosen on purpose — "most", "generally", or an explicit list — not the empty scope
that grammar leaves behind. Count the hedges in the original before you rewrite; if
there are two, the claim was fragile and needs more care, not less.

**Corollary:** moving an ordinal from course-scope to book-scope ("the third fate for
a consonant **in this book**") makes it *checkable*, which is the point — so check it.
Mine did not check out: the Punjabi volume never labels a first or second fate.


## Moving a claim in-volume makes it checkable — so check it

Rewriting "the course" as "this book" is usually right: the reader is holding the
book, so a claim about it can be answered. But it converts a vague claim into a
falsifiable one, and three of mine turned out false the moment they became checkable:

- *"three genuinely separate calendar traditions across this book"* — the Tamil book
  teaches one. Nine lines above, unchanged, sat the disproof: *"By now you've seen
  Arabic and Hindi each juggle two calendars."*
- *"closing this book on the root that opened it"* — chapter 4 of a 32-chapter book.
- *"a third fate for a consonant in this book"* — the Punjabi volume never labels a
  first or second.

**Rule:** an ordinal or count moved to book scope must be walked against that
volume's actual contents. "First", "third", "closing", "so far" are all claims about
a table of contents, and the table of contents is right there.

**Corollary — carry a guard's lesson to its new patterns.** My `earlier in this arc`
pattern already carried a comment explaining that one intervening adjective ("this
ENTIRE arc") defeats a pattern demanding the noun come straight after "this". I then
wrote six new patterns with the same flaw, and seven sites were hiding behind
"this **whole** curriculum" / "this **single** course". When a guard grows, re-read
the comments on the patterns already there — they are notes from the last person who
got it wrong, and that was me.

## Python file-rewrite helpers introduce CRLF on Windows, and it breaks content hashes

**Context:** `human-language-data`, PR #10068. CI failed `check:books` with "generated
output is missing or stale" on two chapters, while the identical check passed locally and
`git status` was clean.

**What happened:** I edit files with small Python helpers — `s = io.open(p, encoding='utf8')
.read()` … `io.open(p, 'w', encoding='utf8').write(s)`. On Windows that **write translates
`\n` into `\r\n`**. The curriculum's book/narration hashes are computed over file content,
line endings included, so:

- **Locally** the lesson Markdown was CRLF, the generator read CRLF, the hash was computed
  from CRLF, and the check compared CRLF to CRLF. Self-consistent, therefore green.
- **In CI** git checks out LF (the repo stores LF), the generator produced LF output, and
  the committed hash — computed from CRLF — no longer matched. Stale.

The mismatch is invisible to `git status` and `git diff`, because git normalises on the way
in. It is only visible to something that hashes bytes.

**The tell I ignored:** git printed `warning: in the working copy of '<file>', CRLF will be
replaced by LF the next time Git touches it` on *every single write*, across several
commits. I read a wall of repeated warnings as noise. It was the defect announcing itself in
plain language.

**Fix (three parts):**
1. When rewriting a file from Python, do it in **binary**: read `'rb'`, write `'wb'`, and
   keep the bytes you did not intend to change. Or pass `newline=''` to `io.open` in text
   mode, which disables translation on both read and write.
2. If a content hash or generated artifact is involved, **normalise before hashing** —
   `raw.replace(b'\r\n', b'\n')` — rather than trusting the editor's defaults.
3. **A gate that passes locally and fails in CI on "stale generated output" is almost always
   line endings or path separators, not logic.** Check `file <path>` for "CRLF line
   terminators" and compare `git show HEAD:<path> | file -` against the working tree before
   investigating anything else.

**Generalisation:** any check that compares *bytes* rather than *parsed structure* will
diverge between a CRLF working tree and an LF repository. That includes content hashes,
signature files, and golden-output tests. Prefer hashing normalised content, and never let
a helper script decide line endings for a file it did not create.

## Before authoring a lesson, read what the track already taught

I wrote a four-lesson B1 chapter for Spanish. Two of the four re-taught material the
book already owned, and I did not notice because I designed the chapter from the
spine node downward instead of from the corpus upward.

- The **imperfect** is taught in full at chapter 16, three irregulars and all, with
  the same Latin-endings etymology I "introduced" at chapter 38.
- **`luego`** is taught at chapter 5 inside *hasta luego*, with the same `locus`
  etymology, as the atoms `ES-LEX-LUEGO` and `ES-ETYMON-LOCUS` — which my new lesson
  re-introduced under new names, so `atomsTaught` counted the word twice.

Neither is visible from the spine. Both are one grep away.

**Rule:** before writing a lesson, grep the track for the headword AND for the
concept, and read any lesson that already covers either. Then write the lesson for
what is genuinely left — which is usually narrower, and better, than what you planned.
Chapter 38's imperfect lesson went from "here is a tense" (wrong, and 22 chapters
late) to "here is which of the two you already have to reach for" — the actual B1
skill, and a smaller, truer step.

**Corollary — a "you already own every word" claim is checkable, so check it.** The
payoff story used five words the course teaches nowhere, printed directly under the
sentence "every word is one you already own". The `forwardReferences` metric cannot
catch this: its blind spot is vocabulary the corpus never teaches at all. Diff the
story's words against the track's headword list by hand.

## A character no book has rendered before is invisible to every local gate (human-language books)

The round-two Hindi tranche passed tsc, 723 data-package tests, 725 app tests and all
five `check:` targets locally, then failed CI with:

    hindi missing_character rose to 1 against a baseline of 0

The books all COMPILED — `latexmk` exited 0 and wrote every PDF. What failed was
`scan_latex_log_warnings.py`, reading the `book.log` afterwards. The cause was one
character: **U+0325 COMBINING RING BELOW**, in the reconstructed PIE form *séh₂wl̥*
cited in the sun lesson. `\setmainfont{Latin Modern Roman}` has no glyph for it, so
XeLaTeX dropped it silently and logged one `Missing character` line.

**Why no local gate could catch it.** There is no LaTeX toolchain in the container, so
the only check that reads a `book.log` cannot run at all. Every gate that CAN run reads
the Markdown or the AST, and at that level the character is perfectly valid UTF-8 in a
perfectly valid sentence. The defect exists only in the font.

**The cheap pre-push check, which is also how it was found.** Diff the character set of
the .tex you just generated against the union of every .tex already in the repo:

```python
novel = chars(new_tex_files) - chars(all_existing_tex_files)
```

Anything in `novel` is a character no book has ever successfully rendered, so it is
exactly the candidate set for a missing glyph — two characters here, and the rare one
was the culprit. Run it against the LESSON MARKDOWN and you get the wrong answer: that
comparison missed U+0325 entirely, because the frontmatter was stripped and because
seven lessons in other tracks already use the character in prose **that no book
renders**. Compare the artifact the engine actually reads, not the source it came from.

**Corollary — "used elsewhere in the corpus" is not evidence a glyph renders.** I
cleared `ṓ` on exactly that reasoning (11 lessons, 8 tracks, all with baseline 0) and it
was sound for `ṓ`, which really is in the font. It would have been unsound for U+0325,
which appears in seven lesson files and zero rendered books. The question is never
whether the corpus contains the character; it is whether a BOOK has ever printed it.

**Fix in prose, not in the baseline.** The citation became "the Proto-Indo-European word
for the sun", keeping every cousin claim (*sōl*, *hḗlios*, *sun*) and dropping only the
reconstructed spelling. Raising `missing_character` to 1 would have blessed a character
the reader never sees — the glyph is dropped from the page, so the book silently prints
a different word than the source says.

## Verifying a chapter number in prose is the wrong instinct — the gate forbids the reference, not the staleness

Authoring the round-two Hindi tranche I wrote seven pointers back at earlier
material by number ("since chapter 37", "back in chapter 46"), and — following the
existing lesson that a claim moved to book scope must be checked — I looked every
one of them up in the corpus first. Two were wrong and I corrected them. I felt
careful. All seven then failed `chapter-references.test.ts`, which pins
cross-chapter prose references per track and holds hindi at 20; the tranche took it
to 27.

The point of HL-C102 is not that the number might be wrong today. It is that a
number **correct when written goes stale the next time a chapter splits**, silently,
pointing the reader into the wrong chapter with nothing failing. Spanish is pinned
at zero because Spanish is the track that actually renumbers. Verifying my numbers
made them accurate and left the debt exactly as toxic.

**Rule: never write "chapter N" as a cross-reference in lesson prose. Name the
thing** — "when the ear was named", "at the end of the welcome chapter", "when you
first counted to five". The test's own docstring says it: *the fix in prose is
never a fresher number.* Check for the gate before assuming the careful version of
a habit is the wanted one; here, being careful about the numbers was a more
polished way of doing the forbidden thing.

**Corollary — a blunt content detector can be right about a sentence that was
already wrong.** The same tranche had one lesson flagged `sight` on the cue "see
the", from *once you can see the seam in the middle*. The lesson teaches hearing a
morpheme boundary and its own drill line three lines below said **hear**. The
detector's comment explicitly warns against rewriting correct prose to appease it —
but this prose was not correct, and the flag found an internal contradiction that
reading had not. Check whether the sentence is right before deciding the detector
is wrong.

## A cousin list needs the ROUTE checked, not just the family

Chapter 41's etymologies were all in the right families and five were still wrong,
because a word can reach English from Latin by more than one road and the road
changes what is true:

- **`explicit`** and **`application`** were filed under "straight from Latin". Both
  came through French. The family was right; the column was not.
- **`comply`** was listed under `plicāre` "fold". It is from `complēre` "fill" — a
  different PIE root, disguised by a shared French verb ending. That is precisely the
  look-alike trap the lesson was written to teach.
- **`endeavour`** was said to have "reached English" from French. No French word
  `endeavour` ever existed; English **calqued** the phrase `mettre en devoir`.
- **`ad sīc`** was given as `así`'s etymon. It is the weakest of four proposals and
  the dictionary of record no longer prints it.
- **`*ḱred-` = heart** was stated flat. It is traditional and *disputed*: the heart
  root is `*ḱḗr`, never `*ḱred-`.

**Rule:** for every cousin, ask three questions, not one. Same root? Same *route*
(direct, through French, calqued, coined)? And is the etymon the *current* consensus
or a superseded proposal? A lesson that teaches learners to spot false friends has to
hold itself to the standard it is teaching.

**Corollary — check a new lesson against the chapters it names.** Chapter 41 said
`explicar` "takes the same shape as `contar`" — but chapter 38 teaches `contar` as a
stem-changer, and `explicar` is not one. A reader coming through in order would have
hit the contradiction and trusted the book a little less.

## A character can be present in one book's font and absent from another's — check the GENERATED .tex against the actual font file (human-language books)

**Context:** HL-C212, the Latin pre-A1 vocabulary tranche. 744 local tests green,
security review clean, pushed. CI failed 11 minutes later with a single line:

    latin missing_character rose to 2 against a baseline of 0

**What happened:** the Old English form of *egg* was written *ǣg* — U+01E3 LATIN
SMALL LETTER AE WITH MACRON — and Latin Modern, which the Latin book sets as its
main font, does not have that glyph. Nor U+01E2, its capital. Exactly two
characters, exactly the two the scanner counted.

**Why no local gate caught it.** Every local check reads the corpus, the schema,
the ramp and the manifests. **None of them opens a font.** Glyph coverage is only
discovered when XeLaTeX runs, which happens in CI, which is 11 minutes away.

**The instrument that would have been wrong.** My first instinct was that the
offender was `ǵ` (U+01F5, in the PIE reconstruction *\*ǵʰórtos*) — it looks far
more exotic. I checked instead of guessing, by loading the actual `.otf` with
`fontTools` and querying its cmap, and **`ǵ` is present**. The guess would have
sent me to rewrite the wrong lesson and left the failure in place.

**And the second wrong instrument, caught the same way.** Scanning the LESSON
SOURCES for characters outside the font reported four more offenders — `ʰ`, `₁`,
`₂`, `ḗ`. All four are false: the book generator turns them into
`\textsuperscript{h}` and `\textsubscript{2}` and decomposes the rest, so they
never reach the `.tex` at all. **The artifact to check is the GENERATED .tex,
because that is what the typesetter reads** — checking the source measures a file
nothing compiles.

**The check, worth running before any content PR that adds an unusual character:**

```python
from fontTools.ttLib import TTFont
font = TTFont(".../lmroman10-regular.otf")
cmap = set()
for t in font["cmap"].tables: cmap |= set(t.cmap.keys())
assert ord("a") in cmap and ord("好") not in cmap   # self-test, both directions
missing = {c for p in book.rglob("*.tex") for c in p.read_text()
           if ord(c) > 127 and ord(c) not in cmap}
```

Self-test it in **both** directions before trusting a clean result — a cmap that
loaded empty reports every character missing, and one that loaded wrong reports
none.

**Generalisation:** per-track fonts mean per-track repertoires. The Indic and
Arabic books load vendored Noto faces with wide coverage; the Latin, Spanish and
other Latin-script books load Latin Modern, which is a **typesetter's** font and
is much narrower outside western European orthography. A character that has
rendered fine in twenty books can still be missing from the twenty-first. Cross-
refs the existing lesson about a character no book has rendered before.

**Cheapest content fix:** use the citation form that stays inside the font.
Old English *æg* (plain ash) is a standard rendering of the same word and is
present in every Latin-script face in the repo.

## The font a ROMANIZATION renders in is not the font the script renders in — scope the glyph probe to the whole page, not to the script (human-language books)

**Context:** HL-C222, the Bengali script tranche. HL-C214 had been written ONE
TRANCHE EARLIER, saying in so many words: check the generated `.tex` against the
actual font file. I did. CI still failed:

    bengali: missing_character rose to 29 against a baseline of 0

**What happened:** the lessons romanise Bengali's inherent vowel as `nɔ`, `kɔ`,
`mɔ` — U+0254 LATIN SMALL LETTER OPEN O, the correct IPA. Bengali's book sets
`\setmainfont{Latin Modern Roman}`, and **Latin Modern does not have U+0254**.
Twenty-nine occurrences, exactly the number CI counted.

**Why the probe missed it, and this is the whole lesson.** I scanned
`bengali/book/**/*.tex` for codepoints **in the Bengali range** against
**NotoSansBengali**. Both halves of that are too narrow:

- the Bengali characters were never the risk — they are inside `\bn{...}`, which
  selects the Noto face that was chosen *because* it covers them;
- **everything else on the page — the prose, the romanization, the IPA — renders
  in the MAIN font**, and nothing was checking it.

A book is not one font. Scope the probe to **every non-ASCII character not inside
a script wrapper, against the main font**, and separately to the wrapped runs
against their own face.

```python
text = re.sub(r"\\bn\{(?:[^{}]|\{[^{}]*\})*\}", "", tex)   # strip wrapped runs
missing = {c for c in text if ord(c) > 127 and ord(c) not in latin_modern_cmap}
```

Note the nested-brace form of that regex: `\bn{\textbf{x}}` is common and a
`[^{}]*` version silently fails to strip it, which puts Bengali characters into
the main-font bucket and produces a flood of false positives.

**And a second instrument failure inside the same investigation.** My first pass
scanned `bengali/book/` **while checked out on a different branch** — the Hindi
one, which has no Bengali chapter — and reported the five pre-existing preamble
characters as if they were the answer. Same family as `git stash` not stashing
untracked files: *the tool ran perfectly against the wrong tree.* **Print
`git branch --show-current` in the same command as any cross-branch diagnosis.**

**Rule of thumb for this corpus:** the Latin-script books (Spanish, Latin, French…)
have only Latin Modern, which is a **typesetter's** font — good Western European
coverage, thin outside it. It lacks `ǣ` (HL-C214) and `ɔ` (this one), and it will
lack the next IPA symbol somebody reaches for. If a romanization needs a phonetic
character, check it before writing 29 of them; `ô` (U+00F4) and `ə` (U+0259) are
present and usually say what is needed.

## A CI gate keyed on a package name string can go stale silently when the classifier improves (dependabot/CodeQL alert fix PR)

**Context:** PR #11876 (5x extract-zip, 5x js-yaml, 2x CodeQL alert fixes). The
diff touched exactly one file outside `code/programs/go/unix-tools`: a JS test
under `code/programs/mosaic/venture-browser/host/web/`. CI's
`build (ubuntu-latest)` job failed with:

    error: failed to run custom build command for `cairo-sys-rs v0.20.10`
    Package cairo was not found in the pkg-config search path.

**What happened:** the "Build and test affected packages" step is driven
directly by the go build-tool's dependency graph, which correctly marked the
whole `mosaic/programs/venture-browser` package (a single Starlark BUILD unit
covering `host/web/*.js` alongside its Rust workspace) as affected and ran
`cargo test` on it — which needs `libcairo2-dev`. But the *separate* workflow
step that installs `libcairo2-dev` doesn't key off the build plan directly; it
keys off a `needs_venture_windows` flag computed by
`code/scripts/venture_windows_ci_acceptance.py`, which checks the affected
package list against a hardcoded `ACCEPTANCE_PACKAGES` set containing
`"unknown/programs/venture-browser"`. The build-tool, at some point, learned to
classify `.mil/.mll/.msl`-based Mosaic packages under `language: "mosaic"`
instead of the generic `"unknown"` fallback — so the real affected-package name
had become `"mosaic/programs/venture-browser"`. The acceptance script's set was
never updated, silently stopped matching, and the cairo install step got
skipped every time — undetected because no PR since the reclassification had
touched *only* venture-browser's non-Rust files without also touching Rust
elsewhere (which would have triggered `needs_rust` independently and installed
cairo anyway, masking the gap).

**Diagnosis path that worked:** don't trust the coarse `needs_*` boolean in
isolation — reproduce the build plan locally (`build-tool -diff-base
origin/main -detect-languages -emit-plan plan.json`) and grep it for the
package in question to see its *actual* current name, then diff that against
whatever string the gating script hardcodes.

    python3 -c "
    import json
    d = json.load(open('plan.json'))
    for p in d['packages']:
        if 'venture' in p['name'].lower():
            print(p['name'], p['language'])
    "
    # -> mosaic/programs/venture-browser | mosaic   (script expected "unknown/programs/venture-browser")

**Generalisation:** any CI gate that pattern-matches a package-classifier's
output by a hardcoded string (not by re-deriving it from the current
classifier) will silently rot when the classifier gets more precise. Five
sibling scripts (`mosaic_swift_runtime_ci_acceptance.py`,
`mosaic_compose_runtime_ci_acceptance.py`,
`mosaic_flutter_runtime_ci_acceptance.py`,
`mosaic_xaml_windows_ci_acceptance.py`, `mosaic_qt_runtime_ci_acceptance.py`)
hardcode the same stale `"unknown/programs/task-app"` and are presumed to have
the identical latent gap — flagged as a follow-up rather than bundled into an
unrelated security-alert PR, since fixing them isn't required to unblock this
one and touches five extra files with their own test fixtures.

## Verifying a package by running its tests can miss the check that actually fails CI

**What happened (HL-C242).** Chinese chapter 7 was verified locally with
`npx vitest run` in all three packages that read `human-languages`, plus the
five `check:*` gates. All green. CI then failed on `language-ladder` with:

    bundle check: largest eager chunk is 500087 bytes (limit 500000)

Eighty-seven bytes over, on a chapter that added five lessons.

**Why local verification could never have caught it.** `scripts/check-bundle.mjs`
is not a vitest test. It runs in the package's BUILD, after `vite build`, and
reads `dist/`. No amount of running the test suite reaches it. The script even
guards against a stale `dist/` — it refuses to report if any source is newer
than the built `index.html` — precisely because a measurement that cannot move
reads as evidence.

**Rule:** for a package whose BUILD does more than compile — bundle budgets,
size gates, generated-artifact checks — run the build, not just the tests:

    npm run build && node scripts/check-bundle.mjs

## `grep "Tests "` on vitest output hides whole test FILES failing

Same incident. Verification was scripted as
`npx vitest run 2>&1 | grep -E "Tests "`, which prints:

    Tests  243 passed (243)

and looks clean. The line above it read `Test Files  7 failed | 27 passed (34)`.
Seven files had failed to *load* — so their tests never ran, never failed, and
never appeared in the count that was being read. A run reporting "243 passed"
was actually a broken run, and the real suite is 727.

**Rule:** grep `"Test Files|Tests "`, never `"Tests "` alone. A collection error
is invisible in the passing-test count by construction.

## Splitting a chunk to satisfy a "largest chunk" gate is gaming the metric

The first fix for the bundle failure gave the oversized `curriculum-plans` group
a `maxSize` so it split into four chunks, each under the ceiling. The gate went
green. It was the wrong fix, and `main` landed the right one concurrently:
make the per-track plans lazy so the bytes leave the preload set entirely.

The tell was in the commit message that shipped it — it conceded that "the bytes
are eager either way, so first paint downloads the same total" and argued the
cacheability gain made it acceptable. A gate measuring the LARGEST eager chunk
cannot see four chunks totalling the same half-megabyte. HL-C110 had written
this exact trade down in advance as gaming the metric.

**Generalisation:** when a budget is expressed as a max over a partition, any fix
that changes the partition rather than the total satisfies the gate without
buying the thing the gate exists to protect. Ask what the gate is a proxy for —
here, bytes before first paint — and move that number.

## Local Unix-socket agent (vault-pm VLT-PM48): three non-obvious hazards

Building a permission-checked local-agent IPC transport (Unix domain socket
under a per-user runtime directory, `SO_PEERCRED`/`getpeereid` peer
verification, a detached long-lived process spawned from a one-shot CLI)
surfaced three bugs that were each surprising in isolation and easy to miss
in review.

1. **A generic "walk every ancestor with `O_NOFOLLOW`" private-directory
   helper is the wrong tool for a path rooted at the *system* temp
   directory.** On macOS both `/tmp` and `/var` are themselves
   platform-placed symlinks (`/tmp -> private/tmp`). A helper that resolves
   `/`, then `var`, then `folders`, ... with `O_NOFOLLOW` at every step
   rejects the walk with "unsafe object type" before it ever reaches the
   directory the code actually owns — even though nothing attacker-controlled
   is involved. Fix: trust the *parent* (opened via ordinary path resolution,
   no `O_NOFOLLOW`) and defend only the *leaf* directory this crate itself
   creates, with `O_NOFOLLOW` + owner/mode verification on that one component.
   General rule: a "walk-and-verify-every-ancestor" directory helper is
   correct only for a root the crate owns end to end; a root anchored under a
   platform-managed directory (system temp, `/var/run`, etc.) needs a
   leaf-only variant, and the platform ancestry is trusted the same way
   `ProjectDirs`' own resolved roots already are.

2. **A bare `connect()` succeeding is not proof a Unix-domain-socket server
   is actually serving requests.** A listener that has stopped calling
   `accept()` (e.g. mid-shutdown, before its process has fully exited) can
   still have a connection sit in its kernel backlog long enough for a
   `connect()` from a *different* process to succeed. A "is another instance
   already running" check built on bare `connect().is_ok()` intermittently
   misreports a dying server as live — reproduced as a real, non-flaky-once
   race between `agent stop` immediately followed by `agent start`, and
   independently as CI-parallelism-sensitive flake in a test that bound,
   dropped, and immediately rebound a raw `UnixListener` in the same
   process (passed alone, ~20% failure rate under `cargo test`'s default
   thread count). Fix: the liveness check must be a real bounded
   request/response round trip (send `Ping`, wait for `Ok` with a timeout),
   never a bare connect.

3. **An "opportunistically reuse a cached credential" feature and a
   "sensitive command should always re-collect its own credential fresh"
   feature will silently interact if the second command's collection path
   is not explicitly exempted from the first's reuse seam.** Wiring
   `passphrase rotate`'s *current*-passphrase prompt through the same
   opportunistic-agent-reuse helper every other authenticated command uses
   made rotation silently skip its first prompt whenever a long-lived agent
   already had that vault unlocked — an E2E test that scripted rotate's two
   expected prompts then hung for the test suite's full 60-second read
   timeout waiting for a prompt that opportunistic reuse had made not
   happen. The fix was a deliberate carve-out, not a protocol change:
   `passphrase_rotate` never consults the cache, matching this codebase's
   pre-existing "the interactive shell refuses to delegate `passphrase
   rotate` at all" rule for the same underlying reason (a rotation's whole
   point is proving fresh knowledge of the current secret). General rule:
   before wiring a new "skip re-collecting this value" seam through an
   existing multi-prompt/multi-step command, check whether any step of that
   command is a *deliberate* re-confirmation ceremony rather than an
   ordinary unlock, and audit the E2E test itself when a PTY-based test
   hangs at a `read_until` for a prompt — a hang there just as often means
   "the prompt legitimately stopped happening" as "the process is stuck."

## A struct literal inside a platform `cfg` gate is invisible to the other platforms' compilers (cowsay, PR #12168)

`rust/programs/cowsay` failed only on `build (macos-latest)` with

```
error[E0063]: missing field `wrap` in initializer of `layout_ir::TextContent`
```

`layout-ir` had gained a `wrap: bool` field on `TextContent`. Every other
construction site in the repo was updated at the time; the one that was missed
sat inside `#[cfg(target_vendor = "apple")] fn render_cowsay_png_metal`.

The generalisable point is that this is **not** the usual cross-platform bug.
It has nothing to do with path separators, line endings, locale, filesystem
case-sensitivity, `/tmp` vs `$TMPDIR`, terminal width, or Unicode — the list
you reach for when a job is red on one OS only. Those all describe code that
*runs* differently. Here the code did not *exist* for two of the three
compilers, so no amount of correctness on Linux or Windows could say anything
about it. When a **compile** error (rather than a test failure) is
platform-specific, suspect a `cfg` gate before suspecting behaviour. The clue
was in the timing: `FAILED 12.2s` means it compiled and stopped, not that a
long build ran or a test executed.

Three consequences worth internalising:

1. **Grepping for construction sites is necessary but not sufficient — grep
   for gated ones specifically.** The existing lesson "grep every consumer for
   record construction, not just pattern matches" (Haskell section) assumes
   your compiler will show you the consumers. It will not show you the ones
   behind a `cfg` for a platform you are not building. After changing a shared
   struct, run `grep -rn "cfg(target_" --include=*.rs` across the consumers you
   found and check each gated site against the target it is gated to.

2. **Prefer un-gating the data from the platform code.** The durable fix was
   not filling in the field, it was moving the struct literal out of the gated
   function into an un-gated helper — `layout_ir` is pure data, and only the
   *rendering* needed Metal/CoreText. All three legs then type-check the
   literal, so the next field addition fails on Ubuntu in minutes instead of on
   macOS in months. Gate the code that genuinely cannot compile elsewhere, and
   no more than that. `#[cfg_attr(not(target_vendor = "apple"),
   allow(dead_code))]` handles the resulting dead-code warning — and note that
   a lint allowance does **not** suppress type checking, which is exactly why
   this works.

3. **You can verify an Apple-gated path from Windows or Linux.** `cargo check`
   type-checks without linking, so `rustup target add aarch64-apple-darwin`
   followed by `cargo check --all-targets --target aarch64-apple-darwin`
   compiles the gated body with no Mac involved. This works whenever no
   dependency has a `build.rs` driving the `cc` crate. Do the same for
   `cargo clippy --target <t> --all-targets -- -D warnings`, once per target.

   **Always pair it with a control run.** Restore the pre-fix file
   (`git checkout origin/main -- <path>`, having first copied your version to
   the scratchpad — `git stash` is unsafe in these shared worktrees) and
   confirm the cross-check reproduces CI's exact error at the exact line, then
   restore your version. A check that cannot fail proves nothing; the control
   is what turns a clean run into evidence.

Finally, the reason it rotted at all: cowsay had **no `BUILD` file**, so the
build tool never discovered it and no CI leg ever compiled it. A `cfg`-gated
literal inside an unwatched package is doubly invisible. Before concluding "CI
is green, so this package is fine", confirm the package is actually in the
build graph — `ls <pkg>/BUILD`.
## A fixed-once-per-package fix doesn't reach a fallback path — an absent `BUILD_windows` still runs the POSIX `BUILD` verbatim on Windows

PR #11553 (Swift ZIP) repaired the Windows dependency-closure detection so far more
packages actually get *evaluated* on `windows-latest` than before. That correctly
exposed 28 packages that were never Windows-tested until this PR, even though the
`.[dev]`-quoting bug they hit (lesson at line 31) has been documented for a long time.

- **`GetBuildFileForPlatform` (`code/programs/go/build-tool/internal/discovery/discovery.go`)
  falls back to the generic POSIX `BUILD` file when no `BUILD_windows` exists — for
  EVERY platform, including Windows.** There is no "this package doesn't support
  Windows" opt-out mechanism anywhere in the build tool (no marker file, no
  platform field). A package with only a `BUILD` file is not "skipped on Windows" —
  its POSIX script is handed to `cmd /C` verbatim. `#!/bin/sh` / `set -eu` headers,
  `if [ "$(uname)" = "Linux" ]; then ...; fi` guards, and quoted `-e ".[dev]"` all
  fail loudly (`Environment variable -eu not defined`, `"$(uname)" was unexpected at
  this time.`, `not a valid editable requirement`) the first time such a package
  actually gets exercised on Windows.
- **A known, documented fix (line 31) does not self-propagate.** 21 packages had the
  exact `-e ".[dev]"` quoting bug the lesson already named, either because they never
  had a `BUILD_windows` at all, or because their hand-authored `BUILD_windows`
  predated/diverged from `code/programs/python/scaffold-generator/scaffold_generator.py`
  (which has always emitted the correct unquoted form). Two packages
  (`prolog-core`, `swi-prolog-lexer`) used `.venv\Scripts\python -m pip install`
  instead of `uv pip install` — a second hand-authored variant of the same bug.
  **Fix:** unquoted `-e .[dev]` — copy an existing multi-dep `BUILD_windows` (e.g.
  `nib-parser`) as the template rather than re-deriving it.
- **A POSIX-only guard line (`if [ "$(uname)" = "Linux" ]; then cargo tarpaulin ...;
  fi`) is not "harmless to skip" on Windows — it is a syntax error there.** `cmd.exe`
  has no `[ ]`/`$()` and reads `$(uname)` as a literal command name. Fix: give the
  package a `BUILD_windows` that drops the tarpaulin line entirely (tarpaulin is
  Linux-only anyway) — see `bytecode-compiler/BUILD_windows` for the established
  one-line precedent.
- **When newly exposing a previously-untested platform for dozens of packages at
  once, budget review time per failure, not just per PR.** Six inherited defects
  were fixed before this head; 28 were still failing after — because "expose more of
  the closure" and "fix everything the closure exposes" are different amounts of
  work, and the former doesn't bound the latter.

## A non-recursive function can still overflow a 1 MiB Windows test-thread stack — debug builds don't reuse stack slots across `match` arms

`chief-of-staff-smart-home-tools` aborted on `windows-latest` with
`0xC00000FD`/`STATUS_STACK_OVERFLOW` in a two-call, non-recursive integration test.
The existing stack-overflow lessons (line ~1980, ~2557) are all about *recursive*
walkers whose frame gets multiplied by depth. This one had no recursion at all.

**Cause:** the tool dispatcher is one `match tool_id.as_str() { ... }` with 364 arms,
each declaring its own small `let query = ...;` local before calling a handler
function. In a debug build, rustc does not reuse stack slots across mutually
exclusive `match` arms (same mechanism as the recursive-eval lessons, just applied
to breadth instead of depth) — so the function's single stack frame sums roughly
364 arms' worth of locals. That is comfortably under macOS/Linux's ~8 MiB default
test-thread stack but over Windows' ~1 MiB floor. Reproduced locally without a
Windows box: build the test binary, then run it directly (not via `cargo test`,
which also applies `RUST_MIN_STACK` to `rustc` itself and breaks compilation) with
`RUST_MIN_STACK=1048576 ./target/debug/deps/<binary> tests::<name> --exact` —
overflowed at 1 MiB, passed cleanly at 2 MiB.

**Fix, and why raising the stack here is correct (not a band-aid):** the earlier
recursive-`eval` lesson fixed depth×frame-size by shrinking the frame, because a
depth cap that keeps growing would eventually re-overflow any fixed stack. Here
there is no depth to bound — the frame size is fixed regardless of input, so
widening the stack for this one test binary is a proportionate, permanent fix, not
a deferral. Set it in `BUILD_windows` (not committed to `Cargo.toml` or a global
CI env var, since only this package's frame is oversized):
`set "RUST_MIN_STACK=4194304" && cargo test -p <pkg> && cargo clippy -p <pkg> --all-targets -- -D warnings`.
Chose 4 MiB (2x the proven-sufficient 2 MiB) for margin. `set "VAR=value" &&
command` (not bare `VAR=value command`) is required — see the existing Windows
env-var lesson (line 35); each `BUILD_windows` line runs as its own `cmd /C`
process, so the `set` and the command it configures must be chained with `&&` on
one line.

## `Path.write_text(...)` without `newline=""` silently adds a byte on Windows and breaks exact-size assertions

`logic-builtins`' `test_file_path_metadata_facade` asserted a Prolog `size_fileo/2`
builtin returned `len("fact(a).\n")` (9) for a file it had just written with
`source_path.write_text("fact(a).\n", encoding="utf-8")`. It got 10 on Windows.

**Cause:** `Path.write_text` defaults to platform newline translation unless
`newline=""` is passed (the exact mechanism documented at the CRLF lesson,
line ~3121, but there for a content-generation script — here it silently
corrupted a test's own fixture size instead of a hash). `\n` became `\r\n` on
write, so the file the test created was one byte longer than the string literal's
`len()`, and the byte-count assertion — computed from the string, not the file —
went stale relative to what was actually on disk.

**Fix:** `source_path.write_text("fact(a).\n", encoding="utf-8", newline="")`
whenever a test both writes a fixture file with embedded `\n` AND later asserts
something about that file's exact byte size (not just its content). Content-only
assertions (`read_text()` back and compare) are unaffected because Python
normalizes `\r\n` back to `\n` on text-mode read — only a *size*/byte-count
assertion computed from the pre-write string is at risk. The general write-side
guidance from line 3145 (open in binary, or pass `newline=''`) applies to test
fixtures exactly as much as to generated production files.

## A test's own `#[cfg(not(target_os = "windows"))]` guard can be stale evidence, not a platform limitation

`embeddable-http-server`'s `sharded_http_server_serves_concurrent_clients_across_shards`
test — the ONLY caller of the `#[cfg(target_os = "windows")]` variant of
`bind_native_sharded_http_server` — was itself gated `#[cfg(not(target_os =
"windows"))]`. Once the build tool started actually building this package on
Windows (see the BUILD-closure lesson above), that made the Windows
`bind_native_sharded_http_server` dead code under `-D warnings`
(`function 'bind_native_sharded_http_server' is never used`).

**Investigation, not just suppression:** `#[allow(dead_code)]` would have silenced
this without checking whether the underlying claim (sharded Windows HTTP serving
doesn't work / isn't tested) was still true. It wasn't: `bind_windows_sharded`
(`ShardedHttpServer<WindowsTransportPlatform>`, IOCP-backed) is a complete,
real implementation sitting right next to the Unix ones, not a stub. Two sibling
tests in the same file (`mailbox_http_server_*`) ARE legitimately Windows-excluded
because `MailboxHttpServer` has no Windows reactor at all — so the sharded test's
exclusion reads, at a glance, like it follows the same precedent, but it doesn't:
different underlying type, different actual platform support.

**Fix:** remove the `#[cfg(not(target_os = "windows"))]` from the sharded test so
it runs everywhere the implementation exists — verified with
`cargo check --target x86_64-pc-windows-gnu --tests` under `RUSTFLAGS="-D warnings"`
(no Windows box needed for a cross-compile check; only actually *running* the test
needs one, which real Windows CI will do on push). **Lesson:** when a `-D warnings`
lint fires on a `#[cfg(target_os = "windows")]` item, check whether its only caller
is excluded from Windows before reaching for `allow` — the cfg may be describing a
gap that was since closed, not a permanent constraint.

**Correction (same PR, next CI round):** this was wrong, and the wrongness was
only reachable by actually running on Windows — `cargo check --target
x86_64-pc-windows-gnu` type-checks but never *executes* `bind_windows_sharded`,
so it happily passed while the runtime path was broken. Real windows-latest CI
panicked: `"SO_REUSEPORT is not supported by the Windows TCP provider"`. Tracing
into `tcp_runtime::bind_sharded_runtime_with_state`, the "default" sharding path
(used by every platform except macOS/BSD) sets `reuse_port = true` for any
`worker_count > 1` — correct for Linux (where `SO_REUSEPORT` load-balances) but
fatal for Windows (no `SO_REUSEPORT` at all). macOS/BSD hit the identical
"`SO_REUSEPORT` exists but doesn't load-balance" problem and already fixed it
with an explicit accept fan-out (`FanoutAcceptor`) — but that fan-out's
`spawn`/`run` are `#[cfg(unix)]`-only, so Windows has no route around the
`reuse_port` panic today. `bind_windows_sharded` is also a **real, currently
broken for `worker_count > 1`** part of the public API — `web-core`'s
`ShardedWebServer` and `conduit` both wrap it — but neither is exercised on
Windows CI yet either, so this was a pre-existing gap this PR's build-plan fix
merely exposed here first, not something this PR introduced or should silently
paper over. Reverted the test back to `#[cfg(not(target_os = "windows"))]`, kept
the Windows-only dispatcher function alive with a `#[allow(dead_code)]` whose
comment states exactly why (a rare case where the suppression IS the honest
answer — see next lesson) and what would need to exist (a Windows fan-out
acceptor in `tcp-runtime`) to remove it.

**Generalized lesson:** "verified with a cross-compile check" only proves the
code *type-checks* for that target — it does not execute a single line of
target-specific runtime logic (socket options, syscalls, ABI-dependent
behavior). Treat a cross-compile check as ruling out compile errors only, never
as confirmation that a previously-excluded-from-Windows test will actually pass
there. When you can't run the real target and the round-trip to find out is
expensive (~80 minutes of CI here), say so plainly in the commit/PR rather than
implying the cross-compile check was sufficient evidence.

## Sometimes the honest fix after investigation IS the suppression you started with — don't force a "real" fix past what the evidence supports

Follow-on to the `embeddable-http-server` correction above. The instinct from
"investigate before suppressing" is to keep digging until you find a positive
fix. But investigation can just as validly conclude "the exclusion was correct,
the capability really is incomplete, and fixing it properly is out of scope for
this change" — and forcing a deeper fix at that point (e.g. hand-rolling a
Windows fan-out acceptor inside a CI-rescue with no way to test it before a
slow, blind CI round trip) trades a well-understood, honestly-labeled gap for
an *unverified* one. The deliverable of "investigate, don't just suppress" is
not always a positive change — sometimes it's a suppression with a comment that
correctly explains the boundary of what was and wasn't fixed, and a decision
about scope that a human reading the commit message can evaluate and revisit.

## The SAME "`Path::is_absolute()`/`is_symlink()` is platform-dependent" bug class recurs across unrelated crates — check by INTENT, not by pattern-matching the previous fix

Three different Windows CI failures in one PR rescue all trace back to
`std::path::Path`'s platform-dependent behavior, but needed **three different
fixes** because the code's *intent* differed each time:

1. **`sql-parser`** (`grammar_path.read_text()`): platform default *encoding*
   (cp1252 on Windows) silently misreads UTF-8 bytes. Fix: pin
   `encoding="utf-8"` explicitly — the file's encoding is a property of the
   file, never the host platform.
2. **`chief-of-staff-daemon-service-files`** (`validate_unix_path`): the
   function validates paths for a launchd/systemd config being rendered for
   **macOS/Linux, regardless of the host compiling and testing this crate**.
   `Path::is_absolute()` uses the *build host's* semantics, which is wrong
   here on principle, not just on Windows — the fix (`value.starts_with('/')`)
   hardcodes POSIX semantics because the target is always POSIX.
3. **`chief-of-staff-daemon-config`** (`ConfigPath::resolve`'s `home` param):
   this validates the **actual runtime host's** home directory — a real
   Windows deployment legitimately passes a `C:\Users\...` path. Here
   `Path::is_absolute()` is *correct as written* (there's already a
   `#[cfg(windows)] fn absolute_home()` test helper proving the crate expects
   platform-native paths); the bug was three tests hardcoding a POSIX path
   directly instead of calling that helper, like every other test in the file
   already did.

**Before reaching for the fix pattern that worked last time on `is_absolute()`
or `is_symlink()`, ask: is this code validating "a path meaningful on THIS host
right now" (use platform-native `Path` semantics — case 3) or "a path meaningful
on some OTHER, possibly-fixed target regardless of build host" (use an explicit,
target-specific string check — case 2)?** Getting this backwards either breaks
real Windows deployments (over-hardcoding POSIX) or reintroduces the original
bug (using native semantics for a fixed-target validator). Grep for an existing
`#[cfg(windows)]` test helper near the failing assertion before writing a new
fix — case 3's `absolute_home()` was sitting three functions away and would
have caught the real bug (a test authoring slip) immediately instead of
requiring a `Path::is_absolute()` investigation that turned out to be a red
herring.

## `os.remove()`/`os.replace()` on a file another handle still has open: fine on POSIX, `PermissionError`/`WinError 32` on Windows — check every early-return path, not just the obvious one

`storage-sqlite`'s `Pager._recover()` opened the journal file for reading
(`with open(self._journal_path, "rb") as j:`) and, on two of its three exit
paths, called `self._drop_journal()` (an `os.remove()`) from **inside** that
`with` block — while `j` was still open. POSIX unlinks an open file just fine
(the inode survives until the last handle closes); Windows refuses
(`PermissionError: [WinError 32] ... being used by another process`).

The third exit path (successful replay) already called `_drop_journal()`
**after** the `with` block closed `j`, so it never had this bug — which is
exactly what made the two early-return call sites easy to miss on a read-through
that only checks "does this function eventually close its handles," rather than
"does *every* removal of a path happen after every handle to that path is
closed." **When auditing a function for this class of bug, don't stop at
finding one instance that does it correctly — the correct and incorrect
versions can coexist in the same function, on different branches, and a partial
read makes the whole function look safe.**

**Fix:** collapse the three early-return branches into one control-flow path
that always exits the `with` block before removing the file — a `header = None`
sentinel plus a combined boolean condition, rather than three separate
`self._drop_journal(); return` call sites. One removal call site, reached after
every branch, is easier to audit than three copies of the same call scattered
across early returns — and this class of bug is specifically about a removal
call's *position relative to a `with` block*, so collapsing to one call site is
the fix, not just a refactor.

## A documented, non-fully-TOCTOU-proof fallback can still be the right call — say why in the code, not just that it's imperfect

`ir-to-jvm-class-file`'s POSIX symlink defense (`os.open(..., dir_fd=...)`
chained through every path component) has no Windows equivalent — `os.open()`
can't even open a bare directory there. The Windows fallback
(`Path.is_symlink()` checks before `mkdir`/write) has a real TOCTOU gap: a
symlink swapped in between the check and the use isn't caught, unlike the
atomic dir_fd chain.

Two things made this an acceptable ship, not a deferred vulnerability:

1. **Close the cheap half of the gap.** The final file write went through a
   temp file + `os.replace()` instead of opening the checked path directly —
   `os.replace()`/`rename(2)` replace the directory entry itself rather than
   following a symlink placed there afterward, so that specific race is fully
   closed for near-zero extra code. The intermediate-directory `mkdir` race is
   harder to close without a dir_fd equivalent and was left as documented,
   accepted risk rather than force-fixed.
2. **State the actual threat model in the comment, not just "not fully safe."**
   `class_filename` is compiler-internal input already validated (no absolute
   paths, no `.`/`..`) before either write path runs, so the residual race
   requires an attacker who already has write access to the output tree,
   racing a local build — not a remote/network-facing threat. A reviewer (or a
   future security audit) reading "not fully TOCTOU-proof" alone has to
   re-derive this; reading it inline saves that re-derivation and makes the
   accepted-risk decision auditable instead of just asserted.

**Correction, one CI round later:** point 2's premise — "already validated,
no absolute paths" — was itself broken on Windows by the SAME `Path.is_absolute()`
platform-dependence bug documented two lessons up, just in a fourth location
this time: `_validated_output_relative_path()` used `Path(class_filename).is_absolute()`
to reject `class_name=".Escape"` → `class_filename="/Escape.class"`. That
check passes (correctly rejects) on POSIX and silently **passed the file
through unrejected on Windows** (`PureWindowsPath("/Escape.class").is_absolute()`
is `False` — no drive/UNC prefix). `class_filename` is always POSIX-separated
(built via `class_name.replace(".", "/")`, see `JVMClassArtifact.class_filename`),
so the fix is the same pattern as the Rust cases: check `class_filename.startswith("/")`
directly instead of asking the platform-native `Path` what it thinks
"absolute" means. **The generalized lesson from three lessons up undersold its
own stakes**: this wasn't just "pick the right fix for the code's intent" in
the abstract — a stale platform-dependent validator can invalidate a security
argument that was written assuming the validator worked. When accepting a
residual risk *because* an earlier check already narrows the threat model,
verify that check cross-platform before relying on it, not just on the
platform you happen to be testing on.

**Second correction, before push this time (security review, not CI):** the
`class_filename.startswith("/")` fix above was itself an incomplete swap, not
a complete one — it closed the reported gap (POSIX-style `/...` under-rejected
on Windows) but reopened a *different* one the original `Path.is_absolute()`
had actually caught when the validator happened to run in Windows CI:
`class_name` is adversarial, unrestricted, attacker-supplied input (the test
suite constructs it deliberately), so nothing stops it from containing `:`
or `\` — neither of which `.replace(".", "/")` touches. `startswith("/")`
alone does not reject `"C:\\evil.class"`, `"C:/evil.class"`, or
`"\\\\server\\share\\evil.class"` on ANY host platform (not just Windows —
these were never rejected when the validator ran on POSIX either, since
POSIX's native `Path.is_absolute()` doesn't recognize Windows drive/UNC
syntax at all; the original code only caught them by accident, when the
validator happened to execute on an actual Windows host). **Correct fix:**
check both conventions explicitly and platform-independently — `PurePosixPath(x).is_absolute()
or PureWindowsPath(x).is_absolute()` — plus reject any `":"` outright to catch
drive-relative paths (`"C:evil.class"`, absolute under neither convention,
also never caught by the original code). `PurePosixPath`/`PureWindowsPath`
(unlike bare `Path`) are always available and always mean the same thing
regardless of host OS — exactly the tool this whole bug class was missing.
**Lesson: when a security check depends on "is this path absolute," check
it means "absolute under every convention the input could plausibly use,"
not "absolute under the one convention this function's `Path` happens to
resolve to on whichever host runs it."** A narrow platform-specific patch for
a reported failure is exactly the moment to ask what the *general* property
being checked for is, not just what makes the one failing test pass.

## A quoted `set "VAR=value" && command` in `BUILD_windows` can silently fail to set the variable — and the failure is invisible unless something downstream crashes

`chief-of-staff-smart-home-tools`' `BUILD_windows` used
`set "RUST_MIN_STACK=33554432" && cargo test -p ... && cargo clippy ...` (the
form the existing Windows env-var lesson, line 35, recommends for defensive
quoting). CI still overflowed the stack at the exact same test, with the exact
same crash, as before the fix was written. Direct evidence the variable was
never applied: grepping the full job log for the literal string
`RUST_MIN_STACK` — anywhere in the log, not just this package's section —
returned zero matches, and cargo doesn't otherwise announce which env vars a
subprocess inherited.

**Suspected mechanism** (not independently confirmed on a real Windows box,
but consistent with every observed symptom): the build tool's Go executor
(`code/programs/go/build-tool/internal/executor/executor.go`) runs
`exec.Command("cmd", "/C", command)` — the ENTIRE `BUILD_windows` line as one
argument. Because that line contains spaces, Go's own Windows argument-escaping
wraps it in an outer pair of `"..."` and backslash-escapes any quotes already
inside it (standard CRT-argv escaping rules) before handing it to `CreateProcess`.
`cmd.exe`'s `/C` parsing has its own, different, long-documented quirk: when
the string after `/C` starts with a `"`, cmd applies special stripping logic
and does NOT understand `\"` as an escaped quote the way CRT argv parsing
does. The net effect: the literal backslash-quote sequences Go inserted around
`"RUST_MIN_STACK=33554432"` can survive into the variable name/value `set`
actually parses, silently setting a garbage-named variable instead of
`RUST_MIN_STACK` — no error, no warning, just a `set` that ran and did
something other than what the line says.

**Fix:** drop the quotes when the value has no spaces or shell metacharacters
that need protecting — `set RUST_MIN_STACK=33554432 && cargo test ...` — which
sidesteps the whole Go-escaping/cmd-`/C`-quirk interaction rather than
depending on it working correctly. The existing "defensive quoting" lesson
(line 35) is not wrong for values that DO contain `&|()`/`%CD%`/spaces — where
quoting is genuinely necessary and presumably already proven by the Lua/Perl
`BUILD_windows` files that use it for such values — but it's an unnecessary
risk for a value that doesn't need it, and this PR shipped one real,
CI-confirmed case of the quoted form silently failing.

**Generalized lesson:** an env-var-setting line in `BUILD_windows` that "did
nothing" fails silently — there's no error, just whatever downstream default
behavior kicks in. If a fix that sets an env var to work around a platform
failure doesn't change the failure at all on the next CI run, suspect the
`set` line itself before re-deriving the original bug's root cause a second
time. Grep the job log for the variable name as a first, cheap diagnostic —
its total absence from the log (build tools rarely echo inherited env) doesn't
prove failure by itself, but combined with an unchanged crash, it's strong
evidence the assignment never reached the process that needed it.

## Stop waiting for CI's truncated logs to dribble out failures one round at a time — run the build tool's own detection locally against the same diff base

Four consecutive CI rounds on PR #11553 each fixed a batch of Windows failures
only to have the NEXT push reveal another batch — 21, then 17, then 16 (13
identifiable). Two things made this slower than it needed to be:

1. **`get_job_logs` truncates to roughly the last 5,000 lines of a job's
   combined output, regardless of the `tail_lines` value requested** (100,000
   and 2,000,000 both returned the identical last-~564KB slice). For a
   22,784-line "build and test affected packages" step, that's the last
   ~22% only — the earlier ~78% (including some `--- FAILED: pkg ---`
   markers) is simply not retrievable through that tool, and there's no
   `offset` parameter to page through it. Downloading the raw log directly
   (the Azure blob SAS URL `get_job_logs` returns with `return_content=false`)
   is also not an option here — the sandbox's outbound proxy denies that
   specific blob-storage host outright (403 on CONNECT), a policy decision,
   not a transient failure.
2. **The fix doesn't require CI at all.** The build tool that decides what's
   "windows-affected" runs anywhere: `go build -o /tmp/build-tool-local .`
   in `code/programs/go/build-tool/`, then
   `./build-tool-local -root . -diff-base <same base CI diffs against> -dry-run
   -emit-plan plan.json -validate-build-files=false`. The emitted plan JSON's
   `platform_overrides.windows.affected_packages` is the EXACT list windows-latest
   CI is about to attempt — computed from git diff + BUILD graph, not from
   actually running anything, so it doesn't need Windows or even a full build.
   Cross-referencing every package's resolved `BUILD_windows`/`BUILD` content
   against the two known bug regexes (`"\.\[dev\]"`, `\$\(uname\)`/`set -eu`)
   found 23 more failures in one pass — MORE than the partial CI log could
   even show, comprehensively, without waiting ~80 minutes for a Windows
   runner and then only seeing the tail of the result.

**Lesson:** once a bug's regex/pattern signature is known (from even one
confirmed CI hit), don't wait for CI to reveal the rest of its instances one
truncated log at a time — grep for the pattern across whatever the build
tool's own "affected on this platform" computation says is in scope, using
the SAME diff base CI will use. This is strictly more complete than the log
(reaches packages CI hasn't logged yet) and doesn't burn an ~80-minute round
trip per increment. Reserve actual CI runs for what can't be predicted from
static analysis: real test failures, runtime-only platform gaps (permission
enforcement, ABI-dependent stack sizes), and confirming the fix set is
actually exhaustive.

## `datetime.fromtimestamp()` silently fails for pre-1970 dates on Windows — pure `timedelta` arithmetic doesn't

`sql-vm`'s `unixepoch` date handling used `datetime.fromtimestamp(seconds,
tz=UTC)` to convert Unix-epoch seconds to a date. `SELECT date('-1234567890',
'unixepoch')` — a pre-1970 date — returned `NULL` on Windows only; the
correct answer (`'1930-11-18'`) came back fine on Linux/macOS, and the code
already had `except (ValueError, OSError, OverflowError): return None`
around the call, which is precisely what swallowed the platform-specific
failure into silent NULL instead of surfacing it.

**Cause:** `datetime.fromtimestamp()` converts through the platform C
library's time functions. glibc's `gmtime` handles negative `time_t` (dates
before 1970-01-01) without complaint; the Windows CRT's `_gmtime64`/
`_localtime64` explicitly reject negative `time_t` with an error — a
documented, longstanding Windows CRT limitation, not a Python bug.

**Fix:** compute the date by pure calendar arithmetic instead —
`datetime(1970, 1, 1, tzinfo=UTC) + timedelta(seconds=seconds)` — which
never touches the platform C library, so it behaves identically on every
host `datetime`/`timedelta` run on. Verified this doesn't trade the fixed
platform bug for a worse one: extreme inputs (`1e300`, `inf`, `nan`,
`-1e300`) still raise `OverflowError`/`ValueError` in microseconds, same
failure class as before, just now bounded by `datetime`'s actual year
1-9999 representable range instead of an incidental platform ceiling.
**Generalizable lesson:** any `datetime.fromtimestamp()` (or `time.gmtime()`/
`time.localtime()`, same underlying mechanism) call that might see a
pre-1970 value is a Windows-only landmine — prefer epoch-relative
`timedelta` arithmetic when the code needs to work the same way
cross-platform, not just "not crash."

## `RUST_MIN_STACK` correction: two shell-quoting theories, two CI round trips, both wrong — the fix that actually worked doesn't depend on the shell at all

Continuing the `chief-of-staff-smart-home-tools` saga (two lessons up): the
"drop the unnecessary quotes" fix (`set "RUST_MIN_STACK=..." &&` → `set
RUST_MIN_STACK=... &&`) was itself wrong. The NEXT CI round crashed at the
exact same test with the exact same `0xC00000FD` — proving the quoting
theory, however plausible it sounded, was not the actual mechanism (or was
only part of a bigger propagation problem never fully diagnosed).

**What actually fixed it:** stop trying to get an environment variable
through the Go executor → `cmd.exe` `/C` → `cargo test` → spawned
test-binary chain at all. Wrap the test body in
`std::thread::Builder::new().stack_size(64 * 1024 * 1024).spawn(...).join()`
instead — the stack size is a Rust-level property of the thread `cargo
test`'s harness spawns to run the test, not something inherited from a
shell environment several processes up the chain. Verified locally that this
is the actual fix (not just "different enough to maybe work"): running the
compiled test binary directly with `RUST_MIN_STACK=1048576` (the exact value
that reproduced the original overflow) now passes, because the explicit
`stack_size()` call overrides whatever `RUST_MIN_STACK` would have set —
proof the fix no longer depends on that env var reaching the process at all.

**Lesson, sharper than the one two entries up:** when an env-var-based
workaround for a platform bug fails to change the failure on the very next
CI round, don't spend a SECOND blind ~80-minute round trip on a second theory
about why the env var isn't propagating (quoting, escaping, `cmd.exe`
quirks, Go's argument handling — there are a lot of plausible-sounding
culprits in that chain, and this got two wrong in a row). If the *tool*
whose behavior you're trying to influence exposes a way to set the same
property directly in code — here, `Thread::Builder::stack_size()` for
exactly what `RUST_MIN_STACK` configures — prefer that path outright rather
than debugging an indirect shell/env mechanism you can't test end-to-end
locally (only the CI round trip proves an env var actually reached the
target process; a direct API call you can verify with a single local run).

## Postscript: the RUST_MIN_STACK mystery above has a concrete answer — `set VAR=value &&` puts a TRAILING SPACE in the value

The entry above correctly concludes "prefer `Thread::Builder::stack_size()`
over an env var you can't verify locally," but leaves *why* the env var never
propagated as an open question after two wrong theories. It is worth naming,
because 19 other `BUILD_windows` files depend on getting this exact detail
right:

`cmd.exe`'s `SET` consumes **everything** between `=` and the `&&`, including
the space immediately before it, and does not trim. So

    set RUST_MIN_STACK=33554432 && cargo test ...

assigns the string `"33554432 "` — with a trailing space. Rust then reads it
as `env::var("RUST_MIN_STACK").ok().and_then(|s| s.parse().ok())`, and
`"33554432 ".parse::<usize>()` is an `Err`, so the value is **silently
discarded** and the default stack is used. The variable *was* set; it was just
unparseable. That is why raising 8 MiB → 32 MiB changed nothing: no stack
value was ever arriving.

The repo already encodes the correct idiom in 19 places — `set PYTHONPATH=src&&
...`, `set BISECT_FILE=%CD%\bisect&& ...`, `set PYTHONIOENCODING=utf-8&& ...`
— all with **no space before `&&`**. `smart-home-tools` was the lone deviation.
The readable spelling is the broken one, which is exactly why this recurs.

Corollaries worth keeping:

1. **A malformed env var is indistinguishable from an unset one** whenever the
   reader uses `.parse().ok()` / `unwrap_or(default)`. The symptom is "my fix
   did nothing," never an error — so confirm the value *as the process
   received it* before escalating the magnitude of the fix.
2. Still-outstanding: several `code/packages/perl/*/BUILD_windows` files use the
   spaced form for `PERL5LIB` (e.g. `set PERL5LIB=..\hash-functions\lib && perl
   ...`). Those happen to be tolerable — the value is a path list whose last
   entry merely gains a trailing space, and each also passes explicit `-I`
   flags — but they are the same latent bug and should move to `&&`-adjacent
   form if they ever start failing to find a sibling lib.

## `chief-of-staff-daemon-credential`: 250ms retry budget too tight under real CI contention (ubuntu-latest, non-root)

`concurrent_creators_converge_without_overwrite` spawns 8 threads that all race
`load_or_create_credential()` on the same not-yet-existing path. Exactly one
thread wins the `O_CREAT|O_EXCL` (Unix) / `CREATE_NEW` (Windows) create; the
other seven get `Exists` and fall back to polling `open_existing` in a loop
(`PUBLICATION_RETRIES` iterations of a 1ms sleep) waiting for the winner to
finish `write_all` + two `sync_all` + `fchmod`/ACL-set on the file it just
created — until then the file is either mode-000 (Unix) or opened exclusively
(Windows), so losers see `EACCES`/`ERROR_SHARING_VIOLATION` → `Busy` → retry.

`PUBLICATION_RETRIES = 250` (a ~250ms budget) is enough on an idle machine but
not guaranteed on a loaded/shared GitHub Actions runner: real CI (ubuntu-latest,
**not** root, so this is not the root-permission-bypass artifact documented
elsewhere in this file) showed multiple losing threads exhausting the retry
loop and surfacing `AccessFailed` from `tests::concurrent_creators_converge_
without_overwrite` — a genuine robustness gap in the retry budget, not a logic
bug in the create/publish protocol itself. Fix: widen `PUBLICATION_RETRIES` to
3000 (~3s) in both `unix.rs` and `windows.rs` — same protocol, just more
headroom for CI scheduling noise. This is a case where an env-specific-looking
failure (multiple threads, same panic site, same error) needs the same "is
this the known root-sandbox artifact or a real gap?" check as any other
concurrency test: reproduced locally as root it presents as
`InsecurePermissions` (root bypasses the mode-000 gate entirely, so the loser
opens+reads before the winner's `fchmod`, then fails the strict-permissions
check) — a different symptom from the real CI's `AccessFailed`, confirming
these are two distinct issues and the local root reproduction cannot be used
to validate this particular fix.

## `smart-home-platform-http`: Windows RST-on-close race in test HTTP client — same accepted quirk as the embeddable TCP tests, this time on the client side

`build (windows-latest)` failed with exactly 2 of 54 `smart-home-platform-http`
tests down — `runtime_web_app_serves_runtime_snapshot_over_repo_http_server`
and `runtime_web_app_serves_smoke_script_over_repo_http_server` — both
panicking at the *first* `read_line` call for the HTTP status line:
`Os { code: 10054, kind: ConnectionReset, message: "An existing connection
was forcibly closed by the remote host." }`. The other 52 tests, which hit the
same shared `WebServer::bind_windows` instance through the same `http_request`
test helper with `Connection: close` on every request, passed.

This is the client-side twin of a quirk this very PR already normalized on
the server side (see `embeddable-tcp-server::assert_peer_closed`, which
explicitly treats `io::ErrorKind::ConnectionReset` as an acceptable outcome
of a deliberate peer close): on Windows, closing a socket while there is any
unflushed/unacknowledged data in play can produce a hard RST instead of a
graceful FIN, and it appears more readily on the two endpoints serving the
largest response bodies (a JSON runtime snapshot and a generated shell
script) — plausibly a larger multi-chunk write racing the reactor's own
`Connection: close` teardown. It reproduces only on `windows-latest`; the
identical test passes reliably on Linux and macOS CI.

Rather than touch the Windows reactor's write/close sequencing in
`tcp-runtime`/`embeddable-http-server` (large, shared, security-sensitive
surface, out of scope for this rescue), keep the fix in the test helper. A
fresh-connection retry alone was not sufficient under a later loaded Windows
build: all five attempts for the runtime snapshot could hit the same close
race. The helper now sends `Connection: keep-alive`, reads exactly the
advertised `Content-Length`, and lets dropping the client stream initiate the
close only after the response is complete. It retains the narrow whole-request
retry for `io::ErrorKind::ConnectionReset`; any other error still panics
immediately. This avoids depending on close timing while preserving the known
Windows-platform-quirk handling already established elsewhere in the repo.

## `chief-of-staff-cli`: fifth recurrence of native `Path::join` used to build a path for a *different target platform* than the host

Once `chief-of-staff-daemon-credential` stopped failing, `chief-of-staff-cli`
and `chief-of-staff-daemon` came off `DEP-SKIP` and ran for the first time
this rescue — immediately surfacing `derives_reviewable_requests_for_every_
native_supervisor`, failing on Windows CI with
`left: "/opt/chief\\chief-of-staff-daemon"` vs.
`right: "/opt/chief/chief-of-staff-daemon"`.

`sibling_executable()`'s non-Windows branch built the launchd/systemd sibling
path with `Path::new(current).parent().join("chief-of-staff-daemon")`. The
test deliberately feeds a POSIX `current_executable` (`"/opt/chief/chief-of-
staff"`) for the `Launchd` platform variant *unconditionally* — this
function must produce a POSIX path regardless of which OS built and ran the
test binary, because the target is always a macOS/Linux daemon. `Path::join`
uses the **host** platform's preferred separator, so on a Windows-built test
binary it silently produced a mixed `/opt/chief\chief-of-staff-daemon`.

This is the same family as the four earlier `is_absolute()` bugs in this
rescue (`ir_to_jvm_class_file`, `chief-of-staff-daemon-service-files`,
storage paths in general): **`std::path::Path`/`PathBuf` encode the host's
path conventions, not the path string's own.** Whenever code manipulates a
path that is contractually for a specific *other* platform — a POSIX daemon
supervisor path, a Windows registry path, anything not "this process's own
filesystem" — reach for explicit string splitting/joining (as the sibling
`windows: bool` branch in this very function already did with
`rsplit_once('\\')`), never `Path`. The fix (`unix_sibling_path()`) hand-rolls
`Path`'s own POSIX parent semantics (root normalizes to `/`, a trailing slash
is insignificant, a bare relative name has no usable parent) with plain
string operations, verified against all three cases already covered by
`invalid_sibling_paths_fail_stably` plus the new-to-Windows-CI
`derives_reviewable_requests_for_every_native_supervisor`.

Tell: whenever a test constructs an `InstallEnvironment`/similar struct with
a path literal that does **not** match `cfg!(windows)` (a `/`-style path
fed into a test that runs unconditionally, not gated by `#[cfg(unix)]`), any
`Path`-based manipulation of that literal downstream is host-dependent and
therefore Windows-CI-only-broken — grep for `Path::new(` / `.parent()` /
`.join(` near a hardcoded POSIX or Windows path literal before trusting it
is portable.

## Two packages that emit the same assembly name will eventually collide — and coverlet reports the collision as 0% coverage, not as an error

`fsharp/conduit` failed `build (macos-latest)` with all 40 tests passing and
coverage reported as a flat **0% line / 0% branch / 0% method** against a
`/p:Threshold=80` gate. It had passed the two previous macOS rounds on code
neither intervening commit touched, so it was intermittent, not a regression.

Two independent defects were behind it, and the first one hid the second.

**1. The coverage margin was less than one line.** Measured locally, the
package sits at 169 of 211 lines = **80.09%** against a threshold of **80** —
it needs 168.8 lines, so the entire margin is **0.2 of a line**. One covered
line failing to execute takes it to 79.62% and the build red. Eight sequential
local runs all produced exactly 80.09%, so the number is stable *locally*;
it is the CI environment that perturbs it. CLAUDE.md already requires coverage
to "WELL exceed 80%" — 80.09% is the letter of that rule and the opposite of
its intent.

**2. Two different libraries emitted the same assembly.** Neither
`code/packages/csharp/conduit/CodingAdventures.Conduit.csproj` nor
`code/packages/fsharp/conduit/CodingAdventures.Conduit.fsproj` set
`<AssemblyName>`, so both defaulted to their project file name and both
produced `CodingAdventures.Conduit.dll`, with test assemblies both named
`CodingAdventures.Conduit.Tests` — and coverlet derives its mapping file name
from the *test* assembly (`CoverletSourceRootsMapping_CodingAdventures.Conduit.Tests`),
so renaming only the library would have left half the collision standing. The
build tool schedules packages in
parallel, and the failing macOS run shows `csharp/conduit` (18.3s) and
`fsharp/conduit` (20.0s) overlapping. Two concurrent coverlet runs keyed on
one module name is the only mechanism found that yields *exactly* zero rather
than a low-but-nonzero number. The F# project already declared
`PackageId = CodingAdventures.Conduit.FSharp` and its namespace is
`CodingAdventures.Conduit.FSharp`; only the assembly name disagreed.

Lessons:

1. **A coverage threshold with sub-line margin is a scheduled outage.** Treat
   `covered - threshold*total < ~2 lines` as failing, whatever the gate says.
   Raise it by adding tests, never by lowering the threshold — the gate was
   right, the coverage was thin.
2. **Set `<AssemblyName>` explicitly whenever two packages could plausibly
   share a project file name.** The default (project filename) makes assembly
   identity an accident of directory layout, and nothing warns you: the build
   succeeds, the tests pass, and only a name-keyed tool downstream
   (coverage, instrumentation, a plugin loader) notices there are two of them.
3. **"0%" from a coverage tool means "measured nothing," not "covered
   nothing."** With 40 tests passing, zero is categorically impossible as a
   real measurement — read it as a collection failure and hunt the
   infrastructure, rather than as a signal to go write tests.
4. **When an unrelated package's broken *measurement* blocks real work,
   quarantine the measurement, not the tests.** This gate was disabled (issue
   #11859) so PR #11553's Swift ZIP work could land: `/p:Threshold=80` is
   omitted while `/p:CollectCoverage=true` stays, so all 47 tests still run and
   still fail the build if any breaks — verified by injecting a deliberate
   failure and confirming exit 1 — and the coverage number stays visible in the
   log for whoever re-arms the gate. Disabling the whole package would have
   thrown away a working 47-test suite to silence one broken number.
5. Verify a rename like this by checking the *consumers* build, not just the
   renamed package: `programs/fsharp/conduit-hello` references it via
   `ProjectReference`, which resolves by path and so survives an assembly
   rename — but a by-name reference would not have.

## `BUILD_windows` drifts from `BUILD` silently, because almost no PR runs the Windows shard

Adding BUILD files to 93 orphan Rust crates pulled several Swift packages
into the affected set, which flipped `needs_swift=true`, which is the only
condition under which CI schedules `build (windows-latest)` at all. That job
then failed instantly — not on anything this branch wrote, but on **17
pre-existing `BUILD_windows` files** that had fallen out of sync with their
`BUILD` counterparts. `origin/main` fails identically; it just almost never
runs the job that would say so.

The mechanism is that `discovery.getBuildFile()` returns the *platform*
BUILD file, so on Windows every downstream check — the dependency graph, the
`# build-tool: deps=` directive scan, `ValidateBuildFiles` — reads
`BUILD_windows` and never sees `BUILD`. A dep added to `BUILD` is therefore
invisible on Windows until someone adds it there too, and nothing complains
until a Swift-touching PR happens by.

Three distinct shapes, all repairs of the same drift:

1. **Missing prereqs** (12 Python packages). `BUILD` had `-e ../cas-solve`,
   `BUILD_windows` did not. Purely additive fix.
2. **Stale over-listing** (2 Perl packages). These are skip-stubs whose
   `REM prereqs:` line exists only to declare refs; the line still named
   packages that had long since stopped being dependencies, which trips the
   *undeclared local package refs* check from the opposite direction.
3. **Missing `# build-tool: deps=` directive** (3 Swift packages). The Linux
   `BUILD` declares `deps=rust/conduit-capi`; the Windows twin referenced the
   same crate with no directive, so the ref was undeclared.

The reliable repair rule: **make `BUILD_windows` declare exactly the package
set `BUILD` declares**, adapting only path separators and the venv/interpreter
invocation. `BUILD` already passes this validator on Linux, so mirroring its
package list is correct by construction — no need to re-derive the graph by
hand. In all three shapes above the trimmed/extended list reproduced the
Linux file's list exactly, which is a good self-check that the repair landed.

Corollary: put `# build-tool: deps=` in the default `BUILD` so the Linux detect
job carries the edge into its shared plan. Repeat it as
`REM # build-tool: deps=` in `BUILD_windows`: the resolver still sees the
embedded marker, while `cmd /C` can execute the line safely if that platform
file is invoked directly.

## Human-language book generation rejects ad-hoc lesson block titles

The lesson parser accepts arbitrary `##` headings, so focused curriculum,
activity, continuity, and writing-stage tests can all pass while
`npm run generate:books` later fails with `generated books require known body
blocks`. This happened when Gujarati retrieval lessons used `Reading check` and
`Writing from sound`. Reuse the corpus's canonical taxonomy — for example
`Guided Practice` and `Writing — from sound` — and run book generation before
publishing any new lesson shape.

## A late retrieval lesson still needs an explicit prerequisite path to the atom it assesses

Curriculum reading order does not make every earlier atom transitively available.
The validator follows each lesson's declared prerequisite graph, so a late review
lesson that names only the immediately preceding content lesson can still fail
with `required atom ... is not introduced by a transitive prerequisite`. Preserve
the local sequence prerequisite, but also name the original teaching lesson (or
the latest prerequisite that transitively contains it). This keeps the review
honest when paths and optional extensions are composed independently.

## Auto-merge can immediately merge a stacked PR into an unprotected feature base

`gh pr merge --auto` does not mean “wait for the parent PR.” It asks GitHub to
merge into the PR's current base whenever that base's own requirements allow it.
If the base is an unprotected feature branch, a newly opened stacked PR can merge
there immediately even while checks are queued, moving the parent PR's head and
combining the changes. Before arming auto-merge, confirm that the PR targets the
protected default branch. Keep a stacked child open without auto-merge until its
parent lands, then retarget the child to the default branch and arm auto-merge.

## One inline curriculum extension cannot own lessons from two path segments

An extension listed in a path segment's `inline` array may contain only lessons
owned by that same segment. A chapter can still cross two spine nodes, but its
lessons need separate path segments and separate inline extensions. Chain the
second extension to the first when both are required; do not use one umbrella
extension whose lesson list reaches across segment ownership.
## Hardening added call-site-by-call-site reaches every call site except the one that matters — and the "belt and braces" flag can be the only one working

`latexmk` reads `latexmkrc` / `.latexmkrc` from its **working directory** and hands
the contents to Perl's `eval`. Every book in this repo is compiled with that
directory set to `code/learning/human-languages/<track>/book`, which is pull-request
content. The control is `-norc`.

`-norc -r code/scripts/latexmk-safe.rc` was present and correct in
`check-book-compile.sh`, `build-books-locally.sh`, `verify-human-languages.sh`, and in
all 23 tracks' `book/build.sh` **and** `book/build.ps1`. It was absent from
`.github/workflows/human-languages-books.yml`, which is the only place the books are
compiled at scale, on every pull request, in a job holding a `contents: write` token.

Nothing detected that, because a grep for the *protection* over the scripts directory
returns plenty of hits and looks reassuring. The grep that finds this is the one over
the **invocations**:

    grep -rn 'latexmk\|xelatex\|pdflatex' .github/workflows/

and then checking each hit for the flag — not grepping for the flag and counting.

**The general rule.** When you harden a dangerous call, the unit of work is *every
invocation in the repository*, enumerated from the dangerous verb. Fixing them one at a
time as you encounter them guarantees the set you never encountered stays unfixed, and
the ones you never encounter are disproportionately in CI — nobody runs CI locally, so
its call sites are not the ones you trip over. Better still, delete the second call site:
this fix made the workflow invoke `check-book-compile.sh` rather than maintain a
parallel `latexmk` command line, so there is now one invocation and drift is not
possible.

**A flag is only present where someone typed it; make the payload impossible instead.**
`code/scripts/check_no_book_latexmkrc.py` now fails the build if any casing of
`latexmkrc` appears under the book tree. That protects call sites not yet written.

**Assert the control at runtime, not in review.** XeTeX writes its shell-escape state
into every `book.log`: `" restricted \write18 enabled."`, `"\write18 enabled."`, or —
when off — no line at all. CI now greps the logs the build just produced. This is the
only check that catches the failure mode below, which no amount of reading the YAML
would find.

**The measured control matrix (latexmk 4.88), which is not the one you would guess:**

    invocation                                    latexmkrc    shell escape
    --------------------------------------------  -----------  -------------------
    latexmk -xelatex ...            (old CI)      EXECUTED     restricted, enabled
    latexmk -norc -xelatex ...                    not read     restricted, enabled
    latexmk -norc -r latexmk-safe.rc -xelatex     not read     DISABLED
    shell_escape=f latexmk -xelatex ...           EXECUTED     restricted, enabled

Two things fall out. `-norc` alone does **not** turn shell escape off — it only stops
the rc being read; the `-r` that loads `$xelatex = "xelatex -no-shell-escape %O %S"` is
what disables it, so the two flags are not redundant with each other. And
`shell_escape=f` in the environment did nothing on this box: **MiKTeX ignores the
kpathsea `shell_escape` and `openout_any` environment variables entirely** (`openout_any`
set to `p`, `a`, or unset all behaved identically, all blocking `../` writes under
MiKTeX's own configuration). TeX Live honours them; MiKTeX does not. So a Windows
verification of an env-var-based TeX hardening proves nothing either way — say so
rather than reporting it as tested.

**`TEXMFOUTPUT` is not a sibling of `openout_any`.** It reads like another lockdown knob
and is the opposite: paths under `TEXMFOUTPUT` are *exempt* from `openout_any=p`. Setting
it widens what TeX may write. Leave it unset.

## A gate that cannot run its check must fail, not skip — "compiled 0, skipped 1, failed 0" and exit 0

`check-book-compile.sh` skips any track whose SVG figures it cannot convert, because a
missing figure PDF is a compile failure that says nothing about the LaTeX under test.
That is right on a laptop. Wired into CI unchanged it is a gate that reports success
having verified nothing — on a runner without `rsvg-convert`, every illustrated track
skips, the summary reads `compiled 0, skipped 1, failed 0`, and the exit status is 0.

This is the same absent-vs-could-not-determine conflation as #12731 and #12734, one
layer up: there it was a bare `catch → "missing"`, here it is `skip → pass`.

**The shape of the fix, reusable:** keep the lenient behaviour, add `--strict` that
turns every "could not verify" into a failure **naming the missing dependency**, and
additionally fail when the run verified *zero* items — because a selection typo or a
renamed directory produces a clean-looking green with an empty work list. Then make the
lenient path announce its own weakness in its output (`"CI runs this script with
--strict, where each of those is a failure"`), so a local pass is never quoted as
evidence the gate would pass.

**And prove the red before trusting the green.** A gate that has only ever been observed
passing has not been observed working. Both directions were run here: `--strict` on a
track with a figure and no converter fails with the dependency named; the same command
with a converter on `PATH` compiles the book and reports `compiled 1, skipped 0,
failed 0`.

## Fixing the RCE is not the fix if the job still hands the attacker a write token

The security review of the latexmk hardening above found the real ranking, and it was not
the one the PR started with. Closing the `latexmkrc` `eval` mattered — but the same job
was `pull_request`-triggered with `permissions: contents: write`, ran `npm ci` (without
`--ignore-scripts`), a TypeScript build and seven `node dist/*.js` checks, all
pull-request-controlled, and `actions/checkout@v7` defaults to
`persist-credentials: true`, which leaves the write-scoped token in `.git/config` **inside
the workspace latexmk `cd`s into**. Anyone who could have used the latexmk hole already
had a dozen easier ones.

**Ask what the job is holding, not only what it is running.** The durable fix is
structural: the job that executes repository content gets `contents: read` and
`persist-credentials: false`; a separate job holds `contents: write`, runs no repository
code, has no `actions/checkout` at all, and is gated at JOB level (not step level) on
`github.event_name == 'push' && github.ref == 'refs/heads/main'`. Step-level `if:` on the
publish steps was already there and was not enough — the token is scoped to the job, so it
is live for every earlier step in it regardless.

When splitting, remember the aggregating gate job: `needs: [detect, build-and-publish]`
and `needs['build-and-publish'].result` both have to move, and the new publish job must
**not** become a dependency of the gate, or the gate can never pass on a pull request.

## "Which artifacts did we build?" and "which directories look like books?" are different questions

The same review caught a regression this PR introduced. Splitting compile from collection
left the collector re-deriving its work list with `find -type d -name book`, while the
compiler skipped any directory with no `book.tex`. Two enumerations, one of them
attacker-extensible: a PR adding only `code/learning/human-languages/anything/book/book.pdf`
gets that file uploaded as a build artifact and, on `main`, pushed to Pages and attached
to the Release — an attacker-authored PDF served as a book. Make it a symlink and `cp`
dereferences it, so the bytes of whatever it names go out instead.

**The rule: a producer that hands work downstream must say what it produced.** Add a
`--manifest` the compile appends to on success, and have the consumer read that file.
One writer, one reader, nothing to disagree about. Re-deriving a list is not a check on
the first derivation — it is a second, independent, silently-different answer.

Two corollaries, both bit here:
- Guard the OUTPUT path for a symlink, not only the input. `figures/*.svg` was vetted with
  `find -type f` and the derived `${svg%.svg}.pdf` was written unchecked; `book.pdf` had no
  guard at all even though XeLaTeX opens it for writing. `[ -L "$out" ]` before writing,
  and again before publishing, because the two steps should not have to trust each other.
- Deleting a duplicated step beats hardening it. The workflow's own SVG conversion existed
  only because the script's conversion was invisible from the YAML; the script's copy had
  the symlink guard and the workflow's did not.

## Count what you inspected, or the loop that inspects nothing reports success

Also from the same review, in — of all places — the step added to *prove* the hardening
was in effect:

```bash
offenders=0
while IFS= read -r log; do ... done < <(find … -name book.log | sort)
test "$offenders" -eq 0
echo "Shell escape disabled in every book.log"     # printed after reading zero files
```

`find` inside a process substitution has an exit status `set -o pipefail` cannot see, and
a loop body that never executes leaves the counter at its initial value. Zero matches is
indistinguishable from a clean corpus. Any later change that adds `-outdir=` to the
latexmk line disarms the check silently, and the job stays green while asserting the
opposite.

**Every verification loop needs a `checked` counter and a floor**, and the floor should
come from the producer (`wc -l < manifest`) rather than a hardcoded number that goes stale
when a track is added. This is the same failure as `compiled 0, skipped 1, failed 0` two
lessons up — which is the point: the shape recurs, including inside the fix for itself.

## `openout_any=p` vets the NAME and then opens it — it does not resolve symlinks, so guarding one output filename guards one of eight

Round two of the same security review. The `book.pdf` symlink guard added in round
one — itself a review finding — was **still wrong**, in the same shape as the bug it
fixed: it enumerated a filename instead of banning a category.

A XeLaTeX run writes at least eight files into the book directory:

    book.aux  book.log  book.toc  book.out  book.xdv  book.pdf
    book.fdb_latexmk   book.fls        <- latexmk's own, written from Perl

`openout_any=p` looks like it covers these and does not. It is a **name** check: it
rejects absolute paths, `..`, and dotfiles, and then hands the name to `fopen(name, "w")`,
which follows the link. `book.fdb_latexmk` and `book.fls` never reach a TeX-side check at
all, because latexmk writes them itself with a plain Perl `open(…, ">")`.

So `<track>/book/book.aux -> /home/runner/.ssh/authorized_keys` is an **arbitrary write as
the build user, from a pull request, with no shell escape and no `latexmkrc` involved.**
Seven doors stood open next to the one that got locked.

**The rule: ban the category, do not enumerate the cases.** The fix moved into the
tree-walking lint, which now refuses *any* symlink under the book tree. Cost of the
blanket ban: zero — `git ls-files -s <tree> | awk '$1=="120000"'` returned nothing, and no
curriculum book has a reason to contain a link. Cost of the enumeration: one missed
filename is a full compromise, and the list grows whenever XeLaTeX or latexmk decides to
write something new. An allowlist of safe things beats a denylist of dangerous ones, and
"the file XeLaTeX happens to write today" is a denylist.

**`.gitignore` is not a boundary.** `book.aux` and friends are ignored — and `git add -f`
commits them anyway. Ignoring a path says where files come from, not what may exist.

**Corollary on where the guard belongs.** It went in the Python lint that already walks
the tree, not in the shell script, because the walk is one pass that already runs twice
and the shell script would need the check at every write site. Put a categorical ban where
the enumeration already happens.

## Adjacent code being correct is not evidence about this line

Three times in one day, in unrelated files: `doc-shard.ts` kept a sanitization gap after
the CLI beside it was fixed; `shard.ts` kept a bare `catch → "missing"` while
`shard-cli.ts` had it right; and the `book.pdf` symlink guard was defended in review with
"same `[ -L ]` idiom as the figure guard twenty lines above." The first two were real
misses. The third was a real miss too — the idiom was right and the *coverage* was wrong,
which the analogy could not have revealed, because the sibling guarded a genuinely
complete set (`figures/*.pdf`) and this one did not.

**A sibling proves the idiom compiles, never that the coverage is complete.** When
tempted to write "same pattern as X, so it is fine", the honest move is to enumerate what
this line must cover and check the list — the sibling's list is the sibling's.

### The fourth instance, and the sharper form: copying a guard at half strength

A week later, a fifth: the assessment-artifact generator (`assessment-artifact-cli.ts`)
needed the same symlink protection `book-cli.ts` already carries for the generated
`.tex`. `book-cli.ts` carries **two** checks, and says why:

1. `lstat` the last component and refuse a non-regular-file, and
2. `realpath` the whole path and re-assert containment, *because* — in its own words —
   "`lstat` vets only the LAST component… resolving the whole chain and re-asserting
   containment is the only check that covers every component at once."

The new code cited `book-cli.ts` as its model and took only check 1. A committed
`core -> /somewhere/else` then passed: the `lstat` lands on a real directory *inside the
linked-to tree*, `isDirectory()` returns true, and every generated shard is written
outside the repository root. Security review caught it; a second round was needed.

This is not the same mistake as the three above, and the difference is the useful part.
Those were *appeals* — "same pattern as X, so it is fine". This was a **transcription**:
the sibling was opened, read, used as a template, and truncated. The comment explaining
the dropped half was on screen at the time.

**A guard copied from a sibling must be copied whole, and the sibling's comments are part
of the guard.** A comment that says "this check alone is not enough, which is why the next
one exists" is not commentary on the code — it is the specification for the line you are
about to not write. Before shipping a borrowed guard, diff it against its source and
account for every check the original has that yours does not.

Corollary, and the reason this keeps recurring across five instances in unrelated files:
**correct code sitting next to incorrect code does not make the neighbour correct.**
Proximity, shared idiom, and a shared author are all zero evidence. The only evidence is
enumerating what this call site must cover and checking the list.

## If you cannot execute the proof locally, move the proof to where it runs — do not infer it

The `book.pdf` symlink guard could not be exercised on the authoring box: native symlinks
need elevation or Developer Mode, `New-Item -ItemType SymbolicLink` fails with
"Administrator privilege required", and WSL is not installed. The tempting write-up is
"same idiom as the guard above, so it is fine" — an inferred pass, and the thing this
repository has been burned by.

**Better than a local demo: make it a test that runs where the capability exists.**
`tests/test-check-book-compile-guards.sh` and the lint's `SymlinkBanTests` create a real
symlink, run the real code, and assert the real refusal. They **skip with a printed
reason** on a filesystem that cannot make one, and run on the Linux CI runner — which is
also where the guard actually matters. A one-off local observation would have proved it
once; this proves it on every run.

Two details that make the skip honest rather than a dodge:
- **Probe, don't guess.** Try `ln -s` / `Path.symlink_to` and check `[ -L ]`; do not branch
  on `$OSTYPE` or `os.name`. Git Bash reports `msys` whether or not Developer Mode is on.
- **Scope the skip to the test that needs the capability**, not the whole file. The first
  draft `exit 0`-ed the entire suite before the `book.tex`/manifest cases, which need no
  symlink — so Windows verified nothing at all. Scoped, a Windows run still executes three
  real assertions.

**And mutation-test the new test before believing it.** Removing
`[ -f "$dir/book.tex" ] || continue` turned the suite red; restoring it turned it green. A
test that has only ever been observed passing has not been observed working — the same
principle as proving a gate's red path, applied to the gate's own tests.

## A lint that only CI runs protects only CI — the script a human runs must carry its own guarantee

Round three of the same review, and the same lesson eating its own tail for the third
time. The general symlink ban was moved into `check_book_tree_hygiene.py` — correct — and
`check-book-compile.sh` never calls it. CI was safe, because the workflow runs the lint
immediately before compiling. But the script is documented as *the same one a human runs
locally*, and locally nothing runs the lint:

    git checkout <contributor-branch>
    ./code/scripts/check-book-compile.sh          # on Linux or macOS

still wrote through `<track>/book/book.aux -> ~/.ssh/authorized_keys`. Not a mere
destructive overwrite, either: `.aux` content is substantially author-controlled through
labels and TOC entries. The two surviving `[ -L ]` guards covered 2 of the 9 files a
XeLaTeX run writes.

This is the PR's own thesis one level up. The thesis was "a flag only protects the call
sites somebody remembered to type it at", and the answer was "move the guarantee into a
lint". Then the lint became the thing only some call sites invoke. **Ask, of every control
you extract into a shared checker: who calls the checker, and what happens to the callers
who do not?**

The fix is four lines in the existing idiom — a whole-directory
`find -type l -print -quit` sweep before any write — after which the narrow `[ -L ]` guards
are genuinely redundant in CI and the script finally carries its guarantee everywhere.
Keeping the narrow guards anyway is right: they cost nothing and they name the specific
file rather than the first link found.

## A test that skips is a test that did not run — say so where it was supposed to run

Both new symlink suites probe the filesystem and skip when it cannot create links. Correct
on Windows. On the Linux runner the probe will succeed — but **nothing asserted that it
had**. A runner image that broke symlink creation would silently stop exercising the
security tests, and the step would stay green.

That is `compiled 0, skipped 1, failed 0` wearing a different hat, and it would have
shipped in the same pull request that removes the original.

**The fix is exactly the `--strict` shape, applied to the test suite instead of the gate:**
an environment variable the CI job sets (`REQUIRE_SYMLINK_TESTS=1`) that turns the skip
into a failure. Local runs keep the honest skip; the machine where the capability must
exist asserts that it does.

Generalised: *a conditional skip needs a caller who can say "not here it isn't."* Any
`skipTest`/`SKIP` guarding a capability that is mandatory somewhere should have a way for
that somewhere to demand it. Verify both directions — the skip path locally, and the fatal
path by setting the variable on a box that genuinely lacks the capability, which is the
one place you can observe the failure for free.

## Two bans on one file need two messages, or the remediation advice contradicts itself

A symlink named `latexmkrc` violates both of this lint's rules. Collapsing them with
`elif` — reporting only the symlink — looked like sensible de-duplication and was a bug in
the *advice*, not the detection. The symlink message ends:

    Replace the link with the real file, or delete it.

For this path that instructs the author to create a real `latexmkrc`, which is precisely
what the other ban exists to prevent. The detection was fine either way; the report was
telling somebody to do the dangerous thing.

**When one artefact trips several rules, de-duplicate the FAILURE, never the GUIDANCE.**
One non-zero exit, one entry per rule — because the remediation for rule A can be a
violation of rule B, and the reader follows whichever text you printed.

## A script documented as `./script.sh` must ship mode 100755 — and only the platform that cannot detect it will introduce the bug

`check-book-compile.sh` documents its own usage as `./code/scripts/check-book-compile.sh`
and shipped git mode **100644**. The documented command had therefore never worked on a
fresh Linux or macOS clone: `Permission denied`, exit **126**.

Two things hid it, and they reinforce each other:

- **Windows does not model the executable bit.** `[ -x file ]` is true for every readable
  file there, so an authoring box in this repo can neither cause nor detect the problem —
  and the same box is where most of these scripts get written.
- **Every automated caller sidesteps it.** CI ran `bash code/scripts/check-book-compile.sh`,
  which works at 100644. So the one invocation form that was broken was the one only
  humans use, and no gate used it.

`verify-human-languages.sh` had the identical defect. `build-books-locally.sh`,
`generate-compiled-grammars.sh` and `miri-twig-vm.sh` were all already 100755, which is
what marks the other two as oversights rather than a convention.

**Fix the mode, not the caller.** The test that found this invoked `"$SCRIPT"` directly.
The tempting repair is `bash "$SCRIPT"` — the suite goes green in one character. It also
leaves the documented command broken forever, and deletes the only thing in the repo that
was executing it the way a human would. When a test catches a real defect, the defect is
what moves.

**Assert the index mode, not `-x`.** A `[ -x ]` regression test is vacuous on Windows, so
it would pass on the platform most likely to reintroduce the bug. `git ls-files -s` reports
what actually ships and is meaningful everywhere:

    git_mode=$(git ls-files -s -- "$SCRIPT" | awk '{print $1}')
    [ "$git_mode" = "100755" ] || fail

Mutation-check it with `git update-index --chmod=-x` / `--chmod=+x`, which flips the index
mode without touching the filesystem — so it works as a test even on Windows.

**Generalisable check:** for every script whose header documents a `./` invocation,
`git ls-files -s` it. The two are independent facts and nothing in this repo tied them
together.

## A repertoire check is only as good as the corpus you scope it to

The Spanish book compiles with **Latin Modern**; the Indic and CJK books load vendored
**Noto** faces. A missing glyph does not fail locally — nothing opens a font until
XeLaTeX runs in CI — so the only pre-push defence is to diff the characters your new
`.tex` uses against the characters a book has already rendered.

The first version of that check scanned **`code/learning/human-languages/**/*.tex`**, the
whole corpus. That is wrong in the direction that produces silence: the union of 1,311
`.tex` files includes every Devanagari, Arabic and CJK character in the repo, so it would
have certified as safe any glyph Latin Modern cannot draw. A corpus-wide repertoire is not
a conservative approximation of a per-track one — it is the opposite of one.

**Scope the repertoire to the track whose font will render it.** 368 Spanish `.tex` files,
152 distinct characters. That is the set that has actually been proved.

**And self-test the instrument in BOTH directions.** The bug above was not caught by
reasoning; it was caught by an assertion:

    if "ñ" not in repertoire:  fail   # loaded empty -> everything reads novel
    if "好" in repertoire:      fail   # loaded the wrong thing -> nothing reads novel

The second assertion is the one that fired. A one-directional self-test ("did I load
anything?") passes cheerfully on a repertoire that loaded 1,311 files when it should have
loaded 368. **Every check that compares against a baseline needs a negative control**, or
it cannot tell "nothing is wrong" from "I am not looking at the right thing."

## A vocabulary gate can be inflated by a word the course already owns

`vocabularyOf` counts distinct headwords on lessons whose `type` is `word`/`phrase`. That
restriction is correct — it stops drill titles and grammar labels being counted as
vocabulary. Its side effect is that **a lexeme introduced by a `grammar` lesson is owned
but uncounted.**

Spanish teaches `dar`. `ES-C65-di` introduces the atom `ES-LEX-DAR`, and its `type` is
`grammar`. So `dar` appears in no headword list, passes every duplicate test — no article,
no compound, no shared stem, no spent root — and adding it as a new `word` headword would
have **raised the A1 number while re-teaching a word the learner already had**.

That is the same failure direction as a near-duplicate, through a different door, and no
string rule of any sophistication reaches it.

**Check the atom ledger, not only the headword list.** `grep -l "ES-LEX-<WORD>" lessons/*.md`
is enough by hand. The permanent fix is in `validate.ts`: flag a new `word` lesson whose
headword matches a lexical atom another lesson already introduces.

## Near-duplicates: initial stem is the signal, not containment

Tranches keep answering the whole-string matcher by lengthening the delimiter list.
By tranche 4 the delimiter list caught **nothing**; every real drop came from a shared
stem or from the root ledger.

The rule that separates the two cases cleanly:

**A taught word sharing an INITIAL stem is a drop. A taught word appearing as a
non-initial substring across no morpheme boundary is a false alarm.**

Drops: `la camisa` beside `caminar`, `el abrigo` beside `abrir`, `el hombre` beside
`el hombro` (one vowel apart), `el cuerpo` beside `la cuerda` — all unrelated
etymologically, all confusable exactly where a learner keys on the word.

Kept: `el dedo` inside `alrededor` (which is *retrō*), `el corazón` ending in the taught
`razón` (which is *ratiō*), `la lámpara` containing the taught `para`. A blunt containment
rule kills all three for nothing.

**Check the morpheme, not the letters** — in both directions. Containment that fires on a
letter accident is as expensive as a stem match that never fires.

## "The course" is a claim a reader holding one volume cannot check

`standalone-book` refuses prose that tells a reader they already learned something which
may live in a different volume. Four sentences in one tranche said "in this course" or
"the course has already".

The tempting repair is a vaguer word. **The right repair is to re-anchor the claim on the
reader's own experience**, which travels with them into any volume: "nearly every word you
have met has a paper trail", not "nearly every word in this course". "The tallest thing you
have named so far", not "that the course has named".

`this book` is the sanctioned scope — the reader is holding it — but it converts a vague
claim into a falsifiable one, so an ordinal or a count moved to book scope has to be
walked against the volume's actual contents first.

## A grouping parameter is not a budget

`language-ladder` fails at >353 lazy lesson batches, and separately caps the largest batch.
Content growth walks into the count ceiling every few tranches.

The reflex is to read both numbers as budgets and refuse to touch either. But `maxSize` in
`vite.config.ts` is a **bundler grouping parameter**: raising it 49 kB → 56 kB took the
measured count **401 → 353**, moving the real ceiling *down* by 48 while the corpus grew by
35 lessons. Raising the *count* would have been the violation; this is its opposite.

Two things make that legitimate rather than convenient, and both should be checked before
reaching for it: the change had **in-repo precedent** (a previous author did 32 kB → 49 kB
for the identical recurrence and wrote it into the file), and the size increase stayed far
inside the budget that actually protects the browser (54,688 B against a 500 kB eager-chunk
limit — about 11%).

**And a ceiling that may fall must actually fall.** Lower the pin to the new measurement in
the same commit, or the slack you just created silently becomes room for the next
regression to hide in.

## An unvalidated tag vocabulary drifts, and nothing reports it

`sounds:` frontmatter is a cross-lesson index; its value is entirely in tags being shared.
Nothing validates them. Six authors independently coined **20 tags** outside the attested
set (`h-silent` for `silent-h`, `ll-as-y` for `ll-y`, `z-seseo` for `z-as-th-or-s`), and
none of them failed anything — an unknown tag just becomes a singleton no lesson ever joins,
so the index loses the lesson and the lesson loses the index.

Note the shape: every invented name was *reasonable*. Nothing tells an author which of two
equally good spellings this corpus already uses.

**Generalisable check:** any free-text field that exists to be JOINED ON needs a closed
vocabulary. If coining a new value is meant to be possible, make it cost one line in a
registry — that turns a typo into a decision.

## A shard for a level-N document must contain exactly one level-N heading

`BACKLOG.md` is doc-sharded at heading level 2, so each `BACKLOG.d/*.md` holds one `##`
section. Editing an existing shard to add a second `##` sub-heading looks harmless and is
not: the document then has 131 sections while the shards define 130, because the second
heading rides inside the first shard instead of owning one.

**`check:doc-shards` does not catch this.** It rebuilds the monolith from the shards and
compares bytes, and that round trip is stable in the direction it tests — the extra
heading is emitted verbatim either way. What is unstable is the *other* direction: re-run
`--shard` and the file splits in two, with a new ordinal and a new digest.

`doc-shard.test.ts`'s "shard order on disk reproduces REAL section order" is the check that
notices, because it counts sections on both sides instead of comparing bytes.

**Use a lower heading level for structure inside a shard** (`###` under a `##` shard), and
if a finding genuinely deserves top billing, give it its own shard file with its own
ordinal.

**Generalisable check:** whenever a round-trip has two directions, a byte-comparison of one
direction is not evidence about the other. Assert on the structure both sides define, not
on the bytes one side produces.

## Unused capacity in a size-capped bundler group is not headroom

Follow-up to "A grouping parameter is not a budget", and a correction to the reasoning
recorded there. Raising `maxSize` 49 kB → 56 kB took the lesson-batch count 401 → 353 and
left 32% of the aggregate cap unused. That 32% was written into a merged PR body as
"6.29 MB of fill headroom before the batch count can grow again". It is not headroom, and
the next tranche proved it:

```
origin/main   353 batches   13,478,418 B   32% of cap unused
+35 lessons   359 batches   13,624,129 B   32% of cap unused
```

Thirty-five lessons weighing 145,711 B — about **2.6** batches at the cap, and lighter than
the previous thirty-five — added **six** batches, and the unused fraction did not move at
all. Rolldown groups by track and *then* splits each track greedily by size, so every other
track's tail batch is sealed and never revisited. A Spanish tranche can only extend
Spanish's tail. **Aggregate slack in a partitioned bundler group is stranded by
construction.**

**Generalisable check:** before treating unused capacity as headroom, ask whether the
allocator can *reach* it. Summing free space across N independently-sealed partitions
answers a question nobody asked; the number that predicts growth is the free space in the
one partition the next write lands in. The same error is available in disk allocators,
shard maps and connection pools.

**How it was actually fixed, since "stop treating slack as headroom" is not a fix.** Group
by something the corpus *has* rather than by size — here a five-chapter band — so the count
follows a structural property instead of bytes. Then **derive the budget from that property
instead of hardcoding it**: count the bands and require the emitted chunks to correspond.
Adding lessons inside a band moves neither side; adding chapters moves both together; a
regression moves only one and fails. The ceiling stops needing to be raised, which is what
made it debt in the first place.

**And it exposed how weak the constant had been.** 353 was met on a corpus that only needed
281, so **72 batches of drift would have passed unremarked**. A constant sized once is a
gate that loosens every day the corpus grows without ever telling you.

## `--shard` renames other people's shards, and `--check` does not ask it to

`doc-shard-cli --shard` regenerates **every** filename from its heading text. Several
committed shards were named by hand and no longer match what the generator produces —
one whose heading contains a non-ASCII letter the slug drops, several committed before the
digest suffix existed. So a routine re-shard after a merge deleted and recreated three
files this branch never touched, twice, putting unrelated renames into a content diff and
manufacturing conflicts for whoever else was editing them.

`--check` compares the **bytes of the rebuilt document**, not filenames, and its own comment
says so explicitly: ordinals are author-chosen by design, so requiring canonical names would
break the promise that wedging an entry in at `00155-…` needs no renumber.

So after a re-shard: restore every shard that is not yours to its committed name, delete the
regenerated duplicates, and then run `--unshard` so the monolith is rebuilt from the shard
set actually on disk. Only the **ordinal** has to be right, because filename order is
document order.

**The generalisable half:** when a generator is idempotent in *content* but not in *naming*,
running it wholesale attributes other people's history to your commit. Regenerate the entry
you added; leave the ones you did not.

Doing that by hand is fine. Doing it with a helper that *infers* which shards are yours is
not — see "A helper that decides ownership by pattern will discard your own work
when you rename it" below, which is how I lost three of my own.

## A curriculum gate can silently constrain what may be authored

Chasing "600 headwords at or below A1", ten fully-verified verb candidates were dropped —
not for duplication, not for etymology, but because **no A1 spine node can host a verb**.
`vocabularyOf` credits a headword to a level through its curriculum segment's spine node,
and the three A1 nodes in use are "mark out a specific known thing", "ask where something
is" and "the numbers one to five". A `canDo` reading "I can say *aprender*, *olvidar* and
*necesitar*, and mark out which specific thing I mean" is a slot being filled, not a
capability.

Nothing reports this. The gate counts headwords and is satisfied; the fact that the only
headwords it can count are nouns and adjectives is invisible to it, and the resulting
curriculum shape is one nobody chose.

**Generalisable check:** when a metric is defined by a join against a taxonomy, the taxonomy
silently bounds what can be built. Ask what the metric *cannot* count before assuming a
shortfall is a content problem.

## A gate that recovers a constant by regex can be disarmed by a comment

Replacing a hardcoded CI ceiling with one **derived** from the corpus was the right move and
almost shipped with the same class of hole it was removing.

The bundler config declared the band width; the checker recovered it with
`/export const LESSON_BAND_CHAPTERS = (\d+);/.exec(configSource)`. `exec` returns the
**first** match anywhere in the file — and that file carries eighty lines of prose that
discuss the constant by name, because a number nobody can explain is a number the next
person bumps. So a line as innocent as

    // historical note: this was `export const LESSON_BAND_CHAPTERS = 1;` before

hands the checker a band width of 1 while the bundler goes on using 5. Smaller bands mean
*more* bands, so the derived budget inflated from 281 to **1,158**, and a grouping
regression all the way back to the byte-linear shape the change existed to kill would have
passed unremarked. Documenting the constant well was what armed the attack.

**The fix is not a better regex** — `^`-anchoring or "last match wins" both lose to a
slightly different comment. Put the value in a module and `import` it from both sides. An
import cannot be shadowed by a comment, and the two consumers stop being two
implementations that merely look alike.

**Generalisable check:** any time a checker recovers a value by *parsing the source of the
thing it checks*, ask what happens when that source also *talks about* the value. Config
files, migration scripts and lockfile linters all do this. If both sides can import, they
must.

**Corollary — for a CI gate, ask which direction an error moves it.** Every guard on that
corpus walk pointed the same way once the question was framed: a symlinked `lessons/`
directory, an unbounded digit run reaching `Infinity`, a track name Rollup's sanitiser
mangles — each *invents* a band, each *raises* the budget, each makes the gate pass when it
should fail. None of them threatened the build. A gate's own permissiveness is the only bug
class it cannot catch for you.


## A helper that decides ownership by pattern will discard your own work when you rename it

Re-sharding a merged document renames shards it did not author (previous entry), so I wrote
a helper to restore the foreign ones: it matched shard filenames against a regex of *my*
slugs, kept those, and reverted the rest.

Then I rewrote my three entries' headings — which is what the whole edit was for — and ran
it. The slugs no longer matched, the helper classified my own new shards as foreign,
deleted all three, and the subsequent `--unshard` rebuilt the document from what was left.
The monolith lost three sections and `--check` passed, because the shards and the monolith
agreed perfectly about the reduced content.

The section count is what caught it: 132 where 135 was expected. The byte-level round trip
cannot see this, and neither can a grep for the titles — the titles were the thing that had
changed.

**Rules:**

1. **Ownership is a fact you know, not a pattern you infer.** Pass the file list explicitly.
   A helper that guesses which changes are yours will guess wrong exactly when you have
   changed them.
2. **Count before and after, and make the expected count a number you wrote down first.**
   "135 sections, 135 shards" is a check; "shards and monolith agree" is not, because a
   tool that deletes from both keeps them agreeing.

## Making a safety detector *less* trigger-happy can add a false negative in exactly the direction it exists to prevent

`modality.ts` decides whether a lesson can be done while driving. Its stated bias is to
over-report `sight`, because a lesson wrongly called drivable sends a driver to a page at
speed. Issue #12665 was about the opposite failure — figurative prose ("Look at what
English built on that jar") costing lessons the driving edition — so the whole change was
a loosening, and every rule added was a new way to say "this cue does not count."

One of them, a use-versus-mention rule dropping cues inside a quoted gloss, accepted `'`
as both an opening and a closing quote mark. English contractions and possessives are
apostrophes. Any two within sixty characters forged a "gloss":

    Don't look at the chart's third bar.
       └───────── "quoted" ──────────┘     -> `look at` silently dropped

That is ordinary English, no adversary. It had already stripped a real lesson
(`ES-W00-hola-observe`) of its `sight-cue` reason and removed the narration line telling
a listener the lesson "points at something written down" — and the corpus flip count did
not move, because the lesson was `pen` for other reasons, so the damage was invisible in
every summary number I was watching.

- **When a change loosens a safety rule, the review question is not "does it still catch
  the cases I listed" but "what does it now MISS".** All my tests asserted the new drops
  were correct. None asked what else got dropped.
- **A generated artifact is a better diff than a count.** The regeneration showed
  `"sight-cue"` disappearing from a `reasons` array on a lesson whose modality was
  unchanged. Diff the reason lists, not just the labels.
- **Quote-delimiter classes must exclude the apostrophe in English prose**, and the curly
  `’` too — it is the typographic apostrophe far more often than a closing single quote.

## A regression test for a scan can pass against the bug when the scanned function short-circuits

Fixing an occurrences x spans quadratic, I wrote the timing test as
`'"a" look at '.repeat(40_000)` — many quoted spans, many cue occurrences. It passed
instantly against the *unfixed* code. The function returns on the first surviving
occurrence, and the first `look at` was outside the quotes, so it returned after one
iteration and never touched the span list.

The shape that exercises it is `'"look at" '.repeat(40_000)`: every occurrence must be
DROPPED for the loop to run to the end.

**Rule: to benchmark a loop, the input must make the loop run.** For any function with an
early return, the worst case is the input where the early return never fires — which is
usually the *negative* result, not the positive one. Check the timing test actually got
slower against the old code, exactly as you would check a correctness test goes red.

## Lookaheads are already atomic in ECMAScript — the fix for a per-position scan is a BOUND, not an atomic group

`/!\[[^\]]*\]\(/` on a body full of `![` with no `]` took 49 seconds for 400 KB. The
reflex fix is the atomic-group idiom, `(?=([^\]\n]*))\1`, and I shipped it. It changed
nothing measurable: 54 seconds after, 54 before.

A lookahead in JS is already atomic — the engine does not backtrack into it — so there was
no backtracking left to remove. The cost was never backtracking. It was that an unbounded
`*` scans to end-of-text at EVERY starting position, and `!` appeared 200,000 times.
`{0,200}` is what made it linear (303 ms on 1.6 MB).

- **Distinguish "backtracks catastrophically" from "does O(n) work per start position".**
  Only the first is fixed by atomicity; the second needs a bound on the quantifier.
- **Measure the fix, do not reason about it.** Both readings were one `node -e` away, and
  the wrong fix survived a security review round because the story was plausible.
- **A bound on a quantifier is a new false negative unless you handle the cap.** `{0,200}`
  made a figure with 250 characters of alt text report as "no figure", which gated off the
  cues that depended on it and marked the lesson drivable. Treat "the cap was reached" as
  evidence the thing IS there, not evidence it is absent.

## The agent scratchpad is shared across concurrently running agents, including ones in other worktrees

Mid-task, my `scratchpad/measure.mjs` was overwritten by a different agent working in a
different worktree (`es-a1-t4`) on unrelated Spanish content. Same scratchpad path, same
generic filename. My measurement script became theirs; a re-run would have silently
measured something else, or failed in a way that looked like my own bug.

**Give scratchpad files a task-scoped subdirectory** (`scratchpad/<issue-or-slug>/`) and
copy anything you intend to diff against later into it immediately. Generic names —
`measure.mjs`, `out.json`, `tmp.txt` — are the ones that collide.

## A generator that hardcodes a shard ordinal writes a DUPLICATE the moment `--shard` moves the stride

Bit me twice in one session, in two different scripts, and the second time it threw
`duplicate ES-EXT-382-VERB id` from inside the loader — after `--check` had already
passed once.

Sharded ledgers (`core/spine.d/`, `<track>/curriculum.d/{path,extensions,spine}/`) are
named `NNNN-<ID>.json` at a canonical stride of ten, and `--shard` **recomputes every
ordinal from scratch**. Insert a spine node at A1 and everything from `SPINE-SAY-WHAT-I-DO`
onward shifts by one slot, in `core/spine.d/` *and* in all 22 track ledgers. Any script
holding a literal `"0115-"` or `"3820-"` now writes beside the renamed file instead of over
it, and you have two shards claiming one id.

The failure is quiet in the direction that matters: the *stale* copy keeps its old contents,
so a re-run "succeeds", and the corruption only surfaces when something enumerates the
directory.

**Resolve a shard by its ID, never by its ordinal:**

```js
const hit = readdirSync(dir).find((f) => f.endsWith(`-${id}.json`));
return join(dir, hit ?? `${fallbackOrdinal}-${id}.json`);
```

**Generalisable check:** whenever a filename encodes both an identity and a position, a tool
that regenerates positions makes every hardcoded filename a time bomb. Address by identity;
let the position be derived. The same shape applies to `doc-shard` ordinals, migration
numbers, and anything else with a "canonical stride".

## `parse.ts` FLATTENS nested lesson frontmatter to dotted keys — reading the nested shape returns an empty ledger, silently

Screening sixteen candidate headwords against the atom ledger, I wrote:

```js
const kn = lesson.frontmatter.introduces?.knowledge;   // always undefined
```

The lesson file really does say

```yaml
introduces:
  knowledge: [ES-LEX-BEBER]
```

but the parser stores it as the literal key `"introduces.knowledge"`. `frontmatter.introduces`
is `undefined`, the `?.` swallows it, and the screen loaded **zero atoms** — so every
candidate came back clean, including any word the course already owned through a `grammar`
lesson. That is the exact failure the atom-ledger screen exists to prevent, arriving through
the screen itself.

**What caught it was a two-directional self-test**, not the code review:

```js
if (!atoms.has("ES-LEX-BEBER")) fail("atom ledger did not load");
if (atoms.has("ES-LEX-LAVAR"))  fail("atom ledger loaded something impossible");
```

The first assertion fired. A one-directional "did I load anything?" check would have passed
on a ledger of zero, because zero *is* something.

**Rules:** confirm a parser's storage shape by printing `Object.keys(...)` before indexing
into it; and every screen that compares against a corpus needs a positive control *and* a
negative one, because "found no problems" and "looked at nothing" produce identical output.

## A `roots:` ledger only knows what a lesson CHOSE to declare — screen the PROSE too

Two candidate headwords came back clean from a three-way screen (headword list, atom
ledger, root ledger) and both were already taught. `ES-C288-vuelta` declares

```yaml
roots: []
```

and then spends *volvere*, *volume*-as-a-rolled-scroll and *revolve* across its gloss,
its `etymology_hook` and its body. `ES-C286-dolor` does the same with *dolere*,
*condolence* and *indolent*. Neither etymon is in the root ledger, because neither
lesson listed one — so `volver` and `doler` screened as free vocabulary, and a second
lesson telling the identical story would have shipped.

The root ledger is **opt-in metadata**. The prose is where the teaching actually is.

**Index the text as a fourth ledger**: gloss + `etymology_hook` + body, lowercased and
NFD-stripped so `saccāre` matches `saccare`, matched on word boundaries against each
candidate's proposed etymon *and* its intended English payoff. Over one track that is
~1,000 short documents — a regex sweep, not a search problem.

It caught six more on the same run: `llorar` (*plorare*, already told by `el llanto`),
`mover` (*movere*, by `el momento`), `pintar` (*pingere*, by `el pimiento`), `firmar`
(*firmus*, by `enfermo`), `curar` (*secure*, by `seguro`) and `dibujar` (by `el bosque`).
None was visible to any other screen.

**Expect false positives and read them.** An English-cognate hit may be an incidental
use of an ordinary word — "collocation" as a taught linguistics term, "increase" in a
plain sentence — rather than an etymological claim. The SOURCE-etymon hits are
decisive; the cognate hits are a prompt to go look. Auto-dropping on both costs good
candidates.

**Generalisable:** whenever a corpus has a structured ledger *and* free text that can
carry the same commitment, the ledger is a lower bound and the text is the truth.
Screening against the ledger alone measures how diligently authors filled it in.

## Discharging a criterion can DELETE the only test that proves it works

Authoring sixteen verbs cleared `verb-vocabulary` for Spanish — and Spanish was the
only track in the corpus that criterion had ever failed. The suite went green with
`verbVocabularyOf` no longer asserted anywhere: delete the function, delete the
blocker, and every test still passes.

This is the failure mode of pinning a gate to real corpus data. The pin is excellent
evidence right up until somebody fixes the corpus, and then it silently becomes no
evidence at all — with no failing test to announce the transition.

**When a tranche closes a criterion's last real-world failure, it owes the criterion a
synthetic one in the same PR.** The fixture here is a track that satisfies the criterion
it partitions *exactly* and misses the composition floor by one, asserting the blocker
is its sole failure, plus the counterfactual: retag that one lesson and the level is
attained. Authoring can never turn a fixture green.

Same shape as an assertion pinned to a known-bad number that someone later fixes. Ask,
whenever a pin goes from red to green: **what was that pin also proving, and is anything
still proving it?**

## Paired atoms drift apart: the reviews declared one twin and never the other (human-language-data)

Spanish's A1 `reinforcement` blocker stood at 83 atoms "revisited fewer than twice."
Twenty-three of them were being revisited on the page and were never declared.

Every object-pronoun lesson introduces a **pair** — `ES-C42-lo` introduces
`ES-LEX-LO-OBJECT` *and* `ES-GRAMMAR-DIRECT-OBJECT-LO`, and *la*, *te*, *nos*, *os*,
*le* do the same. Every downstream review then declares the `ES-GRAMMAR-*` twin and
none of them declares the `ES-LEX-*` twin, while printing the word itself in its own
recap table:

```
| it (masculine) | **lo** | *lo tengo* — I have it |
| you            | **te** | *te quiero* — I love you |
```

The word is on the page. The ledger said it had not been seen since the chapter that
taught it. Closing those cost **no new prose at all** — only
`practises.knowledge` plus the body block's `assesses`, which is what `curriculum.ts`
already requires them to agree on.

This is the `roots: []` lesson one field over, and the general form is worth stating:
**when a lesson introduces two atoms for one teaching moment, nothing keeps later
lessons citing both.** The one that names the *rule* gets cited, because reviews are
written about rules; the one that names the *word* gets dropped. Any `X-LEX-*` /
`X-GRAMMAR-*` pair in any track is a candidate. Diff the two twins' revisit counts.

**Reject about a third of what the screen proposes, and expect to.** A prose match is
a prompt to go read the line, not a verdict. *ocho* in "Tengo ocho años" does not
revisit ROMAN-MONTH-NAMES; *libro* in "Un libro" does not revisit ORDINAL-APOCOPE;
*grande* in "muy grande" does not revisit the exclamative `¡Qué grande!`. Each of
those would have "closed" an atom and taught nothing — **the detector matching rather
than the teaching**. A wiring pass that accepts every hit is indistinguishable from
one that edits the frontmatter at random and is worse than leaving the atom open,
because it retires the atom from the report.

**Wiring has a hard ceiling, and it is the prerequisite closure.** A practised atom
must already be in the carrier's transitive `prerequisites` closure
(`schema-v2-practice-before-introduction`), and Spanish's chains break at chapter
boundaries — `ES-C346-escuela` reaches back to `ES-C334-edad`, skipping 345 entirely.
So "some later lesson says the word" is not sufficient; check closure first, or the
validator rejects the edit.

**Corollary for reinforcement specifically:** the windows are only judged where the
track is long enough to contain them, so the last lessons are never measured. Measure
that tail explicitly rather than reasoning about it — Spanish's came to exactly two
atoms, both in the final chapter, and both become blockers the moment anything lands
after them.

## Criterion 4 counts distinct later LESSONS, so a zero-revisit atom needs TWO reviews — plan against slots, not atoms

The HL09 §3.1 reinforcement criterion is `revisits < 2`, and `measureContinuity`
computes `revisits` as **the number of distinct later lessons whose
`practises.knowledge` names the atom**. A single review lesson therefore contributes
**at most one** revisit to any given atom, no matter how many times it drills it.

The consequence is that the open-atom count is not the size of the job. Spanish's A1
residue read as **59 atoms** and was **72 slots**, because every atom sitting at
`revisits=0` needs two further passes rather than one. The gap is not evenly spread:
chapters 100–199 held 11 atoms in 18 slots with **seven at zero revisits**, so a
single review there would have left all seven open *while appearing to cover the
cluster* — the atoms named in the frontmatter, the lesson landed, the number barely
moving. That is the same failure mode as a wiring pass that accepts every prose hit:
coverage that looks complete because the surface count matches, while the underlying
requirement does not.

Plan against slots, verify against slots, and report both numbers so the discrepancy
cannot hide. `slots = Σ (2 − revisits)` over the atoms the gate calls thin.

Two related facts about the gate, both easy to get backwards:

- **It is a count, not a window check.** An atom can keep its `ReinforcementDefect`
  (it still misses R2/R3/R4) while the gate's verdict flips, because only `revisits`
  moved. Do not read "the defect is gone" as the success condition.
- **An atom in a stretch too short to contain even R1 produces no defect at all**, so
  it is invisible to criterion 4 rather than failing it. That is why the tail must be
  measured explicitly and why appending a lesson can *manufacture* a blocker.

## A lesson's `gloss` and `etymology_hook` reach the generated `.tex`, so prose gates read them — and `npm run validate` does not

`standalone-book`'s cross-volume claim scan (the "the course"/"the curriculum" ban,
lessons above) runs over the **generated book surfaces**, not over lesson body blocks.
Frontmatter `gloss` and `etymology_hook` are rendered into those surfaces, so a phrase
that is forbidden in prose is equally forbidden in those two fields — and neither the
`banned-words` scan (which reads `blocks[].markdown`) nor the 18-test corpus validator
(`npm run validate`, i.e. `tests/integration.test.ts`) looks at them.

A review lesson shipped on this basis: it passed `validate` cleanly, and only the full
`vitest run` caught `this course` twice in its frontmatter. **`npm run validate` is not
the gate; it is one of 105 test files.** Run the whole suite before believing a new
lesson is clean, and remember that "learner-facing prose" for the purposes of any given
gate is whatever *that* gate's surface is — the corpus has at least three different
answers (block markdown, generated `.tex`, narration export).
## A duplicate screen must decompose multiword headwords, and a confusability screen must not

Screening a candidate word against the corpus has two independent failure modes, and the
Spanish A1 exam tranches have now been bitten by both.

**Miss-mode five: a word owned only as a fragment of a multiword headword.** Four
already-owned traps were already on record, each invisible to a headword-only screen —
`llevar` inside `ES-C39-traer`, `andar` inside `ES-C36-caminar`, `dar` in the `grammar`
lesson `ES-C65-di`, `llover` in the `phrase` lesson `ES-C30-llueve`. The fifth is
different again: **`amigo` is owned by `ES-C09-falsos-amigos`**, whose headword is the
two-word term of art *falsos amigos*. No headword screen sees it, no atom-id screen sees
it, and the root ledger does not either. **Only a screen that splits multiword headwords
into their component words finds it.** Screen on articles, compounds, `+` patterns, U+2026
ellipsis, and morphology — and treat a fragment as ownership.

**But that same decomposition, applied to CONFUSABILITY, invents drops.** The two
questions need two indexes and it is tempting to build one:

- *Is this already taught?* — the **wide** index. Fragments count, because a word the
  corpus utters anywhere is a word the learner has met.
- *Will a learner conflate this with something we teach?* — the **narrow** index, whole
  displayed headword forms only. A fragment was never presented as a word, so it cannot be
  the thing the learner confuses it with.

Conflating them dropped `menor` against `mejor`, where `mejor` occurs only inside the idiom
*pasar a mejor vida* and is never taught as a word. One index gives a false duplicate or a
false drop depending on which way you lean it; build both.

## A screen calibrated on surplus changes meaning when the pool becomes the requirement

The Spanish confusability rule — *a same-length pair differing in one position is a drop
only when the differing position is not the first* — was derived in vocabulary tranche 6,
which screened roughly a hundred candidates to place thirty-five. Dropping `el codo` there
cost nothing: a replacement was waiting.

Applied unchanged to an **exam-derived** list, where every entry is on the list because a
measured item requires it, the identical rule became a veto on passing the exam. Ten of 103
candidates flagged, and dropping all ten moved a mock from `APTO` to `NO APTO`. Nothing
about the rule changed. **What changed is that there was no longer anywhere to substitute
from — and nothing in the rule's statement said it had depended on that.**

The verdict is therefore context-dependent, and must be written down as such:

- **drop** when the pool has surplus;
- **disambiguate** when the candidate is required.

`tren`/`tres`, `pollo`/`polvo`, `playa`/`plaza`, `costar`/`contar` are minimal pairs, and
the standard way to teach discrimination is to present the pair and make the contrast the
lesson. That turns every one of the ten liabilities into an asset. **A screen that always
drops silently shrinks the curriculum toward whatever is easy to teach**, and it does it
without ever reporting that it made a curricular decision.

The general form: **any filter tuned against an abundant candidate pool encodes an
unstated assumption that substitutes exist.** Re-derive its action — not its criterion —
whenever the pool stops being abundant.

## A hash mismatch with no visible content difference: compare byte counts to line counts

`check:books` failed in CI with `ch394-...tex: generated output is missing or stale`, while the
identical check passed locally with exit 0. That phrasing reads like a generator bug, and the
first instinct is to suspect the generator or the environment. It was neither.

**The diagnostic that settled it in one step:** compare each source file's working-tree size to
its blob size, and compare the difference to the file's line count.

| lesson | worktree | blob | delta | lines |
|---|---|---|---|---|
| `ES-C394-guitarra` | 5652 | 5546 | **106** | 106 |
| `ES-C394-medico` | 5469 | 5354 | **115** | 115 |
| `ES-C394-universidad` | 5309 | 5309 | 0 | 103 |

**Delta exactly equal to the line count means exactly one byte per line was dropped**, which is
CRLF in the working tree against LF in the blob. Nothing else produces that signature. The
generator hashes lesson *sources*, so the hash computed on Windows against CRLF could never match
the hash CI computes against the LF blobs — and only the chapter containing those files went
stale, which is why the failure looked oddly narrow.

`.gitattributes` had `text=auto, eol=lf` all along, and `git add` printed
`CRLF will be replaced by LF the next time Git touches it` for exactly those files. **That warning
is the whole diagnosis, printed in advance and scrolled past.** Read the add warnings.

## A `--check` gate that reads only the working tree cannot see this class at all

`book-cli --check` regenerates from the working tree and compares to the working tree. When both
sides carry the same CRLF, it agrees with itself and exits 0 — **the disagreement it needs to
find is between the working tree and the blob, and it never looks at the blob.** So the gate is
green locally and red in CI, by construction, for every line-ending or filter-normalization skew.

This is a real blind spot, not merely an operator error: a check that validates one copy against
itself is vacuous with respect to what will actually be committed. The fix worth considering is
for `--check` to compare against `git show :path` (the staged blob) rather than the file on disk.

## A filter must be proven against a known-positive input, never only a quiet one

Two filters lied in the same session, both by silently matching the wrong thing while looking healthy.

**`gh pr checks` is TAB-separated and check names contain spaces.** So `awk '$2=="fail"'` tests the
*second word of the check name* — `message`, `channel`, `17` — and can never equal `fail`. A PR
monitor built on it reported healthy while a required check was failing. Use `awk -F'\t'`.

**`grep -c $'\r'` does not count carriage returns** where `$'...'` is not expanded: grep receives
the two characters `\r`, and in a basic regex `\r` is just a literal **r**. Every line containing
the letter *r* matches — which yields a count *close to the file's line count* and therefore looks
exactly like a plausible CRLF count. It reported CRLF in all 35 files when only 3 had it, and it
did so with numbers convincing enough to be quoted in a report. Count bytes with a real tool
(`node`, `xxd`, `file`) instead.

The general rule: **a filter that has only ever been observed silent has not been observed
working.** Before trusting one, run it against an input you know is positive and confirm it fires.
Both bugs above would have died instantly under that test, and both survived because the only
evidence collected was "it didn't complain."

### Correction: a known-positive is NOT sufficient, and the remedy above was itself unsound

The paragraph above prescribes one test — run the filter against a known-positive. **That test
passes on a filter that is still broken**, and the same `grep -c $'
'` proves it.

Re-run in Git Bash here, the command fails a *third* way. Not "matches the letter `r`": the
pattern degrades to **empty**, and an empty pattern matches **every line**.

```
printf 'a
b
' > pos.txt ; grep -c $'
' pos.txt   # 2   <- known-positive: looks correct
printf 'a
b
'     > neg.txt ; grep -c $'
' neg.txt   # 2   <- known-NEGATIVE: same answer
```

`neg.txt` contains no carriage return *and no letter `r`*. A known-positive test alone returns 2
and reads as a pass. Only the **known-negative** exposes it. So the rule needs both arms:

> **Prove a filter against a known-positive AND a known-negative, and require the answers to
> DIFFER.** A filter that cannot distinguish the two is measuring something else, whatever it
> prints on the positive case.

This also breaks the earlier entry's reasoning that a bogus count is recognisable because it lands
"*close to* the file's line count". Under the empty-pattern failure it lands **exactly on** the
line count, which is indistinguishable from a file that is genuinely all-CRLF.

**Use a byte-level check instead**, which has no pattern to degrade:

```
python -c "b=open('f','rb').read(); print(len(b), b.count(b'
'), b.count(b'
'))"
```

The wider point, and the reason this is a correction rather than a new entry: **#13190 documented
one failure of this command and then left behind a remediation carrying the same defect.** A wrong
lesson recorded as a lesson is worse than no lesson, because the next reader trusts it instead of
re-deriving it. When an entry prescribes a fix, the fix needs the same evidence the diagnosis got.

## A duplicate screen run AFTER authoring reports the tranche to itself

The Spanish A1 qualities tranche screened its candidates against the corpus with a wide index —
headwords, atom ids, concept tags, multiword fragments, the root ledger. Run again after the
twelve lessons were written, **every new word came back `DROP (already owned)`**, and the owner
named in each case was the tranche's own file. The screen was green, fluent, and worthless.

**A duplicate screen must read the corpus as it stood BEFORE the tranche** — `origin/main`, or the
lesson set minus the files under authorship. A screen that includes its own output has redefined
"already taught" to include "taught by this very commit", and it will report a clean drop-everything
verdict no matter what you feed it. This is the same silent-pass shape as the filter entries above:
the failure mode is a confident answer, not an error.

## Anything you author is a candidate for the duplicate screen — including words you added to satisfy a checklist

The same tranche screened **76 exam-derived candidates** and found eight already-owned. It then
added **six more words** of its own, to close three syllabus inventory points that justified a new
spine node, and screened none of them. One — `feo` — was already taught at A1.

It was caught by **arithmetic, not by the screen**: the headword count came out 685 where twelve
new lessons predicted 686. The screen had never been pointed at the word.

Two rules fall out:

- **The candidate set is everything the tranche will author**, not the subset that arrived through
  the process that motivated the tranche. Words added to satisfy a coverage target, a reviewer, or
  an inventory point are candidates on exactly the same footing.
- **Keep a corpus-wide duplicate-headword invariant in the pre-commit sweep.** One line — does any
  new headword equal an existing headword, case-folded — catches what a bespoke multi-index screen
  missed, and it cannot be pointed at the wrong set because it has no set to be pointed at.

## A rule adopted after the defect exists protects only the future — someone has to sweep the past

`HL23` §12.2 refused, in writing, the option of filing adjectives under `SPINE-COUNT-ONE-TO-FIVE`
("*the cardinal numbers one through five*"), calling it "precisely the mis-filing this programme
exists to undo", and recorded the refusal "so that the next tranche does not rediscover it as a
shortcut."

**It had already happened, at scale.** Twenty-two quality adjectives — `alto`, `gordo`, `alegre`,
`feo`, `necesario`, `dulce` — were sitting on the numbers node when that paragraph was written.
The refusal was phrased forward-looking, so nobody looked backwards.

This is the second instance of the shape in the same programme: #13154 found
`SPINE-SAY-WHAT-I-WANT` mis-staged at A2 by a prerequisite nobody had checked. **When you write
down a rule prohibiting a defect, the same change must sweep the corpus for existing instances,
because the rule's own existence is evidence the defect was attractive enough to commit at least
once.** Ban and audit are one task, not two.

## `git worktree remove --force` followed npm's junctions and deleted three packages in ANOTHER worktree

To test whether a failing test was pre-existing, a throwaway worktree was created at `origin/main`
and seeded by copying `node_modules` from the working worktree, to skip a slow `npm ci`:

```
cp -r $WORK/packages/typescript/$p/node_modules $TEMP/packages/typescript/$p/node_modules
```

The identity test worked and answered the question. Then:

```
git worktree remove --force $TEMP
```

…and **three package source directories vanished from the WORKING worktree** —
`paint-instructions`, `paint-vm`, `pixel-container`, 29 tracked files, none of them anywhere near
the temporary tree.

**Why.** These packages depend on each other by `file:` reference, so npm materialises
`node_modules/@coding-adventures/<pkg>` as a **directory junction** pointing at the sibling
package's real directory. Copying `node_modules` copied junctions that still pointed **into the
original worktree**. `git worktree remove --force` deletes the tree recursively, walked into a
junction, and deleted the target's contents.

This is the same hazard `shard-cli.ts` documents for `existsSync` + `rmSync` — *"entries reached
THROUGH a symlinked parent report `isSymbolicLink() === false`"* — arriving from a completely
different direction. There, the guard was `lstatSync`. Here, there is no guard to add: the mistake
was **copying a tree containing junctions to somewhere it would later be recursively deleted.**

**Rules:**

- **Never `cp -r` a `node_modules` that contains `file:` deps between worktrees.** The junctions do
  not get rewritten and they point back at the source. Run `npm ci` in the new tree, or seed it
  with `cp -r --dereference` / `robocopy /SJ`, which copies the junction as a junction rather than
  following it.
- **Before `git worktree remove --force`, check for junctions:** on Windows,
  `cmd //c dir /AL /S <path>` lists reparse points. If any point outside the tree, delete
  `node_modules` first and then remove the worktree.
- **It was recoverable only because the files were tracked and unmodified.** `git checkout -- <dirs>`
  restored all 29. Had those directories held uncommitted work, `--force` would have destroyed it
  with no warning and no prompt.

The wider point: the identity test was the right call and produced the right answer. **The
shortcut taken to make it cheap was the dangerous part**, and it was dangerous in a way that had
nothing to do with what was being tested.
## A TypeScript package can pass its own typecheck and fail a stricter source-importing consumer

Splitting Script Ductus into owner modules passed the package's `tsc --noEmit`, but Language
Ladder imports the package's TypeScript source through a `file:` dependency and compiles it under
the app's stricter `noUnusedLocals` setting. Broad type-only imports left by the mechanical split
therefore failed the downstream typecheck even though the library was green.

**Rule:** after a TypeScript source split, run the typecheck of every source-importing `file:`
consumer, not only the changed package. Consumer compiler options can be stricter. Remove imports
with zero use sites rather than weakening either tsconfig; then run the consumer's full BUILD,
including bundle checks that package tests cannot reach.
