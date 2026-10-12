---
category: Testing & coverage
---

# Vitest does not accept Jest's runInBand flag

Passing `--runInBand` through this package's `npm test` script made Vitest
reject the command before collecting tests. The flag belongs to Jest and is
not a portable request for serial execution.

Run this repository's Vitest suite with its checked-in configuration. If a
serial diagnostic is truly needed, consult the installed Vitest version's
supported pool and worker options rather than borrowing a Jest flag.
