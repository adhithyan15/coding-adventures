---
category: Testing & coverage
---

# A Gradle falsification run can report UP-TO-DATE and pass after you edit a generated source, so falsify with --rerun-tasks

**What happened.** To check that a Compose UI test of a generated project
really depended on the emitted `onPreviewKeyEvent` handler, the handler was
stripped from `src/main/kotlin/CommitProbe.kt` with `perl -i` and
`gradle test` was rerun. It exited 0. The log showed why: `compileKotlin`,
`testClasses` and `test` were all `UP-TO-DATE`. The project lived under
`/private/tmp`, and the warm Gradle daemon's file-system watching did not
register the in-place edit, so it reused the old classes and the old test
result. Read at face value, that "pass" said the test was vacuous.

**Fix.** Rerun with `gradle test --rerun-tasks`. The test then failed at its
first commit assertion with the handler stripped, and passed with it restored.

**Do differently.** A falsification run is only evidence if the build
actually rebuilt. Use `--rerun-tasks` (or check the log for `Task :test` with
no `UP-TO-DATE`) whenever you edit sources outside Gradle's view, and treat
an unexpected pass on a mutated build as a question about the build before
it is a verdict on the test.
