# Lessons Learned

This file tracks mistakes made during development so they are not repeated. Check this file before starting any new work.

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
