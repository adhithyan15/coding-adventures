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
9. **Publishable packages** — Every package must have a proper pyproject.toml with src layout, ready for PyPI/RubyGems/npm.
10. **>80% test coverage** — Every package must have extensive unit tests. Coverage must exceed 80%.
11. **Changelogs required** — Every package must have a CHANGELOG.md updated before pushing, with detailed accounting of changes.
12. **Package READMEs** — Every package must have its own README.md describing what it does, how it fits in the stack, and usage examples.

## Workflow

- Specs → Tests → Implementation → Changelog → Commit
- Each new package or feature gets its own feature branch
- Commits follow the pattern: `type(scope): description` (e.g., `feat(logic-gates): add AND gate implementation`)

## Project Structure

- `code/specs/` — Specifications for each package (numbered by layer)
- `code/learning/` — Notes and learning materials organized by language/topic
- `code/packages/` — Publishable libraries organized by language, then package
- `code/programs/` — Standalone programs organized by language
