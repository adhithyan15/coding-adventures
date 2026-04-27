"""Command-line entry point for the Python Mini Redis worker."""

from __future__ import annotations

import sys
from pathlib import Path

if __package__ in {None, ""}:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from mini_redis_python_worker import run_stdio_worker
else:
    from . import run_stdio_worker


def main() -> None:
    """Run the worker on stdin/stdout."""

    run_stdio_worker()


if __name__ == "__main__":
    main()
