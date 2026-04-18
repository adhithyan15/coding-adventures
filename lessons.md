# Lessons Learned

This file is a compact reference for mistakes we have already paid for in CI or production-like tests.
Read it before starting work, and update it by tightening an existing bullet whenever possible instead of
appending another long incident report.

## How to maintain this file

- Prefer short rules over narratives.
- Group new lessons under an existing section when they share the same root cause.
- Record the current canonical rule, not every historical workaround.
- If a rule changes, replace the old one instead of keeping contradictory versions.

## Workflow and Git hygiene

- Sync with `origin/main` before diagnosing CI; if local untracked work would be overwritten, use a fresh `git worktree` from `origin/main` instead of forcing a merge.
- Keep work on feature branches. Do not merge to `main` without green CI and explicit user sign-off.
- If the remote repo has no `main` yet, push an initial commit to `main` before trying to open the first PR.
- Update the PR description after each meaningful push so it matches the current state of the branch.
- Before committing, run both `git status --short` and `git diff --name-only` to catch late agent edits, unstaged files, and accidental workspace-manifest drift.
- Shared manifests and workspace files must only reference package directories that are actually present on the branch being pushed.
- Avoid reasoning from stale state in a noisy checkout; dirty worktrees are especially dangerous when editing shared manifests or workspace roots.
- Local runtimes are managed through `mise`; if a tool seems missing locally, check `mise.toml` rather than assuming the system install is authoritative.

## Generated files and staging discipline

- Never commit generated artifacts. Common offenders: `.build/`, `.swiftpm/`, `cover/`, `vendor/`, `node_modules/`, `.venv/`, `_build/`, `deps/`, `__pycache__/`, `blib/`, `MYMETA.*`, `pm_to_blib`, and other test/build output.
- Do not use `git add .` in package directories after running tests; stage explicit paths instead.
- New Swift packages need a `.gitignore` with at least `.build/` and `.swiftpm/` before the first `swift test`.
- New Elixir packages need a `.gitignore` with at least `cover/`, `_build/`, `deps/`, and `.elixir_ls/` before the first `mix test --cover`.
- Keep `.gitattributes` enforcing LF line endings; CRLF checkouts break heredoc- and snapshot-based tests across multiple languages.

## Package scaffolding and repo standards

- Always use the scaffold generator for new packages. If the generator would be wrong, fix the generator first and then regenerate.
- Every package needs `BUILD`, `README.md`, `CHANGELOG.md`, package metadata, and tests in the same change.
- After adding a package, verify the build tool actually discovers it. For multi-language packages, confirm every language variant has its own `BUILD`.
- New source files must ship with tests; otherwise coverage will fall below the repo thresholds.
- Shared infrastructure changes can trigger huge rebuild sets. Preview with `./build-tool --list-affected --diff-base origin/main` before pushing.
- Do not mass-edit working `BUILD` files just to normalize style; touching them widens the affected set and surfaces unrelated breakage.
- Pin tool versions in CI. Do not use `latest` for critical setup actions.

## Build-tool and BUILD file rules

- Treat each `BUILD` line as an independent shell invocation. `cd` does not persist between lines.
- Do not use backslash line continuations in `BUILD` files.
- Prefer the canonical minimal pattern for each language and only add extra prerequisite lines when the validator or platform needs them.
- All standalone sibling-package prerequisites referenced by `BUILD` must also be declared in the package metadata; the validator trusts metadata, not ad hoc shell commands.
- When a `BUILD` file references a sibling subdirectory such as `../foo/lib` or `../bar/src`, treat it as a reference to the sibling package root for dependency purposes.
- Use bare tool commands in `BUILD` files, not local absolute `mise` paths. CI provides its own runtimes on `PATH`.
- On Windows, shell-specific packages need a `BUILD_windows` file. The build tool runs `cmd /C`, not `sh -c`.
- In `BUILD_windows`, use Windows path separators for executable paths like `.venv\Scripts\python`.
- In `BUILD_windows`, use `set "VAR=value"` for environment variables, especially when the value contains `%CD%` or another path.
- Go `BUILD` commands must run from the package directory and use `./...`, not parent-directory patterns.
- Java/Kotlin packages must redirect Gradle output to `gradle-build/` to avoid colliding with the repo's `BUILD` file on case-insensitive filesystems.
- Do not pin a Java toolchain version in Gradle unless CI is explicitly provisioning that exact JDK.
- CI detect outputs must be normalized through `steps.toolchains`, not taken directly from `steps.detect`.
- When adding a new language to CI, update the build tool's language list, the workflow outputs, and both branches of the toolchain-normalization step.

