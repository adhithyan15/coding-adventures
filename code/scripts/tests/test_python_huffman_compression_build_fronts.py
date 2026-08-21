from __future__ import annotations

import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
PYTHON_ROOT = REPO_ROOT / "code" / "packages" / "python"

DEPENDENCIES = {
    "huffman-tree": "../heap",
    "brotli": "../heap ../huffman-tree",
    "deflate": "../heap ../huffman-tree ../lzss",
    "huffman-compression": "../heap ../huffman-tree",
}


class PythonHuffmanCompressionBuildFrontTests(unittest.TestCase):
    def test_unix_fronts_pin_python_313_and_preserve_dependency_order(self) -> None:
        for package, dependencies in DEPENDENCIES.items():
            with self.subTest(package=package):
                self.assertEqual(
                    self._recipe(package, "BUILD"),
                    [
                        "uv venv .venv --quiet --no-project --clear --python 3.13",
                        f"uv pip install --python .venv {dependencies} --quiet",
                        "uv pip install --python .venv -e .[dev] --quiet",
                        ".venv/bin/python -m pytest tests/ -v",
                    ],
                )

    def test_windows_fronts_pin_python_313_and_preserve_dependency_order(self) -> None:
        for package, dependencies in DEPENDENCIES.items():
            with self.subTest(package=package):
                self.assertEqual(
                    self._recipe(package, "BUILD_windows"),
                    [
                        "uv venv .venv --quiet --no-project --clear --python 3.13",
                        f"uv pip install --python .venv {dependencies} --quiet",
                        "uv pip install --python .venv -e .[dev] --quiet",
                        r".venv\Scripts\python.exe -m pytest tests/ -v",
                    ],
                )

    @staticmethod
    def _recipe(package: str, name: str) -> list[str]:
        return [
            line
            for line in (PYTHON_ROOT / package / name)
            .read_text(encoding="utf-8")
            .splitlines()
            if line and not line.startswith("#")
        ]


if __name__ == "__main__":
    unittest.main()
