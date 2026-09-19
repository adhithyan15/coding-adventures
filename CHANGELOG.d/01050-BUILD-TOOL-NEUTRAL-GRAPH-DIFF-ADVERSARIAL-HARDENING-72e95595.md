### Build-tool neutral graph/diff adversarial hardening

- Expand the language-neutral graph and diff-selection corpus from 14 to 19
  cases with empty and partial-cycle graphs, portable character-class globs,
  known near-BUILD paths, and exact recursive BUILD fronts.
- Require exactly one stable cycle diagnostic and update the Java, Kotlin, and
  Go consumers to exercise the strengthened fixture roster.
- Repair the Go build tool so arbitrary `BUILD_*` near names no longer select
  strict Starlark packages.