## Dependency and workspace rules

- BUILD files must account for transitive local dependencies in leaf-to-root order whenever the package manager cannot discover them from a clean checkout.
- If a package references a sibling package in `BUILD`, declare that sibling in the language-appropriate metadata as well.
- Rust workspace `Cargo.toml` must include every crate that belongs to the workspace, and exclude crates with their own `[workspace]`.
- After adding a new Go module, run `go mod tidy` in every transitively dependent package, not just the new leaf.
- Do not add Python packages to the `uv` workspace members list unless they are intentionally sharing a workspace-root environment.
- Widely shared packages such as lexers, grammar tooling, and core infrastructure can cascade rebuilds across dozens of downstream packages; expect to fix neighbor packages if you touch them.

## Python

- Do not use `@ file:../path` direct references in `pyproject.toml`; use the package name in `dependencies` and let the `BUILD` file pre-install the sibling package.
- If a sibling package is installed from `BUILD`, declare it in `pyproject.toml` dependency metadata as well. `tool.uv.sources` alone is not enough for the validator.
- For local sibling dependencies, prefer: declare the dep by name in `dependencies`, add a matching entry in `[tool.uv.sources]`, install local siblings first in `BUILD`, then install `.[dev]`.
- Include transitive local siblings required by editable installs; otherwise `uv` will fall back to the registry and fail.
- Linux/macOS `BUILD` files should use package-local venvs and Unix paths like `.venv/bin/python`.
- Windows `BUILD_windows` files must use Windows paths like `.venv\Scripts\python` or `uv run --no-project python`, depending on the package pattern.
- Use unquoted `.[dev]`; newer `uv` rejects quoted extras syntax.
- On Windows, use `-e .[dev]` without quotes. `".[dev]"` is parsed literally by `cmd.exe`, and `.[dev]` without `-e` breaks editable path assumptions.
- For Python packages with no runtime deps on Windows, use `python -m venv` plus direct `pip` paths when `uv` workspace behavior would otherwise wipe test tools.
- Do not assume `uv run` is package-local unless you passed `--no-project`; workspace discovery can create a shared `.venv`.
- Every Python package that relies on shell-specific behavior should have a `BUILD_windows`.
- When changing a low-level Python package, inspect all affected Python `BUILD_windows` files for Windows-safe patterns before pushing.
- Do not write Python metadata tooling with naive cross-line regexes over TOML; comments containing `[` or similar syntax will break section detection.
- Python `Enum` does not accept arbitrary invalid integers. In tests, use a sentinel or `IntEnum` instead of constructing impossible enum values.

## TypeScript and JavaScript

- TypeScript package `main` fields must point at source files such as `src/index.ts`, not `dist/*.js`, unless the package is actually prebuilt before tests.
- Some older packages use a non-`index` source entry point such as `src/trig.ts`; keep `main` aligned with the real source file.
- TypeScript packages may re-export from `code/src/typescript/...`; if a package source file is just a re-export stub, edit the shared source instead.
- For packages under `code/packages/typescript`, prefer the canonical `BUILD` shape: install the package's own deps, then run Vitest. Add explicit sibling prerequisite installs only when the validator or path depth requires them.
- For programs under `code/programs/typescript`, count relative `file:` paths carefully; they are usually `../../../packages/typescript/<pkg>`.
- List all required local `file:` dependencies directly in `package.json`, including transitives when standalone installs need them.
- Keep `package-lock.json` in sync if the `BUILD` file uses `npm ci`; otherwise use the repo's accepted install pattern for that subtree.
- Do not put raw glob patterns like `src/**/*.py` inside TSDoc examples; they can break comment parsing.
- Exclude build scripts such as `scripts/**` from Vitest coverage if they are not meant to be executed by tests.
- For Vite-based apps with `file:` source dependencies, do not use `tsc -b` in the build script; Vite should handle the build.
- JavaScript bitwise operators are signed 32-bit. Guard masks and widths explicitly, and use `>>> 0` when you need an unsigned result.

