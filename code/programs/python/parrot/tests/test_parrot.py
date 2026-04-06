"""Tests for the Parrot REPL program.

Test Strategy
-------------
All tests drive the REPL through :func:`~coding_adventures_repl.run_with_io`
with injected I/O.  No real stdin or stdout is touched.  This makes every
test:

- **Deterministic** — no timing or terminal state.
- **Fast** — no I/O syscalls.
- **Isolated** — tests don't interfere with each other.

I/O Injection Pattern
---------------------
Each test (via :func:`run_parrot`) creates:

1. A mutable list acting as the input queue.
2. An output list that ``output_fn`` appends to.

``run_with_io`` is called with:

- ``input_fn=lambda: queue.pop(0) if queue else None``
  — pops the next item; returns ``None`` when the queue is empty.
- ``output_fn=lambda text: output.append(text)``
  — collects every string the loop writes.

This is the canonical test pattern for this REPL framework.

Note on Prompts in Output
-------------------------
The loop calls ``output_fn(prompt.global_prompt())`` before each input read.
With :class:`~parrot.prompt.ParrotPrompt`, that means the banner string appears
in the output list before every result.  Tests that check for specific values
either:

- Use :func:`result_lines` to filter out prompt strings, or
- Check the full output list when testing prompt behaviour specifically.
"""

from __future__ import annotations

import pytest

from coding_adventures_repl import EchoLanguage, Language, SilentWaiting, run_with_io
from parrot.prompt import ParrotPrompt


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def run_parrot(inputs: list[str | None], mode: str = "async") -> list[str]:
    """Run the Parrot REPL with injected inputs; return all collected outputs.

    This is the primary test helper.  It creates a mutable queue from
    ``inputs`` and collects everything the loop writes to ``output_fn``.

    Parameters
    ----------
    inputs:
        A list of strings (and optionally ``None``) to feed as successive
        inputs.  ``None`` signals end-of-input (like Ctrl-D); once the
        queue is empty the helper also returns ``None``.
    mode:
        Evaluation mode to pass to :func:`run_with_io`.  ``"async"``
        (default) runs eval in a background thread; ``"sync"`` runs eval
        directly on the calling thread.

    Returns
    -------
    list[str]
        Everything written via ``output_fn``, in chronological order.
        Includes prompt strings (the banner) as well as result values.
    """
    # Mutable queue: we use a plain list and pop from the front.
    # list.pop(0) is O(n) but perfectly fine for test-scale inputs.
    # An alternative is collections.deque with popleft(), but list is simpler.
    output: list[str] = []
    queue = list(inputs)  # copy so the caller's list is not mutated

    run_with_io(
        language=EchoLanguage(),
        prompt=ParrotPrompt(),
        waiting=SilentWaiting(),
        # Pop from the front of the queue. When the queue is empty, return
        # None to signal end-of-input to the loop.
        input_fn=lambda: queue.pop(0) if queue else None,
        # Append every string the loop writes. This captures both prompt
        # strings (the banner) and result values (echoed input).
        output_fn=lambda text: output.append(text),
        mode=mode,
    )
    return output


def result_lines(output: list[str]) -> list[str]:
    """Filter out prompt strings, returning only result values.

    The loop writes the banner (``ParrotPrompt.global_prompt()``) before
    every input read.  Tests that care only about echoed results should
    call this to strip the banner noise.

    Parameters
    ----------
    output:
        The raw list returned by :func:`run_parrot`.

    Returns
    -------
    list[str]
        Lines that are not the global prompt or line prompt string.
    """
    # We filter out both the global_prompt and line_prompt strings.
    # Everything else is a result value produced by the language.
    prompts = {ParrotPrompt().global_prompt(), ParrotPrompt().line_prompt()}
    return [line for line in output if line not in prompts]


# ---------------------------------------------------------------------------
# Test 1 — Basic echo
# ---------------------------------------------------------------------------


