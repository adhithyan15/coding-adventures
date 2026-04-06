defmodule ParrotTest do
  @moduledoc """
  Tests for the Parrot REPL program.

  ## Test Strategy

  All tests use I/O injection — no real stdin or stdout is touched. Instead:

  - An `Agent` holds the input queue (list of strings to return in sequence).
  - A second `Agent` accumulates the output strings.
  - `Loop.run/6` receives closures that read from / write to these Agents.

  This pattern is the standard way to test REPL programs in this codebase. It
  is deterministic (no concurrency surprises), fast (no I/O syscalls), and
  clean (no global state pollution between tests).

  ## Why Agent?

  Elixir closures cannot capture mutable state — a closure over a list variable
  will always see the original list. We need a mutable cell so that successive
  calls to `input_fn` return successive inputs.

  `Agent` is the Elixir idiom for a simple stateful process. Think of it as a
  single-value mailbox that supports atomic read-and-update:

      Agent.get_and_update(agent, fn
        [] -> {nil, []}          # empty list → return nil, keep empty
        [h | t] -> {h, t}        # non-empty → return head, keep tail
      end)

  This is safe even with async evaluation (the default `:async` mode) because
  Agent serialises all operations.

  ## Output Collection

  The output Agent stores strings in *prepend order* (each new string goes to
  the front). At the end, `Enum.reverse/1` restores chronological order.
  Prepend-then-reverse is O(n) overall; append-at-end via `++ [elem]` is O(n²).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.Repl.{EchoLanguage, SilentWaiting, Loop}

  # ---------------------------------------------------------------------------
  # Test helper: run_parrot/2
  # ---------------------------------------------------------------------------

  # run_parrot/2 is the workhorse. It:
  #   1. Creates two Agents: one for inputs, one to collect outputs.
  #   2. Wires them up as input_fn / output_fn.
  #   3. Runs the full Loop.run with Parrot.Prompt and SilentWaiting.
  #   4. Returns the collected outputs in chronological order.
  #
  # `opts` is forwarded to Loop.run so callers can switch between :sync and
  # :async mode.
  defp run_parrot(inputs, opts \\ []) do
    {:ok, q} = Agent.start_link(fn -> inputs end)
    {:ok, out} = Agent.start_link(fn -> [] end)

    # input_fn pops from the front of the list.
    # Returns nil when the list is exhausted — the loop treats nil as EOF.
    input_fn = fn _prompt ->
      Agent.get_and_update(q, fn
        [] -> {nil, []}
        [h | t] -> {h, t}
      end)
    end

    # output_fn prepends to the accumulator.
    # We reverse at the end to recover chronological order.
    output_fn = fn text ->
      Agent.update(out, &[text | &1])
    end

    Loop.run(
      EchoLanguage,
      Parrot.Prompt,
      SilentWaiting,
      input_fn,
      output_fn,
      opts
    )

    result = Agent.get(out, &Enum.reverse/1)
    Agent.stop(q)
    Agent.stop(out)
    result
  end

  # ---------------------------------------------------------------------------
  # Helpers for analysing output
  # ---------------------------------------------------------------------------

  # Returns only the non-prompt strings from the output list.
  # Prompt strings contain "Parrot" or start with "🦜" — we filter those out
  # to check only the echo results.
  defp result_lines(output) do
    Enum.reject(output, fn line ->
      String.contains?(line, "Parrot") or
        String.starts_with?(line, "🦜")
    end)
  end

  # ===========================================================================
  # 1. Basic echo
  # ===========================================================================

  describe "basic echo" do
    # The fundamental contract: whatever the user types is echoed back.
    test "single input 'hello' is echoed back" do
      output = run_parrot(["hello", ":quit"])
      results = result_lines(output)

      assert "hello" in results,
             "Expected 'hello' in results, got: #{inspect(results)}"
    end

    test "echoed value appears in raw output list" do
      output = run_parrot(["echo-me", ":quit"])
      assert "echo-me" in output
    end
  end

  # ===========================================================================
  # 2. Quit
  # ===========================================================================

  describe "quit handling" do
    test ":quit ends the session cleanly" do
      # run_parrot wraps Loop.run which returns :ok. If :quit were not handled,
      # the call would hang waiting for more input and eventually return when
      # the Agent returned nil — but we want to verify :quit specifically.
      output = run_parrot([":quit", "never-seen"])
      results = result_lines(output)

      # The string ":quit" must NOT appear as output (it's a control signal).
      refute ":quit" in results
      # Input added after ":quit" must never be echoed.
      refute "never-seen" in results
    end

    test "session ends on :quit even with inputs still queued" do
      output = run_parrot([":quit", "a", "b", "c"])
      results = result_lines(output)

      refute "a" in results
      refute "b" in results
      refute "c" in results
    end
  end

  # ===========================================================================
  # 3. Multiple inputs echoed in order
  # ===========================================================================

  describe "multiple inputs" do
    test "three inputs are echoed in sequence" do
      output = run_parrot(["alpha", "beta", "gamma", ":quit"])
      results = result_lines(output)

      # All three values must appear.
      assert "alpha" in results
      assert "beta" in results
      assert "gamma" in results
    end

    test "inputs appear in the correct order" do
      output = run_parrot(["first", "second", "third", ":quit"])
      results = result_lines(output)

      # Filter to just the three known results.
      ordered = Enum.filter(results, &(&1 in ["first", "second", "third"]))
      assert ordered == ["first", "second", "third"],
             "Expected in-order results, got: #{inspect(ordered)}"
    end

    test "five inputs all echoed before quit" do
      words = ["one", "two", "three", "four", "five"]
      output = run_parrot(words ++ [":quit"])
      results = result_lines(output)

      for word <- words do
        assert word in results, "Expected '#{word}' in results"
      end
    end
  end

  # ===========================================================================
  # 4. Sync mode
  # ===========================================================================

  describe "sync mode" do
    # :sync mode evaluates language.eval/1 directly on the calling process
    # instead of spawning a Task. It produces the same results as :async.

    test "sync mode: basic echo works" do
      output = run_parrot(["hello", ":quit"], mode: :sync)
      results = result_lines(output)

      assert "hello" in results
    end

    test "sync mode: quit works" do
      output = run_parrot([":quit", "unreachable"], mode: :sync)
      results = result_lines(output)

      refute "unreachable" in results
    end

    test "sync mode: multiple inputs echoed in order" do
      output = run_parrot(["x", "y", "z", ":quit"], mode: :sync)
      results = result_lines(output)

      ordered = Enum.filter(results, &(&1 in ["x", "y", "z"]))
      assert ordered == ["x", "y", "z"]
    end
  end

  # ===========================================================================
  # 5. Global prompt
  # ===========================================================================

  describe "global prompt" do
    test "global prompt is printed before first input" do
      # The output list always starts with the global prompt because the loop
      # calls output_fn.(prompt.global_prompt()) before reading the first line.
      output = run_parrot([":quit"])

      # The first item in the output must be the global prompt string.
      assert List.first(output) == Parrot.Prompt.global_prompt(),
             "Expected global prompt first, got: #{inspect(List.first(output))}"
    end

    test "global prompt contains 'Parrot'" do
      # Smoke test: the prompt must mention what program this is.
      assert String.contains?(Parrot.Prompt.global_prompt(), "Parrot")
    end

    test "global prompt appears in output" do
      output = run_parrot([":quit"])
      assert Parrot.Prompt.global_prompt() in output
    end
  end

  # ===========================================================================
  # 6. Line prompt
  # ===========================================================================

  describe "line prompt" do
    test "line_prompt contains the parrot emoji" do
      assert String.contains?(Parrot.Prompt.line_prompt(), "🦜")
    end

    test "line_prompt is a non-empty string" do
      lp = Parrot.Prompt.line_prompt()
      assert is_binary(lp)
      assert byte_size(lp) > 0
    end

    test "line_prompt is different from global_prompt" do
      # They serve different roles; they should not be identical strings.
      refute Parrot.Prompt.line_prompt() == Parrot.Prompt.global_prompt()
    end
  end

  # ===========================================================================
  # 7. EOF (nil from input_fn) exits gracefully
  # ===========================================================================

  describe "EOF handling" do
    test "nil input (EOF) exits without error" do
      # Passing an empty list means the Agent immediately returns nil,
      # which the loop treats as EOF.
      output = run_parrot([])

      # We just want to confirm it returned at all (no hang or crash).
      # The output may contain only the prompt.
      assert is_list(output)
    end

    test "inputs before EOF are still echoed" do
      output = run_parrot(["before-eof"])
      # No :quit — the loop exits when the Agent returns nil.
      assert "before-eof" in output
    end
  end

  # ===========================================================================
  # 8. Empty string echoed
  # ===========================================================================

  describe "empty string" do
    test "empty string input is echoed back" do
      # EchoLanguage.eval("") returns {:ok, ""}.
      # The loop calls output_fn.(""), which adds "" to the output list.
      output = run_parrot(["", ":quit"])

      # "" must appear in the output list. We can't use `result_lines` here
      # because "" matches nothing in the filter, so we check `output` directly.
      assert "" in output
    end
  end

  # ===========================================================================
  # 9. Whitespace echoed
  # ===========================================================================

  describe "whitespace" do
    test "whitespace-only input is echoed back" do
      output = run_parrot(["   ", ":quit"])
      assert "   " in output
    end

    test "tab character is echoed back" do
      output = run_parrot(["\t", ":quit"])
      assert "\t" in output
    end
  end

  # ===========================================================================
  # 10. ParrotPrompt module
  # ===========================================================================

  describe "Parrot.Prompt module" do
    test "global_prompt/0 returns a binary" do
      assert is_binary(Parrot.Prompt.global_prompt())
    end

    test "global_prompt/0 contains 'Parrot'" do
      assert String.contains?(Parrot.Prompt.global_prompt(), "Parrot")
    end

    test "global_prompt/0 contains the parrot emoji" do
      assert String.contains?(Parrot.Prompt.global_prompt(), "🦜")
    end

    test "global_prompt/0 mentions :quit instruction" do
      # The banner should tell the user how to exit.
      assert String.contains?(Parrot.Prompt.global_prompt(), ":quit")
    end

    test "line_prompt/0 returns a binary" do
      assert is_binary(Parrot.Prompt.line_prompt())
    end

    test "line_prompt/0 contains parrot emoji" do
      assert String.contains?(Parrot.Prompt.line_prompt(), "🦜")
    end
  end

  # ===========================================================================
  # 11. Error result prints "ERROR: ..."
  # ===========================================================================

  describe "error output" do
    # EchoLanguage never returns {:error, _} — it only returns :quit or {:ok, _}.
    # To test the error path we define an inline language module that always
    # returns an error. We call Loop.run directly with that language.
    defmodule AlwaysErrorLanguage do
      @moduledoc "Test helper: always returns an error."
      @behaviour CodingAdventures.Repl.Language

      @impl true
      def eval(":quit"), do: :quit
      def eval(input), do: {:error, "bad: #{input}"}
    end

    test "error result is prefixed with 'ERROR: '" do
      {:ok, q} = Agent.start_link(fn -> ["bad-input", ":quit"] end)
      {:ok, out} = Agent.start_link(fn -> [] end)

      input_fn = fn _prompt ->
        Agent.get_and_update(q, fn
          [] -> {nil, []}
          [h | t] -> {h, t}
        end)
      end

      output_fn = fn text -> Agent.update(out, &[text | &1]) end

      Loop.run(
        AlwaysErrorLanguage,
        Parrot.Prompt,
        SilentWaiting,
        input_fn,
        output_fn,
        mode: :sync
      )

      output = Agent.get(out, &Enum.reverse/1)
      Agent.stop(q)
      Agent.stop(out)

      error_lines = Enum.filter(output, &String.starts_with?(&1, "ERROR: "))
      assert length(error_lines) == 1
      assert hd(error_lines) == "ERROR: bad: bad-input"
    end

    test "error does not end the session" do
      # After an error the loop should continue accepting input.
      {:ok, q} = Agent.start_link(fn -> ["bad1", "bad2", ":quit"] end)
      {:ok, out} = Agent.start_link(fn -> [] end)

      input_fn = fn _p ->
        Agent.get_and_update(q, fn
          [] -> {nil, []}
          [h | t] -> {h, t}
        end)
      end

      output_fn = fn text -> Agent.update(out, &[text | &1]) end

      Loop.run(
        AlwaysErrorLanguage,
        Parrot.Prompt,
        SilentWaiting,
        input_fn,
        output_fn,
        mode: :sync
      )

      output = Agent.get(out, &Enum.reverse/1)
      Agent.stop(q)
      Agent.stop(out)

      error_lines = Enum.filter(output, &String.starts_with?(&1, "ERROR: "))
      # Two bad inputs → two ERROR lines.
      assert length(error_lines) == 2
    end
  end

  # ===========================================================================
  # 12. Banner printed each iteration (once per cycle)
  # ===========================================================================

  describe "banner behaviour" do
    test "banner (global_prompt) is printed once per REPL cycle" do
      # With two real inputs + :quit, there are 3 cycles (including the :quit
      # read). Each cycle prints global_prompt once.
      output = run_parrot(["a", "b", ":quit"])
      banner = Parrot.Prompt.global_prompt()

      count = Enum.count(output, &(&1 == banner))
      # Each of the 3 input reads is preceded by one global_prompt call.
      assert count == 3,
             "Expected 3 banners for 3 cycles, got #{count}. Output: #{inspect(output)}"
    end
  end
end
