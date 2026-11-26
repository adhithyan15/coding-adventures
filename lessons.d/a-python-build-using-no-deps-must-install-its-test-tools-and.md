---
category: BUILD files & dependency management
---

# A Python BUILD using `--no-deps` must install its test tools and seed pip for wheel checks

The heap package BUILD installed the editable package with `--no-deps`, then
listed pytest, coverage, Ruff, and mypy explicitly. The new stateful heap tests
also import Hypothesis, so a clean runner would fail test collection even
though `pyproject.toml` declared Hypothesis in the `dev` extra. The wheel
consumer helper also invokes `.venv`'s `python -m pip`; a plain `uv venv` does
not seed pip by default.

Both BUILD scripts now explicitly install `hypothesis>=6.100` with the other
tools and create the environment with `uv venv --seed`. When a BUILD uses
`--no-deps`, enumerate the tools tests actually import. Seed pip when helpers
inside that environment use `python -m pip`.
