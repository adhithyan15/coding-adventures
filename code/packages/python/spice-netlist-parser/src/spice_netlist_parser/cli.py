"""Command-line entrypoint for Berkeley SPICE netlist execution."""

from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence
from pathlib import Path

from spice_netlist_parser.parser import CLI_ERROR_CODE, run_netlist_json


def _arguments(argv: Sequence[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="spice-netlist-parser")
    parser.add_argument("command", choices=["run"])
    parser.add_argument("--json", action="store_true", dest="json_output")
    parser.add_argument("deck", help="SPICE deck path, or - for stdin")
    arguments = parser.parse_args(argv)
    if not arguments.json_output:
        parser.error("the following argument is required: --json")
    return arguments


def main(argv: Sequence[str] | None = None) -> int:
    """Run the command-line interface and return a process exit status."""

    try:
        arguments = _arguments(argv)
        text = sys.stdin.read() if arguments.deck == "-" else Path(arguments.deck).read_text()
        sys.stdout.write(run_netlist_json(text))
        return 0
    except (OSError, ValueError) as error:
        sys.stderr.write(f"{CLI_ERROR_CODE}: {error}\n")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
