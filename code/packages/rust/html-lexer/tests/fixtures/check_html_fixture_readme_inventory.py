#!/usr/bin/env python3

"""Check that HTML fixture READMEs mention the user-facing fixture artifacts."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path


FIXTURE_DIR = Path(__file__).resolve().parent
RUST_DIR = FIXTURE_DIR.parents[2]
PARSER_FIXTURE_DIR = RUST_DIR / "html-parser" / "tests" / "fixtures"
SCRIPT_INVENTORY = FIXTURE_DIR / "html-fixture-scripts.txt"

DATA_SUFFIXES = {".dat", ".json", ".test"}
SCRIPT_PREFIXES = ("audit_", "check_", "generate_", "normalize_")
IGNORED_SCRIPT_PREFIXES = ("test_",)
IGNORED_SCRIPT_NAMES = {"generated_fixture_io.py"}


@dataclass(frozen=True)
class ReadmeInventoryStats:
    documented_artifact_count: int


def main() -> int:
    parse_args()
    errors, stats = check_readme_inventory()

    print("HTML fixture README inventory")
    print(f"documented artifacts: {stats.documented_artifact_count}")

    if errors:
        raise SystemExit("\n\n".join(errors))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Check that checked-in HTML lexer/parser fixture artifacts that users "
            "run, regenerate, or inspect are mentioned in their package README."
        )
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compatibility flag for generated-fixture stale-check manifests.",
    )
    return parser.parse_args()


def check_readme_inventory() -> tuple[list[str], ReadmeInventoryStats]:
    errors: list[str] = []
    artifacts = readme_artifacts()
    seen = set()

    for readme_path, artifact_name in artifacts:
        key = (readme_path, artifact_name)
        if key in seen:
            errors.append(f"{relative_fixture(readme_path)}: duplicate README inventory target {artifact_name}")
            continue
        seen.add(key)

        readme_text = readme_path.read_text(encoding="utf-8")
        if artifact_name not in readme_text:
            errors.append(
                f"{relative_fixture(readme_path)}: missing README mention for {artifact_name}"
            )

    stats = ReadmeInventoryStats(documented_artifact_count=len(artifacts))
    return errors, stats


def readme_artifacts() -> list[tuple[Path, str]]:
    return sorted(
        [
            *readme_data_artifacts(FIXTURE_DIR),
            *readme_data_artifacts(PARSER_FIXTURE_DIR),
            *readme_script_artifacts(),
        ],
        key=lambda item: (relative_fixture(item[0]), item[1]),
    )


def readme_data_artifacts(fixture_dir: Path) -> list[tuple[Path, str]]:
    readme_path = fixture_dir / "README.md"
    return [
        (readme_path, fixture_path.name)
        for fixture_path in fixture_dir.iterdir()
        if fixture_path.is_file()
        and fixture_path.suffix in DATA_SUFFIXES
        and fixture_path.name != readme_path.name
    ]


def readme_script_artifacts() -> list[tuple[Path, str]]:
    artifacts: list[tuple[Path, str]] = []
    for relative_path in read_script_inventory():
        script_path = RUST_DIR / relative_path
        if not script_path.exists():
            continue
        if not is_user_facing_fixture_script(script_path.name):
            continue

        if script_path.parent == FIXTURE_DIR:
            artifacts.append((FIXTURE_DIR / "README.md", script_path.name))
        elif script_path.parent == PARSER_FIXTURE_DIR:
            artifacts.append((PARSER_FIXTURE_DIR / "README.md", script_path.name))
    return artifacts


def read_script_inventory() -> list[Path]:
    return [
        Path(line.strip())
        for line in SCRIPT_INVENTORY.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.startswith("#")
    ]


def is_user_facing_fixture_script(script_name: str) -> bool:
    if script_name in IGNORED_SCRIPT_NAMES:
        return False
    if script_name.startswith(IGNORED_SCRIPT_PREFIXES):
        return False
    return script_name.startswith(SCRIPT_PREFIXES)


def relative_fixture(path: Path) -> str:
    try:
        return str(path.relative_to(RUST_DIR))
    except ValueError:
        return str(path)


if __name__ == "__main__":
    raise SystemExit(main())
