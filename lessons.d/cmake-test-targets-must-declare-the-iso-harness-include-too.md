---
category: BUILD files & dependency management
---

# CMake test targets must declare the ISO harness include too

The strict compiler harness found `iso_test.h` through its exported include
path, but the package's separately configured CMake test target did not. A
passing direct harness compile therefore did not prove the CMake test graph was
self-contained. Declare the harness include on each package-local CMake test
target, resolve the path from the package directory (the sibling dependency is
under `code/packages/c/`, not `code/c/`), and rerun the install-consumer path.
