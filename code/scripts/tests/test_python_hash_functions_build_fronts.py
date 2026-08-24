from __future__ import annotations

import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
HASH_FUNCTIONS_ROOT = REPO_ROOT / "code" / "packages" / "python" / "hash-functions"


class PythonHashFunctionsBuildFrontTests(unittest.TestCase):
    def test_unix_front_is_repeatable_and_uses_its_pinned_venv(self) -> None:
        self.assertEqual(
            self._recipe("BUILD"),
            [
                "uv venv .venv --quiet --no-project --clear --python 3.13",
                "uv pip install --python .venv -e .[dev] --quiet",
                ".venv/bin/python -m ruff check src tests",
                ".venv/bin/python -m ruff format --check src tests",
                ".venv/bin/python -m mypy src tests",
                ".venv/bin/python -m pytest tests/ -v",
            ],
        )

    def test_windows_front_is_repeatable_and_uses_its_pinned_venv(self) -> None:
        self.assertEqual(
            self._recipe("BUILD_windows"),
            [
                "uv venv .venv --quiet --no-project --clear --python 3.13",
                "uv pip install --python .venv --no-deps -e .[dev] --quiet",
                "uv pip install --python .venv pytest pytest-cov ruff mypy --quiet",
                r".venv\Scripts\python.exe -m ruff check src tests",
                r".venv\Scripts\python.exe -m ruff format --check src tests",
                r".venv\Scripts\python.exe -m mypy src tests",
                r".venv\Scripts\python.exe -m pytest tests/ -v",
            ],
        )

    @staticmethod
    def _recipe(name: str) -> list[str]:
        return [
            line
            for line in (HASH_FUNCTIONS_ROOT / name)
            .read_text(encoding="utf-8")
            .splitlines()
            if line and not line.startswith("#")
        ]


if __name__ == "__main__":
    unittest.main()
