"""ParrotPrompt — the parrot-themed prompt for the Parrot REPL.

What Is a Prompt?
-----------------
A prompt is the short string printed before the user's cursor to signal
"I am ready; type something."  The :class:`~coding_adventures_repl.Prompt`
abstract base class requires two methods:

- :meth:`global_prompt` — shown at the start of each fresh REPL cycle
  (before every input read).
- :meth:`line_prompt` — shown on continuation lines when an expression
  spans multiple lines. Parrot doesn't use multi-line input, but the
  framework requires this method.

Why a Separate Module?
----------------------
Separating the prompt from the main entry point follows the Single
Responsibility Principle:

- :class:`ParrotPrompt` knows what to *say* to the user.
- :mod:`parrot.main` knows how to *wire* everything together.

This separation also makes the prompt independently testable — we can
verify that the text contains "Parrot" and the parrot emoji without
running the full REPL loop.

Parrot Theme
------------
The 🦜 emoji reinforces the parrot metaphor throughout. A parrot repeats
what it hears — this REPL repeats what you type.

The :meth:`global_prompt` is called once per REPL cycle, so it acts as
both a per-line prompt *and* a banner. The content tells the user what
the program does and how to exit — all the information they need in one
glance.
"""

from __future__ import annotations

from coding_adventures_repl import Prompt


class ParrotPrompt(Prompt):
    """Parrot-themed prompt with emoji and friendly messages.

    This class implements the :class:`~coding_adventures_repl.Prompt`
    interface for the Parrot REPL.  Both methods return plain strings —
    no ANSI codes, no stateful tracking, no external dependencies.

    The :class:`~coding_adventures_repl.Prompt` ABC uses Python's
    :mod:`abc` machinery to enforce that both abstract methods are
    overridden.  If either were missing, instantiating this class would
    raise a :class:`TypeError`.
    """

    def global_prompt(self) -> str:
        """Return the banner/prompt shown before each input read.

        The framework's ``run_with_io`` loop calls this method once per
        cycle, writes the result via ``output_fn``, then reads the next
        line of input::

            output_fn(prompt.global_prompt())   # ← called here
            line = input_fn()
            ...

        We return a multi-line string so the banner and the blank line
        separator are part of the prompt itself.  The double newline at
        the end creates a blank line between the banner text and wherever
        the user's cursor ends up (the framework writes the string as-is).

        Returns
        -------
        str
            The multi-line banner string, ending with two newlines.
        """
        return "🦜 Parrot REPL\nI repeat everything you say! Type :quit to exit.\n\n"

    def line_prompt(self) -> str:
        """Return the continuation prompt for multi-line input.

        EchoLanguage is always single-line, so this prompt is never shown
        in practice when using Parrot.  It is implemented because:

        1. The :class:`~coding_adventures_repl.Prompt` ABC requires it.
        2. Future language plugins wired with :class:`ParrotPrompt` will
           get a sensible continuation prompt automatically.

        The parrot emoji keeps the brand consistent with :meth:`global_prompt`.

        Returns
        -------
        str
            A short prompt string with the parrot emoji.
        """
        return "🦜 > "
