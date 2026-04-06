"""REPL loop — the engine that ties Language, Prompt, and Waiting together.

Architecture
------------
The REPL loop has one job: read a line, evaluate it, print the result,
repeat.  In pseudocode::

    while True:
        print(prompt.global_prompt())
        line = input_fn()
        if line is None:           # end of input (piped file exhausted)
            break
        result = language.eval(line)
        if result == "quit":
            break
        status, value = result
        if value is not None:
            print(value)

The real implementation is more elaborate in two ways:

1. **Async evaluation** — ``language.eval`` runs in a
   :class:`threading.Thread` so that slow evaluators (network calls,
   compilation) do not freeze the main thread.  The main thread polls the
   thread with ``thread.join(timeout)`` in a loop, calling
   ``waiting.tick(state)`` between polls to drive animations.

2. **I/O injection** — Instead of calling the built-in ``input()`` and
   ``print()`` directly, the loop accepts ``input_fn`` and ``output_fn``
   callables.  This makes the loop trivially testable without patching
   builtins and lets embedders redirect I/O to sockets, files, GUIs, etc.

Thread Safety and Exception Handling
-------------------------------------
The background thread wraps ``language.eval`` in a ``try/except Exception``
block.  If the evaluator raises an uncaught exception the thread catches it,
stores it, and the main loop converts it into an ``("error", message)``
result.  This means a buggy language plugin cannot crash the REPL host.

The result is stored in a mutable list (a Python closure trick) because
you cannot assign to a free variable across a thread boundary without
``nonlocal`` or a mutable container.  A list of length 1 is the simplest
possible mutable container for this purpose::

    result_box: list[tuple[str, str | None] | str | None] = [None]

    def worker() -> None:
        try:
            result_box[0] = language.eval(line)
        except Exception as exc:
            result_box[0] = ("error", f"Unhandled exception: {exc}")

I/O Contract
------------
``input_fn: Callable[[], str | None]``
    Called to read the next line of input.  Must return the line as a
    ``str`` (trailing newline already stripped is conventional but not
    required — the loop strips it).  Return ``None`` to signal end-of-input
    (equivalent to ``Ctrl-D`` or a pipe being closed).

``output_fn: Callable[[str], None]``
    Called to write a string to the output.  The loop appends ``"\\n"`` to
    every piece of output it emits so that callers don't have to.

Public API
----------
Two entry points are provided:

- :func:`run_with_io` — takes explicit ``input_fn`` / ``output_fn``
  callables; primarily for testing and embedding.
- :func:`run` — wraps the built-in ``input()`` / ``print()``; the
  interactive-use entry point.
"""

from __future__ import annotations

import threading
from collections.abc import Callable
from typing import Any

from coding_adventures_repl.default_prompt import DefaultPrompt
from coding_adventures_repl.echo_language import EchoLanguage
from coding_adventures_repl.language import Language
from coding_adventures_repl.prompt import Prompt
from coding_adventures_repl.silent_waiting import SilentWaiting
from coding_adventures_repl.waiting import Waiting

# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def _eval_in_thread(
    language: Language,
    line: str,
) -> tuple[str, str | None] | str:
    """Run ``language.eval(line)`` in a background thread and return the result.

    If the evaluator raises an exception the exception is caught here and
    converted to an ``("error", message)`` tuple so the loop always receives
    a well-formed result.

    Parameters
    ----------
    language:
        The :class:`Language` implementation to evaluate with.
    line:
        The stripped input line.

    Returns
    -------
    tuple[str, str | None] | str
        The result from :meth:`Language.eval`, or an error tuple if the
        evaluator raised an exception.

    Notes
    -----
    We use a one-element list as a mutable container to pass the result out
    of the thread closure.  Python closures can *read* free variables from
    the enclosing scope but cannot *rebind* them without ``nonlocal``.  A
    list is simpler and more portable than a ``threading.Event`` + separate
    variable pair for this trivial use case.
    """
    # Mutable box to hold the result produced by the background thread.
    # Initialised to None; the thread overwrites index 0 before it exits.
    result_box: list[tuple[str, str | None] | str | None] = [None]

    def worker() -> None:
        """Thread target: evaluate and store result, catching all exceptions."""
        try:
            result_box[0] = language.eval(line)
        except Exception as exc:  # noqa: BLE001 — deliberate broad catch
            # Convert any unhandled exception into an error result so the
            # loop always gets a well-typed value and the REPL keeps running
            # rather than crashing.
            result_box[0] = ("error", f"Unhandled exception: {exc}")

    thread = threading.Thread(target=worker, daemon=True)
    return thread, result_box