class TestBasicEcho:
    """The REPL echoes back whatever the user types."""

    def test_single_input_echoed(self) -> None:
        """A single input string appears in the output.

        This verifies the core read→eval→print cycle: EchoLanguage returns
        ``("ok", input)`` for any non-quit string, and the loop writes that
        value via ``output_fn``.
        """
        output = run_parrot(["hello", ":quit"])
        results = result_lines(output)

        assert "hello" in results, f"Expected 'hello' in results, got: {results}"

    def test_echoed_value_in_raw_output(self) -> None:
        """The echoed value appears somewhere in the full output list.

        Complementary to the filtered test above: checks the raw output
        without filtering so we know the value isn't being lost.
        """
        output = run_parrot(["raw-check", ":quit"])
        assert "raw-check" in output


# ---------------------------------------------------------------------------
# Test 2 — Quit
# ---------------------------------------------------------------------------


class TestQuit:
    """:quit ends the session immediately."""

    def test_quit_stops_loop(self) -> None:
        """:quit terminates the loop; subsequent inputs are never read.

        After ``:quit``, the loop must break immediately.  Any inputs placed
        after ``:quit`` in the queue must NOT appear in the output.
        """
        output = run_parrot([":quit", "should-never-appear"])
        results = result_lines(output)

        assert "should-never-appear" not in results

    def test_quit_produces_no_result_output(self) -> None:
        """:quit itself does not produce any output value.

        The loop breaks silently — no "Goodbye" or echo of ":quit".
        """
        output = run_parrot([":quit"])
        results = result_lines(output)

        assert results == [], f"Expected no results, got: {results}"

    def test_session_ends_on_quit_with_more_inputs_queued(self) -> None:
        """The loop stops at :quit even with many inputs still queued.

        Verifies that :quit is respected regardless of queue depth.
        """
        output = run_parrot([":quit", "a", "b", "c", "d", "e"])
        results = result_lines(output)

        for letter in ["a", "b", "c", "d", "e"]:
            assert letter not in results, f"'{letter}' should not appear after :quit"


# ---------------------------------------------------------------------------
# Test 3 — Multiple inputs echoed in sequence
# ---------------------------------------------------------------------------


class TestMultipleInputs:
    """The loop processes multiple inputs in order."""

    def test_three_inputs_in_sequence(self) -> None:
        """Three distinct inputs produce three distinct outputs in order.

        Verifies that the loop does not conflate, skip, or reorder results.
        """
        output = run_parrot(["alpha", "beta", "gamma", ":quit"])
        results = result_lines(output)

        # All three must be present.
        assert "alpha" in results
        assert "beta" in results
        assert "gamma" in results

    def test_inputs_appear_in_correct_order(self) -> None:
        """Results appear in the same order as the inputs.

        The REPL is sequential — it processes one input at a time and
        writes the result before reading the next.
        """
        output = run_parrot(["first", "second", "third", ":quit"])
        results = result_lines(output)

        # Extract just the known three results to check order.
        ordered = [r for r in results if r in ("first", "second", "third")]
        assert ordered == ["first", "second", "third"], f"Wrong order: {ordered}"

    def test_five_inputs_all_echoed(self) -> None:
        """Five inputs are all echoed before the session ends."""
        words = ["one", "two", "three", "four", "five"]
        output = run_parrot(words + [":quit"])
        results = result_lines(output)

        for word in words:
            assert word in results, f"Expected '{word}' in results"


# ---------------------------------------------------------------------------
# Test 4 — Sync mode
# ---------------------------------------------------------------------------


class TestSyncMode:
    """mode="sync" evaluates directly on the calling thread."""

    def test_sync_mode_basic_echo(self) -> None:
        """Echo works in sync mode — same result as async.

        In sync mode, ``language.eval`` runs on the calling thread with no
        background threading.  The output must be identical to async mode.
        """
        output = run_parrot(["hello", ":quit"], mode="sync")
        results = result_lines(output)

        assert "hello" in results

    def test_sync_mode_quit_works(self) -> None:
        """:quit ends the session in sync mode.

        Verifies the quit path is reached even without background threading.
        """
        output = run_parrot([":quit", "unreachable"], mode="sync")
        results = result_lines(output)

        assert "unreachable" not in results

    def test_sync_mode_ordering(self) -> None:
        """Multiple inputs are echoed in order in sync mode."""
        output = run_parrot(["x", "y", "z", ":quit"], mode="sync")
        results = result_lines(output)

        ordered = [r for r in results if r in ("x", "y", "z")]
        assert ordered == ["x", "y", "z"]


