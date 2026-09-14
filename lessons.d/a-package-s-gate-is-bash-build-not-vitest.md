# A package's gate is `bash BUILD`, not `vitest`

Splitting a German book chapter added 27 lessons that all keep the `GE-C09-`
prefix. The Language Ladder bundles lazy lesson batches by the lesson's **own id
number**, not by the book chapter it is assigned to, so all 27 landed in one
band, pushed it past the 256 kB backstop, and `check:bundle` failed CI with

    the size backstop split 2 band(s) beyond the 1 allowed

Every `human-language-data` gate was green, and so was `npx vitest run` in the
ladder itself. The ladder's CI job runs `bash BUILD`, which is
`typecheck -> build -> check:bundle -> vitest run`; running only the last step
reaches none of the first three. **Read the package's BUILD file and run what it
runs.** A test suite is one of the gates, not the gate.

Two things generalise past this fix. **A content change can fail in a consumer's
budget rather than in its own package's checks** — nothing in the curriculum data
knows the ladder has a bundle. And when a size knob has to move, **project it
forward against the work already queued**: width 4 cleared that day and would have
failed again two PRs later, so the fix went to width 3 and the debt counter fell
to zero instead of being re-budgeted.
