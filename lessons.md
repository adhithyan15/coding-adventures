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