# ---------------------------------------------------------------------------
# Test 5 — Async mode (default)
# ---------------------------------------------------------------------------


class TestAsyncMode:
    """mode="async" (the default) runs eval in a background thread."""

    def test_async_mode_echo(self) -> None:
        """Echo works in async mode (the default)."""
        # run_parrot defaults to mode="async" — no explicit argument needed.
        output = run_parrot(["async-hello", ":quit"])
        results = result_lines(output)

        assert "async-hello" in results

    def test_async_mode_explicit_parameter(self) -> None:
        """Passing mode='async' explicitly produces the same result."""
        output = run_parrot(["explicit-async", ":quit"], mode="async")
        results = result_lines(output)

        assert "explicit-async" in results


# ---------------------------------------------------------------------------
# Test 6 — Banner content
# ---------------------------------------------------------------------------


class TestBannerContent:
    """The global_prompt banner contains the expected content."""

    def test_banner_contains_parrot(self) -> None:
        """The banner must mention 'Parrot' so users know what program this is."""
        assert "Parrot" in ParrotPrompt().global_prompt()

    def test_banner_contains_parrot_emoji(self) -> None:
        """The banner must contain the 🦜 emoji to reinforce the parrot theme."""
        assert "🦜" in ParrotPrompt().global_prompt()

    def test_banner_contains_quit_instruction(self) -> None:
        """The banner must tell the user how to exit the REPL."""
        assert ":quit" in ParrotPrompt().global_prompt()

    def test_banner_appears_in_output(self) -> None:
        """The banner is written to output before the first input read."""
        output = run_parrot([":quit"])
        assert ParrotPrompt().global_prompt() in output


# ---------------------------------------------------------------------------
# Test 7 — Line prompt
# ---------------------------------------------------------------------------


class TestLinePrompt:
    """The line_prompt contains the parrot emoji."""

    def test_line_prompt_contains_emoji(self) -> None:
        """line_prompt must contain the 🦜 emoji for visual consistency."""
        assert "🦜" in ParrotPrompt().line_prompt()

    def test_line_prompt_is_string(self) -> None:
        """line_prompt returns a plain str (not None, not bytes)."""
        lp = ParrotPrompt().line_prompt()
        assert isinstance(lp, str)
        assert len(lp) > 0

    def test_line_prompt_format(self) -> None:
        """line_prompt has the expected format: emoji + space + > + space."""
        # The format "🦜 > " is human-readable and consistent with shell
        # prompt conventions (prompt character followed by a space).
        lp = ParrotPrompt().line_prompt()
        assert lp == "🦜 > ", f"Expected '🦜 > ', got: {lp!r}"

    def test_line_prompt_differs_from_global(self) -> None:
        """line_prompt and global_prompt are different strings.

        They serve different roles in the UI; they must be distinct.
        """
        p = ParrotPrompt()
        assert p.line_prompt() != p.global_prompt()


# ---------------------------------------------------------------------------
# Test 8 — EOF exits gracefully
# ---------------------------------------------------------------------------


