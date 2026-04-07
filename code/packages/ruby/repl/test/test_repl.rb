# frozen_string_literal: true

# ============================================================================
# Tests for the REPL framework
# ============================================================================
#
# All tests use I/O injection (`run_with_io`) so they never touch stdin/stdout.
# The pattern is:
#
#   inputs  = ["line1", "line2", nil]   — nil signals EOF / end of input
#   outputs = []
#   input_fn  = -> { inputs.shift }
#   output_fn = ->(s) { outputs << s }
#   Repl.run_with_io(language: ..., prompt: ..., waiting: ...,
#                     input_fn: input_fn, output_fn: output_fn)
#   assert_includes outputs, "expected string"
#
# Each test constructs its own input/output pair so tests are independent.

require_relative "test_helper"

# Bring the module into scope so test names read cleanly.
include CodingAdventures

class TestRepl < Minitest::Test
  # ── helpers ──────────────────────────────────────────────────────────────

  # Build a fresh run context with the default built-in collaborators.
  # Returns [outputs_array, run_proc] where run_proc accepts an inputs array.
  def run_repl(inputs, language: Repl::EchoLanguage.new)
    outputs = []
    input_fn  = -> { inputs.shift }
    output_fn = ->(s) { outputs << s }

    Repl.run_with_io(
      language:  language,
      prompt:    Repl::DefaultPrompt.new,
      waiting:   Repl::SilentWaiting.new,
      input_fn:  input_fn,
      output_fn: output_fn
    )

    outputs
  end

  # ── Test 1: EchoLanguage echoes a single line ────────────────────────────
  #
  # The simplest possible session: type "hello", see "hello" back, then quit.
  #
  # Expected output sequence:
  #   "> "          ← prompt for "hello"
  #   "hello"       ← echo result
  #   "> "          ← prompt for ":quit"
  #   (loop exits)
  def test_echo_language_echoes_input
    outputs = run_repl(["hello", ":quit"])

    # The echo should appear somewhere in the outputs.
    assert_includes outputs, "hello",
      "EchoLanguage should echo 'hello' back to the output function"
  end

  # ── Test 2: :quit terminates the loop ────────────────────────────────────
  #
  # After ":quit" is sent, the loop must stop. We verify this by checking that
  # subsequent inputs (if any) are never consumed — the loop has exited.
  #
  # We also verify that no "quit" string appears in the output: the quit
  # signal should terminate silently, not print a goodbye message.
  def test_quit_terminates_loop
    # "after_quit" should never be eval'd because the loop exits on ":quit".
    outputs = run_repl([":quit", "after_quit"])

    refute_includes outputs, "after_quit",
      "After :quit the loop must terminate; subsequent inputs must not be evaluated"
  end

  # ── Test 3: Multiple turns work correctly ────────────────────────────────
  #
  # Verify that the loop cycles correctly across multiple rounds of input,
  # echoing each line in order before quitting.
  def test_multiple_turns
    outputs = run_repl(["foo", "bar", "baz", ":quit"])

    assert_includes outputs, "foo", "First input 'foo' should appear in output"
    assert_includes outputs, "bar", "Second input 'bar' should appear in output"
    assert_includes outputs, "baz", "Third input 'baz' should appear in output"
  end

  # ── Test 4: nil output from eval prints nothing ──────────────────────────
  #
  # When the language returns [:ok, nil], the loop should print nothing for
  # that turn (only the prompt). This mirrors IRB's behaviour for assignments.
  #
  # We use a custom language that always returns [:ok, nil].
  def test_nil_output_prints_nothing
    # A language that signals success but has no output to show.
    silent_language = Object.new
    def silent_language.eval(_input)
      [:ok, nil]
    end

    # Two turns: first returns nil, then we send EOF via nil input.
    outputs = run_repl(["anything", nil], language: silent_language)

    # Only the prompts should appear. No echo of the input, no result string.
    # Prompts are "> " strings. The loop emits one prompt before each input_fn
    # call, so with inputs ["anything", nil] we get exactly two prompts:
    #   - "> " before reading "anything" (eval returns [:ok, nil], no output)
    #   - "> " before reading nil (loop then breaks)
    prompt_outputs = outputs.select { |o| o == "> " }
    assert_equal 2, prompt_outputs.size,
      "Two prompts should appear: one before 'anything', one before the nil EOF"

    # No non-prompt output should appear (nothing from the silent language).
    non_prompt = outputs.reject { |o| o == "> " }
    assert_empty non_prompt,
      "No output other than the prompt should appear when language returns [:ok, nil]"
  end

  # ── Test 5: Language returning [:error, msg] prints error prefix ─────────
  #
  # When the language backend signals a failure via [:error, "some message"],
  # the loop should display "Error: some message" (the prefix is added by Loop).
  def test_error_result_prints_error_message
    error_language = Object.new
    def error_language.eval(_input)
      [:error, "syntax error on line 1"]
    end

    outputs = run_repl(["bad input", nil], language: error_language)

    assert_includes outputs, "Error: syntax error on line 1",
      "Error results should be printed with the 'Error: ' prefix"
  end

  # ── Test 6: Exception safety — unhandled exception in eval ───────────────
  #
  # If the language backend raises an unhandled exception (a bug in the
  # backend, not an intentional error return), the loop must:
  #   1. NOT crash — the session should survive
  #   2. Print an error message containing the exception message
  #
  # This verifies the begin/rescue wrapper in Loop around the eval call.
  def test_exception_in_eval_is_caught_and_reported
    explosive_language = Object.new
    def explosive_language.eval(_input)
      raise RuntimeError, "kaboom"
    end

    # Give one input that will trigger the exception, then EOF.
    outputs = run_repl(["trigger", nil], language: explosive_language)

    # The loop should have caught the exception and printed an error.
    error_output = outputs.find { |o| o.start_with?("Error:") }
    refute_nil error_output,
      "An exception in eval must produce an Error: output line"
    assert_includes error_output, "kaboom",
      "The error output must include the exception message"
  end

  # ── Additional: Prompt strings appear in output ──────────────────────────
  #
  # Verifies that DefaultPrompt's global_prompt "> " is written to the output
  # function before each input is requested.
  def test_prompt_appears_before_each_input
    outputs = run_repl(["a", "b", ":quit"])

    # There should be three prompts: one before "a", one before "b", one
    # before ":quit".
    prompts = outputs.select { |o| o == "> " }
    assert_equal 3, prompts.size,
      "A prompt should be emitted before each input read (3 inputs → 3 prompts)"
  end

  # ── Additional: nil from input_fn acts as EOF / quit ─────────────────────
  #
  # When input_fn returns nil (simulating Ctrl-D or end of test input),
  # the loop should exit cleanly without printing an error.
  def test_nil_input_fn_causes_clean_exit
    outputs = run_repl([nil])

    # No error output should appear — nil input is a clean termination signal.
    error_output = outputs.find { |o| o.start_with?("Error:") }
    assert_nil error_output,
      "nil from input_fn should cause a clean exit, not an error message"
  end

  # ── Unit tests for built-in classes ──────────────────────────────────────

  def test_echo_language_returns_ok_for_regular_input
    lang = Repl::EchoLanguage.new
    assert_equal [:ok, "hello world"], lang.eval("hello world")
  end

  def test_echo_language_returns_quit_for_quit_command
    lang = Repl::EchoLanguage.new
    assert_equal :quit, lang.eval(":quit")
  end

  def test_default_prompt_global_prompt
    prompt = Repl::DefaultPrompt.new
    assert_equal "> ", prompt.global_prompt
  end

  def test_default_prompt_line_prompt
    prompt = Repl::DefaultPrompt.new
    assert_equal "... ", prompt.line_prompt
  end

  def test_silent_waiting_start_returns_nil
    w = Repl::SilentWaiting.new
    assert_nil w.start
  end

  def test_silent_waiting_tick_returns_nil
    w = Repl::SilentWaiting.new
    assert_nil w.tick(nil)
  end

  def test_silent_waiting_tick_ms_is_100
    w = Repl::SilentWaiting.new
    assert_equal 100, w.tick_ms
  end

  def test_silent_waiting_stop_returns_nil
    w = Repl::SilentWaiting.new
    assert_nil w.stop(nil)
  end

  # ── Sync mode tests ──────────────────────────────────────────────────────

  # Helper that runs run_with_io in sync mode.
  # `waiting` defaults to SilentWaiting so existing helpers still work, but
  # the sync-mode tests also verify that nil is accepted.
  def run_repl_sync(inputs, language: Repl::EchoLanguage.new, waiting: Repl::SilentWaiting.new)
    outputs = []
    input_fn  = -> { inputs.shift }
    output_fn = ->(s) { outputs << s }

    Repl.run_with_io(
      language:  language,
      prompt:    Repl::DefaultPrompt.new,
      waiting:   waiting,
      input_fn:  input_fn,
      output_fn: output_fn,
      mode:      :sync
    )

    outputs
  end

  # ── Sync Test 1: test_sync_mode_echo ─────────────────────────────────────
  #
  # In sync mode the language backend is called directly (no Thread). The REPL
  # loop must still echo the input back, show the prompt, and exit on EOF.
  #
  # This mirrors test_echo_language_echoes_input but forces mode: :sync.
  def test_sync_mode_echo
    outputs = run_repl_sync(["hello", ":quit"])

    assert_includes outputs, "hello",
      "Sync mode should echo 'hello' back to the output function"
  end

  # ── Sync Test 2: test_sync_mode_quit ─────────────────────────────────────
  #
  # `:quit` must terminate the loop in sync mode just as in async mode.
  # Any input after `:quit` must never be evaluated.
  def test_sync_mode_quit
    outputs = run_repl_sync([":quit", "after_quit"])

    refute_includes outputs, "after_quit",
      "Sync mode: loop must terminate on :quit; subsequent inputs must not be evaluated"

    refute_includes outputs, ":quit",
      "Sync mode: the :quit command itself must not appear in output"
  end

  # ── Sync Test 3: test_sync_mode_error ────────────────────────────────────
  #
  # When the language returns [:error, msg] in sync mode, the loop must format
  # and print it with the "Error: " prefix — exactly the same as async mode.
  #
  # This exercises the begin/rescue path inside eval_sync, verifying that
  # both intentional error returns and unhandled exceptions are handled.
  def test_sync_mode_error
    # A language that always signals an error.
    error_language = Object.new
    def error_language.eval(_input)
      [:error, "sync error message"]
    end

    outputs = run_repl_sync(["bad input", nil], language: error_language)

    assert_includes outputs, "Error: sync error message",
      "Sync mode should print error results with the 'Error: ' prefix"
  end
end
