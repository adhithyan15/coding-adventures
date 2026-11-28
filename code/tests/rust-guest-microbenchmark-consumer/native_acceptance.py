"""Run one guest's default paired benchmark and an external replay consumer."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import tempfile
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package", required=True, help="driver Cargo.toml, relative to cwd")
    parser.add_argument("--bin", required=True)
    parser.add_argument("--guest", required=True)
    parser.add_argument("--workload", required=True)
    args = parser.parse_args()

    package_manifest = Path(args.package).resolve()
    consumer_manifest = Path(__file__).resolve().parent / "Cargo.toml"
    environment = os.environ.copy()
    with tempfile.TemporaryDirectory(prefix="guest-microbenchmark-") as directory:
        core_path = Path(directory) / "report.json"
        envelope_path = Path(directory) / "manifest.json"
        run = subprocess.run(
            [
                "cargo", "run", "--release", "--locked", "--manifest-path",
                str(package_manifest), "--bin", args.bin, "--", "--pairs", "40",
                "--revision", "build-acceptance", "--manifest", str(envelope_path),
            ],
            cwd=package_manifest.parent,
            env=environment,
            text=True,
            capture_output=True,
            check=False,
        )
        if run.returncode:
            raise RuntimeError(f"native driver failed ({run.returncode}):\n{run.stderr}")
        document = json.loads(run.stdout)
        pairs = document.get("pairs")
        if document.get("config", {}).get("pairs") != 40 or not isinstance(pairs, list) or len(pairs) != 40:
            raise RuntimeError("native driver did not emit the requested 40-pair core report")
        if any(pair.get("baseline_ns", 0) <= 0 or pair.get("candidate_ns", 0) <= 0 for pair in pairs):
            raise RuntimeError("native driver emitted a non-positive timed pair")
        core_path.write_text(run.stdout, encoding="utf-8")
        for replay_args in (("--replay", str(core_path)), ("--replay-envelope", str(envelope_path))):
            replay = subprocess.run(
                ["cargo", "run", "--release", "--locked", "--manifest-path", str(package_manifest),
                 "--bin", args.bin, "--", *replay_args],
                cwd=package_manifest.parent,
                env=environment,
                text=True,
                capture_output=True,
                check=False,
            )
            if replay.returncode or "replay valid:" not in replay.stdout:
                raise RuntimeError(f"driver replay {replay_args[0]} failed: {replay.stderr or replay.stdout}")
        consumer = subprocess.run(
            [
                "cargo", "run", "--locked", "--manifest-path", str(consumer_manifest), "--",
                args.guest, args.workload, str(core_path), str(envelope_path),
            ],
            cwd=package_manifest.parent,
            env=environment,
            text=True,
            capture_output=True,
            check=False,
        )
        if consumer.returncode:
            raise RuntimeError(f"external report consumer failed ({consumer.returncode}):\n{consumer.stderr}")
        print(consumer.stdout.strip())
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, json.JSONDecodeError) as error:
        raise SystemExit(str(error))
