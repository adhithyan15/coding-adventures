---
category: CI & GitHub Actions
---

# Multi-consumer fixture seeds must sort affected package names before emitting build plans

Adding Haskell as a second native consumer of the STP01 fixture exposed that
`affectedListFromSet` copied Go map keys directly into each platform build plan.
The same focused test consequently observed both possible package orders across
runs.  Although the affected *set* was correct, unstable JSON makes plans noisy,
can invalidate caches, and lets tests or downstream tooling see nondeterministic
input.

Sort package names at the single set-to-list boundary before serializing them.
Keep the distinction between a nil set (force/all packages) and an empty set
(no packages): return nil early, and sort only the allocated non-nil slice.  A
multi-consumer fixture test should assert the exact sorted order repeatedly and
cover every emitted platform override, not weaken the assertion into set
comparison.
