from __future__ import annotations

import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
TYPESCRIPT_PACKAGES = REPO_ROOT / "code" / "packages" / "typescript"
BUILD_FRONTS = ("BUILD", "BUILD_windows")
ALGOL_OWNED_PACKAGES = {"algol-lexer", "algol-parser"}
STATEFUL_SIBLING_CD = re.compile(r"^\s*cd\s+\.\.[/\\]")


def is_parser_or_lexer(package_name: str) -> bool:
    return package_name in {"lexer", "parser"} or package_name.endswith(
        ("-lexer", "-parser")
    )


def stateful_sibling_cd_lines(source: str) -> list[int]:
    return [
        line_number
        for line_number, line in enumerate(source.splitlines(), start=1)
        if STATEFUL_SIBLING_CD.match(line)
    ]


class TypeScriptFrontendBuildDirectoryTests(unittest.TestCase):
    def test_detector_distinguishes_stateful_and_subshell_directory_changes(
        self,
    ) -> None:
        source = "\n".join(
            (
                "cd ../lexer && npm ci",
                "(cd ../parser && npm ci)",
                "npm ci",
                "  cd ..\\state-machine && npm ci",
            )
        )

        self.assertEqual(stateful_sibling_cd_lines(source), [1, 4])

    def test_non_algol_parser_and_lexer_build_fronts_keep_package_directory(
        self,
    ) -> None:
        violations: list[str] = []

        for package_root in sorted(TYPESCRIPT_PACKAGES.iterdir()):
            if not package_root.is_dir():
                continue
            if not is_parser_or_lexer(package_root.name):
                continue
            if package_root.name in ALGOL_OWNED_PACKAGES:
                continue

            for build_front in BUILD_FRONTS:
                build_path = package_root / build_front
                if not build_path.is_file():
                    continue
                for line_number in stateful_sibling_cd_lines(
                    build_path.read_text(encoding="utf-8")
                ):
                    violations.append(
                        f"{build_path.relative_to(REPO_ROOT)}:{line_number}"
                    )

        self.assertEqual(
            violations,
            [],
            "dependency installs must use subshells so package tests run from "
            "their own parser/lexer directory",
        )


if __name__ == "__main__":
    unittest.main()
