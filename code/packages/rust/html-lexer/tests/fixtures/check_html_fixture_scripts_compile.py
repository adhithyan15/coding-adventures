#!/usr/bin/env python3

"""Check that HTML lexer/parser fixture Python scripts compile."""

from __future__ import annotations

import argparse
import py_compile
import tempfile
from pathlib import Path


FIXTURE_DIR = Path(__file__).resolve().parent
RUST_DIR = FIXTURE_DIR.parents[2]
PARSER_FIXTURE_DIR = RUST_DIR / "html-parser" / "tests" / "fixtures"
SCRIPT_GLOB = "*.py"


def main() -> int:
    parse_args()
    script_paths = fixture_scripts()

    errors: list[str] = []
    with tempfile.TemporaryDirectory(prefix="html-fixture-pycompile-") as temp_dir:
        temp_path = Path(temp_dir)
        for script_path in script_paths:
            target = temp_path / f"{script_path.stem}.pyc"
            try:
                py_compile.compile(
                    str(script_path),
                    cfile=str(target),
                    doraise=True,
                )
            except py_compile.PyCompileError as exc:
                errors.append(f"{relative_script(script_path)}: {exc.msg}")

    print("HTML fixture Python scripts")
    print(f"scripts: {len(script_paths)}")

    if errors:
        raise SystemExit("\n\n".join(errors))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Compile all checked-in Python fixture helpers and generators for "
            "the Rust HTML lexer/parser conformance corpora."
        )
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compatibility flag for generated-fixture stale-check manifests.",
    )
    return parser.parse_args()


def fixture_scripts() -> list[Path]:
    fixture_dirs = (FIXTURE_DIR, PARSER_FIXTURE_DIR)
    return sorted(
        script_path
        for fixture_dir in fixture_dirs
        for script_path in fixture_dir.glob(SCRIPT_GLOB)
        if script_path.is_file()
    )


def relative_script(path: Path) -> str:
    try:
        return str(path.relative_to(RUST_DIR))
    except ValueError:
        return str(path)


if __name__ == "__main__":
    raise SystemExit(main())
