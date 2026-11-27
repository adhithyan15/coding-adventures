"""Build the Elixir library and run its test-only historical fixture adapter."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


FIXTURE_ROOT = Path(__file__).resolve().parent
PACKAGE_ROOT = Path.cwd()
WINDOWS = os.name == "nt"
MIX = ("mix",)


def run_mix(*args: str, extra_environment: dict[str, str] | None = None) -> None:
    command = (*MIX, *args)
    if WINDOWS:
        # setup-beam exposes Mix as a .bat launcher. CreateProcess cannot start
        # that launcher directly; cmd.exe performs normal PATHEXT resolution.
        command = (os.environ.get("COMSPEC", "cmd.exe"), "/d", "/c", *command)
    environment = os.environ.copy()
    if extra_environment:
        environment.update(extra_environment)
    subprocess.run(command, cwd=PACKAGE_ROOT, env=environment, check=True)


def replay_fixture(command: str) -> int:
    adapter = os.path.relpath(
        FIXTURE_ROOT / "test-adapters/elixir/fixture_cli.exs", PACKAGE_ROOT
    )
    run_mix(
        "run",
        "--no-compile",
        adapter,
        extra_environment={"HISTORICAL_UNIX_FIXTURE_COMMAND": command},
    )
    return 0


def main() -> int:
    if len(sys.argv) == 3 and sys.argv[1] == "--fixture":
        return replay_fixture(sys.argv[2])
    if len(sys.argv) != 1:
        print("usage: build_elixir.py [--fixture COMMAND]", file=sys.stderr)
        return 2

    run_mix("format", "--check-formatted")
    run_mix("compile")
    run_mix("test")

    subprocess.run(
        [
            sys.executable,
            str(FIXTURE_ROOT / "replay_cli.py"),
            sys.executable,
            str(Path(__file__).resolve()),
            "--fixture",
        ],
        cwd=PACKAGE_ROOT,
        check=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
