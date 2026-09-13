---
category: BUILD files & dependency management
---

# Don't install sibling deps in parallel

from inside a TS BUILD — two packages racing `cd ../state-machine && npm ci` corrupt each other's `node_modules` (ETXTBSY on esbuild). The build tool already handles topological order; only install what your own package needs.
