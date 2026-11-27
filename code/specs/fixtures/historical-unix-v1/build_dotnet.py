"""Build external .NET fixture adapters and replay the canonical HOS00 trace."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path


FIXTURE_ROOT = Path(__file__).resolve().parent
REPLAY = FIXTURE_ROOT / "replay_cli.py"
PACKAGE_ROOTS = {
    "csharp": FIXTURE_ROOT.parents[2] / "packages" / "csharp" / "unix-history-simulator",
    "fsharp": FIXTURE_ROOT.parents[2] / "packages" / "fsharp" / "unix-history-simulator",
}
ADAPTER_PROJECTS = {
    "csharp": FIXTURE_ROOT / "test-adapters" / "csharp" / "FixtureCli.csproj",
    "fsharp": FIXTURE_ROOT / "test-adapters" / "fsharp" / "FixtureCli.fsproj",
}


def run(arguments: list[str], *, cwd: Path, environment: dict[str, str]) -> None:
    subprocess.run(arguments, cwd=cwd, env=environment, check=True)


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in PACKAGE_ROOTS:
        print("usage: build_dotnet.py csharp|fsharp", file=sys.stderr)
        return 2

    language = sys.argv[1]
    package_root = PACKAGE_ROOTS[language]
    adapter_project = ADAPTER_PROJECTS[language]
    dotnet = os.environ.get("DOTNET", "dotnet")

    with tempfile.TemporaryDirectory(prefix=f"hos-{language}-dotnet-") as temporary:
        temporary_root = Path(temporary)
        artifacts = temporary_root / "artifacts"
        environment = os.environ.copy()
        environment.update(
            {
                "DOTNET_CLI_HOME": str(temporary_root / "dotnet-home"),
                "DOTNET_SKIP_FIRST_TIME_EXPERIENCE": "1",
                "DOTNET_CLI_TELEMETRY_OPTOUT": "1",
                "NUGET_PACKAGES": str(temporary_root / "nuget-packages"),
                "TMPDIR": str(temporary_root),
                "TEMP": str(temporary_root),
                "TMP": str(temporary_root),
            }
        )

        run(
            [
                dotnet,
                "build",
                str(adapter_project),
                "--configuration",
                "Release",
                "--disable-build-servers",
                "--artifacts-path",
                str(artifacts),
            ],
            cwd=package_root,
            environment=environment,
        )

        adapter_command = [
            dotnet,
            "run",
            "--project",
            str(adapter_project),
            "--configuration",
            "Release",
            "--no-build",
            "--no-restore",
            "--disable-build-servers",
            "--artifacts-path",
            str(artifacts),
            "--",
        ]
        run(
            [*adapter_command, "--self-test"],
            cwd=package_root,
            environment=environment,
        )
        print(f"PASS {language} component checks", flush=True)
        run(
            [sys.executable, str(REPLAY), *adapter_command],
            cwd=package_root,
            environment=environment,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
