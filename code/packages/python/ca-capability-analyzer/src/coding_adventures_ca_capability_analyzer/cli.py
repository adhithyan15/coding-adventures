"""Command-line interface for ca-capability-analyzer (VER08).

Pure Python with zero external dependencies.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence

from coding_adventures_ca_capability_analyzer.analyzer import analyze_package


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="ca-capability-analyzer",
        description="Pure Python AST capability scanner and static CWE detector.",
    )
    parser.add_argument(
        "target",
        nargs="?",
        default=".",
        help="Path to Python package directory or single file (default: .)",
    )
    parser.add_argument(
        "--manifest",
        "-m",
        default=None,
        help="Path to explicit required_capabilities.json manifest",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Fail on all warnings as well as errors",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output analysis report as JSON",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    """CLI entrypoint."""
    args = parse_args(argv)
    try:
        report = analyze_package(
            args.target,
            manifest_path=args.manifest,
            strict=args.strict,
        )
    except Exception as ex:
        sys.stderr.write(f"Error during analysis: {ex}\n")
        return 2

    if args.json:
        sys.stdout.write(json.dumps(report.to_dict(), indent=2) + "\n")
    else:
        sys.stdout.write(report.render_text() + "\n")

    return 0 if report.passed else 1


if __name__ == "__main__":
    sys.exit(main())
