---
category: TypeScript / JavaScript
---

# A new TypeScript project changes the pinned portability inventory count

Adding the NN37 neural-framework package passed native training and checkpoint
conformance, but CI's repository portability test failed because its explicit
project inventory still expected 492 projects instead of 493.

The package also adds one compiler lock, one project whose tests/CLI use Node
APIs, and one directly owned Node type provider. Update those exact census
assertions together, keeping all zero-gap portability assertions intact. A
standalone no-emit project does not increase shared-config or emitter counts.

The GAME10 `game-clock` package raised the portability inventory from 515 to 516
projects and from 308 to 309 shared-config projects. Its explicit `rootDir` and
`outDir` did not change the inherited-path or standalone-emitter counts. Add the
package's exact effect to the repository assertion comments and counts; do not
remove the inventory assertion or weaken the portability checks.

The shared-source boundary fixture also enumerates TypeScript packages that
consume `tsconfig.base.json`. Add a new package there, recompute the boundary
digest, and update its versioned conformance-case digests and Go digest assertion.
Passing the package portability test alone does not update that separate build
input contract.

When adding a TypeScript package, run the repository portability contract as
well as package-local tests. In a sparse worktree, audit missing tracked inputs
from the exact Git head rather than interpreting the sparse filesystem as the
complete repository or expanding a disk-constrained checkout.
The repository source-boundary test derives TypeScript consumers from package roots whose `BUILD` front and `tsconfig.json` extend the shared base. A new qualifying root must be added to the exact-root list even when its local portability tests pass; then refresh every `boundary_sha256` pin and both language digest assertions.
