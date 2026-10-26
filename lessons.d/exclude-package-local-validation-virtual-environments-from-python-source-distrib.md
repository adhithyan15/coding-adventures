---
category: Python
---

# Exclude package-local validation virtual environments from Python source distributions

A wheel smoke test created `.venv-wheel` inside a Python package. Git ignored
the directory, but Hatch's next source-distribution walk still reached its
absolute interpreter symlink. `uv build --clear` then failed because an sdist
may not contain a symlink whose target escapes the package, and the repository
orphan validator saw the same unexpected package-local tree.

Do not treat Git cleanliness as packaging cleanliness. Prefer temporary
validation environments outside the package. For packages that support local
venvs, add explicit Hatch build exclusions for `/.venv*`, coverage and tool
caches, `dist`, and `__pycache__`; then prove a repeat `uv build --clear` while
an ignored validation venv still exists. Remove ignored environments and build
artifacts after validation so repository-wide path validators see the tracked
package tree only.
