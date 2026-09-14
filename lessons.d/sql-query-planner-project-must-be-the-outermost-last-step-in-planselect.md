---
category: Compiler / VM / language pipeline
---

# SQL query planner: `Project` must be the OUTERMOST (last) step in `planSelect`

The correct 8-step pipeline is `Scan → Filter → Aggregate → Having → Distinct → Sort → Limit → Project`. Building `Project` before `Distinct`/`Sort`/`Limit` produces the wrong tree shape — e.g. `Sort(Project(Scan))` instead of `Project(Sort(Scan))`. Tests C5 (ORDER BY), C6 (LIMIT), C7 (DISTINCT), and Struct stacking all fail if `Project` is wrapped too early. This bug appeared identically in Java, Kotlin, and Haskell `planSelect` implementations (PR #7045, #7047, #7048). Fix: move `Project` construction to the last step after all sorting/pagination nodes are wrapped.
