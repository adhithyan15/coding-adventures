#!/usr/bin/env python3
"""Report package parity across the repository's language buckets.

The inventory is built from Git-visible files (tracked plus untracked and not
ignored), rather than raw filesystem directories. That keeps build outputs such
as ``target``, ``node_modules``, and ``.pytest_cache`` out of the report while
still allowing a newly scaffolded package to appear before its first commit.

Directory presence is structural evidence, not proof of API or behavioural
parity. The roadmap/spec layer decides which package identities are portable.
"""

from __future__ import annotations

import argparse
import csv
import io
import json
import re
import subprocess
from collections.abc import Iterable, Mapping, Sequence
from pathlib import Path
from typing import Any


IMPLEMENTATION_LANGUAGES = (
    "csharp",
    "dart",
    "elixir",
    "fsharp",
    "go",
    "haskell",
    "java",
    "kotlin",
    "lua",
    "perl",
    "python",
    "ruby",
    "rust",
    "swift",
    "typescript",
)

BUCKET_CLASSES = {
    "implementation": IMPLEMENTATION_LANGUAGES,
    "emerging_implementation": ("c", "cpp"),
    "execution_target": ("wasm",),
    "domain_language": ("mosaic", "twig"),
    "build_language": ("starlark",),
}

ALL_BUCKETS = tuple(bucket for buckets in BUCKET_CLASSES.values() for bucket in buckets)

DISPLAY_PRIORITY = (
    "rust",
    "python",
    "typescript",
    "go",
    "ruby",
    "elixir",
    "csharp",
    "fsharp",
    "haskell",
    "java",
    "kotlin",
    "lua",
    "perl",
    "dart",
    "swift",
    "cpp",
    "c",
    "wasm",
    "mosaic",
    "twig",
    "starlark",
)

HIGH_CONSENSUS_MIN_LANGUAGES = 10

IGNORED_PACKAGE_DIRS = {
    ".cargo",
    ".pytest_cache",
    "node_modules",
    "target",
}

