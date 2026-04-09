"""Parrot — the world's simplest REPL.

Whatever you type, I repeat back. Type :quit to exit.

What This Module Does
---------------------
This module is the entry point for the ``parrot`` program.  It wires together:

- :class:`~coding_adventures_repl.EchoLanguage` — evaluates input by echoing
  it back unchanged; ``:quit`` ends the session.
- :class:`~parrot.prompt.ParrotPrompt` — provides the parrot-themed banner
  and ``🦜 > `` line prompt.
- :class:`~coding_adventures_repl.SilentWaiting` — shows nothing while
  "evaluating" (EchoLanguage is instant).
- ``stdin`` / ``stdout`` — real terminal I/O for interactive use.

The Data Flow
-------------
::

    stdin
      │
      ▼
    readline()  ──► EchoLanguage.eval() ──► output_fn ──► stdout
                         │
                    :quit? ──► stop loop

stdin Reading Contract
----------------------
``sys.stdin.readline()`` has three distinct return values:

- ``""``   — end of file (Ctrl-D, piped file exhausted, closed pipe)
- ``"\\n"`` — empty line (user pressed Enter with no input)
- ``"text\\n"`` — normal input

We distinguish EOF from empty line with the ``_read_line`` helper:

.. code-block:: python

    def _read_line() -> str | None:
        line = sys.stdin.readline()
        if line == "":        # EOF sentinel
            return None       # signals the loop to stop
        return line.rstrip("\\n")   # strip trailing newline

Returning ``None`` signals end-of-input to :func:`run_with_io`, which then
exits the loop cleanly — exactly as if the user had typed ``:quit``.

Returning ``""`` (empty string after stripping) passes the empty string to
:class:`~coding_adventures_repl.EchoLanguage`, which echoes it back as ``""``.
This is the correct behaviour: pressing Enter on an empty line echoes nothing
visible but confirms the REPL is still alive.
"""

from __future__ import annotations

import sys

from coding_adventures_repl import EchoLanguage, SilentWaiting, run_with_io

from parrot.prompt import ParrotPrompt


def _read_line() -> str | None:
    """Read one line from stdin, returning None on EOF.

    This helper exists as a named function (rather than a lambda) for two
    reasons:

    1. **Clarity** — the EOF logic is non-trivial; a name explains intent.
    2. **Testability** — tests can inspect what ``_read_line`` does without
       mocking ``sys.stdin``.

    The distinction between EOF and empty line
    ------------------------------------------
    ``sys.stdin.readline()`` returns:

    - ``""`` on end-of-file (pipe closed, Ctrl-D in terminal)
    - ``"\\n"`` when the user presses Enter on an empty line
    - ``"text\\n"`` for normal input

    We return ``None`` for EOF and the stripped string for everything else.

    Returns
    -------
    str | None
        The input line with trailing newline stripped, or ``None`` on EOF.
    """
    # readline() is the correct choice here over input() because:
    # - input() raises EOFError on EOF (harder to handle uniformly)
    # - readline() returns "" on EOF (easy sentinel value)
    # - readline() preserves the empty-line vs. EOF distinction
    line = sys.stdin.readline()

    # The empty string is the exclusive EOF sentinel from readline().
    # Any real line (even an empty one the user typed) comes as "\n".
    if line == "":
        return None

    # Strip the trailing newline that readline() always includes.
    # After stripping, an empty line becomes "", which EchoLanguage echoes
    # back as "" — correct behaviour.
    return line.rstrip("\n")


def main() -> None:
    """Entry point for the ``parrot`` executable.

    This function is referenced in ``pyproject.toml`` under
    ``[project.scripts]``::

        parrot = "parrot.main:main"

    So running ``parrot`` (after ``pip install``) calls this function.

    The function delegates entirely to :func:`~coding_adventures_repl.run_with_io`,
    passing:

    - ``language=EchoLanguage()`` — the trivial echo evaluator
    - ``prompt=ParrotPrompt()`` — the 🦜 banner
    - ``waiting=SilentWaiting()`` — no animation
    - ``input_fn=_read_line`` — reads from stdin with EOF detection
    - ``output_fn=sys.stdout.write`` — writes to stdout without adding newlines
      (the framework and prompt strings handle their own newlines)
    - ``mode="async"`` — evaluation runs in a background thread (default)

    Why ``sys.stdout.write`` instead of ``print``?
    -----------------------------------------------
    ``print()`` always appends ``\\n``.  ``sys.stdout.write()`` writes exactly
    what it receives.  This is important because:

    - The prompt string ``"🦜 Parrot REPL\\nI repeat…\\n\\n"`` has its own newlines.
    - Echo results (e.g. ``"hello"``) are written as-is; the loop does not add
      a newline.  If ``print`` were used, results would have a double newline.

    If you want line-by-line output (e.g. for a script), replace ``output_fn``
    with ``lambda text: print(text)`` or ``lambda text: print(text, end="")``.
    """
    run_with_io(
        language=EchoLanguage(),
        prompt=ParrotPrompt(),
        waiting=SilentWaiting(),
        # _read_line handles the readline() → None-on-EOF conversion.
        input_fn=_read_line,
        # sys.stdout.write writes the string exactly as given.
        # The framework calls output_fn with prompt strings (which contain
        # their own newlines) and with echo results (plain strings).
        output_fn=sys.stdout.write,
        mode="async",
    )


if __name__ == "__main__":
    # Allow running directly: `python -m parrot.main` or `python src/parrot/main.py`
    main()
