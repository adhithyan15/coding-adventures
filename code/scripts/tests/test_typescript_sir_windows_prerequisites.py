from __future__ import annotations

import json
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
TYPESCRIPT_PACKAGES = REPO_ROOT / "code" / "packages" / "typescript"


class TypeScriptSirWindowsPrerequisiteTests(unittest.TestCase):
    def test_compiler_front_doors_declare_node_types(self) -> None:
        for package in ("sir-runtime-core", "sir-runtime-oop"):
            with self.subTest(package=package):
                package_json = json.loads(
                    (TYPESCRIPT_PACKAGES / package / "package.json").read_text(
                        encoding="utf-8"
                    )
                )

                self.assertEqual(
                    package_json["devDependencies"]["@types/node"], "^22.0.0"
                )

    def test_selected_windows_front_doors_materialize_exact_closures(self) -> None:
        expected = {
            "sir-runtime-core": [
                "cd ../sir-runtime-exceptions && npm ci --quiet",
                "cd ../sir-runtime-pairs && npm ci --quiet",
                "npm ci --quiet",
                "npx tsc --noEmit",
                "npx vitest run --coverage",
            ],
            "sir-runtime-oop": [
                "cd ../sir-runtime-core && npm ci --quiet",
                "cd ../sir-runtime-exceptions && npm ci --quiet",
                "cd ../sir-runtime-pairs && npm ci --quiet",
                "npm ci --quiet",
                "npx tsc --noEmit",
                "npx vitest run --coverage",
            ],
            "sir-runtime-symbolic": [
                "cd ../cas-pattern-matching && npm ci --quiet",
                "cd ../symbolic-ir && npm ci --quiet",
                "npm ci --quiet",
                "npx tsc --noEmit",
                "npx vitest run --coverage",
            ],
        }

        for package, expected_lines in expected.items():
            with self.subTest(package=package):
                build_windows = TYPESCRIPT_PACKAGES / package / "BUILD_windows"
                actual_lines = [
                    line
                    for line in build_windows.read_text(encoding="utf-8").splitlines()
                    if line
                ]
                self.assertEqual(actual_lines, expected_lines)


if __name__ == "__main__":
    unittest.main()
