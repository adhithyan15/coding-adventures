#!/usr/bin/env python3
"""Exercise Go module and Python wheel consumers outside the source tree."""

from __future__ import annotations

import os
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GO_MICROBENCHMARK = ROOT / "code/packages/go/microbenchmark"
GO_RANK_STATISTICS = ROOT / "code/packages/go/rank-statistics"
PYTHON_MICROBENCHMARK = ROOT / "code/packages/python/microbenchmark"
PYTHON_RANK_STATISTICS = ROOT / "code/packages/python/rank-statistics"
GO_MICROBENCHMARK_MODULE = (
    "github.com/adhithyan15/coding-adventures/code/packages/go/microbenchmark"
)
GO_RANK_STATISTICS_MODULE = (
    "github.com/adhithyan15/coding-adventures/code/packages/go/rank-statistics"
)


def run(command: list[str], *, cwd: Path, env: dict[str, str] | None = None) -> None:
    subprocess.run(command, cwd=cwd, env=env, check=True)


def go_consumer() -> None:
    """Compile a separate Go module with explicit local source replacements."""
    with tempfile.TemporaryDirectory(prefix="microbenchmark-go-consumer-") as raw:
        consumer = Path(raw)
        (consumer / "go.mod").write_text(
            "\n".join(
                [
                    "module example.com/microbenchmark-consumer",
                    "",
                    "go 1.23",
                    "",
                    "require (",
                    "    " + GO_MICROBENCHMARK_MODULE + " v0.0.0",
                    "    " + GO_RANK_STATISTICS_MODULE + " v0.0.0",
                    ")",
                    "",
                    "replace "
                    + GO_MICROBENCHMARK_MODULE
                    + " => "
                    + GO_MICROBENCHMARK.as_posix(),
                    "replace "
                    + GO_RANK_STATISTICS_MODULE
                    + " => "
                    + GO_RANK_STATISTICS.as_posix(),
                    "",
                ]
            ),
            encoding="utf-8",
        )
        (consumer / "main.go").write_text(
            """package main

import (
    "fmt"
    microbenchmark "@MICROBENCHMARK_MODULE@"
)

func main() {
    pairs := make([]microbenchmark.Pair, 11)
    for i := range pairs {
        pairs[i] = microbenchmark.Pair{
            BaselineNS: 100,
            CandidateNS: 100,
            Order: "baseline-first",
        }
    }
    result, err := microbenchmark.Analyze(pairs, 95, 1.05)
    if err != nil { panic(err) }
    fmt.Printf("%.1f %s\\n", result.Ratio, result.Verdict)
}
""".replace("@MICROBENCHMARK_MODULE@", GO_MICROBENCHMARK_MODULE),
            encoding="utf-8",
        )
        env = os.environ.copy()
        env.update({"GOWORK": "off", "GOPROXY": "off", "GO111MODULE": "on"})
        result = subprocess.run(
            ["go", "run", "."],
            cwd=consumer,
            env=env,
            check=True,
            capture_output=True,
            text=True,
        )
        if result.stdout != "1.0 equivalent\n":
            raise RuntimeError(
                f"unexpected external Go consumer output: {result.stdout!r}"
            )
        print(
            "Go external module consumer passed "
            "with explicit local replace (source-backed)."
        )


def python_consumer() -> None:
    """Build both distributions and use the wheel from an isolated venv."""
    uv = os.environ.get("UV", "uv")
    with tempfile.TemporaryDirectory(prefix="microbenchmark-python-consumer-") as raw:
        temp = Path(raw)
        dist = temp / "dist"
        dist.mkdir()
        for package in (PYTHON_RANK_STATISTICS, PYTHON_MICROBENCHMARK):
            run(
                [uv, "build", "--wheel", "--sdist", "--out-dir", str(dist)], cwd=package
            )

        sdist = next(dist.glob("coding_adventures_microbenchmark-*.tar.gz"))
        with tarfile.open(sdist, "r:gz") as archive:
            fixture_member = next(
                member
                for member in archive.getmembers()
                if member.name.endswith("/tests/microbenchmark-pairs-v1.json")
            )
            extracted = archive.extractfile(fixture_member)
            if extracted is None:
                raise RuntimeError("microbenchmark sdist fixture is not readable")
            fixture = temp / "packaged-fixture.json"
            fixture.write_bytes(extracted.read())

        venv = temp / ".venv"
        interpreter = os.environ.get("UV_PYTHON", sys.executable)
        run([uv, "venv", str(venv), "--python", interpreter], cwd=temp)
        python = venv / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
        run(
            [
                uv,
                "pip",
                "install",
                "--python",
                str(python),
                "--no-index",
                "--find-links",
                str(dist),
                "coding-adventures-rank-statistics",
                "coding-adventures-microbenchmark",
            ],
            cwd=temp,
        )
        smoke = """import json, sys
from microbenchmark import Observation, analyze

with open(sys.argv[1], encoding="utf-8") as handle:
    fixture = json.load(handle)
for case in fixture["cases"]:
    pairs = [
        Observation(case["baseline_ns"], candidate, "baseline-first")
        for candidate in case["candidate_ns"]
    ]
    result = analyze(pairs, fixture["confidence_percent"], fixture["minimum_factor"])
    assert result.verdict == case["verdict"], case["name"]
    assert abs(result.ratio - case["ratio"]) <= 1e-12, case["name"]
    expected_interval = case["interval"]
    actual_interval = None if result.interval is None else list(result.interval)
    assert actual_interval == expected_interval, case["name"]
print("installed package analyzed packaged fixture")
"""
        env = os.environ.copy()
        env.pop("PYTHONPATH", None)
        run([str(python), "-I", "-c", smoke, str(fixture)], cwd=temp, env=env)
        run(
            [
                str(
                    venv
                    / (
                        "Scripts/array-microbenchmark.exe"
                        if os.name == "nt"
                        else "bin/array-microbenchmark"
                    )
                ),
                "--help",
            ],
            cwd=temp,
        )
        print(
            "Python wheel installed without index/editable sources; "
            "sdist fixture and CLI passed."
        )


if __name__ == "__main__":
    if len(sys.argv) != 2 or sys.argv[1] not in {"go", "python"}:
        raise SystemExit("usage: microbenchmark-installed-consumer.py go|python")
    {"go": go_consumer, "python": python_consumer}[sys.argv[1]]()
