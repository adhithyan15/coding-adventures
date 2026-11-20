# 00376 — Unreleased: negctl users declare Python, enforced by -validate-build-files (BT-BUILDLOCAL-1)

- New `validator.ValidateNegctlUsersDeclarePython`, wired into `-validate-build-files` beside the other
  checks (same aggregation, exit 1). Every `BUILD*` file directly in a package directory discovered for
  any platform (outside plan mode via `DiscoverPackagesAllPlatforms`, so a `BUILD_windows`-only package
  is covered on Linux CI; nested non-package BUILD files are not checked) that contains
  `warrant_negctl.py` must declare `# needs-toolchain: python` for the platform it serves (`_windows`
  as windows, `_mac` as darwin, otherwise linux). The rule is the old test's, unchanged: plain
  substring match, so comments and `warrant_negctl.py.bak` count as uses.
- Why: package-local BUILD edits (#19669) never select `go/programs/build-tool`, so the repo-wide Go
  test never ran on a PR touching only a Rust package's BUILD. `-validate-build-files` runs on every PR.
- `TestEveryBuildFileUsingTheNegctlCheckerDeclaresPython` now runs the validator over the real checkout
  (all-platform discovery) instead of re-implementing the scan, so build-tool PRs still exercise it.
  Variant, comment and lookalike cases are unit-tested in `internal/validator/negctl_python_test.go`.
- README and `code/specs/12-build-system.md` document the check.
- Review fixes: unreadable files / BUILD* directories / dangling symlinks are reported as findings
  (only an absent package directory is skipped); errors print repo-relative paths
  (`code/packages/rust/lzw/BUILD`); a flagged non-build name (`BUILD.bak`) gets rename-or-remove advice;
  the repo-wide test again asserts non-vacuity when `rust/warrant-grammar/BUILD` exists.
