---
category: BUILD files & dependency management
---

# `uv run` must select the package environment explicitly

With `--no-project`, `uv run` can choose the host Python interpreter instead
of the package's `.venv`, even after the build installed dependencies into that
environment. Pass `--python .venv` to test and tooling commands so the runtime
uses the dependencies installed by the package's BUILD recipe.
