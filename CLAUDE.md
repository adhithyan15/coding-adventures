# Claude Instructions for coding-adventures

## Working Principles

1. **Always plan first** — Enter plan mode for any non-trivial task. Spec it out in detail.
2. **Ask questions** — Clarify requirements before implementing. Don't assume.
3. **Feature branches for everything** — Never commit directly to main.
4. **Commit frequently** — Small, focused commits with detailed log messages explaining what and why.
5. **Protect files** — Never force-delete. Always ensure files can be retrieved via git.
6. **Check lessons.md first** — Before starting any work, review lessons.md to avoid repeating mistakes.
7. **Document mistakes** — If a mistake is made, add it to lessons.md immediately.

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

## Project Structure

- `code/specs/` — Specifications for each package (numbered by layer)
- `code/learning/` — Notes and learning materials organized by language/topic
- `code/packages/` — Publishable libraries organized by language, then package
- `code/programs/` — Standalone programs organized by language
