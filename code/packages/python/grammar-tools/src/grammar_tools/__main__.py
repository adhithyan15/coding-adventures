"""Allow ``python -m grammar_tools`` to run the CLI."""

from __future__ import annotations

from grammar_tools.cli import main

if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
