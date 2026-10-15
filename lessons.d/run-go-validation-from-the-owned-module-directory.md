---
category: Testing & coverage
---

# Run Go validation from the owned module directory

The first Windows reactor selection check invoked go test from the repository root and stopped because no go.mod exists there. Run build-tool discovery, validator, plan tests and vet from code/programs/go/build-tool, preserving the failed command as a setup failure rather than claiming test acceptance. The reactor BUILD change does not require a new root Go module.
