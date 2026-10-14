---
category: Testing & coverage
---

# A reused checkout needs its ignored validator runtime before a documentation check

The reused ordering sibling had clean tracked sources but lacked the ignored
human-language-data/dist document CLI. The first documentation check exited1
with MODULE_NOT_FOUND before examining a single shard. That is a bootstrap
failure, not successful validation or a malformed-document finding.

Preserve that failed receipt/log. Verify the complete validator package's tracked
objects equal the previously validated checkout, then bootstrap its ignored
runtime with recorded byte hashes and no source edits. The CLI resolves the
repository from its own location, so invoking another checkout's copy would
check the wrong tree. Rerun the real CLI inside this checkout and accept only
its actual successful exit, along with lesson and whitespace checks.

Run Git object proofs with an explicit repository cwd. The first new runtime
retention query ran from the artifact folder and git returned usage; it proved
nothing. The corrected query and complete source/config object comparison ran
in the actual checkout. All388 compiled runtime files retain bootstrap hashes;
this reuses a validated runtime and is not a fresh-build claim.