# PascalCase Swift packages predate the current kebab-case convention. These
# overrides preserve their established cross-language display names.
DISPLAY_OVERRIDES = {
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


def package_identity(name: str) -> str:
    """Return a punctuation- and case-insensitive package identity key."""

    return re.sub(r"[^a-z0-9]", "", name.lower())


def package_display_name(name: str) -> str:
    """Return the preferred human-readable spelling for a directory name."""

    identity = package_identity(name)
    if identity in DISPLAY_OVERRIDES:
        return DISPLAY_OVERRIDES[identity]

    # Split acronym/PascalCase boundaries before normalizing separators.
    value = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1-\2", name)
    value = re.sub(r"([a-z0-9])([A-Z])", r"\1-\2", value)
    value = re.sub(r"[_.]+", "-", value)
    value = re.sub(r"-+", "-", value)
    return value.lower().strip("-")


def parse_package_paths(
    paths: Iterable[str],
) -> tuple[dict[str, dict[str, set[str]]], set[str]]:
    """Parse Git-visible paths into bucket/identity/directory mappings."""

    packages: dict[str, dict[str, set[str]]] = {bucket: {} for bucket in ALL_BUCKETS}
    unknown_buckets: set[str] = set()

    for raw_path in paths:
        parts = raw_path.strip().replace("\\", "/").split("/")
        if len(parts) < 5 or parts[:2] != ["code", "packages"]:
            continue

        bucket, directory = parts[2], parts[3]
        if bucket not in packages:
            unknown_buckets.add(bucket)
            continue
        if directory.startswith(".") or directory in IGNORED_PACKAGE_DIRS:
            continue

        identity = package_identity(directory)
        if not identity:
            continue
        packages[bucket].setdefault(identity, set()).add(directory)

    return packages, unknown_buckets


def git_visible_package_paths(root: Path) -> list[str]:
    """Return tracked plus untracked, non-ignored paths under code/packages."""

    result = subprocess.run(
        [
            "git",
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            "code/packages",
        ],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return [
        path
        for raw_path in result.stdout.split(b"\0")
        if raw_path
        for path in [raw_path.decode("utf-8", errors="surrogateescape")]
        if (root / Path(path)).is_file()
    ]


def discover_packages(
    root: Path,
) -> tuple[dict[str, dict[str, set[str]]], set[str]]:
    return parse_package_paths(git_visible_package_paths(root))


def identity_sets(
    packages: Mapping[str, Mapping[str, set[str]]],
) -> dict[str, set[str]]:
    return {bucket: set(entries) for bucket, entries in packages.items()}


def preferred_display_name(
    identity: str, packages: Mapping[str, Mapping[str, set[str]]]
) -> str:
    for bucket in DISPLAY_PRIORITY:
        directories = packages.get(bucket, {}).get(identity)
        if directories:
            return package_display_name(sorted(directories)[0])
    return identity


def display_names(
    identities: Iterable[str], packages: Mapping[str, Mapping[str, set[str]]]
) -> list[str]:
    return sorted(preferred_display_name(identity, packages) for identity in identities)


def completion_band(language_count: int) -> str:
    if language_count >= 10:
        return "10-15"
    if language_count >= 5:
        return "5-9"
    if language_count >= 2:
        return "2-4"
    return "1"


def build_report(
    packages: dict[str, dict[str, set[str]]], unknown_buckets: set[str]
) -> dict[str, Any]:
    sets = identity_sets(packages)
    implementation_union: set[str] = set().union(
        *(sets[language] for language in IMPLEMENTATION_LANGUAGES)
    )
    all_reported_union: set[str] = set().union(
        *(sets[bucket] for bucket in ALL_BUCKETS)
    )

    rust_packages = sets["rust"]
    python_packages = sets["python"]
    rust_python_core = rust_packages & python_packages
    rust_python_union = rust_packages | python_packages

    implementation_frequency = {
        identity: sum(
            identity in sets[language] for language in IMPLEMENTATION_LANGUAGES
        )
        for identity in implementation_union
    }
    high_consensus = {
        identity
        for identity, count in implementation_frequency.items()
        if count >= HIGH_CONSENSUS_MIN_LANGUAGES
    }

    coverage: list[dict[str, Any]] = []
    for language in IMPLEMENTATION_LANGUAGES:
        language_set = sets[language]
        missing_core = rust_python_core - language_set
        missing_high_consensus = high_consensus - language_set
        rust_overlap = rust_packages & language_set
        coverage.append(
            {
                "language": language,
                "present": len(language_set),
                "union_coverage": (
                    len(language_set) / len(implementation_union)
                    if implementation_union
                    else 1.0
                ),
                "rust_overlap": len(rust_overlap),
                "missing_from_rust_set": len(rust_packages - language_set),
                "missing_core": len(missing_core),
                "core_coverage": (
                    (len(rust_python_core) - len(missing_core)) / len(rust_python_core)
                    if rust_python_core
                    else 1.0
                ),
                "missing_core_packages": display_names(missing_core, packages),
                "missing_high_consensus": len(missing_high_consensus),
                "missing_high_consensus_packages": display_names(
                    missing_high_consensus, packages
                ),
            }
        )

    package_frequency: list[dict[str, Any]] = []
    for identity in sorted(all_reported_union):
        languages = [bucket for bucket in ALL_BUCKETS if identity in sets[bucket]]
        implementation_languages = [
            language
            for language in IMPLEMENTATION_LANGUAGES
            if identity in sets[language]
        ]
        package_frequency.append(
            {
                "identity": identity,
                "package": preferred_display_name(identity, packages),
                "language_count": len(implementation_languages),
                "languages": languages,
                "implementation_languages": implementation_languages,
            }
        )

    collisions: list[dict[str, Any]] = []
    for bucket in ALL_BUCKETS:
        for identity, directories in packages[bucket].items():
            if len(directories) > 1:
                collisions.append(
                    {
                        "language": bucket,
                        "identity": identity,
                        "package": preferred_display_name(identity, packages),
                        "directories": sorted(directories),
                    }
                )

    breadth: dict[str, dict[str, int]] = {
        "10-15": {"packages": 0, "missing_slots": 0},
        "5-9": {"packages": 0, "missing_slots": 0},
        "2-4": {"packages": 0, "missing_slots": 0},
        "1": {"packages": 0, "missing_slots": 0},
    }
    for language_count in implementation_frequency.values():
        band = completion_band(language_count)
        breadth[band]["packages"] += 1
        breadth[band]["missing_slots"] += len(IMPLEMENTATION_LANGUAGES) - language_count

    singleton_by_language = {
        language: sum(
            implementation_frequency[identity] == 1 for identity in sets[language]
        )
        for language in IMPLEMENTATION_LANGUAGES
    }

    return {
        "schema_version": 2,
        "bucket_classes": {key: list(value) for key, value in BUCKET_CLASSES.items()},
        "package_count": {
            "all_reported_union": len(all_reported_union),
            "implementation_union": len(implementation_union),
            "implementation_directories": sum(
                len(directories)
                for language in IMPLEMENTATION_LANGUAGES
                for directories in packages[language].values()
            ),
            "implementation_package_slots": sum(
                len(sets[language]) for language in IMPLEMENTATION_LANGUAGES
            ),
            "high_consensus": len(high_consensus),
            "rust_python_union": len(rust_python_union),
            "rust_python_core": len(rust_python_core),
            "rust": len(rust_packages),
            "python": len(python_packages),
        },
        # Backwards-compatible Rust/Python delta fields.
        "rust_only": display_names(rust_packages - python_packages, packages),
        "python_only": display_names(python_packages - rust_packages, packages),
        "outside_rust_python": display_names(
            all_reported_union - rust_python_union, packages
        ),
        "rust_singletons": display_names(
            {
                identity
                for identity in rust_packages
                if implementation_frequency[identity] == 1
            },
            packages,
        ),
        "singleton_by_language": singleton_by_language,
        "completion_bands": breadth,
        "coverage": coverage,
        "package_frequency": package_frequency,
        "collisions": collisions,
        "unknown_language_buckets": sorted(unknown_buckets),
        "special_buckets": [
            {
                "language": bucket,
                "class": class_name,
                "present": len(sets[bucket]),
                "packages": display_names(sets[bucket], packages),
            }
            for class_name, buckets in BUCKET_CLASSES.items()
            if class_name != "implementation"
            for bucket in buckets
        ],
    }


def render_markdown(report: Mapping[str, Any]) -> str:
    counts = report["package_count"]
    lines = [
        "# Package Parity Report",
        "",
        "The report inventories Git-visible package files. It measures structural",
        "presence only; portable/native applicability remains a roadmap decision.",
        "",
        "## Summary",
        "",
        "| Baseline | Count |",
        "|---|---:|",
        f"| Established implementation languages | {len(IMPLEMENTATION_LANGUAGES)} |",
        f"| Implementation package directories | {counts['implementation_directories']} |",
        f"| Distinct implementation package identities | {counts['implementation_union']} |",
        f"| Packages present in at least {HIGH_CONSENSUS_MIN_LANGUAGES} languages | {counts['high_consensus']} |",
        f"| Rust packages | {counts['rust']} |",
        f"| Python packages | {counts['python']} |",
        f"| Rust/Python shared core | {counts['rust_python_core']} |",
        "",
        "## Implementation Coverage",
        "",
        "| Language | Packages | Union Coverage | High-Consensus Gaps |",
        "|---|---:|---:|---:|",
    ]

    for row in sorted(report["coverage"], key=lambda item: item["language"]):
        lines.append(
            "| {language} | {present} | {union_coverage:.1%} | "
            "{missing_high_consensus} |".format(**row)
        )

    lines.extend(
        [
            "",
            "## Completion Bands",
            "",
            "| Present In | Packages | Missing Slots To All 15 |",
            "|---|---:|---:|",
        ]
    )
    for band in ("10-15", "5-9", "2-4", "1"):
        row = report["completion_bands"][band]
        language_label = "language" if band == "1" else "languages"
        lines.append(
            f"| {band} {language_label} | {row['packages']} | {row['missing_slots']} |"
        )

    lines.extend(
        [
            "",
            "## Singleton Packages",
            "",
            "| Language | Singletons |",
            "|---|---:|",
        ]
    )
    for language, count in report["singleton_by_language"].items():
        lines.append(f"| {language} | {count} |")

    lines.extend(
        [
            "",
            "## Special Buckets",
            "",
            "| Bucket | Class | Packages |",
            "|---|---|---:|",
        ]
    )
    for row in report["special_buckets"]:
        lines.append(f"| {row['language']} | {row['class']} | {row['present']} |")

    lines.extend(["", "## Identity Findings", ""])
    if report["collisions"]:
        for collision in report["collisions"]:
            directories = ", ".join(f"`{name}`" for name in collision["directories"])
            lines.append(
                f"- `{collision['language']}/{collision['package']}`: {directories}"
            )
    else:
        lines.append("- No within-language canonical identity collisions.")

    if report["unknown_language_buckets"]:
        unknown = ", ".join(
            f"`{bucket}`" for bucket in report["unknown_language_buckets"]
        )
        lines.append(f"- Unclassified package buckets: {unknown}")
    else:
        lines.append("- No unclassified package buckets.")

    lines.extend(
        [
            "",
            "Use `--format json` for complete gap lists and `--format csv` for the",
            "full package-by-language presence matrix.",
            "",
        ]
    )
    return "\n".join(lines)


def render_csv(report: Mapping[str, Any]) -> str:
    output = io.StringIO(newline="")
    writer = csv.writer(output, lineterminator="\n")
    writer.writerow(["package", *ALL_BUCKETS])
    for row in report["package_frequency"]:
        present = set(row["languages"])
        writer.writerow(
            [
                row["package"],
                *("1" if bucket in present else "0" for bucket in ALL_BUCKETS),
            ]
        )
    return output.getvalue()


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="repository root",
    )
    parser.add_argument(
        "--format",
        choices=("json", "markdown", "csv"),
        default="markdown",
        help="output format",
    )
    parser.add_argument(
        "--fail-on-collisions",
        action="store_true",
        help="exit non-zero when two directories normalize to one identity",
    )
    args = parser.parse_args(argv)

    packages, unknown_buckets = discover_packages(args.root.resolve())
    report = build_report(packages, unknown_buckets)

    if args.format == "json":
        print(json.dumps(report, indent=2, sort_keys=True))
    elif args.format == "csv":
        print(render_csv(report), end="")
    else:
        print(render_markdown(report))

    if report["unknown_language_buckets"]:
        return 2
    if args.fail_on_collisions and report["collisions"]:
        return 3
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
