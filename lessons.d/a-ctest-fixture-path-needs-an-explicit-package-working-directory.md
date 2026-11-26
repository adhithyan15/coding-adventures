---
category: Testing & coverage
---

# A CTest fixture path needs an explicit package working directory

The same C++ fixture test passed under the ISO harness and failed in CTest
because the harness runs from the package directory while CTest defaults to its
build directory. Set `WORKING_DIRECTORY` explicitly for package-local fixture
tests, and stop safely after recording a parse failure instead of dereferencing
an absent document.
