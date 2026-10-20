#!/usr/bin/env python3
"""Measure how long each package takes in main's full CI build.

Why this exists
===============

Main CI splits the forced full build across shards (``ci.yml``, "Full build
on main merge"). How well those shards balance decides whether the run fits
inside the 150-minute step ceiling, and the build tool can only balance on
what it is told a package costs. Its built-in guess is a heuristic -- "a Rust
crate costs 6, a Python package 2" -- which cannot know that
``rust/smart-home-integration-catalog`` takes twenty minutes while most Rust
crates take twenty seconds.

Every shard already prints the truth at the end of its log::

    Build Report
    ============
    Package                     Status     Duration    Reason
    rust/regex-engine           BUILT      1150.0s
    python/zstd                 BUILT      289.3s
    ...

This script reads those reports from finished runs and writes the median
duration of each package to ``code/specs/data/ci-package-durations.json``,
which ``ci.yml`` hands to the build tool as ``-shard-weights``.

Why the median
--------------
A package is measured once per shard that builds it, on a runner whose other
workers are doing other things, so samples are noisy and occasionally wild (a
cold cache, a neighbour hogging the disk). The median ignores a minority of
outliers in either direction; a mean would let one stalled run drag a
package's weight up for months.

What it does NOT measure
------------------------
Only ``BUILT`` rows count. A failed build stops early and a skipped one costs
nothing, so neither says what the package costs when it works. A package
with no sample at all is simply absent; the build tool then assumes the
median of its toolchain (see ``internal/plan/shard_weights.go``).

Usage
=====

Refresh from the last few completed main runs::

    python3 code/scripts/ci_package_durations.py \\
        --repo adhithyan15/coding-adventures --runs 39139594621 39124927319

or, offline, from job logs already on disk (one file per shard job)::

    python3 code/scripts/ci_package_durations.py --logs path/to/*.log
"""

from __future__ import annotations

import argparse
import json
import re
import statistics
import subprocess  # nosec B404 -- fixed argv to the gh CLI, never a shell
import sys
from collections.abc import Iterable, Mapping, Sequence
from pathlib import Path

SCHEMA = "ci-package-durations-v1"
DEFAULT_OUTPUT = Path("code/specs/data/ci-package-durations.json")

# Shard jobs are named "build (shard-N-of-M)". Only those run the full build;
# the PR legs ("build (ubuntu-latest)") build a diff-selected subset.
SHARD_JOB_NAME = re.compile(r"^build \(shard-\d+-of-\d+\)$")

# GitHub numbers runs and jobs with plain decimal ids. Anything else is not
# a run id and must never reach the gh command line.
NUMERIC_ID = re.compile(r"^[0-9]{1,20}$")
REPO_NAME = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")

# One report row, after the Actions timestamp prefix:
#
#   2026-10-18T18:59:34.8953371Z c/assembler     BUILT    0.2s
#   ^ timestamp                  ^ package       ^ status ^ seconds
#
# Package names are slash-separated words; statuses are upper-case words
# (BUILT, FAILED, DEP-SKIP, ...).
REPORT_ROW = re.compile(
    r"^(?:\S+Z )?(?P<name>[A-Za-z0-9_.+-]+(?:/[A-Za-z0-9_.+-]+)+)\s+"
    r"(?P<status>[A-Z][A-Z-]*)\s+(?P<seconds>\d+(?:\.\d+)?)s(?:\s|$)"
)
REPORT_HEADING = re.compile(r"^(?:\S+Z )?Build Report\s*$")


def parse_build_report(log_text: str) -> dict[str, float]:
    """Return ``{package: seconds}`` for every BUILT row of every report.

    A log normally holds one report; if it holds several (a retried step),
    later rows for the same package win, because they describe the attempt
    that actually finished.
    """

    durations: dict[str, float] = {}
    in_report = False
    for line in log_text.splitlines():
        if REPORT_HEADING.match(line):
            in_report = True
            continue
        if not in_report:
            continue
        match = REPORT_ROW.match(line)
        if match is None:
            continue
        if match["status"] == "BUILT":
            durations[match["name"]] = float(match["seconds"])
    return durations


