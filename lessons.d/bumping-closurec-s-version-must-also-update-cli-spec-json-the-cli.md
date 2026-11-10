---
category: BUILD files & dependency management
---

# Bumping closurec's version must also update cli.spec.json, the CLI-surface audit's spec hash, and the --help_markdown fixture, or three separate tests fail one at a time in CI

PR #19103 bumped `closurec` from 0.247.6 to 0.247.7 in `Cargo.toml` and its changelog and
nothing else. CI (the `build` matrix) then failed three different ways, and only the
first was visible at a time because the build stops at the first failing test binary:

1. `version_string_matches_crate_version` (unit test): `cli.spec.json` carries
   `"version"`, and it must equal the crate version.
2. `cli_surface::checked_in_surface_is_complete_and_current`: the audit report
   `tests/cli-surface/v20260915-audit.json` pins `local_spec_sha256`, the SHA-256 of
   `cli.spec.json`, so fixing (1) breaks it. Recompute with `sha256sum cli.spec.json`.
3. `diff_help_markdown::help_markdown_fixture_matches_expected_dump`: the `Version:`
   line of `tests/diff/help-markdown/expected.stdout`.

I had run the closurec tests locally only before the version bump, so none of the three
showed up. When bumping a crate's version, `grep -rIn --exclude-dir=target "<old
version>" <crate dir>` for pinned copies first, then run that crate's *whole* test suite
(`cargo test --release --no-fail-fast`), after the bump, not before it.
