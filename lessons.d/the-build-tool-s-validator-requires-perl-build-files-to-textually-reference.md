---
category: Perl
---

# The build-tool's validator requires perl BUILD files to textually reference every transitive local prerequisite's path

(perl is in `requiresExplicitPrereqs` in `validator.go`, alongside python/typescript) — it scans the literal command text in `BUILD` for `../relative/paths` and resolves them against known packages; `use lib` statements inside `.pl`/`.t` files (even via `FindBin`) are invisible to it. Symptom: CI fails with `missing prerequisite refs for standalone builds: perl/foo, perl/bar` even though the program runs fine locally and its tests pass. Fix: add matching `-I../../../packages/perl/foo/lib` flags directly to the `prove`/`perl` invocation in `BUILD`, mirroring the existing `paint-vm-ascii` package's own BUILD. Verify locally by building `code/programs/go/build-tool` and running `./build-tool -root . -diff-base origin/main -validate-build-files -detect-languages -emit-plan plan.json` — confirm your package no longer appears in the failure list (ignore pre-existing failures in unrelated packages).
