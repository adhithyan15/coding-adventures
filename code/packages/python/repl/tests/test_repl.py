"""Tests for the coding-adventures-repl package.

Test Strategy
-------------
All tests drive the REPL through :func:`Repl.run_with_io` with injected
I/O.  This avoids patching built-ins and makes every test deterministic and
fast.

The six test cases mirror the canonical REPL test suite used across all
language ports of this package:

1. **Echo** — a single input is echoed back.
2. **Quit** — the ``:quit`` sentinel ends the session cleanly.
3. **Multiple turns** — several inputs are processed in order.
4. **Nil output** — a language that returns ``("ok", None)`` produces no
   output (e.g. an assignment statement that has no value to display).
5. **Error** — a language that returns ``("error", ...)`` prints the error.
6. **Exception safety** — a language that raises an unhandled exception
   does *not* crash the REPL; the exception is converted to an error result.

I/O Injection Pattern
---------------------
Each test creates an iterator over a list of input strings and a list to
collect output strings, then passes them as ``input_fn`` / ``output_fn``::

    inputs = iter(["hello", ":quit"])
    outputs: list[str] = []
    Repl.run_with_io(
        input_fn=lambda: next(inputs, None),
        output_fn=outputs.append,
    )

``next(inputs, None)`` returns ``None`` once the iterator is exhausted,
which the loop treats as end-of-input (equivalent to Ctrl-D).

Note on prompts in output
-------------------------
The loop calls ``output_fn(prompt.global_prompt())`` before each input.
With :class:`DefaultPrompt` that means ``"> "`` appears in the ``outputs``
list before every result.  Tests that check for specific output values
filter by index or use :meth:`list.__contains__` to ignore prompts.
"""

from __future__ import annotations

import pytest

