from __future__ import annotations

import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_PACKAGES_ROOT = REPO_ROOT / "code" / "packages" / "python"
PACKAGES = ("bloom-filter", "hash-map", "hyperloglog")

CANONICAL_RECIPE = [
    "uv venv .venv --quiet --no-project --clear --python 3.13",
    "uv pip install --python .venv -e ../hash-functions --quiet",
    "uv pip install --python .venv -e .[dev] --quiet",
    ".venv/bin/python -m ruff check src tests",
    ".venv/bin/python -m ruff format --check src tests",
    ".venv/bin/python -m mypy src tests",
    ".venv/bin/python -m pytest tests/ -v",
]

WINDOWS_RECIPE = [
    "uv venv .venv --quiet --no-project --clear --python 3.13",
    "uv pip install --python .venv -e ../hash-functions --quiet",
    "uv pip install --python .venv --no-deps -e .[dev] --quiet",
    "uv pip install --python .venv pytest pytest-cov ruff mypy --quiet",
    r".venv\Scripts\python.exe -m ruff check src tests",
    r".venv\Scripts\python.exe -m ruff format --check src tests",
    r".venv\Scripts\python.exe -m mypy src tests",
    r".venv\Scripts\python.exe -m pytest tests/ -v",
]


class PythonHashCollectionsBuildFrontTests(unittest.TestCase):
    def test_canonical_fronts_are_repeatable_and_use_their_pinned_venv(self) -> None:
        for package in PACKAGES:
            with self.subTest(package=package):
                self.assertEqual(self._recipe(package, "BUILD"), CANONICAL_RECIPE)

    def test_windows_fronts_are_repeatable_and_use_their_pinned_venv(self) -> None:
        for package in PACKAGES:
            with self.subTest(package=package):
                self.assertEqual(
                    self._recipe(package, "BUILD_windows"), WINDOWS_RECIPE
                )

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