## Ruby

- Ruby entry points must `require` external package dependencies before loading the package's own files.
- Predicate methods use a `?` suffix. Do not port boolean method names literally from other languages.
- `include SomeModule` belongs at class or module scope, not inside a method body.
- The gemspec dependency parser expects `spec.add_dependency`, not `s.add_dependency`.
- On macOS, Ruby 3.4 builds need `libyaml`, and `mise settings ruby.compile=false` may still compile from source.
- Verify exact Ruby constant names from the gem entry point before referencing them; casing like `VM` versus `Vm` matters.

## Perl

- JSON `null` values may deserialize as sentinel objects, not `undef`; normalize them explicitly with `is_null()` checks.
- In list construction, `(reverse @list, $extra)` reverses the extra item too. Use `((reverse @list), $extra)`.
- Perl modules that `use` sibling packages must add the sibling `lib/` path with `use lib` inside the module file, not only in tests.
- `Test2::V0` does not export `use_ok`; use `ok(eval { require ...; 1 }, ...)` instead.
- For 32-bit arithmetic on 64-bit Perl, mask after bitwise NOT: `(~$x) & 0xFFFFFFFF`.
- Perl right shift on negative integers is not an arithmetic shift; use division-based logic when you need sign extension.
- Strawberry Perl does not support `cpanm --with-test`; use supported flags only.
- Perl `BUILD_windows` should usually skip tests if the CI workflow already skips Perl on Windows.

## Lua

- Lua rockspecs must use `https://` and pin an immutable tag or commit, not a moving branch tip.
- Every Lua test file must prepend `package.path` for `../src/?.lua` and `../src/?/init.lua` before the first `require`.
- Lua package directories should use `snake_case`, matching Ruby and Elixir package naming.
- Lua regex handling in token grammars does not understand `\v` and `\f` inside character classes; sanitize those escapes before parsing `.tokens` files.
- In Lua 5.4, `^` returns a float. Use bit shifts like `1 << n` for integer powers of two in tests.
- Lua packages with native LuaRocks deps must install those deps in `BUILD`.
- In `BUILD_windows`, set env vars with `set "VAR=value" && command`, not Unix-style `VAR=value command`.

## Elixir

- Elixir reserved words such as `after`, `rescue`, and `catch` cannot be used as variable names.
- When iterating `0..(n - 1)` in Elixir and `n` may be zero, use `//1` so the range is empty instead of descending into invalid indexes.
- `if` returns a value. Capture the result instead of assuming rebinding inside the block mutates the outer variable.
- VM-backed function calls need a fresh execution context; save caller state, reset for the callee, and restore after return.
- Indentation-sensitive parsers need explicit `INDENT` and `DEDENT` tokens; do not let newline-skipping swallow block boundaries.
- Integration tests that exercise the full Elixir Starlark pipeline may need generous timeouts or selective skipping to avoid CI hangs.
- If the `BUILD` file compiles a NIF externally, do not also enable `:make` via `elixir_make` in `mix.exs`.
- Elixir NIF module names must use the full Erlang atom form, e.g. `Elixir.CodingAdventures.Foo`.
- Do not commit Elixir coverage HTML artifacts.

## Rust and native bridges

- After changing a Rust crate, run `cargo build --workspace`; individual crate tests do not catch missing exports or workspace-membership mistakes.
- If a workspace build fails while parsing dependency manifests, check whether the local stable toolchain is too old for a dependency's edition.
- Report real Rust coverage with `cargo tarpaulin`; do not leave it implied.
- Final `cdylib` crates must emit their own linker flags in `build.rs`; bridge-library `build.rs` output does not propagate to the final artifact.
- Python C API functions that use `long` must use Rust `c_long`, not hardcoded `i64`.
- OTP on Linux may not export `enif_get_int64` and `enif_make_int64`; use `enif_get_long` and `enif_make_long` instead.
- CPython slot numbers for `PyType_FromSpec` must match `typeslots.h` exactly. Do not guess them from memory.

