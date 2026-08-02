# Lessons Learned

A condensed quick-reference of mistakes made during development, grouped by category. Read this file before starting work that touches BUILD files, CI, native extensions, or any of the language-specific pitfalls below. Entries are kept short on purpose — when a rule recurs, the canonical entry is here, not buried in chronology.

---

## BUILD files & dependency management

- **Each BUILD line runs as a separate `sh -c` (Unix) / `cmd /C` (Windows) process.** `cd` and shell variables do NOT persist between lines. Chain with `&&` on one line, use subshells `(cd ../dep && ...)`, or keep each line absolute. Multiline `if/then/fi`, `for`, and backslash continuations all break — the runner sees `\` as a literal command and fails with `\: not found`.
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

## Workspace & package metadata

- **Rust workspace `Cargo.toml` `members` must match what's pushed.** Listing a member whose dir hasn't been pushed breaks the entire workspace in CI (`failed to load manifest`). Crates with their own `[workspace]` (node-bridge, python-bridge, ruby-bridge) must be EXCLUDED from the parent — including them gives "multiple workspace roots". After merge conflicts on `members`, dedupe — modern CI rejects duplicate entries even though older Cargo tolerated them. Run `cargo build --workspace` to catch missing exports; expect platform-only crates (paint-vm-direct2d, paint-vm-gdi) to fail compile on the wrong OS — that's not a regression.
- **Keep the Rust toolchain current.** External deps adopting Edition 2024 require `rustup toolchain install stable` before declaring the workspace broken.
- **Don't put `@ file:../path` in Python `pyproject.toml` dependencies.** Hatchling rejects them, and even with `allow-direct-references = true`, uv resolves the relative path from a temp build dir. Use bare names + BUILD pre-installation + `[tool.uv.sources]` for local-path redirection.
- **Python downstream tests should not assert exact dependency versions.** Assert minimum-compatible (`__version__ >= "0.3.0"`) or capability — exact-version asserts fail when a foundational package bumps and downstream gets force-rebuilt.
- **TypeScript `package.json` must use `"main": "src/index.ts"`** (not `dist/index.js`) because Vitest resolves `file:` deps via `main` and we don't pre-compile. Also: `"type": "module"`, `@vitest/coverage-v8` in devDeps, run real coverage gate locally before pushing. Never commit `.js`/`.d.ts` transpile outputs alongside `.ts` sources.
- **Vite-based TS programs with `file:` deps must NOT use `tsc -b` in build script.** `tsc -b` follows imports into nested `node_modules` (npm copies, not symlinks on Windows) and fails on un-installed transitives. Use plain `vite build`; type-check via vitest.
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
- **Fixture document IDs and executable node IDs can require different grammars.** Corpus IDs commonly use descriptive hyphens (`nn27-dynamic-graph-and-saved-values`), while graph node IDs benefit from a tighter identifier grammar for portable map keys. Reusing the node-ID regex for the top-level lab ID rejected a valid checked-in fixture before execution. Validate each namespace according to its contract instead of sharing the strictest helper by convenience.
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
  (5) **The gate runs on EVERY affected Rust package the build tool knows, not just the `code/packages/rust` workspace.** `code/programs/rust/*` are separate cargo projects and wasm/rust packages too; a `--workspace` clippy in `packages/rust` misses them all. It also surfaces pre-existing host-un-buildable crates (`os-kernel`: `#![no_std]` + crates.io `uefi` dep with `panic_handler` → `cargo test` can never link on a std host) — guard such a crate's BUILD to skip on hosts (it targets `x86_64-unknown-uefi`).
- **Getting a large workspace to zero clippy warnings: `cargo clippy --fix` first, but it stops at the first deny-by-default hard error.** `absurd_extreme_comparisons`/`approx_constant`/`not_unsafe_ptr_arg_deref`/`never_loop` are deny-by-default, and a deny error aborts that crate's compile so `--fix` can't touch its other (machine-applicable) warnings. Clear/allow the hard errors first, then re-run `--fix`; crates that previously failed to compile now get auto-fixed. `--fix` only applies `MachineApplicable` suggestions — `approx_constant` (replacing `3.14159` with `PI` changes the value) is `MaybeIncorrect`, so it is NEVER auto-fixed; resolve those with a scoped `#[allow(clippy::approx_constant)]` + justification, never by editing the literal (it's usually test data / codegen input / an intentional hand-written constant). FFI crates that expose raw-pointer C ABIs (`node-bridge`, `ruby-bridge`) get a crate-level `#![allow(clippy::not_unsafe_ptr_arg_deref)]` with a comment rather than ~80 per-fn annotations.
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

## sql-execution-engine — grammar drift between a shared grammar and its hand-written tree-walker

- **A hand-written AST tree-walker that switches on rule names will silently rot when the shared grammar grows new precedence layers — and the failure mode is `nil`, not a parse error.** `sql-execution-engine` (Go) walks the `sql-parser` AST with a `switch node.RuleName` in `evalExpr`. Over many PRs (#4055–#4164) `code/grammars/sql.grammar` adopted SQLite's full operator-precedence ladder, inserting two new rules — `collated` (`COLLATE` postfix) and `bitwise` (`& | << >>`) — *between* `comparison` and `additive`, and rewrote `comparison` to take `collated` operands. The evaluator had no `case "collated"`/`case "bitwise"`, so every expression fell through to `default: return nil` *before reaching* `column_ref`. Result: `SELECT id, name` returned `<nil>` for every cell, `WHERE` matched zero rows, and `TestWhereNumericComparison` panicked on an empty slice. `SELECT *` kept working because it reads the row map directly and never touches `evalExpr` — a misleading "half the package works" signal. Separately, `limit_clause` changed from bare `NUMBER` tokens to `signed_number` child nodes (plus negative-LIMIT-means-unbounded and MySQL `LIMIT m, n`), so `executeLimit` found no NUMBER tokens and ignored LIMIT/OFFSET entirely. **Why it lay hidden**: the Go build tool only rebuilds/tests packages whose diff touches them or a transitive dep, so the break stayed dormant until an unrelated `go/lexer` change re-triggered the engine's tests on a `main` CodeQL build. **Fix pattern**: when you add a precedence layer to a shared `.grammar`, grep every consumer that walks the parse tree by rule name (`grep -rl 'RuleName' code/packages/*/sql-*`) and add the passthrough/eval case in the same PR. A new wrapper rule needs at minimum a passthrough case (`evalCollated` just evaluates its inner node); operator layers need real evaluation (`evalBitwise`, `||` concat, unary `~`/`+`). **Prevention idea**: give the tree-walker a `default:` that returns a sentinel error instead of `nil`, so an unhandled rule surfaces as a loud `EvaluationError` rather than silent NULLs — file a follow-up to do this across the grammar-driven Go engines.

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
