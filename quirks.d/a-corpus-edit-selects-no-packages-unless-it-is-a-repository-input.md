---
package: *
lane: *
rung: infra
status: open
---

# A corpus edit selects no packages unless it is a repository input

The build tool maps changed files to packages by path prefix. Files under
`code/specs/` (specs, fixtures, corpora) belong to no package, so editing a
conformance corpus re-runs **nothing** -- a corpus change could break every lane
and still merge green.

**Handling:** each corpus must be registered as a repository input of every
package that consumes it, in `code/programs/go/build-tool/repository_inputs.go`
(VER26 steps 3b and 4 do this for the bootstrap harness and `ct-compare`).