## Go

- Use `go test ./...` and `go vet ./...` from the package directory.
- After adding a new Go dependency module, run `go mod tidy` in every transitively dependent module.
- Tools that touch Unix-only APIs must ship Windows-safe stubs or build-tag splits from the start.

## Swift

- In XCTest classes, qualify module-level `load` calls like `FontParser.load(...)`; `XCTestCase` shadows `load`.
- Create `.gitignore` before the first Swift build so `.build/` and `.swiftpm/` never enter git.
- Synthetic OpenType fixtures must match their declared byte sizes exactly; for example, the `head` table is 54 bytes.
- When parsing `kern` subtables, the format is in the high byte of `coverage` (`coverage >> 8`), not the low byte.
- On macOS, prefer `xcrun swift test`; on other platforms, use the platform-appropriate `swift test`.
- Windows CI can run real Swift tests if the toolchain is installed; do not skip Windows builds unless the package is genuinely platform-specific.
- Wrap ambiguous POSIX functions such as `bind` at module scope instead of calling them directly inside Swift closures.
- On Glibc, socket type constants like `SOCK_STREAM` may need `.rawValue`.
- When replacing a Swift `let` binding with an overflow-checked tuple binding, remove the original binding to avoid redeclaration errors.

## Java, Kotlin, .NET, C#, and F#

- Gradle output must go to `gradle-build/`, not `build/`, in this repo.
- Parallel `.NET` package builds must isolate outputs with `dotnet test --artifacts-path .artifacts`.
- On Linux `.NET` builds, set both `HOME="$PWD/.dotnet"` and `DOTNET_CLI_HOME="$PWD/.dotnet"` to avoid first-run races.
- On Windows `.NET` `BUILD_windows`, use the quoted env-var assignment form and quote path arguments.
- Recursive parsers for markdown or similar user input need both maximum input-size limits and maximum nesting-depth limits.
- Inline parsers must cap unmatched-delimiter scans; otherwise malformed delimiter-heavy input becomes quadratic.
- Ordered-list marker parsing must use `TryParse`-style conversion and treat overflow as plain text, not an exception.
- F# unsigned ranges like `0u .. count - 1u` need an explicit zero guard before subtracting one.
- F# binary deserializers must cap header-declared counts to the remaining payload before looping.
- C# files that use `BinaryPrimitives` need `using System.Buffers.Binary;` explicitly.

## Testing, coverage, and parser/VM behavior

- Socket tests should wait for stable invariants with bounded retries; do not assume immediate accept batching or instant EOF propagation.
- Security fixes that intentionally change error text require test assertion updates across the tree.
- Handwritten parsers need manual token-name updates; changing a grammar file is not enough if another implementation hardcodes token types.
- Lexer token renames break downstream parsers unless both ends change together or the grammar accepts both names during the transition.
- If a grammar's skip pattern includes `\n`, downstream tests should not expect emitted `NEWLINE` tokens.
- `.tokens` parsers must treat `/` inside `[...]` as part of the regex, not the closing delimiter.
- Swift lexer wrappers may need to promote generic `KEYWORD` tokens to keyword-specific token types.
- Grammar-based string tokenization may strip surrounding quotes; tests should assert the normalized value, not the source spelling.
- Generic VM opcode handlers must advance the program counter unless they halt or explicitly jump.
- `CALL_FUNCTION` handlers that receive `[arg0, arg1, ..., closure]` on the stack must pop the closure first, then the args.
- QOI encoders must update the seen-pixel table with the current pixel after emission, not with the previous pixel.

## Domain-specific implementation gotchas

- On the Intel 4004, `AND_IMM reg, reg, 15` and `AND_IMM reg, reg, 255` are no-ops because registers are already 4-bit.
- Intel 4004 codegen must not use `R1` as scratch when the source operand already lives in `R1`.
- The Intel 4004 simulator treats `HLT` as the halt sentinel; do not model halt as `JUN $`.
