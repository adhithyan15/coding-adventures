"""Language — the heart of the REPL's pluggability.

What Is a Language?
-------------------
In a REPL, a *language* is anything that can take a string of text and
produce a result.  That result is one of three things:

1. ``("ok", str)``   — evaluation succeeded and there is a value to show.
2. ``("ok", None)``  — evaluation succeeded but there is nothing to display
   (e.g. an assignment that produces no value, like ``x = 5`` in Python).
3. ``("error", str)``— evaluation failed with a message to display.
4. ``"quit"``        — the user wants to exit the session.

Why an Abstract Base Class?
---------------------------
Using Python's ``abc.ABC`` lets the REPL loop remain completely agnostic
about the language it is evaluating.  You can plug in:

- A trivial echo language (for testing and demos)
- A mathematical expression evaluator
- A full Python interpreter via ``exec``
- A Brainfuck engine
- Anything else that can map ``str → result``

The loop only knows about this contract.  Nothing more.

Implementing a Language
-----------------------
Subclass :class:`Language` and implement :meth:`eval`::

    class MathLanguage(Language):
        def eval(self, input: str) -> tuple[str, str | None] | str:
            if input.strip() == ":quit":
                return "quit"
            try:
                result = str(eval(input))          # noqa: S307 – demo only
                return ("ok", result)
            except Exception as exc:
                return ("error", str(exc))

The loop calls ``eval`` in a background thread so slow evaluators do not
freeze the main thread.
"""

from __future__ import annotations

from abc import ABC, abstractmethod


class Language(ABC):
    """Abstract base class for REPL language evaluators.

    A *language* maps a single line of user input to one of four outcomes:

    +---------------------+----------------------------------------------+
    | Return value        | Meaning                                      |
    +=====================+==============================================+
    | ``("ok", str)``     | Success with a displayable result string.    |
    +---------------------+----------------------------------------------+
    | ``("ok", None)``    | Success, but nothing to display.             |
    +---------------------+----------------------------------------------+
    | ``("error", str)``  | Failure with a human-readable message.       |
    +---------------------+----------------------------------------------+
    | ``"quit"``          | The user requested an end to the session.    |
    +---------------------+----------------------------------------------+

    The loop will never call :meth:`eval` with ``None``; it handles the
    end-of-input sentinel itself before dispatch.
    """

    @abstractmethod
    def eval(self, input: str) -> tuple[str, str | None] | str:
        """Evaluate a single line of input in this language.

        Parameters
        ----------
        input:
            The raw text the user typed.  The trailing newline has already
            been stripped by the loop before this method is called.

        Returns
        -------
        tuple[str, str | None] | str
            One of:

            - ``("ok", value)``   — success; *value* is a printable string
              or ``None`` if there is nothing to display.
            - ``("error", msg)``  — failure; *msg* describes what went wrong.
            - ``"quit"``          — end the session.

        Notes
        -----
        This method is invoked in a **background thread** by the loop so that
        slow evaluators do not block the main thread from running waiting-
        animation ticks.  Implementations must therefore be thread-safe with
        respect to any mutable state they hold.
        """
