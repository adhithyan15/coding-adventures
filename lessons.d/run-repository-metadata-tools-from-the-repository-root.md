---
category: Repo policy / workflow reminders
---

# Run repository metadata tools from the repository root

Deep package directories make hand-counted relative paths to repository-owned
scripts easy to get wrong. Run cross-repository metadata tools such as
`code/scripts/lessons.py` and capability-taxonomy tests from the repository
root, using their root-relative paths and an explicit installed interpreter
when needed. Keep package-local BUILD, Cargo, and documentation checks in the
package or language workspace where their manifests define the intended scope.
