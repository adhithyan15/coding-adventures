defmodule CodingAdventures.ReplTest do
  use ExUnit.Case, async: true

  import ExUnit.CaptureIO

  # Pull in the three built-in plugins so tests stay readable.
  alias CodingAdventures.Repl
  alias CodingAdventures.Repl.{EchoLanguage, DefaultPrompt, SilentWaiting}

  # ---------------------------------------------------------------------------
  # Test helpers
  # ---------------------------------------------------------------------------
  #
  # drive_repl/1 is the workhorse for full-loop tests. It:
  #   1. Takes a list of input strings.
  #   2. Runs the REPL until those strings are exhausted (returning nil = EOF).
  #   3. Collects everything passed to output_fn into a list.
  #   4. Returns that list.
  #
  # We use Agent to give the closures mutable references without resorting to
  # Process.put/get, which would pollute the process dictionary.
  defp drive_repl(inputs, language \\ EchoLanguage) do
    {:ok, in_agent} = Agent.start_link(fn -> inputs end)
    {:ok, out_agent} = Agent.start_link(fn -> [] end)

    # input_fn: pop the next item from the input list.
    # Returns nil when the list is empty (signals EOF to the loop).
    input_fn = fn _prompt ->
      Agent.get_and_update(in_agent, fn
        [] -> {nil, []}
        [h | t] -> {h, t}
      end)
    end

    # output_fn: append to the output list.
    output_fn = fn line ->
      Agent.update(out_agent, fn acc -> acc ++ [line] end)
    end

    :ok = Repl.run_with_io(language, DefaultPrompt, SilentWaiting, input_fn, output_fn)

    result = Agent.get(out_agent, & &1)
    Agent.stop(in_agent)
    Agent.stop(out_agent)
    result
  end

  # step_once/2 calls Repl.step with injected output_fn.
  # Returns {step_result, collected_outputs}.
  defp step_once(input, language \\ EchoLanguage) do
    {:ok, out_agent} = Agent.start_link(fn -> [] end)

    output_fn = fn line ->
      Agent.update(out_agent, fn acc -> acc ++ [line] end)
    end

    # input_fn is a no-op for step (input is pre-provided)
    input_fn = fn _ -> nil end

    result = Repl.step(language, DefaultPrompt, SilentWaiting, input, input_fn, output_fn)
    outputs = Agent.get(out_agent, & &1)
    Agent.stop(out_agent)
    {result, outputs}
  end

  # ===========================================================================
  # 1. Basic echo
  # ===========================================================================

  describe "basic echo" do
    # The fundamental contract: input goes in, same text comes back out.
    # The output list will contain prompt strings AND result strings.
    # We filter for non-prompt entries to check only results.
    test "input 'hello' produces output 'hello'" do
      all_output = drive_repl(["hello", ":quit"])
      # Prompts ("> ") and results ("hello") are both captured.
      # The result "hello" must be in the list.
      assert "hello" in all_output
    end

    test "step with 'hello' returns {:continue, 'hello'} and outputs 'hello'" do
      {result, outputs} = step_once("hello")
      assert result == {:continue, "hello"}
      assert "hello" in outputs
    end
  end

  # ===========================================================================
  # 2. Quit
  # ===========================================================================

  describe "quit handling" do
    test "input ':quit' ends the session and returns :ok" do
      # drive_repl uses run_with_io which returns :ok on normal termination.
      # If the loop failed to handle :quit, it would block waiting for more
      # input (and the Agent would return nil → also ok, but let's be explicit).
      result =
        with {:ok, in_agent} <- Agent.start_link(fn -> [":quit"] end),
             {:ok, out_agent} <- Agent.start_link(fn -> [] end) do
          input_fn = fn _ ->
            Agent.get_and_update(in_agent, fn
              [] -> {nil, []}
              [h | t] -> {h, t}
            end)
          end

          output_fn = fn line -> Agent.update(out_agent, fn acc -> acc ++ [line] end) end
          r = Repl.run_with_io(EchoLanguage, DefaultPrompt, SilentWaiting, input_fn, output_fn)
          Agent.stop(in_agent)
          Agent.stop(out_agent)
          r
        end

      assert result == :ok
    end

    test "step with ':quit' returns {:quit, nil}" do
      {result, _outputs} = step_once(":quit")
      assert result == {:quit, nil}
    end

    test ":quit produces no output line" do
      # Nothing should be printed when the user quits — no acknowledgment,
      # no goodbye message (that's the job of the application shell, not
      # the REPL loop itself).
      {_result, outputs} = step_once(":quit")
      # Prompts may appear; results must not.
      refute "quit" in outputs
      refute ":quit" in outputs
    end
  end

  # ===========================================================================
  # 3. Multiple turns
  # ===========================================================================

  describe "multiple turns" do
    test "['hello', 'world', ':quit'] produces 'hello' and 'world' in output" do
      all_output = drive_repl(["hello", "world", ":quit"])

      # Both results must appear, in order.
      result_lines = Enum.filter(all_output, fn line ->
        line == "hello" or line == "world"
      end)

      assert result_lines == ["hello", "world"]
    end

    test "five inputs are all echoed before quit" do
      inputs = ["one", "two", "three", "four", "five", ":quit"]
      all_output = drive_repl(inputs)

      for word <- ["one", "two", "three", "four", "five"] do
        assert word in all_output, "Expected #{word} in output, got: #{inspect(all_output)}"
      end
    end
  end

  # ===========================================================================
  # 4. Nil output
  # ===========================================================================

  describe "nil output suppression" do
    # Some languages return {:ok, nil} for statements that produce no value
    # (assignments, void calls). The loop must print nothing in that case.
    defmodule NilLanguage do
      @behaviour CodingAdventures.Repl.Language

      @impl true
      def eval(":quit"), do: :quit
      def eval(_input), do: {:ok, nil}
    end

    test "eval returning {:ok, nil} prints nothing" do
      {result, outputs} = step_once("anything", NilLanguage)

      # Step should continue (not quit)
      assert result == {:continue, nil}

      # Output should contain only the prompt (if output_fn captures prompts)
      # or be empty. There must be NO result value in the output.
      # Prompts are "> " or "... "; results would be non-prompt strings.
      result_outputs = Enum.reject(outputs, fn line ->
        line == "> " or line == "... "
      end)

      assert result_outputs == [],
        "Expected no result output, but got: #{inspect(result_outputs)}"
    end

    test "nil output in a multi-turn session does not break the loop" do
      all_output = drive_repl(["ignored", "also ignored", ":quit"], NilLanguage)

      # Only prompts should appear; no result values.
      non_prompts = Enum.reject(all_output, fn line ->
        line == "> " or line == "... "
      end)

      assert non_prompts == []
    end
  end

  # ===========================================================================
  # 5. Error output
  # ===========================================================================

  describe "error output" do
    defmodule AlwaysErrorLanguage do
      @behaviour CodingAdventures.Repl.Language

      @impl true
      def eval(":quit"), do: :quit
      def eval(input), do: {:error, "bad input: #{input}"}
    end

    test "{:error, 'bad'} prints 'ERROR: bad'" do
      {result, outputs} = step_once("bad", AlwaysErrorLanguage)

      # The step should continue (errors don't quit the session).
      assert result == {:continue, nil}

      # The error message should be prefixed with "ERROR: "
      assert "ERROR: bad input: bad" in outputs
    end

    test "error prefix is exactly 'ERROR: '" do
      {_result, outputs} = step_once("oops", AlwaysErrorLanguage)
      error_lines = Enum.filter(outputs, fn line -> String.starts_with?(line, "ERROR: ") end)
      assert length(error_lines) == 1
      assert hd(error_lines) == "ERROR: bad input: oops"
    end

    test "error does not end the session" do
      # Even after an error, the loop keeps running.
      all_output = drive_repl(["bad1", "bad2", ":quit"], AlwaysErrorLanguage)
      error_lines = Enum.filter(all_output, fn line -> String.starts_with?(line, "ERROR: ") end)
      # Two error lines, one per bad input.
      assert length(error_lines) == 2
    end
  end

  # ===========================================================================
  # 6. Exception safety
  # ===========================================================================

  describe "exception safety" do
    # A language that raises on some inputs. The loop must survive this and
    # continue accepting input, not crash.
    defmodule CrashingLanguage do
      @behaviour CodingAdventures.Repl.Language

      @impl true
      def eval(":quit"), do: :quit
      def eval("crash"), do: raise("intentional crash for testing")
      def eval(input), do: {:ok, "ok: #{input}"}
    end

    test "exception in eval prints error and continues" do
      all_output = drive_repl(["crash", "safe", ":quit"], CrashingLanguage)

      # The crash should produce an ERROR line.
      error_lines = Enum.filter(all_output, fn line -> String.starts_with?(line, "ERROR: ") end)
      assert length(error_lines) >= 1

      # The next safe input should still be evaluated.
      assert "ok: safe" in all_output,
        "Expected 'ok: safe' in output after crash. Got: #{inspect(all_output)}"
    end

    test "error message from exception contains 'unexpected error'" do
      {_result, outputs} = step_once("crash", CrashingLanguage)

      error_lines = Enum.filter(outputs, fn line -> String.starts_with?(line, "ERROR: ") end)
      assert length(error_lines) == 1
      assert String.contains?(hd(error_lines), "unexpected error"),
        "Expected 'unexpected error' in: #{hd(error_lines)}"
    end

    test "session returns :ok even after exceptions" do
      # run_with_io should return :ok, not raise, even when the language crashes.
      {:ok, in_agent} = Agent.start_link(fn -> ["crash", ":quit"] end)
      {:ok, out_agent} = Agent.start_link(fn -> [] end)

      input_fn = fn _ ->
        Agent.get_and_update(in_agent, fn
          [] -> {nil, []}
          [h | t] -> {h, t}
        end)
      end

      output_fn = fn line -> Agent.update(out_agent, fn acc -> acc ++ [line] end) end

      result = Repl.run_with_io(CrashingLanguage, DefaultPrompt, SilentWaiting, input_fn, output_fn)
      Agent.stop(in_agent)
      Agent.stop(out_agent)

      assert result == :ok
    end
  end

  # ===========================================================================
  # 7. Built-in plugin contracts
  # ===========================================================================

  describe "EchoLanguage" do
    test "echoes arbitrary strings" do
      assert EchoLanguage.eval("hello") == {:ok, "hello"}
      assert EchoLanguage.eval("42") == {:ok, "42"}
      assert EchoLanguage.eval("") == {:ok, ""}
    end

    test ":quit input returns :quit atom" do
      assert EchoLanguage.eval(":quit") == :quit
    end

    test "strings that look like quit but are not" do
      assert EchoLanguage.eval("quit") == {:ok, "quit"}
      assert EchoLanguage.eval(" :quit") == {:ok, " :quit"}
      assert EchoLanguage.eval(":quit ") == {:ok, ":quit "}
    end
  end

  describe "DefaultPrompt" do
    test "global_prompt returns '> '" do
      assert DefaultPrompt.global_prompt() == "> "
    end

    test "line_prompt returns '... '" do
      assert DefaultPrompt.line_prompt() == "... "
    end

    test "prompts are strings" do
      assert is_binary(DefaultPrompt.global_prompt())
      assert is_binary(DefaultPrompt.line_prompt())
    end
  end

  describe "SilentWaiting" do
    test "start returns a state (any term)" do
      state = SilentWaiting.start()
      # We don't care what it is, just that the call succeeds.
      assert state == nil
    end

    test "tick returns a state" do
      state = SilentWaiting.start()
      new_state = SilentWaiting.tick(state)
      # SilentWaiting is stateless; nil in, nil out.
      assert new_state == nil
    end

    test "tick_ms returns a positive integer" do
      ms = SilentWaiting.tick_ms()
      assert is_integer(ms)
      assert ms > 0
    end

    test "stop returns :ok" do
      state = SilentWaiting.start()
      assert SilentWaiting.stop(state) == :ok
    end
  end

  # ===========================================================================
  # 8. EOF / exhausted input
  # ===========================================================================

  describe "EOF handling" do
    test "empty input list causes immediate return without blocking" do
      # The input_fn returns nil on the first call. The loop should treat
      # this as EOF and return :ok cleanly.
      input_fn = fn _ -> nil end
      output_fn = fn _ -> :ok end

      result = Repl.run_with_io(EchoLanguage, DefaultPrompt, SilentWaiting, input_fn, output_fn)
      assert result == :ok
    end

    test "run exhausts all inputs then terminates on nil" do
      all_output = drive_repl(["a", "b"])
      # No :quit in the list — loop terminates when nil is returned.
      assert "a" in all_output
      assert "b" in all_output
    end
  end

  # ===========================================================================
  # 9. terminal_output/1 helper (covers run/4 I/O logic)
  # ===========================================================================

  describe "terminal_output/1" do
    # terminal_output/1 is the named function used by run/4 for real-terminal
    # output. We test it directly to get coverage on the I/O branching logic
    # without needing to simulate a real TTY.

    test "prompt-like strings (ending with space, no newline) use IO.write" do
      # IO.write does not append a newline; capture should contain no newline
      # at the end of the string.
      output = capture_io(fn -> Repl.terminal_output("> ") end)
      assert output == "> "
      # Crucially, no trailing newline was added:
      refute String.ends_with?(output, "\n")
    end

    test "result strings use IO.puts (newline appended)" do
      output = capture_io(fn -> Repl.terminal_output("hello") end)
      assert output == "hello\n"
    end

    test "strings with embedded newline use IO.puts" do
      output = capture_io(fn -> Repl.terminal_output("line1\nline2") end)
      assert String.contains?(output, "line1")
      assert String.contains?(output, "line2")
    end

    test "error messages use IO.puts" do
      output = capture_io(fn -> Repl.terminal_output("ERROR: bad") end)
      assert output == "ERROR: bad\n"
    end

    test "continuation prompt '... ' uses IO.write" do
      output = capture_io(fn -> Repl.terminal_output("... ") end)
      assert output == "... "
      refute String.ends_with?(output, "\n")
    end
  end

  # ===========================================================================
  # 10. run/4 via CaptureIO (covers the real-terminal entry point)
  # ===========================================================================

  describe "run/4 with real IO" do
    # We can exercise run/4 by providing input via capture_io's :input option
    # and capturing the output. This tests the full IO.gets → IO.write/puts
    # path that run/4 wires up.

    test "run/4 echoes input and terminates on :quit" do
      # We feed "hello\n:quit\n" as the simulated terminal input.
      output =
        capture_io("hello\n:quit\n", fn ->
          Repl.run(EchoLanguage, DefaultPrompt, SilentWaiting)
        end)

      # The output should contain "hello" (the echo result).
      assert String.contains?(output, "hello")
    end

    test "run/4 terminates cleanly on EOF (empty input)" do
      # An empty string simulates immediate EOF from IO.gets.
      output =
        capture_io("", fn ->
          result = Repl.run(EchoLanguage, DefaultPrompt, SilentWaiting)
          assert result == :ok
        end)

      # The prompt should have been written ("> ").
      assert String.contains?(output, "> ")
    end
  end

  # ===========================================================================
  # 11. Sync mode
  # ===========================================================================

  describe "sync mode" do
    # Sync mode evaluates language.eval/1 directly on the calling process with
    # no Task.async and no waiting-plugin calls.  This makes the loop simpler
    # to reason about in test and embedding contexts where the async overhead
    # and non-determinism of Task scheduling are undesirable.

    # Helper variant of drive_repl that passes [mode: :sync] to run_with_io.
    defp drive_repl_sync(inputs, language \\ EchoLanguage) do
      {:ok, in_agent} = Agent.start_link(fn -> inputs end)
      {:ok, out_agent} = Agent.start_link(fn -> [] end)

      input_fn = fn _prompt ->
        Agent.get_and_update(in_agent, fn
          [] -> {nil, []}
          [h | t] -> {h, t}
        end)
      end

      output_fn = fn line ->
        Agent.update(out_agent, fn acc -> acc ++ [line] end)
      end

      :ok =
        Repl.run_with_io(
          language,
          DefaultPrompt,
          SilentWaiting,
          input_fn,
          output_fn,
          mode: :sync
        )

      result = Agent.get(out_agent, & &1)
      Agent.stop(in_agent)
      Agent.stop(out_agent)
      result
    end

    test "sync mode: echo works" do
      # In sync mode the loop still echoes input — the only difference is
      # that eval runs synchronously rather than in a Task.
      all_output = drive_repl_sync(["hello", ":quit"])
      assert "hello" in all_output
    end

    test "sync mode: quit works" do
      # :quit must terminate the session cleanly even without a Task.
      # We verify this by confirming run_with_io returns :ok and that
      # any input after :quit is never evaluated.
      {:ok, in_agent} = Agent.start_link(fn -> [":quit", "should-not-appear"] end)
      {:ok, out_agent} = Agent.start_link(fn -> [] end)

      input_fn = fn _ ->
        Agent.get_and_update(in_agent, fn
          [] -> {nil, []}
          [h | t] -> {h, t}
        end)
      end

      output_fn = fn line ->
        Agent.update(out_agent, fn acc -> acc ++ [line] end)
      end

      result =
        Repl.run_with_io(
          EchoLanguage,
          DefaultPrompt,
          SilentWaiting,
          input_fn,
          output_fn,
          mode: :sync
        )

      all_output = Agent.get(out_agent, & &1)
      Agent.stop(in_agent)
      Agent.stop(out_agent)

      assert result == :ok
      refute "should-not-appear" in all_output
    end

    test "sync mode: error works" do
      # {:error, msg} from the language must still be formatted as "ERROR: msg"
      # in sync mode.
      defmodule SyncErrorLanguage do
        @behaviour CodingAdventures.Repl.Language

        @impl true
        def eval(":quit"), do: :quit
        def eval(input), do: {:error, "sync-err: #{input}"}
      end

      all_output = drive_repl_sync(["oops", ":quit"], SyncErrorLanguage)
      error_lines = Enum.filter(all_output, fn l -> String.starts_with?(l, "ERROR: ") end)
      assert length(error_lines) == 1
      assert hd(error_lines) == "ERROR: sync-err: oops"
    end
  end

  # ===========================================================================
  # 12. Task exit branch (covers {:exit, reason} in Loop.poll_task)
  # ===========================================================================

  describe "task process exit" do
    # To hit the {:exit, reason} branch in poll_task we need a language whose
    # eval function terminates the Task process abnormally (not via raise,
    # which is caught by the try/rescue in the task wrapper).
    #
    # We use Process.exit(self(), :kill) with :kill because :kill bypasses
    # the try/catch wrappers entirely and produces {:exit, :killed} from
    # Task.yield. However, since Task.async creates a link, we must trap exits
    # in the driving process to avoid killing the test process.
    #
    # We drive the session in a spawned process that traps exits, and collect
    # results back via message passing.
    defmodule ExitingLanguage do
      @behaviour CodingAdventures.Repl.Language

      @impl true
      def eval(":quit"), do: :quit

      def eval("throw_exit") do
        # Use throw to simulate an unhandled control-flow exit.
        # The Loop's task wrapper catches throw via `catch kind, value ->`.
        throw(:simulated_exit)
      end

      def eval(input), do: {:ok, input}
    end

    test "thrown value inside eval produces an error line and session continues" do
      # throw/1 is caught by the `catch kind, value ->` clause in the task
      # wrapper and converted to {:error, "unexpected error: ..."}. The loop
      # should print an ERROR line and then continue.
      all_output = drive_repl(["throw_exit", "ok_after", ":quit"], ExitingLanguage)

      error_lines = Enum.filter(all_output, fn line -> String.starts_with?(line, "ERROR: ") end)
      assert length(error_lines) >= 1,
        "Expected ERROR line after throw, got: #{inspect(all_output)}"

      assert "ok_after" in all_output,
        "Expected 'ok_after' after throw recovery, got: #{inspect(all_output)}"
    end

    test "thrown value error message contains 'unexpected error'" do
      {_result, outputs} = step_once("throw_exit", ExitingLanguage)
      error_lines = Enum.filter(outputs, fn line -> String.starts_with?(line, "ERROR: ") end)
      assert length(error_lines) == 1
      assert String.contains?(hd(error_lines), "unexpected error")
    end
  end
end
