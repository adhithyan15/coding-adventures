---
category: Testing & coverage
---

# Vitest is not Jest and does not accept runInBand

I passed Jest's `--runInBand` option through an npm script backed by Vitest,
which made the verification command fail before running any test. Read the
package script and runner help before adding runner-specific flags. For this
repository's Vitest packages, use the checked-in `npm test` command unless a
documented Vitest option is actually needed.