def _wait_for_thread(
    thread: threading.Thread,
    result_box: list[Any],
    waiting: Waiting,
) -> tuple[str, str | None] | str:
    """Drive the waiting plugin while polling the eval thread for completion.

    The main loop:

    1. Calls ``waiting.start()`` to get the initial animation state.
    2. Joins the thread with a short timeout (``tick_ms / 1000`` seconds).
    3. If the thread is still running, calls ``waiting.tick(state)`` and
       repeats from step 2.
    4. When the thread exits, calls ``waiting.stop(state)`` and returns
       the result from ``result_box[0]``.

    This design lets a spinner animation run smoothly at its own cadence
    while the evaluator is slow, and terminates immediately when it's fast.

    Parameters
    ----------
    thread:
        The already-started background thread running the evaluator.
    result_box:
        One-element list whose index 0 holds the result after the thread
        exits.
    waiting:
        The :class:`Waiting` plugin to drive.

    Returns
    -------
    tuple[str, str | None] | str
        The value stored in ``result_box[0]`` by the worker thread.
    """
    # Start the waiting animation and record the initial state.
    state = waiting.start()

    # Convert the tick interval from milliseconds to fractional seconds
    # for threading.Thread.join().
    timeout_secs = waiting.tick_ms() / 1000.0

    while thread.is_alive():
        # Poll the thread.  join() with a timeout returns whether or not
        # the thread finished — it doesn't raise.  We check is_alive()
        # after to decide whether to tick or exit the loop.
        thread.join(timeout=timeout_secs)
        if thread.is_alive():
            # Thread is still running — advance the animation.
            state = waiting.tick(state)

    # Thread has finished; clean up the waiting animation.
    waiting.stop(state)

    # result_box[0] is guaranteed to be set because the thread has exited
    # and the worker always writes to result_box[0] (even on exception).
    return result_box[0]  # type: ignore[return-value]


# ---------------------------------------------------------------------------
# Public entry points
# ---------------------------------------------------------------------------


def run_with_io(
    language: Language | None = None,
    prompt: Prompt | None = None,
    waiting: Waiting | None = None,
    *,
    input_fn: Callable[[], str | None],
    output_fn: Callable[[str], None],
) -> None:
    """Run the REPL loop with injected I/O.

    This is the primary low-level entry point.  It accepts explicit
    ``input_fn`` and ``output_fn`` callables, making the loop fully testable
    without patching built-ins and embeddable in any context (GUI, socket
    server, Jupyter, etc.).

    Parameters
    ----------
    language:
        The :class:`Language` implementation to evaluate with.
        Defaults to :class:`~coding_adventures_repl.EchoLanguage`.
    prompt:
        The :class:`Prompt` implementation for prompt strings.
        Defaults to :class:`~coding_adventures_repl.DefaultPrompt`.
    waiting:
        The :class:`Waiting` implementation for animation while eval runs.
        Defaults to :class:`~coding_adventures_repl.SilentWaiting`.
    input_fn:
        Callable that returns the next line of user input as a ``str``,
        or ``None`` to signal end-of-input.  Keyword-only.
    output_fn:
        Callable that writes a string to the output.  The loop appends a
        newline to every string it writes.  Keyword-only.

    Notes
    -----
    The loop writes the prompt string followed by a newline because
    ``output_fn`` is assumed to be a line-oriented function.  If you need
    the prompt on the *same line* as the cursor (interactive terminal use),
    use a custom ``output_fn`` that calls ``sys.stdout.write`` and
    ``sys.stdout.flush`` without adding a newline, or use :func:`run`
    which uses ``input()`` for that effect automatically.

    The loop terminates when:

    - ``language.eval`` returns ``"quit"``.
    - ``input_fn`` returns ``None`` (end of input).
    """
    # Apply defaults — using sentinel None + default inside the function
    # rather than mutable default arguments avoids the classic Python trap.
    if language is None:
        language = EchoLanguage()
    if prompt is None:
        prompt = DefaultPrompt()
    if waiting is None:
        waiting = SilentWaiting()

    while True:
        # -------------------------------------------------------------------
        # Step 1 — emit the global prompt and read a line of input.
        # -------------------------------------------------------------------
        output_fn(prompt.global_prompt())

        line = input_fn()

        # None signals end-of-input (e.g. piped file exhausted, or the test
        # has no more inputs to feed).  Treat it as a quit command.
        if line is None:
            break

        # Strip the trailing newline if present (interactive terminals don't
        # add one but piped inputs do).
        line = line.rstrip("\n")

        # -------------------------------------------------------------------
        # Step 2 — evaluate in a background thread, driving waiting ticks.
        # -------------------------------------------------------------------
        thread, result_box = _eval_in_thread(language, line)
        thread.start()
        result = _wait_for_thread(thread, result_box, waiting)

        # -------------------------------------------------------------------
        # Step 3 — handle the result.
        # -------------------------------------------------------------------
        if result == "quit":
            # Language signalled end-of-session.
            break

        # result is now a tuple[str, str | None]
        status, value = result  # type: ignore[misc]

        if status == "error":
            # Evaluation failed — show the error message.
            output_fn(f"Error: {value}")
        elif status == "ok" and value is not None:
            # Evaluation succeeded with a displayable result.
            output_fn(value)
        # else: status == "ok" and value is None — nothing to display
        # (e.g. a statement with no return value like an assignment).


