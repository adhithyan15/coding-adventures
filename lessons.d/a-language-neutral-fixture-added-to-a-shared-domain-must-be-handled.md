---
category: Testing & coverage
---

# A language-neutral fixture added to a shared domain must be handled or skipped by every language's native runner

When PR #18853 introduced test-trigger selection fixtures to
`code/specs/fixtures/build-tool-v1/cases/`, the new JSON files were registered
under the `diff_selection` domain, but used a new schema variant where
`options.packages` are package-name strings rather than structured package
records, and added a `trigger_relations` key.

While Java, Kotlin, and Dart legacy fixture test runners were updated in that
commit to ignore diff-selection fixtures carrying `trigger_relations`, OCaml's
native runner (`code/programs/ocaml/build-tool/test/test_build_tool_core.ml`) was
overlooked. In CI full builds, OCaml's test suite ran, attempted to parse the
new fixtures as legacy path-selection records, and failed closed. Because CI full
builds execute `code/scripts/ci/disk-watch.sh` on exit via a bash `EXIT` trap,
the failure output was followed by a `disk-watch` filesystem breakdown, leading
observers to mistake the test failure for a CI disk space exhaustion issue.

When adding new schema variants to shared cross-language fixture directories:
- Audit every language's native fixture runner across the repository to ensure
  the new variant is either implemented or explicitly filtered out.
- Do not assume an `EXIT` trap diagnostic report (such as `disk-watch.sh`)
  indicates that the preceding step failed due to the condition being reported;
  always check the preceding command's exit code and error output.
