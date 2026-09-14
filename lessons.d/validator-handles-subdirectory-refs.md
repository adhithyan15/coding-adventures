---
category: BUILD files & dependency management
---

# Validator handles subdirectory refs

via `resolvePackageRefFuzzy` — paths like `../sha512/lib` walk up to the package root for the missing-prereq check, but exact-match for the undeclared-ref check.
