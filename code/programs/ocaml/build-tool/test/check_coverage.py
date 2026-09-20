"""Fail closed when bisect_ppx reports less than the reviewed source floor."""

from __future__ import annotations

import argparse
import re
import subprocess  # nosec B404
import xml.etree.ElementTree as ET  # nosec B405
from pathlib import Path, PurePosixPath


def same_path_suffix(candidate: str, expected: str) -> bool:
    candidate_parts = PurePosixPath(candidate.replace("\\", "/")).parts
    expected_parts = PurePosixPath(expected.replace("\\", "/")).parts
    return len(candidate_parts) >= len(expected_parts) and (
        candidate_parts[-len(expected_parts) :] == expected_parts
    )


def source_percentage(summary: str, source: str) -> float:
    for line in summary.splitlines():
        fields = line.split()
        if not fields or not same_path_suffix(fields[-1], source):
            continue
        parenthesized = re.findall(r"\(([0-9]+(?:\.[0-9]+)?)%\)", line)
        if parenthesized:
            return float(parenthesized[-1])
        per_file = re.match(r"\s*([0-9]+(?:\.[0-9]+)?)\s+%\s+", line)
        if per_file:
            return float(per_file.group(1))
    raise ValueError(f"coverage summary has no percentage for {source}")


def require_minimum(summary: str, source: str, minimum: float) -> float:
    percentage = source_percentage(summary, source)
    if percentage < minimum:
        raise ValueError(f"coverage {percentage:.2f}% is below required {minimum:.2f}%")
    return percentage


def report_command(
    kind: str, pattern: str, sources: list[str], output: Path | None = None
) -> list[str]:
    coverage_files = sorted(Path.cwd().glob(pattern))
    if not coverage_files:
        raise ValueError(f"coverage glob matched no files: {pattern}")
    command = ["opam", "exec", "--", "bisect-ppx-report", kind]
    if output is not None:
        command.append(str(output))
    if kind == "summary":
        command.append("--per-file")
    for source in sources:
        command.extend(("--expect", source))
    command.extend(str(path) for path in coverage_files)
    return command


def uncovered_lines(report: Path, source: str) -> list[int]:
    # The report is produced locally by bisect-ppx-report in this invocation.
    root = ET.parse(report).getroot()  # nosec B314
    for entry in root.iter("class"):
        filename = entry.attrib.get("filename", "")
        if same_path_suffix(filename, source):
            return [
                int(line.attrib["number"])
                for line in entry.iter("line")
                if int(line.attrib.get("hits", "0")) == 0
            ]
    raise ValueError(f"coverage XML has no class for {source}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--summary", required=True, type=Path)
    parser.add_argument("--coverage-glob", required=True)
    parser.add_argument("--source", required=True, action="append")
    parser.add_argument("--minimum", required=True, type=float)
    args = parser.parse_args()
    args.summary.parent.mkdir(parents=True, exist_ok=True)
    with args.summary.open("w", encoding="utf-8") as output:
        # Fixed executable/flags are passed directly without a shell.
        subprocess.run(  # nosec B603
            report_command("summary", args.coverage_glob, args.source),
            check=True,
            stdout=output,
            text=True,
        )
    cobertura = args.summary.with_suffix(".xml")
    # Fixed executable/flags are passed directly without a shell.
    subprocess.run(  # nosec B603
        report_command("cobertura", args.coverage_glob, args.source, cobertura),
        check=True,
        text=True,
    )
    summary = args.summary.read_text(encoding="utf-8")
    print(summary, end="" if summary.endswith("\n") else "\n")
    for source in args.source:
        try:
            percentage = require_minimum(summary, source, args.minimum)
        except ValueError as error:
            if "below required" in str(error):
                missed = ",".join(
                    str(line) for line in uncovered_lines(cobertura, source)
                )
                raise ValueError(
                    f"{error}; uncovered source lines: {missed}"
                ) from error
            raise
        print(f"{source}: {percentage:.2f}% (minimum {args.minimum:.2f}%)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
