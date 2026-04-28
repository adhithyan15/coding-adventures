# Lessons Learned

A condensed quick-reference of mistakes made during development, grouped by category. Read this file before starting work that touches BUILD files, CI, native extensions, or any of the language-specific pitfalls below. Entries are kept short on purpose — when a rule recurs, the canonical entry is here, not buried in chronology.

---

## BUILD files & dependency management

- **Each BUILD line runs as a separate `sh -c` (Unix) / `cmd /C` (Windows) process.** `cd` and shell variables do NOT persist between lines. Chain with `&&` on one line, use subshells `(cd ../dep && ...)`, or keep each line absolute. Multiline `if/then/fi`, `for`, and backslash continuations all break — the runner sees `\` as a literal command and fails with `\: not found`.
- **BUILD files must install ALL transitive local deps in leaf-to-root order.** Single most-recurring repo-wide failure (re-learned 8+ times across Python/Ruby/Go/TypeScript/Lua/Perl/Elixir/Rust). CI starts with empty `node_modules`/venvs, so every transitive sibling needs an explicit install line before the package's own. After adding a new low-level package, every package up the chain needs its BUILD updated. Use the scaffold generator — it computes the closure for you.
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

## Rust

- **Recursive local functions need a 2-step declaration.** Short assignment `addConstant := func(...)` can't reference itself. Use `var addConstant func(...)` then `addConstant = func(...)`. (Same pattern in Go.)
- **Validate caller-controlled lengths before `int` casts.** Binary parsers must explicit-bounds-check `u4`/`u8` lengths against host capacity; never recursively decode nested structures unless the format requires it.
- **Don't run `cargo fmt --all` for package-scoped work** — it reformats hundreds of unrelated crates and buries the feature diff. Use `cargo fmt -p <pkg>`.
- **wasm-bindgen `JsValue::from_str` aborts on native test targets.** Gate behind `#[cfg(target_arch = "wasm32")]`; use `JsValue::NULL` placeholders for native error-path tests.
- **FFI input enums must be primitive ints, never `repr(C)` Rust enums.** Foreign callers can pass any bit pattern; observing an out-of-range Rust enum is UB before validation runs. Use `u32`/`c_int` in the ABI struct, then `TryFrom`.
- **Linux `epoll_event` is packed.** A plain `#[repr(C)]` mirror works for single events but corrupts/drops readiness when `epoll_wait` returns multiples. Always use the kernel's packed layout.

## Native extensions & FFI

- **Ruby `QNIL = 0x04` on 64-bit Ruby (USE_FLONUM), not `0x08`.** The pre-FLONUM `0x08` causes Ruby to dereference it as an object pointer (klass at `+8` → SIGSEGV at `0x10`). Constants: `QFALSE=0x00, QNIL=0x04, QTRUE=0x14, QUNDEF=0x24`. Confirm against `ruby/internal/special_consts.h`. When a Ruby native ext SIGSEGVs at low addresses like `0x10`, suspect a special-constant bit-pattern bug.
- **Lua 5.4 `LUA_REGISTRYINDEX = -1_001_000`** (derived from `-LUAI_MAXSTACK - 1000`), NOT the Lua 5.1 value `-10000`. Using `-10000` in `luaL_ref` treats it as a regular negative stack index, landing 10000 slots below the frame and causing SIGBUS/SIGSEGV.
- **Lua userdata GC + raw `luaL_ref` integers**: integer slots aren't tracked by the GC. If Rust holds `i32` registry refs derived from a userdata's state, pin the userdata itself in the registry (extra `lua_pushvalue` + `luaL_ref`) and unref it only after all integer refs retire — otherwise Linux's aggressive incremental GC collects the parent and your slots become nil mid-flight.
- **Lua `__gc` metatable attachment**: do NOT `push_cstr("__gc")` before `lua_rawset_str_top` — the function supplies the key. Pattern: `luaL_newmetatable; lua_pushcclosure(gc_fn); lua_rawset_str_top(-2, "__gc\0"); lua_setmetatable(-2)`.
- **CPython type-slot numbers must match `Include/typeslots.h` exactly.** Wrong slots cause silent memory corruption / `UnicodeDecodeError` / access-violation crashes during module load. Numbers are NOT sequential per category; verify each. Examples: `Py_tp_hash=59`, `Py_tp_iter=62`, `Py_nb_and=8`, `Py_nb_or=31`.
- **Python C API `long` is `c_long`** — on Windows x64, `c_long == i32`, not `i64`. Always use `std::ffi::c_long` for `PyLong_AsLong`/`PyLong_FromLong`/`PyModule_AddIntConstant`. Hardcoding `i64` fails Windows compile.
- **OTP 26 Linux**: `enif_get_int64`/`enif_make_int64` are NOT reliably exported from `beam.smp` — declaring them gives `undefined symbol` at NIF load. Use `enif_get_long`/`enif_make_long` (always exported); on 64-bit POSIX they're equivalent.
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
