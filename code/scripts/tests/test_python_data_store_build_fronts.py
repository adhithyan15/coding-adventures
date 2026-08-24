from __future__ import annotations

import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_PACKAGES_ROOT = REPO_ROOT / "code" / "packages" / "python"

ENGINE_CANONICAL_RECIPE = [
    "uv venv .venv --quiet --no-project --clear --python 3.13",
    "uv pip install --python .venv -e ../hash-functions --quiet",
    "uv pip install --python .venv --no-deps -e ../hyperloglog -e ../in-memory-data-store-protocol --quiet",
    "uv pip install --python .venv -e .[dev] --quiet",
    ".venv/bin/python -m ruff check src tests",
    ".venv/bin/python -m ruff format --check src tests",
    ".venv/bin/python -m mypy --strict --follow-untyped-imports src tests",
    ".venv/bin/python -m pytest tests/ -v",
]

ENGINE_WINDOWS_RECIPE = [
    "uv venv .venv --quiet --no-project --clear --python 3.13",
    "uv pip install --python .venv -e ../hash-functions --quiet",
    "uv pip install --python .venv --no-deps -e ../hyperloglog -e ../in-memory-data-store-protocol --quiet",
    "uv pip install --python .venv --no-deps -e .[dev] --quiet",
    "uv pip install --python .venv pytest pytest-cov ruff mypy --quiet",
    r".venv\Scripts\python.exe -m ruff check src tests",
    r".venv\Scripts\python.exe -m ruff format --check src tests",
    r".venv\Scripts\python.exe -m mypy --strict --follow-untyped-imports src tests",
    r".venv\Scripts\python.exe -m pytest tests/ -v",
]

STORE_CANONICAL_RECIPE = [
    "uv venv .venv --quiet --no-project --clear --python 3.13",
    "uv pip install --python .venv -e ../hash-functions --quiet",
    "uv pip install --python .venv --no-deps -e ../hyperloglog -e ../in-memory-data-store-protocol -e ../resp-protocol --quiet",
    "uv pip install --python .venv --no-deps -e ../in-memory-data-store-engine --quiet",
    "uv pip install --python .venv -e .[dev] --quiet",
    ".venv/bin/python -m ruff check src tests",
    ".venv/bin/python -m ruff format --check src tests",
    ".venv/bin/python -m mypy --strict --follow-untyped-imports src tests",
    ".venv/bin/python -m pytest tests/ -v",
]

STORE_WINDOWS_RECIPE = [
    "uv venv .venv --quiet --no-project --clear --python 3.13",
    "uv pip install --python .venv -e ../hash-functions --quiet",
    "uv pip install --python .venv --no-deps -e ../hyperloglog -e ../in-memory-data-store-protocol -e ../resp-protocol --quiet",
    "uv pip install --python .venv --no-deps -e ../in-memory-data-store-engine --quiet",
    "uv pip install --python .venv --no-deps -e .[dev] --quiet",
    "uv pip install --python .venv pytest pytest-cov ruff mypy --quiet",
    r".venv\Scripts\python.exe -m ruff check src tests",
    r".venv\Scripts\python.exe -m ruff format --check src tests",
    r".venv\Scripts\python.exe -m mypy --strict --follow-untyped-imports src tests",
    r".venv\Scripts\python.exe -m pytest tests/ -v",
]


class PythonDataStoreBuildFrontTests(unittest.TestCase):
    def test_engine_fronts_preserve_dependency_order_and_use_pinned_venv(self) -> None:
        self.assertEqual(
            self._recipe("in-memory-data-store-engine", "BUILD"),
            ENGINE_CANONICAL_RECIPE,
        )
        self.assertEqual(
            self._recipe("in-memory-data-store-engine", "BUILD_windows"),
            ENGINE_WINDOWS_RECIPE,
        )

    def test_store_fronts_preserve_dependency_order_and_use_pinned_venv(self) -> None:
        self.assertEqual(
            self._recipe("in-memory-data-store", "BUILD"),
            STORE_CANONICAL_RECIPE,
        )
        self.assertEqual(
            self._recipe("in-memory-data-store", "BUILD_windows"),
            STORE_WINDOWS_RECIPE,
        )

    def test_store_does_not_silence_missing_imports(self) -> None:
        pyproject = (
            PYTHON_PACKAGES_ROOT / "in-memory-data-store" / "pyproject.toml"
        ).read_text(encoding="utf-8")
        self.assertNotIn("ignore_missing_imports = true", pyproject)

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