class TestEOF:
    """None from input_fn signals EOF and exits the loop cleanly."""

    def test_immediate_eof_exits_without_error(self) -> None:
        """An empty input queue causes the loop to exit immediately.

        When ``input_fn`` returns ``None`` on the very first call, the loop
        must exit cleanly — no exception, no hang.
        """
        # Empty list → input_fn always returns None.
        output = run_parrot([])

        # Just confirm the function returned (if it hangs, pytest will time out).
        assert isinstance(output, list)

    def test_inputs_before_eof_are_echoed(self) -> None:
        """Inputs before EOF are echoed normally.

        The loop echoes all queued inputs, then exits when the queue is
        exhausted (input_fn returns None).
        """
        output = run_parrot(["before-eof"])
        # No :quit — the loop exits on None from the empty queue.
        assert "before-eof" in output

    def test_explicit_none_in_queue_acts_as_eof(self) -> None:
        """An explicit None in the input queue acts as EOF.

        This mirrors what happens when a pipe is closed mid-session.
        """
        output = run_parrot(["before", None, "after"])
        results = result_lines(output)

        # "before" should be echoed; "after" should NOT (loop stopped at None).
        assert "before" in results
        assert "after" not in results


# ---------------------------------------------------------------------------
# Test 9 — Empty string echoed
# ---------------------------------------------------------------------------


class TestEmptyString:
    """An empty string input is echoed back."""

    def test_empty_string_echoed(self) -> None:
        """EchoLanguage.eval("") returns ("ok", ""), so "" appears in output.

        This verifies that the loop does not treat an empty string as EOF
        or as a special sentinel. The user pressing Enter with no input
        should produce a visible (but blank) echo.
        """
        output = run_parrot(["", ":quit"])
        # "" appears in the raw output list (the loop writes it via output_fn).
        assert "" in output

    def test_empty_string_not_suppressed(self) -> None:
        """The loop does not suppress empty-string results.

        Only ``None`` (no value) from the language causes the loop to skip
        output.  An empty string ``""`` is a valid value and must be written.
        """
        output = run_parrot(["", ":quit"])
        # The empty string must appear regardless of filtering.
        assert "" in output


# ---------------------------------------------------------------------------
# Test 10 — Error from bad language prints "Error: ..."
# ---------------------------------------------------------------------------


class TestErrorOutput:
    """A language returning ("error", msg) produces "Error: msg" in output."""

    def test_error_result_formatted_correctly(self) -> None:
        """The loop formats error results as "Error: <message>".

        EchoLanguage never returns an error, so we define a custom language
        that always errors.  This tests the error-handling path in the
        loop's result dispatcher.
        """

        class AlwaysErrorLanguage(Language):
            """Returns an error tuple for every non-quit input."""

            def eval(self, input: str) -> tuple[str, str | None] | str:  # noqa: ANN001
                if input == ":quit":
                    return "quit"
                return ("error", f"bad: {input}")

        output: list[str] = []
        queue = ["bad-input", ":quit"]

        run_with_io(
            language=AlwaysErrorLanguage(),
            prompt=ParrotPrompt(),
            waiting=SilentWaiting(),
            input_fn=lambda: queue.pop(0) if queue else None,
            output_fn=lambda text: output.append(text),
            mode="sync",
        )

        # Filter to lines starting with "Error: "
        error_lines = [line for line in output if line.startswith("Error: ")]
        assert len(error_lines) == 1, f"Expected 1 error line, got: {error_lines}"
        assert error_lines[0] == "Error: bad: bad-input"

    def test_session_continues_after_error(self) -> None:
        """An error result does not terminate the loop.

        The REPL must continue reading input after an error, just as Python
        keeps running after a SyntaxError.
        """

        class ErrorThenEcho(Language):
            """Errors on first call, echoes on subsequent calls."""

            def __init__(self) -> None:
                self._count = 0

            def eval(self, input: str) -> tuple[str, str | None] | str:  # noqa: ANN001
                if input == ":quit":
                    return "quit"
                self._count += 1
                if self._count == 1:
                    return ("error", "first-call-error")
                return ("ok", input)

        output: list[str] = []
        queue = ["first", "second", ":quit"]

        run_with_io(
            language=ErrorThenEcho(),
            prompt=ParrotPrompt(),
            waiting=SilentWaiting(),
            input_fn=lambda: queue.pop(0) if queue else None,
            output_fn=lambda text: output.append(text),
            mode="sync",
        )

        # "second" must appear — the loop continued after the error.
        assert "second" in output


