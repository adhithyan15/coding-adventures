---
category: BUILD files & dependency management
---

# Scope local replacement scans to the reachable module graph

Searching every workspace `go.mod` for `replace` directives produces a huge,
truncated inventory unrelated to the module being compiled. It does not reveal
the actionable closure clearly.

Start from the target module and the modules named by its direct local
replacements. Read only those manifests, add newly referenced local modules,
and iterate until the reachable set stops growing. Report the resulting union,
not the repository-wide replacement catalog.
