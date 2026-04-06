"""Prompt — controls what the REPL prints before reading each line.

What Is a Prompt?
-----------------
The prompt is the short string printed to the terminal just before the cursor
to signal "I am ready for input."  In Python's own interactive shell:

    >>> print("hello")          ← ``>>> `` is the global prompt
    hello
    >>> if True:
    ...     pass                ← ``... `` is the line (continuation) prompt

Two prompts are enough for most languages:

1. **Global prompt** — shown at the start of every fresh statement.
2. **Line prompt** — shown when the user started a multi-line construct
   (open parenthesis, open block, etc.) and the language needs more input
   before it can evaluate.

Why Pluggable?
--------------
- **Branding** — ``myapp> `` instead of ``> ``
- **Colour** — wrap in ANSI escape codes for green or bold text
- **Stateful** — show the current namespace or scope in the prompt
- **Contextual** — different prompt inside a function definition vs. top-level

Implementing a Prompt
---------------------
Subclass :class:`Prompt` and implement both abstract methods::

    class GreenPrompt(Prompt):
        def global_prompt(self) -> str:
            return "\\033[32m>\\033[0m "          # green ">" then reset

        def line_prompt(self) -> str:
            return "\\033[32m...\\033[0m "
"""

from __future__ import annotations

from abc import ABC, abstractmethod


class Prompt(ABC):
    """Abstract base class for REPL prompt providers.

    The loop calls :meth:`global_prompt` once at the start of each new
    statement and :meth:`line_prompt` for each continuation line.  Both
    methods must return a plain string (the loop writes it to ``output_fn``
    directly without adding any extra characters).
    """

    @abstractmethod
    def global_prompt(self) -> str:
        """Return the primary prompt string, e.g. ``"> "``.

        Shown at the very beginning of each new REPL cycle — before the
        first line of any statement.

        The trailing space is part of the returned string by convention so
        that the cursor is not crammed against the prompt character.

        Returns
        -------
        str
            A short, printable string.  Must not be ``None``.
        """

    @abstractmethod
    def line_prompt(self) -> str:
        """Return the continuation prompt string, e.g. ``"... "``.

        Shown when the language signals that the current input is incomplete
        and further lines are expected.

        The four-character ``"... "`` is conventional in Python, Ruby (irb),
        and many other REPLs because it aligns visually below the two-
        character ``"> "`` while adding one extra distinguishing character.

        Returns
        -------
        str
            A short, printable string.  Must not be ``None``.
        """
