---
category: BUILD files & dependency management
---

# Install CLI builder leaf packages before its local TypeScript file dependency

The repository build validator rejected a new TypeScript package even though its BUILD installed
`cli-builder`: that local package's own manifest depends on `directed-graph` and `state-machine`, so
the standalone prerequisite graph still lacked those leaf setup steps. Declare `cli-builder` in
the consumer's devDependencies and install its local file dependencies first in leaf-to-root order
in both `BUILD` and `BUILD_windows`. Run the repository build-file validator; a passing npm test
does not check that standalone CI can prepare the local dependency chain.
