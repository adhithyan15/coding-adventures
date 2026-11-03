## [Unreleased]

### Fixed

- `ScaffoldGenerator.main` crashed with `ENOENT` before parsing a single flag:
  it looked for `scaffold-generator.json` three directories above
  `generator.rb`, which is inside this package, but the spec every language's
  scaffold-generator shares lives five levels up in `code/programs/`. The path
  is now the `SPEC_FILE` constant, built with `File.expand_path` and an inline
  table of what each `..` reaches.
- `find_repo_root` only accepted a `.git` *directory*. In a linked git worktree
  (or submodule) `.git` is a file, so the walk went past the worktree's root
  and returned the enclosing checkout, which would have scaffolded the new
  package into the wrong tree. Either kind of `.git` now marks the root, the
  same rule the Python generator uses. It also takes an optional start
  directory so tests can exercise it without changing the working directory.

### Added

- `scaffold_generator_tool.rb`, the runnable script the README has always
  told people to invoke but that never existed. It points Bundler at this
  package's Gemfile (so it works from any directory) and calls `main`. The
  README usage now shows the real path from the repository root.
- `main` accepts an optional `argv` (defaulting to `ARGV`).
- CLI tests drive `main` end to end: `--help`, `--version`, an unknown flag, a
  non-kebab name, an unknown language, and a `--dry-run` against a scratch
  repository that checks the plan is printed and nothing is written. Repo-root
  tests cover a `.git` directory, a worktree `.git` file nested inside an outer
  checkout, and running outside any repository. A subprocess test runs the
  script from an unrelated directory.
