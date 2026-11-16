---
category: CI & GitHub Actions
---

# A repository-wide guard test only protects a PR when the build tool selects its package for the files the guard reads

About twenty PRs (#19417-#19474) added `.warrant` documents that did not parse or check. The only test that parses every `.warrant` file, `rust/warrant-grammar`'s `v0_compat`, ran only when the build tool judged that package affected, and those PRs never touched it, so CI was green. Later PRs that did rebuild `warrant-grammar` then failed on files they had not touched.

Fix: register the guard in the build tool's repository input consumer table (`repository_inputs.go`) with the glob `**/?*.warrant`, so any changed `.warrant` selects the guard (and nothing else) on the PR that changes it.

Next time: when a test walks the whole tree instead of its own directory, add the files it walks to that table in the same PR that adds the test. A guard that is not scheduled for the change is indistinguishable from a guard that passes.
