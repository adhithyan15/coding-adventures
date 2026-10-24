---
category: BUILD files & dependency management
---

# Run Go build from the owning module directory

After expanding a sparse checkout for the repository build tool's local Go
replacements, `go build ./code/programs/go/build-tool` was invoked from the
monorepo root. The root is not a Go module, so Go stopped before resolving the
nested package. `cd` to the directory containing the intended `go.mod` (or set
the command working directory there) before building a nested Go program; a
filesystem path from a non-module root is not a substitute for module context.
