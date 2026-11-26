"""Build the heap wheel and test it from a clean environment."""

from __future__ import annotations

import subprocess
import sys
import tempfile
import venv
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
CONSUMER = Path(__file__).with_name("installed_consumer.py")


def run(command: list[str], cwd: Path | None = None) -> None:
    subprocess.run(command, cwd=cwd, check=True)


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="heap-wheel-consumer-") as temporary:
        root = Path(temporary)
        wheel_directory = root / "wheel"
        wheel_directory.mkdir()
        run(
            [
                sys.executable,
                "-m",
                "pip",
                "wheel",
                "--no-deps",
                "--wheel-dir",
                str(wheel_directory),
                str(PACKAGE),
            ]
        )
        wheels = list(wheel_directory.glob("coding_adventures_heap-*.whl"))
        if len(wheels) != 1:
            raise RuntimeError(f"expected one heap wheel, found {wheels!r}")

        consumer = root / "consumer"
        venv.EnvBuilder(with_pip=True).create(consumer)
        python = consumer / (
            "Scripts/python.exe" if sys.platform == "win32" else "bin/python"
        )
        run([str(python), "-m", "pip", "install", "--no-deps", str(wheels[0])])
        run([str(python), str(CONSUMER)])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
