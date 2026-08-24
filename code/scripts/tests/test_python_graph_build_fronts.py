from __future__ import annotations

import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_PACKAGES_ROOT = REPO_ROOT / "code" / "packages" / "python"

GRAPH_CANONICAL_RECIPE = [
    "uv venv .venv --quiet --no-project --clear --python 3.13",
    "uv pip install --python .venv -e .[dev] --quiet",
    ".venv/bin/python -m ruff check src tests",
    ".venv/bin/python -m ruff format --check src tests",
    ".venv/bin/python -m mypy --strict src tests",
    ".venv/bin/python -m pytest tests/ -v",
]

GRAPH_WINDOWS_RECIPE = [
    "uv venv .venv --quiet --no-project --clear --python 3.13",
    "uv pip install --python .venv --no-deps -e .[dev] --quiet",
    "uv pip install --python .venv pytest pytest-cov ruff mypy --quiet",
    r".venv\Scripts\python.exe -m ruff check src tests",
    r".venv\Scripts\python.exe -m ruff format --check src tests",
    r".venv\Scripts\python.exe -m mypy --strict src tests",
    r".venv\Scripts\python.exe -m pytest tests/ -v",
]

DIRECTED_CANONICAL_RECIPE = [
    "uv venv .venv --quiet --no-project --clear --python 3.13",
    "uv pip install --python .venv -e ../graph --quiet",
    "uv pip install --python .venv -e .[dev] --quiet",
    ".venv/bin/python -m ruff check src tests",
    ".venv/bin/python -m ruff format --check src tests",
    ".venv/bin/python -m mypy --strict src tests",
    ".venv/bin/python -m pytest tests/ -v",
]

DIRECTED_WINDOWS_RECIPE = [
    "uv venv .venv --quiet --no-project --clear --python 3.13",
    "uv pip install --python .venv -e ../graph --quiet",
    "uv pip install --python .venv --no-deps -e .[dev] --quiet",
    "uv pip install --python .venv pytest pytest-cov ruff mypy --quiet",
    r".venv\Scripts\python.exe -m ruff check src tests",
    r".venv\Scripts\python.exe -m ruff format --check src tests",
    r".venv\Scripts\python.exe -m mypy --strict src tests",
    r".venv\Scripts\python.exe -m pytest tests/ -v",
]


class PythonGraphBuildFrontTests(unittest.TestCase):
    def test_graph_fronts_use_the_complete_pinned_recipe(self) -> None:
        self.assertEqual(self._recipe("graph", "BUILD"), GRAPH_CANONICAL_RECIPE)
        self.assertEqual(
            self._recipe("graph", "BUILD_windows"), GRAPH_WINDOWS_RECIPE
        )

    def test_directed_graph_fronts_preserve_typed_dependency_order(self) -> None:
        self.assertEqual(
            self._recipe("directed-graph", "BUILD"), DIRECTED_CANONICAL_RECIPE
        )
        self.assertEqual(
            self._recipe("directed-graph", "BUILD_windows"),
            DIRECTED_WINDOWS_RECIPE,
        )

    def test_both_packages_publish_pep_561_markers(self) -> None:
        packages = (("graph", "graph"), ("directed-graph", "directed_graph"))
        for package, module in packages:
            with self.subTest(package=package):
                marker = PYTHON_PACKAGES_ROOT / package / "src" / module / "py.typed"
                self.assertTrue(marker.is_file())

    @staticmethod
    def _recipe(package: str, name: str) -> list[str]:
        return [
            line
            for line in (PYTHON_PACKAGES_ROOT / package / name)
            .read_text(encoding="utf-8")
            .splitlines()
            if line and not line.startswith("#")
        ]


if __name__ == "__main__":
    unittest.main()
