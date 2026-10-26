---
category: Repo policy / workflow reminders
---

# Run the scaffold generator from its owning Go module directory

After materializing `code/programs/go/scaffold-generator`, running `go run .`
from the repository root still fails because the root has no `go.mod`. Invoke
the generator from `code/programs/go/scaffold-generator`, or pass that module
path explicitly, while keeping generated package paths relative to the
repository layout expected by the tool.
