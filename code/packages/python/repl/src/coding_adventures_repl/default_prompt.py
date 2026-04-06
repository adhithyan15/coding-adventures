"""DefaultPrompt — the conventional two-character shell prompt.

The Conventions
---------------
Two-character prompts (``"> "`` and ``"... "``) are deeply ingrained in
interactive programming culture:

- ``"> "`` is used by Node.js, Ruby's irb, Elixir's iex (sort of), and
  countless Unix shells in their simplest form.
- ``"... "`` is used by Python, Ruby's irb, and others to signal that the
  previous line opened a block that hasn't been closed yet.

These prompts are short enough to not crowd the code the user types, and
distinctive enough that the eye immediately finds the input boundary.

When to Replace This
--------------------
- **Branded REPLs** — ``"myapp> "`` instead of ``"> "``
- **Coloured prompts** — wrap in ANSI escape codes for green or bold text
- **Stateful prompts** — show the current namespace, module, or scope
- **Contextual prompts** — different prompt inside a function definition
  vs. at the top level

Swap this class for any other :class:`~coding_adventures_repl.Prompt`
implementation and the loop will use it transparently.
"""

from __future__ import annotations

from coding_adventures_repl.prompt import Prompt


class DefaultPrompt(Prompt):
    """The standard ``"> "`` / ``"... "`` prompt pair.

    Suitable for any REPL that does not need a custom prompt.

    Examples
    --------
    >>> p = DefaultPrompt()
    >>> p.global_prompt()
    '> '
    >>> p.line_prompt()
    '... '
    """

    def global_prompt(self) -> str:
        """Return ``"> "`` — the canonical "ready for input" signal.

        The trailing space keeps the cursor visually separate from whatever
        the user types next.

        Returns
        -------
        str
            ``"> "``
        """
        # "> " is the canonical "ready for input" signal.
        # The trailing space keeps the cursor visually separate from the text
        # that the user types.
        return "> "

    def line_prompt(self) -> str:
        """Return ``"... "`` — the continuation prompt.

        Signals that the previous line opened a construct (open parenthesis,
        open block, etc.) and the language is waiting for more input.

        The four characters ``"... "`` align neatly below the two-character
        ``"> "`` with one extra distinguishing dot, which is the Python and
        Ruby irb convention.

        Returns
        -------
        str
            ``"... "``
        """
        # "... " signals continuation — the expression is incomplete and the
        # user should keep typing.  The four characters align visually below
        # "> " (two chars + one extra dot + space).
        return "... "
