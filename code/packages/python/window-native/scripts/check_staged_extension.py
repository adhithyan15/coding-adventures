"""Stage the compiled extension beside the Python wrapper, then lint and test.

Why this file exists
====================

``window_native`` is two halves that only work together:

    src/window_native/__init__.py      the Python wrapper (what users import)
    target/release/<library>           the Rust extension cargo just built

The wrapper does ``from window_native.window_native import ...``, so the
extension has to sit *inside* the package directory under the name Python
expects (``window_native.pyd`` on Windows, ``window_native.so`` elsewhere).
We never copy it into ``src/``: a stale binary left there would be picked up
by the next run. Instead each run builds a throwaway package directory:

    window-native-build-XXXX/          fresh, private, random name (mkdtemp)
        window_native/
            __init__.py                copied from src/
            window_native.pyd|.so      copied from target/release/

and runs ruff and pytest with ``PYTHONPATH`` pointing at it.

Why a script rather than BUILD lines
====================================

The Unix ``BUILD`` does this with ``sh`` one-liners. On Windows the build
tool runs every ``BUILD_windows`` line as its own ``cmd /C`` invocation, so:

* a variable ``set`` on one line is gone on the next, and
* ``for /f ... %%i`` is batch-*file* syntax; typed at ``cmd /C`` it fails
  with "%%i was unexpected at this time."

The old ``BUILD_windows`` relied on both and had never run cleanly. Doing the
whole stage-check-clean cycle inside one Python process sidesteps cmd.exe's
rules entirely, and works the same on every platform.

Usage: ``python scripts/check_staged_extension.py`` from the package root
(the build tool's working directory). The exit status is ruff's if ruff
fails, otherwise pytest's.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

PACKAGE_ROOT = Path(__file__).resolve().parent.parent

# (what cargo wrote, what Python must find), in the order we look for them.
# cargo names a cdylib ``window_native.dll`` on Windows and
# ``libwindow_native.{so,dylib}`` on Unix; Python wants ``.pyd`` on Windows
# and accepts ``.so`` for an extension module on both Linux and macOS.
ARTIFACTS = (
    ("target/release/window_native.dll", "window_native.pyd"),
    ("target/release/libwindow_native.so", "window_native.so"),
    ("target/release/libwindow_native.dylib", "window_native.so"),
)


def stage(build_dir: Path) -> bool:
    """Fill ``build_dir`` with an importable ``window_native`` package.

    Returns False when cargo produced no extension we recognise.
    """
    package_dir = build_dir / "window_native"
    package_dir.mkdir()
    shutil.copyfile(
        PACKAGE_ROOT / "src" / "window_native" / "__init__.py",
        package_dir / "__init__.py",
    )
    for built, importable in ARTIFACTS:
        source = PACKAGE_ROOT / built
        if source.is_file():
            shutil.copyfile(source, package_dir / importable)
            return True
    return False


def main() -> int:
    # mkdtemp creates the directory itself, atomically and under a random
    # name (owner-only on POSIX; on Windows it inherits the checkout's ACLs),
    # so nothing can be waiting at that path (a planted symlink, say) for the
    # copies below to write through.
    build_dir = Path(tempfile.mkdtemp(prefix="window-native-build-", dir=PACKAGE_ROOT))
    try:
        if not stage(build_dir):
            print("no compiled extension artifact was found", file=sys.stderr)
            return 1
        env = dict(os.environ, PYTHONPATH=str(build_dir))
        for command in (
            [sys.executable, "-m", "ruff", "check", "src", "tests", "scripts"],
            [sys.executable, "-m", "pytest", "tests", "-v"],
        ):
            status = subprocess.run(
                command, cwd=PACKAGE_ROOT, env=env, check=False
            ).returncode
            if status != 0:
                return status
        return 0
    finally:
        shutil.rmtree(build_dir, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
