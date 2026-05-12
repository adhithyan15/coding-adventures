"""Entry point for ``python main.py`` and ``python -m main``.

Wires :class:`MacsymaLanguage` and :class:`MacsymaPrompt` to the
generic REPL framework's :func:`run` and runs it.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from coding_adventures_repl import Repl

from language import MacsymaLanguage
from prompt import MacsymaPrompt

_BANNER = "MACSYMA-on-symbolic-VM 0.1\n(C) 2026 — derived from MACSYMA at MIT\n"


def main(argv: list[str] | None = None) -> int:
    """Run an interactive MACSYMA session or a non-interactive file."""
    parser = argparse.ArgumentParser(description="Run the MACSYMA REPL.")
    parser.add_argument(
        "-f",
        "--file",
        type=Path,
        help="Evaluate a .mac source file non-interactively.",
    )
    args = parser.parse_args(argv)

    language = MacsymaLanguage()
    if args.file is not None:
        result = language.eval_file(args.file)
        if result == "quit":
            return 0
        status, output = result
        if output:
            stream = sys.stderr if status == "error" else sys.stdout
            print(output, file=stream)
        return 0 if status == "ok" else 1

    prompt = MacsymaPrompt(history=language.history)
    print(_BANNER)
    Repl.run(language=language, prompt=prompt)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