from coding_adventures_repl import (
    DefaultPrompt,
    EchoLanguage,
    Language,
    Repl,
    SilentWaiting,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _run(inputs: list[str | None]) -> list[str]:
    """Run the REPL with the given input sequence and return all outputs.

    Parameters
    ----------
    inputs:
        A list of strings to feed as successive inputs.  ``None`` anywhere
        in the list signals end-of-input and stops the loop.

    Returns
    -------
    list[str]
        Everything written via ``output_fn``, in order.  Includes prompt
        strings (``"> "``).
    """
    it = iter(inputs)
    outputs: list[str] = []
    Repl.run_with_io(
        input_fn=lambda: next(it, None),
        output_fn=outputs.append,
    )
    return outputs


def _output_values(outputs: list[str]) -> list[str]:
    """Return only the non-prompt output lines from an outputs list.

    Filters out ``"> "`` and ``"... "`` prompt strings so tests can focus
    on the actual REPL results without dealing with prompt noise.

    Parameters
    ----------
    outputs:
        The raw list returned by :func:`_run`.

    Returns
    -------
    list[str]
        Lines that are not prompt strings.
    """
    prompts = {DefaultPrompt().global_prompt(), DefaultPrompt().line_prompt()}
    return [line for line in outputs if line not in prompts]


# ---------------------------------------------------------------------------
# Test case 1 — Echo: a single input is echoed back.
# ---------------------------------------------------------------------------


class TestEcho:
    """The simplest possible REPL interaction: one input, one output."""

    def test_input_is_echoed(self) -> None:
        """A single input line is echoed back to the output.

        This verifies the core happy-path of the loop: read → eval → print.
        The default EchoLanguage returns ("ok", input) for any non-quit input.
        """
        # Feed "hello" then let the iterator exhaust (returns None = quit).
        outputs = _run(["hello"])
        values = _output_values(outputs)

        # The echoed value "hello" must appear in the result values.
        assert "hello" in values

    def test_prompt_is_emitted_before_input(self) -> None:
        """The global prompt is written before reading each line.

        The loop must call output_fn(prompt.global_prompt()) before calling
        input_fn() each cycle.  With DefaultPrompt that means ``"> "`` must
        appear in the output list.
        """
        outputs = _run(["hello"])
        assert "> " in outputs


# ---------------------------------------------------------------------------
# Test case 2 — Quit: the :quit sentinel ends the session cleanly.
# ---------------------------------------------------------------------------


class TestQuit:
    """:quit ends the session without producing any output."""

    def test_quit_stops_loop(self) -> None:
        """After :quit, the loop terminates and no further inputs are read.

        This verifies that the loop respects the language's quit signal.
        The EchoLanguage returns "quit" (the string) when it receives ":quit".
        """
        # Feed ":quit" followed by more input that should never be reached.
        outputs = _run([":quit", "should-never-be-seen"])
        values = _output_values(outputs)

        assert "should-never-be-seen" not in values

    def test_quit_produces_no_output(self) -> None:
        """The :quit command produces no output value of its own.

        The loop breaks silently on "quit" — there is no "Goodbye" message
        from the framework itself (that belongs to the language or prompt
        plugin if desired).
        """
        outputs = _run([":quit"])
        values = _output_values(outputs)
        assert values == []


# ---------------------------------------------------------------------------
# Test case 3 — Multiple turns: several inputs processed in order.
# ---------------------------------------------------------------------------


class TestMultipleTurns:
    """The loop correctly processes a sequence of inputs."""

    def test_three_inputs_in_order(self) -> None:
        """Three distinct inputs produce three distinct echoed outputs in order.

        Verifies that the loop does not conflate results or skip lines.
        """
        outputs = _run(["alpha", "beta", "gamma"])
        values = _output_values(outputs)

        assert values == ["alpha", "beta", "gamma"]

    def test_prompt_emitted_for_each_turn(self) -> None:
        """A prompt is emitted before each input, including the first.

        With three inputs followed by exhaustion (None), we expect four
        ``"> "`` strings in the output: one before each of the three input
        reads, plus one more before the None read that terminates the loop.
        The loop always emits the prompt *then* reads — it cannot know in
        advance that the next read will return None.
        """
        outputs = _run(["a", "b", "c"])
        prompt_count = outputs.count("> ")
        # Three real inputs + one final read that gets None = four prompts.
        assert prompt_count == 4


# ---------------------------------------------------------------------------
# Test case 4 — Nil output: ("ok", None) produces no output line.
# ---------------------------------------------------------------------------


class TestNilOutput:
    """A language that returns ("ok", None) produces no displayed value.

    This is the "silent success" case — e.g. an assignment statement in
    Python (``x = 5``) evaluates successfully but has nothing to print.
    """

    def test_none_value_not_displayed(self) -> None:
        """When a language returns ("ok", None), nothing is written for that turn.

        The loop must skip the output_fn call when value is None, but must
        still emit the prompt for the next cycle.
        """

        class SilentLanguage(Language):
            """Always returns ("ok", None) — evaluation succeeded, no output."""

            def eval(self, input: str) -> tuple[str, str | None] | str:  # noqa: ANN001
                if input == ":quit":
                    return "quit"
                # Deliberately return None as the value — nothing to display.
                return ("ok", None)

        it = iter(["any-input", ":quit"])
        outputs: list[str] = []
        Repl.run_with_io(
            language=SilentLanguage(),
            input_fn=lambda: next(it, None),
            output_fn=outputs.append,
        )

        values = _output_values(outputs)
        # No non-prompt output should have been written.
        assert values == []

    def test_prompt_still_emitted_after_nil(self) -> None:
        """Even after a None-value result, the next prompt is still emitted."""

        class SilentLanguage(Language):
            def eval(self, input: str) -> tuple[str, str | None] | str:  # noqa: ANN001
                if input == ":quit":
                    return "quit"
                return ("ok", None)

        it = iter(["silent-input", ":quit"])
        outputs: list[str] = []
        Repl.run_with_io(
            language=SilentLanguage(),
            input_fn=lambda: next(it, None),
            output_fn=outputs.append,
        )

        # Two prompts: one before "silent-input", one before ":quit".
        assert outputs.count("> ") == 2


# ---------------------------------------------------------------------------
# Test case 5 — Error: ("error", message) displays the error.
# ---------------------------------------------------------------------------


class TestErrorResult:
    """A language that returns ("error", ...) causes an "Error: ..." line."""

    def test_error_message_displayed(self) -> None:
        """The error message from ("error", msg) is written to the output.

        The loop prefixes the message with ``"Error: "`` so it is visually
        distinct from normal output values.
        """

        class BrokenLanguage(Language):
            """Always returns an error result."""

            def eval(self, input: str) -> tuple[str, str | None] | str:  # noqa: ANN001
                if input == ":quit":
                    return "quit"
                return ("error", f"cannot evaluate: {input!r}")

        it = iter(["bad-input", ":quit"])
        outputs: list[str] = []
        Repl.run_with_io(
            language=BrokenLanguage(),
            input_fn=lambda: next(it, None),
            output_fn=outputs.append,
        )

        values = _output_values(outputs)
        assert len(values) == 1
        assert "Error:" in values[0]
        assert "bad-input" in values[0]

    def test_loop_continues_after_error(self) -> None:
        """An error result does not terminate the loop.

        The REPL should continue reading input after displaying an error,
        just as Python continues after a ``SyntaxError``.
        """

        class ErrorThenEcho(Language):
            """First call returns error, subsequent calls echo."""

            def __init__(self) -> None:
                self._call_count = 0

            def eval(self, input: str) -> tuple[str, str | None] | str:  # noqa: ANN001
                if input == ":quit":
                    return "quit"
                self._call_count += 1
                if self._call_count == 1:
                    return ("error", "deliberate first-call error")
                return ("ok", input)

        it = iter(["first", "second", ":quit"])
        outputs: list[str] = []
        Repl.run_with_io(
            language=ErrorThenEcho(),
            input_fn=lambda: next(it, None),
            output_fn=outputs.append,
        )

        values = _output_values(outputs)
        # First output is the error; second is the echo of "second".
        assert any("Error:" in v for v in values)
        assert "second" in values


# ---------------------------------------------------------------------------
# Test case 6 — Exception safety: unhandled exceptions become error results.
# ---------------------------------------------------------------------------


class TestExceptionSafety:
    """An unhandled exception in eval does not crash the REPL."""

    def test_exception_converted_to_error(self) -> None:
        """A RuntimeError raised inside eval becomes an ("error", ...) result.

        The loop wraps every eval call in a try/except inside the background
        thread.  This means language authors don't have to worry about
        accidentally crashing the REPL with an uncaught exception.
        """

        class ExplodingLanguage(Language):
            """Raises a RuntimeError on every eval call."""

            def eval(self, input: str) -> tuple[str, str | None] | str:  # noqa: ANN001
                if input == ":quit":
                    return "quit"
                raise RuntimeError("deliberate explosion")  # noqa: EM101

        it = iter(["boom", ":quit"])
        outputs: list[str] = []
        Repl.run_with_io(
            language=ExplodingLanguage(),
            input_fn=lambda: next(it, None),
            output_fn=outputs.append,
        )

        values = _output_values(outputs)
        # Must have produced exactly one error output.
        assert len(values) == 1
        assert "Error:" in values[0]

    def test_loop_continues_after_exception(self) -> None:
        """After an exception-as-error, the loop keeps reading input.

        This mirrors Python's behaviour: a traceback is printed but the
        session continues.
        """

        class ExplodeOnceThenEcho(Language):
            """Raises on the first call, echoes on subsequent calls."""

            def __init__(self) -> None:
                self._exploded = False

            def eval(self, input: str) -> tuple[str, str | None] | str:  # noqa: ANN001
                if input == ":quit":
                    return "quit"
                if not self._exploded:
                    self._exploded = True
                    raise ValueError("first-call explosion")  # noqa: EM101
                return ("ok", input)

        it = iter(["explode", "survive", ":quit"])
        outputs: list[str] = []
        Repl.run_with_io(
            language=ExplodeOnceThenEcho(),
            input_fn=lambda: next(it, None),
            output_fn=outputs.append,
        )

        values = _output_values(outputs)
        # Error from the first call, then "survive" echoed.
        assert any("Error:" in v for v in values)
        assert "survive" in values

    def test_none_input_treated_as_quit(self) -> None:
        """When input_fn returns None, the loop terminates cleanly.

        This simulates end-of-file (Ctrl-D in an interactive terminal, or a
        piped file being fully consumed).  The loop must not raise; it must
        simply stop.
        """
        # input_fn immediately returns None (no inputs at all).
        outputs: list[str] = []
        Repl.run_with_io(
            input_fn=lambda: None,
            output_fn=outputs.append,
        )
        # The loop emits one prompt before reading the None, then stops.
        assert "> " in outputs
        # No result values should have been written.
        assert _output_values(outputs) == []


# ---------------------------------------------------------------------------
# Test case 7 — run() function (interactive path, patched input/print)
# ---------------------------------------------------------------------------


class TestRunInteractive:
    """Test the run() entry point by monkeypatching builtins.

    ``run()`` is the high-level function that calls ``input()`` and
    ``print()``.  We patch these builtins to drive the loop programmatically
    so we can verify the interactive code path without opening a terminal.
    """

    def test_run_echoes_input(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """run() echoes input using the default EchoLanguage."""
        inputs = iter(["hello", ":quit"])
        printed: list[str] = []

        def mock_input(_prompt: str = "") -> str:
            return next(inputs, None) or ""

        monkeypatch.setattr("builtins.input", mock_input)
        monkeypatch.setattr("builtins.print", lambda s: printed.append(s))

        Repl.run()

        assert "hello" in printed

    def test_run_eof_stops_loop(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """run() stops cleanly when input() raises EOFError (Ctrl-D)."""
        printed: list[str] = []
        call_count = 0

        def mock_input(_prompt: str = "") -> str:
            nonlocal call_count
            call_count += 1
            raise EOFError

        monkeypatch.setattr("builtins.input", mock_input)
        monkeypatch.setattr("builtins.print", lambda s: printed.append(s))

        Repl.run()

        # Loop should have exited cleanly with no output.
        assert printed == []

    def test_run_error_result(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """run() displays error results from the language."""

        class ErrorLang(Language):
            def eval(self, input: str) -> tuple[str, str | None] | str:  # noqa: ANN001
                if input == ":quit":
                    return "quit"
                return ("error", "deliberate error")

        inputs = iter(["bad", ":quit"])
        printed: list[str] = []
        monkeypatch.setattr("builtins.input", lambda _prompt="": next(inputs, ""))
        monkeypatch.setattr("builtins.print", lambda s: printed.append(s))

        Repl.run(language=ErrorLang())

        assert any("Error:" in p for p in printed)

    def test_run_nil_output_silent(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """run() produces no output for ("ok", None) results."""

        class SilentLang(Language):
            def eval(self, input: str) -> tuple[str, str | None] | str:  # noqa: ANN001
                if input == ":quit":
                    return "quit"
                return ("ok", None)

        inputs = iter(["x", ":quit"])
        printed: list[str] = []
        monkeypatch.setattr("builtins.input", lambda _prompt="": next(inputs, ""))
        monkeypatch.setattr("builtins.print", lambda s: printed.append(s))

        Repl.run(language=SilentLang())

        assert printed == []


# ---------------------------------------------------------------------------
# Unit tests for built-in implementations
# ---------------------------------------------------------------------------


class TestEchoLanguage:
    """Unit tests for EchoLanguage."""

    def test_echo_returns_input(self) -> None:
        lang = EchoLanguage()
        assert lang.eval("anything") == ("ok", "anything")

    def test_quit_sentinel(self) -> None:
        lang = EchoLanguage()
        assert lang.eval(":quit") == "quit"

    def test_empty_string(self) -> None:
        lang = EchoLanguage()
        assert lang.eval("") == ("ok", "")


class TestDefaultPrompt:
    """Unit tests for DefaultPrompt."""

    def test_global_prompt(self) -> None:
        p = DefaultPrompt()
        assert p.global_prompt() == "> "

    def test_line_prompt(self) -> None:
        p = DefaultPrompt()
        assert p.line_prompt() == "... "


class TestSilentWaiting:
    """Unit tests for SilentWaiting."""

    def test_start_returns_none(self) -> None:
        w = SilentWaiting()
        assert w.start() is None

    def test_tick_returns_none(self) -> None:
        w = SilentWaiting()
        assert w.tick(None) is None

    def test_tick_ms_is_100(self) -> None:
        w = SilentWaiting()
        assert w.tick_ms() == 100

    def test_stop_returns_none(self) -> None:
        w = SilentWaiting()
        assert w.stop(None) is None
