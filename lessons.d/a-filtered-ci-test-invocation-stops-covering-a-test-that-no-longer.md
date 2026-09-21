---
category: CI & GitHub Actions
---

# A filtered CI test invocation stops covering a test that no longer matches its filter, and nothing reports it

`code/packages/rust/lang-aot/BUILD` cannot run `cargo test -p lang-aot --test
lang_matrix` unfiltered, because some ALGOL-owned cells are red (VM-D005). So
it runs *name-filtered* invocations instead:

```
cargo test -p lang-aot --test lang_matrix t7_differential_random_basic_
cargo test -p lang-aot --test lang_matrix portable_text_stdout_
cargo test -p lang-aot --test lang_matrix non_algol_matrix_every_proven_cell_agrees -- --exact
```

`feature_coverage_doc_counts_match_programs_source` lives in that same file and
matches **none** of those filters. It compiled, it passed locally, and it never
ran in CI. It is the test that pins `LANG-VM-FEATURE-COVERAGE.md` against
`lang_matrix.rs`'s actual contents — so the thing guarding the status document
was not running, and the ALGOL row duly drifted from 233 rows to 292 with
nothing to catch it.

This is the same defect as omitting a `--test` target from a BUILD list (see
`adding-a-rust-test-target-does-not-protect-it-the-crate-build`), but it hides
better, because the file IS named in BUILD. You see `--test lang_matrix` on
three lines and conclude the suite is covered. Coverage is per-test-name, not
per-file.

**It is worse when the unfiltered target is excluded.** Normally a filter that
matches nothing is harmless, because a broad run covers the test anyway. When
the broad run is deliberately excluded, the filters are the *only* path, and a
test matching none of them is invisible. Nothing in cargo or the build tool
reports "this test matched no invocation" — a filter matching zero tests is a
silent success.

**What to do:**

- Adding a test to a file that CI runs *by filter* is not enough. Check your
  test's name against every filter for that file, in the same commit.
- Prefer one filter per test name (`-- --exact`) over a prefix filter that
  quietly stops matching when someone renames a test.
- For any crate whose BUILD uses filters, audit periodically: list the `#[test]`
  names in the file and check each against the filters. For `lang-aot`:

```bash
grep -oP '(?<=^    fn )\w+' code/packages/rust/lang-aot/tests/lang_matrix.rs
grep -o -- '--test lang_matrix [a-z0-9_]*' code/packages/rust/lang-aot/BUILD
```

Anything in the first list that no entry in the second is a prefix of is
running nowhere.

Found three times now in this repo: VM-062 (four CLR `--test` targets omitted
outright), then this (a test matching no filter), with the coverage-doc drift
(VM-063) as the downstream damage.

