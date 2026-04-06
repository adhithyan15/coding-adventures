"""coding-adventures-repl — a pluggable, async-eval REPL framework.

What Is a REPL?
---------------
REPL stands for **Read–Eval–Print Loop**.  It is the interactive shell model
used by Python (``python -i``), Ruby (``irb``), Node.js (``node``), Elixir
(``iex``), and countless others:

1. **Read** — print a prompt, read a line of input from the user.
2. **Eval** — pass that line to the language evaluator.
3. **Print** — display the result (or error message).
4. **Loop** — go back to step 1.

This package provides the *loop* infrastructure and leaves the language,
prompt, and waiting-animation behaviour entirely pluggable.

Three Pluggable Interfaces
--------------------------
:class:`Language`
    Maps a string of user input to a result.  The loop calls this in a
    background thread so slow evaluators don't freeze the prompt.

:class:`Prompt`
    Supplies the prompt strings shown before each input line (``"> "``
    and ``"... "`` by convention).

:class:`Waiting`
    Drives animation (or silence) while the evaluator is running.  The tick
    model (``start`` / ``tick`` / ``stop``) is inspired by animation loops in
    game engines — simple, composable, and testable.

Built-In Implementations
------------------------
Three concrete implementations are provided out of the box:

:class:`EchoLanguage`
    Mirrors the user's input back unchanged.  ``":quit"`` ends the session.
    Ideal for testing and demos.

:class:`DefaultPrompt`
    Returns ``"> "`` and ``"... "`` — the conventional shell prompts used by
    Python, Ruby, and many others.

:class:`SilentWaiting`
    All lifecycle methods are no-ops.  The tick interval is 100 ms.  Use when
    evaluation is fast and visual feedback would be distracting.

Entry Points
------------
Two entry points are provided on the :class:`Repl` namespace object:

:func:`Repl.run`
    Interactive use — wires ``input()`` and ``print()``.

:func:`Repl.run_with_io`
    Testing and embedding — accepts explicit ``input_fn`` and ``output_fn``
    callables.

Quick Start
-----------
Interactive echo REPL::

    from coding_adventures_repl import Repl
    Repl.run()

Custom language::

    from coding_adventures_repl import Language, Repl

    class DoubleLanguage(Language):
        def eval(self, input: str) -> tuple[str, str | None] | str:
            if input == ":quit":
                return "quit"
            try:
                return ("ok", str(int(input) * 2))
            except ValueError:
                return ("error", f"not an integer: {input!r}")

    Repl.run(language=DoubleLanguage())

Programmatic / testing use::

    from coding_adventures_repl import Repl

    inputs = iter(["hello", "world", ":quit"])
    collected: list[str] = []

    Repl.run_with_io(
        input_fn=lambda: next(inputs, None),
        output_fn=collected.append,
    )
    # collected == ["> ", "hello", "> ", "world", "> "]
"""

from __future__ import annotations

from coding_adventures_repl.default_prompt import DefaultPrompt
from coding_adventures_repl.echo_language import EchoLanguage
from coding_adventures_repl.language import Language
from coding_adventures_repl.loop import run, run_with_io
from coding_adventures_repl.prompt import Prompt
from coding_adventures_repl.silent_waiting import SilentWaiting
from coding_adventures_repl.waiting import Waiting


class Repl:
    """Namespace for the two REPL entry points.

    This class is never instantiated.  It exists purely as a namespace so
    callers can write ``Repl.run(...)`` and ``Repl.run_with_io(...)`` without
    importing the individual functions directly.

    All parameters are forwarded unchanged to :func:`run` and
    :func:`run_with_io` respectively — see those functions for full
    documentation.
    """

    # Make the module-level functions available as class attributes so
    # ``Repl.run(...)`` works without instantiation.
    run = staticmethod(run)
    run_with_io = staticmethod(run_with_io)


__all__ = [
    # Namespace / entry points
    "Repl",
    # Abstract base classes
    "Language",
    "Prompt",
    "Waiting",
    # Built-in implementations
    "EchoLanguage",
    "DefaultPrompt",
    "SilentWaiting",
]