# ---------------------------------------------------------------------------
# Test 11 — Global prompt printed once per cycle
# ---------------------------------------------------------------------------


class TestGlobalPromptFrequency:
    """The global prompt is printed once per REPL cycle."""

    def test_global_prompt_printed_once_per_cycle(self) -> None:
        """With N input reads, the banner appears exactly N times.

        The loop always emits global_prompt before each input read,
        including the final read that gets None (EOF) or :quit.

        With 2 real inputs + :quit: 3 cycles → 3 banner appearances.
        """
        output = run_parrot(["a", "b", ":quit"])
        banner = ParrotPrompt().global_prompt()

        count = output.count(banner)
        # Three reads (a, b, :quit) → three banner emissions.
        assert count == 3, f"Expected 3 banners, got {count}. Output: {output}"

    def test_global_prompt_first_item(self) -> None:
        """The very first item in the output is the global prompt.

        The loop emits the prompt before reading input, so it must always
        be the first thing written.
        """
        output = run_parrot([":quit"])
        assert output[0] == ParrotPrompt().global_prompt()


# ---------------------------------------------------------------------------
# Test 12 — Output collected correctly
# ---------------------------------------------------------------------------


class TestOutputCollection:
    """Verifies that run_parrot collects output in the right order."""

    def test_output_is_list(self) -> None:
        """run_parrot always returns a list."""
        output = run_parrot([":quit"])
        assert isinstance(output, list)

    def test_empty_session_output_contains_banner(self) -> None:
        """Even a zero-input session writes the banner at least once."""
        output = run_parrot([":quit"])
        # At minimum the banner was written before reading :quit.
        assert len(output) >= 1
        assert ParrotPrompt().global_prompt() in output


# ---------------------------------------------------------------------------
# Test 13 — ParrotPrompt.line_prompt format
# ---------------------------------------------------------------------------


class TestLinePromptDetails:
    """Detailed tests for ParrotPrompt.line_prompt."""

    def test_line_prompt_starts_with_emoji(self) -> None:
        """line_prompt starts with the parrot emoji."""
        lp = ParrotPrompt().line_prompt()
        assert lp.startswith("🦜")

    def test_line_prompt_ends_with_space(self) -> None:
        """line_prompt ends with a space (conventional prompt style)."""
        lp = ParrotPrompt().line_prompt()
        assert lp.endswith(" ")

    def test_line_prompt_is_short(self) -> None:
        """line_prompt is short — no more than 20 characters.

        Line prompts should be compact so they don't crowd the user's
        input.  20 characters is generous for a prompt string.
        """
        lp = ParrotPrompt().line_prompt()
        assert len(lp) <= 20, f"line_prompt too long: {lp!r}"


# ---------------------------------------------------------------------------
# Test 14 — ParrotPrompt.global_prompt content
# ---------------------------------------------------------------------------


class TestGlobalPromptContent:
    """Detailed content checks for ParrotPrompt.global_prompt."""

    def test_global_prompt_is_string(self) -> None:
        """global_prompt returns a str."""
        gp = ParrotPrompt().global_prompt()
        assert isinstance(gp, str)

    def test_global_prompt_is_multiline(self) -> None:
        """global_prompt contains at least one newline (it's a banner)."""
        gp = ParrotPrompt().global_prompt()
        assert "\n" in gp

    def test_global_prompt_ends_with_double_newline(self) -> None:
        """global_prompt ends with \\n\\n to create a blank line after the banner."""
        gp = ParrotPrompt().global_prompt()
        assert gp.endswith("\n\n"), f"Expected double newline ending, got: {gp!r}"

    def test_global_prompt_repeat_instruction(self) -> None:
        """global_prompt mentions that the REPL repeats input."""
        gp = ParrotPrompt().global_prompt()
        # Some phrasing about repetition must be present.
        assert "repeat" in gp.lower(), f"Expected 'repeat' in global_prompt: {gp!r}"
