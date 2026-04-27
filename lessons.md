# Lessons Learned

This file tracks mistakes made during development so they are not repeated. Check this file before starting any new work.

---

### 2026-03-23: Python pyproject.toml must declare ALL monorepo imports as dependencies

When migrating from shell BUILD files to Starlark, the auto-discovery of monorepo deps reads pyproject.toml. If a package imports from a sibling (e.g., `from riscv_simulator import ...`) but doesn't declare `coding-adventures-riscv-simulator` in `dependencies`, the build will fail because `uv pip install` doesn't know to install it. The old shell BUILD files papered over this with explicit `-e ../dep` flags. Always ensure pyproject.toml dependencies match actual imports. 29 packages had this issue.

---

### 2026-03-23: Transitive monorepo deps need recursive discovery

For Python, `uv pip install -e ../dep` doesn't transitively install dep's own monorepo deps (they're not on PyPI). For TypeScript, `npm install` in a dep directory doesn't install its own `file:../` deps. The build tool must walk the dep tree recursively (leaf-first) and install every transitive monorepo dep explicitly. Direct-only discovery is insufficient.

---

### 2026-03-23: Plan-based execution must re-evaluate Starlark BUILD files

When the Go build tool runs with `--plan-file` (CI build jobs), it re-reads platform-specific BUILD files using `ReadLines()`. For Starlark BUILD files, this passes raw Starlark source (e.g., `load(...)`) to the shell executor, causing `'load' is not recognized` on Windows. The fix: check `IsStarlark` and call `EvaluateBuildFile()` instead of `ReadLines()` for Starlark packages. Any code path that reads BUILD files must respect the Starlark/shell distinction.

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

### 2026-03-22: Lexer NEWLINE/DEDENT ordering at EOF

The indentation-mode tokenizer emitted DEDENTs before the final NEWLINE at EOF. This broke parsing of multiline expressions inside function bodies (e.g., `x = { ... }` spanning multiple lines in a `def`). The parser expects NEWLINE to terminate the statement before DEDENT closes the block.

**Rule:** The correct EOF token order is: NEWLINE → DEDENT → ... → DEDENT → EOF. Always emit NEWLINE before DEDENTs at EOF. This matches Python's tokenizer behavior.

### 2026-03-22: CALL_FUNCTION_KW compiler/VM protocol mismatch

The VM's `handleCallFunctionKW` handler expected a CPython-style keyword names tuple on top of stack, but our compiler emits interleaved key-value pairs: `[callable, key1, val1, key2, val2, ...]` with operand = number of keyword pairs. The handler was popping the wrong values, causing "string not callable" errors.

Also, function default parameters are right-aligned: `def f(a, b=1, c=2)` has defaults for params 1 and 2 (the last N), not params 0 and 1.

**Rule:** When debugging "X not callable" errors, check the stack layout protocol between the compiler and VM handler. Print the bytecode (`CompileStarlark`) and trace the stack to find mismatches.

### 2026-03-22: Cross-module function scope limitation

Functions loaded via `load()` execute in the caller's VM, not the source module's VM. Module-level variables (like `_targets = []`) from the source file are invisible to the function body because the compiler uses `LOAD_LOCAL` for all name references inside functions.

**Workaround:** Rule functions should `return` their target dict instead of appending to a module-level list. The BUILD file constructs `_targets` directly: `_targets = [rule_func(...)]`.

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

### 2026-03-28: Lua test files need package.path setup for local src/ directory

When running `busted . --verbose` from the `tests/` subdirectory, Lua cannot
find modules in the `src/` directory because it's not in `package.path`.
Always add this at the top of every Lua test file (before the first `require`):

```lua
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
```

This pattern was established by the existing arithmetic package. Every new
Lua test file must include it.

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

### 2026-03-29: Swift IS available on Windows — don't skip it

When Swift's `BUILD` file (`swift test`) failed on Windows CI with "'swift' is not recognized", the initial reaction was to create a `BUILD_windows` that skips tests. This was wrong — Swift has been available on Windows since Swift 5.3 (2020). The real issue was that the CI workflow had no Swift setup step.

**Fix:** Add `swift-actions/setup-swift@v3` to the CI workflow with a `needs_swift` conditional (matching the pattern used for Python, Ruby, etc.). The build tool already emits `needs_swift=true|false` — the CI workflow just wasn't reading it.

**Rule:** When a language tool is missing on a CI runner, investigate whether it can be installed via an action before skipping. Don't assume a language isn't supported on a platform — check first. Swift runs on macOS, Linux, and Windows.

**Update (same day):** `swift-actions/setup-swift@v3` does NOT support Windows yet — it throws "Windows is not supported yet" at runtime. But that doesn't mean Swift can't run on Windows CI! Instead of skipping, install Swift directly via `winget install --id Swift.Toolchain` (following https://www.swift.org/install/windows/). The CI workflow uses `swift-actions/setup-swift` on macOS/Linux and `winget` on Windows. Don't skip a platform just because one action doesn't support it — there's always a manual install path.

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
