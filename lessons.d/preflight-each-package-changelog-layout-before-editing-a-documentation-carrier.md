---
category: Repo policy / workflow reminders
---

# Preflight each package changelog layout before editing a documentation carrier

The first palette documentation helper assumed all four crates had CHANGELOG.d.
diagram-layout-geometric uses CHANGELOG.md, so the helper stopped after two
README writes and one valid shard. Preserve that failed run and finish only the
owned intended Markdown changes. Preflight all destinations/layouts first; use a
shard only beside an existing CHANGELOG.d/_meta.md, otherwise update the existing
monolithic changelog. Guard each command's success before dependent staging or
commit operations; a PowerShell newline does not stop after an earlier failure.
The failed stage/commit published nothing. Run document/lesson/diff gates and
independent exact carrier review after repair; no Rust rerun is inferred.
