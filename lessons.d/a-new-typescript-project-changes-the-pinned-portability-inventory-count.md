---
category: TypeScript / JavaScript
---

# A new TypeScript project changes the pinned portability inventory count

Adding the NN37 neural-framework package passed native training and checkpoint
conformance, but CI's repository portability test failed because its explicit
project inventory still expected 492 projects instead of 493.

The package also adds one compiler lock, one project whose tests/CLI use Node
APIs, and one directly owned Node type provider. Update those four exact census
assertions together, keeping all zero-gap portability assertions intact. A
standalone no-emit project does not increase shared-config or emitter counts.

When adding a TypeScript package, run the repository portability contract as
well as package-local tests. In a sparse worktree, audit missing tracked inputs
from the exact Git head rather than interpreting the sparse filesystem as the
complete repository or expanding a disk-constrained checkout.
