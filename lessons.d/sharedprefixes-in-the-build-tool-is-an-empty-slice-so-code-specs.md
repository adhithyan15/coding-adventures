---
category: CI & GitHub Actions
---

# `sharedPrefixes` in the build tool is an EMPTY slice, so `code/specs/

`, `code/fixtures/**`, `code/grammars/**` and `code/scripts/**` map to ZERO packages and never appear in `affected_packages`.** `gitdiff.MapFilesToPackages` only maps a changed file to a package when the file lives *under* that package's directory. The comment at `main.go` claiming grammar/spec changes "only trigger rebuilds of packages that actually import them" describes a mechanism that does not exist. Consequence for anything keyed off the affected closure: a gate, cache key, or skip heuristic built on packages alone is blind to fixture, grammar, spec and script edits — which is exactly where staleness checks keep their inputs. Always pair a package clause with a path-glob clause. (This is also why `code/specs/data/ci-gates.json` requires both.)