def run(
    language: Language | None = None,
    prompt: Prompt | None = None,
    waiting: Waiting | None = None,
) -> None:
    """Run the REPL loop interactively using ``input()`` and ``print()``.

    This is the high-level, interactive-use entry point.  It wires the
    built-in ``input()`` (which prints the prompt on the *same line* as the
    cursor, handles readline history, etc.) and ``print()`` to the REPL loop.

    Parameters
    ----------
    language:
        The :class:`Language` implementation to evaluate with.
        Defaults to :class:`~coding_adventures_repl.EchoLanguage`.
    prompt:
        The :class:`Prompt` implementation.
        Defaults to :class:`~coding_adventures_repl.DefaultPrompt`.
    waiting:
        The :class:`Waiting` animation plugin.
        Defaults to :class:`~coding_adventures_repl.SilentWaiting`.

    Notes
    -----
    Unlike :func:`run_with_io`, the prompt is passed to Python's built-in
    ``input()`` so it appears *inline* with the cursor rather than on a
    separate line.  This is the standard interactive terminal experience.

    ``KeyboardInterrupt`` (Ctrl-C) is not caught here — it propagates to
    the caller.  If you want Ctrl-C to restart the current input rather
    than quit, wrap this function in a ``try/except KeyboardInterrupt`` loop.

    Example
    -------
    .. code-block:: python

        from coding_adventures_repl import Repl
        Repl.run()                        # interactive echo REPL

        from mypackage import MyLanguage
        Repl.run(language=MyLanguage())   # custom language REPL
    """
    # Apply defaults.
    if language is None:
        language = EchoLanguage()
    if prompt is None:
        prompt = DefaultPrompt()
    if waiting is None:
        waiting = SilentWaiting()

    # For interactive use we wire input() differently: we pass the prompt
    # string directly to input() so readline can display it properly (same
    # line as cursor, readline history, etc.).  The output_fn receives only
    # actual output, not prompts.
    #
    # We capture `prompt` in a closure — the variable is read on every call
    # to input_fn, which is correct because the prompt object is immutable
    # (its methods are called fresh each time, allowing stateful prompts).

    captured_prompt = prompt  # rename to avoid shadowing the parameter

    def input_fn() -> str | None:
        try:
            return input(captured_prompt.global_prompt())
        except EOFError:
            return None

    def output_fn(text: str) -> None:
        print(text)

    # Re-implement the loop directly here (rather than delegating to
    # run_with_io) so that the prompt is passed to input() rather than
    # printed separately.  This gives the correct interactive behaviour
    # where the prompt and the user's typing appear on the same line.

    while True:
        line = input_fn()

        if line is None:
            break

        line = line.rstrip("\n")

        thread, result_box = _eval_in_thread(language, line)
        thread.start()
        result = _wait_for_thread(thread, result_box, waiting)

        if result == "quit":
            break

        status, value = result  # type: ignore[misc]

        if status == "error":
            output_fn(f"Error: {value}")
        elif status == "ok" and value is not None:
            output_fn(value)
