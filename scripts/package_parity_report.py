#!/usr/bin/env python3
"""Report package parity across language buckets.

The report intentionally normalizes only folder-name conventions. It does not
try to decide that every package should exist in every language. That judgment
belongs in the roadmap/spec layer.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Iterable


NORMALIZATION_OVERRIDES = {
    "barcode1d": "barcode-1d",
    "barcode2d": "barcode-2d",
    "barcodelayout1d": "barcode-layout-1d",
    "ean13": "ean-13",
    "imagecodecbmp": "image-codec-bmp",
    "imagecodecppm": "image-codec-ppm",
    "imagecodecqoi": "image-codec-qoi",
    "imagegeometrictransforms": "image-geometric-transforms",
    "imagepointops": "image-point-ops",
    "paintcodecpngnative": "paint-codec-png-native",
    "paintinstructions": "paint-instructions",
    "paintvmdirect2dnative": "paint-vm-direct2d-native",
    "paintvmmetalnative": "paint-vm-metal-native",
    "pixelcontainer": "pixel-container",
    "upca": "upc-a",
}

IGNORED_PACKAGE_DIRS = {
    ".cargo",
}

IGNORED_LANGUAGE_BUCKETS = {
    "starlark",
}


def normalize_package_name(name: str) -> str | None:
    if name in IGNORED_PACKAGE_DIRS:
        return None
    normalized = name.lower().replace("_", "-")
    return NORMALIZATION_OVERRIDES.get(normalized, normalized)


def discover_packages(root: Path) -> dict[str, set[str]]:
    package_root = root / "code" / "packages"
    language_packages: dict[str, set[str]] = {}

    for language_dir in sorted(package_root.iterdir()):
        if not language_dir.is_dir() or language_dir.name in IGNORED_LANGUAGE_BUCKETS:
            continue

        packages: set[str] = set()
        for package_dir in sorted(language_dir.iterdir()):
            if not package_dir.is_dir():
                continue
            normalized = normalize_package_name(package_dir.name)
            if normalized:
                packages.add(normalized)

        language_packages[language_dir.name] = packages

    return language_packages


def sorted_list(values: Iterable[str]) -> list[str]:
    return sorted(values)


def build_report(root: Path) -> dict[str, object]:
    language_packages = discover_packages(root)
    if "rust" not in language_packages or "python" not in language_packages:
        raise SystemExit("package parity report requires rust and python buckets")

    all_packages: set[str] = set()
    for packages in language_packages.values():
        all_packages.update(packages)

    rust_packages = language_packages["rust"]
    python_packages = language_packages["python"]
    rust_python_core = rust_packages & python_packages
    rust_python_union = rust_packages | python_packages

    coverage = []
    for language, packages in sorted(language_packages.items()):
        missing_core = rust_python_core - packages
        present_core = len(rust_python_core) - len(missing_core)
        coverage.append(
            {
                "language": language,
                "present": len(packages),
                "missing_core": len(missing_core),
                "core_coverage": present_core / len(rust_python_core),
                "missing_core_packages": sorted_list(missing_core),
            }
        )

    package_frequency = []
    for package in sorted(all_packages):
        languages = [
            language
            for language, packages in sorted(language_packages.items())
            if package in packages
        ]
        package_frequency.append(
            {
                "package": package,
                "language_count": len(languages),
                "languages": languages,
            }
        )

    return {
        "package_count": {
            "all_languages_union": len(all_packages),
            "rust_python_union": len(rust_python_union),
            "rust_python_core": len(rust_python_core),
            "rust": len(rust_packages),
            "python": len(python_packages),
        },
        "rust_only": sorted_list(rust_packages - python_packages),
        "python_only": sorted_list(python_packages - rust_packages),
        "outside_rust_python": sorted_list(all_packages - rust_python_union),
        "coverage": sorted(coverage, key=lambda row: (row["missing_core"], row["language"])),
        "package_frequency": package_frequency,
    }


def render_markdown(report: dict[str, object]) -> str:
    counts = report["package_count"]
    assert isinstance(counts, dict)
    lines = [
        "# Package Parity Report",
        "",
        "## Summary",
        "",
        "| Baseline | Count |",
        "|---|---:|",
        f"| All normalized package names | {counts['all_languages_union']} |",
        f"| Rust/Python union | {counts['rust_python_union']} |",
        f"| Rust/Python shared core | {counts['rust_python_core']} |",
        f"| Rust packages | {counts['rust']} |",
        f"| Python packages | {counts['python']} |",
        "",
        "## Core Coverage",
        "",
        "| Language | Present | Missing Core | Core Coverage |",
        "|---|---:|---:|---:|",
    ]

    coverage = report["coverage"]
    assert isinstance(coverage, list)
    for row in coverage:
        assert isinstance(row, dict)
        lines.append(
            "| {language} | {present} | {missing_core} | {core_coverage:.1%} |".format(
                **row
            )
        )

    lines.extend(
        [
            "",
            "## Rust Only",
            "",
            ", ".join(report["rust_only"]),  # type: ignore[arg-type]
            "",
            "## Python Only",
            "",
            ", ".join(report["python_only"]),  # type: ignore[arg-type]
            "",
            "## Outside Rust/Python",
            "",
            ", ".join(report["outside_rust_python"]),  # type: ignore[arg-type]
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="repository root",
    )
    parser.add_argument(
        "--format",
        choices=("json", "markdown"),
        default="markdown",
        help="output format",
    )
    args = parser.parse_args()

    report = build_report(args.root)
    if args.format == "json":
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(render_markdown(report))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
