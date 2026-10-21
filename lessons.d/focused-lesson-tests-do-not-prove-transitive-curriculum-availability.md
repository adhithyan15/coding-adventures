---
category: Testing & coverage
---

# Focused lesson tests do not prove transitive curriculum availability

The Marathi A1 form-retrieval unit tests passed, but the full affected-package
CI run failed: `MR-A1M02-name-line` assessed two form atoms without a
prerequisite path to `MR-A1F01-mitrache-naav`, which introduces them. Chapter
order alone does not establish availability. Keep the teaching lesson on the
review lesson's prerequisite path, and run the real-curriculum integration and
CLI validation tests on the exact final source before pushing; focused
reinforcement tests cannot establish graph closure for another lesson.
