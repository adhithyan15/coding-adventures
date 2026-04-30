# Claude Instructions for coding-adventures

## Working Principles

1. **Pull from origin/main first** — Before starting any work, run `git fetch origin && git merge origin/main` to ensure the worktree is up to date. The repo moves fast; reasoning about stale state causes CI failures and merge conflicts.
2. **Always plan first** — Enter plan mode for any non-trivial task. Spec it out in detail.
3. **Ask questions** — Clarify requirements before implementing. Don't assume.
4. **Feature branches for everything** — Never commit directly to main.
5. **Commit frequently** — Small, focused commits with detailed log messages explaining what and why.
6. **Protect files** — Never force-delete. Always ensure files can be retrieved via git.
7. **Check lessons.md first** — Before starting ANY implementation work, read lessons.md cover to cover. Key lessons that recur:
   - BUILD files must install ALL transitive dependencies in leaf-to-root order
   - Elixir reserved words (`after`, `rescue`, etc.) cannot be variables
   - Rust workspace: run `cargo build --workspace` to catch missing exports
   - Ruby require ordering: dependencies must be required before own modules
   - TypeScript: chain-install transitive `file:` deps in BUILD files
   - Go: run `go mod tidy` in ALL transitively dependent packages after adding a new module
   - Every package needs BUILD, README.md, CHANGELOG.md in every language
8. **Document mistakes** — If a mistake is made or a CI failure occurs, add it to lessons.md immediately. Don't wait until later.

## Repo Standards

8. **Specs first** — Specification documents are always committed before any implementation code.
9. **Specs must stay in sync** — After implementing a package, review the spec. If implementation diverged from the spec, update the spec and explicitly call out what changed and why in the commit message.
10. **Publishable packages** — Every package must have a proper pyproject.toml with src layout, ready for PyPI/RubyGems/npm.
11. **>80% test coverage** — Every package must have extensive unit tests. Coverage must WELL exceed 80%. Target 95%+ for libraries, 80%+ for programs.
12. **Changelogs required** — Every package must have a CHANGELOG.md updated before pushing, with detailed accounting of changes.
13. **Package READMEs** — Every package must have its own README.md describing what it does, how it fits in the stack, and usage examples.
14. **Literate programming** — All code must use Knuth-style literate programming. Explanations, truth tables, diagrams, analogies, and examples belong inline with the code. Someone new to programming should be able to learn from reading the source.
15. **Linting required** — All Python code must pass ruff. All Ruby code must pass standardrb. All Go code must pass go vet.

## Workflow

- Specs → Tests → Implementation → Changelog → README → Commit
- Each new package or feature gets its own feature branch
- Commits follow the pattern: `type(scope): description` (e.g., `feat(logic-gates): add AND gate implementation`)
- After implementation, verify: tests pass, coverage high, README exists, CHANGELOG updated, spec still accurate
- If implementation diverged from spec, update spec and note divergence in commit message
- Push early and often — create PRs as soon as a branch has meaningful work, keep pushing commits into the same PR as work progresses
- Keep the todo list updated — check off items as they're completed, add new items as they're discovered
- **Before pushing code, always run `/security-review` to have a sub-agent perform a security code review. Do not push until the review passes.**
- **For changes that touch `twig-vm` or its deps**, also run `scripts/miri-twig-vm.sh` locally before pushing.  Per-PR CI runs Miri on `lang-runtime-core` + `lispy-runtime` (where the unsafe lives) as a blocking check; twig-vm Miri is informational on PRs and runs nightly, so the local script is the canonical verification.  Wallclock ~30-90 min; run in a separate terminal during code review.
- **After creating or pushing to a PR, always run `/babysit-pr` to monitor CI status and merge conflicts until the PR is green**

## Build System

- The primary build tool is the Go implementation at `code/programs/go/build-tool/`
- Build it with `go build -o build-tool .` then run `./build-tool`
- Default mode: git-diff-based change detection (`--diff-base origin/main`)
  - Computes `git diff --name-only <base>...HEAD` to find changed files
  - Maps changed files to packages via path prefix matching
  - Uses directed graph `affected_nodes()` to find all packages needing rebuild
  - No cache file needed — git is the source of truth
- Fallback: `--force` rebuilds everything
- Independent packages run in parallel via goroutines
- Python and Ruby build tools exist as educational implementations

## Project Structure

- `code/specs/` — Specifications for each package (numbered by layer)
- `code/learning/` — Notes and learning materials organized by language/topic
- `code/packages/` — Publishable libraries organized by language, then package
- `code/programs/` — Standalone programs organized by language
