"""Entry point for ``python main.py`` and ``python -m main``.

Wires :class:`MacsymaLanguage` and :class:`MacsymaPrompt` to the
generic REPL framework's :func:`run` and runs it.
"""

from __future__ import annotations

from coding_adventures_repl import Repl

from language import MacsymaLanguage
from prompt import MacsymaPrompt

_BANNER = "MACSYMA-on-symbolic-VM 0.1\n(C) 2026 — derived from MACSYMA at MIT\n"


def main() -> None:
    """Start an interactive MACSYMA session on the current terminal."""
    language = MacsymaLanguage()
    prompt = MacsymaPrompt(history=language.history)
    print(_BANNER)
    Repl.run(language=language, prompt=prompt)


if __name__ == "__main__":
    main()
