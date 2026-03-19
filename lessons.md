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

### 2026-03-18: Always add BUILD files and DIRS entries for new packages

When creating a new package, you MUST:
1. Create a `BUILD` file in the package directory with the test command
2. Add the package directory name to the parent `DIRS` file
3. Verify the build tool discovers the new package

Without both, the CI build tool will not discover or test the package. This was missed for fp-arithmetic, Go logic-gates, Ruby sequential logic, and clock packages — they passed locally but were invisible to CI.

**Checklist for every new package:**
- [ ] BUILD file with test command
- [ ] Added to parent DIRS file
- [ ] `./build-tool -dry-run` shows the package

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
