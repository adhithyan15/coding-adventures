---
category: Testing & coverage
---

# A shared corpus fixture can still leave redundant snapshot reads inside assertions

Run 38213744310 built 532 macOS packages and passed 209 human-language test
files, then failed only standalone-book's frontmatter assertion at 30 seconds.
Its module fixture had already loaded the same lessons for the surface scan;
the assertion loaded them again. All previously repaired corpus cases passed.

Share that parsed lessons fixture, and audit the remaining default-root snapshot
reads together. Hashes and exam inventories repeatedly parse immutable inputs;
level-gate already has REAL_EVERYTHING but several assertions bypass it. Reuse
parsed inputs while keeping derived counterfactual reports and synthetic fixtures
independent. Preserve live CLI work, temporary-root loader regressions, every
assertion, existing deadlines, two workers and the 2 GiB verification constraint.
Run the complete package with coverage before pushing so a single remaining
readonly parse does not cost another full cross-platform CI cycle.