def median_durations(samples: Iterable[Mapping[str, float]]) -> dict[str, float]:
    """Fold per-log durations into one median per package, rounded to 0.1s."""

    collected: dict[str, list[float]] = {}
    for sample in samples:
        for name, seconds in sample.items():
            collected.setdefault(name, []).append(seconds)
    return {
        name: round(statistics.median(values), 1)
        for name, values in sorted(collected.items())
    }


def render(durations: Mapping[str, float], runs: Sequence[str], logs: int) -> str:
    """Serialize the durations file deterministically (sorted, one per line)."""

    document = {
        "schema": SCHEMA,
        "unit": "seconds",
        "source": {
            "workflow": ".github/workflows/ci.yml",
            "step": "Full build on main merge",
            "statistic": "median of BUILT rows across shard job logs",
            "runs": list(runs),
            "shard_logs": logs,
        },
        "durations": dict(sorted(durations.items())),
    }
    return json.dumps(document, indent=1, sort_keys=False) + "\n"


def _gh(args: Sequence[str]) -> str:
    completed = subprocess.run(  # nosec B603 -- argv list, validated ids
        ["gh", *args], check=True, capture_output=True, text=True
    )
    return completed.stdout


def shard_job_ids(repo: str, run_id: str) -> list[str]:
    """List the shard job ids of one workflow run."""

    payload = _gh(
        [
            "api",
            "--paginate",
            f"repos/{repo}/actions/runs/{run_id}/jobs?per_page=100",
            "--jq",
            ".jobs[] | [.id, .name] | @tsv",
        ]
    )
    ids = []
    for line in payload.splitlines():
        job_id, _, name = line.partition("\t")
        if SHARD_JOB_NAME.match(name) and NUMERIC_ID.match(job_id):
            ids.append(job_id)
    return ids


def fetch_logs(repo: str, runs: Sequence[str]) -> list[str]:
    """Download every shard job log of the given runs."""

    texts = []
    for run_id in runs:
        for job_id in shard_job_ids(repo, run_id):
            try:
                texts.append(_gh(["api", f"repos/{repo}/actions/jobs/{job_id}/logs"]))
            except subprocess.CalledProcessError as exc:
                # A job whose runner died (disk full, cancelled) may have no
                # log. Say so and keep the rest: that is exactly the case
                # where the other shards' numbers matter most.
                print(f"warning: no log for job {job_id}: {exc.stderr.strip()}", file=sys.stderr)
    return texts


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--runs", nargs="+", metavar="RUN_ID", help="completed ci.yml main runs")
    source.add_argument("--logs", nargs="+", type=Path, metavar="LOG", help="shard job logs on disk")
    parser.add_argument("--repo", default="adhithyan15/coding-adventures", help="owner/name for --runs")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args(argv)

    if args.runs:
        if not REPO_NAME.match(args.repo):
            parser.error(f"--repo must be owner/name, got {args.repo!r}")
        for run_id in args.runs:
            if not NUMERIC_ID.match(run_id):
                parser.error(f"run ids are decimal numbers, got {run_id!r}")
        texts = fetch_logs(args.repo, args.runs)
        runs = list(args.runs)
    else:
        texts = [path.read_text(encoding="utf-8", errors="replace") for path in args.logs]
        runs = []

    samples = [parse_build_report(text) for text in texts]
    durations = median_durations(samples)
    if not durations:
        print("error: no BUILT rows found in any Build Report", file=sys.stderr)
        return 1
    args.output.write_text(render(durations, runs, len(texts)), encoding="utf-8")
    print(f"wrote {len(durations)} package durations from {len(texts)} logs to {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
