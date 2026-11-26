---
category: BUILD files & dependency management
---

# A Go fuzz smoke command must match the declared fuzz target name exactly

A package BUILD front and README were written with `FuzzPositionalSATAgainstOracle`
while the Go function is `FuzzPositionalSATAgainstOracle`. `go test -fuzz` runs
only a matching target, so a spelling mismatch makes the BUILD command fail
before any fuzz cases execute.

Copy the target identifier from the function declaration into both platform
BUILD fronts and README commands, then execute the exact BUILD command from the
owning module directory.
