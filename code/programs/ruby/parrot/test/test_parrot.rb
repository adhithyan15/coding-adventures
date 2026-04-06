# frozen_string_literal: true

# ============================================================================
# test_parrot.rb — test suite for the Parrot REPL program
# ============================================================================
#
# ## Testing strategy
#
# All tests use I/O injection. `CodingAdventures::Repl.run_with_io` accepts:
#
#   input_fn  — a Proc that returns the next line (String) or nil (EOF)
#   output_fn — a Proc that receives each output string
#
# This means every test runs the full REPL loop without touching $stdin or
# $stdout. Tests are deterministic, fast, and require no TTY.
#
# ## Helper: run_parrot
#
# `run_parrot(inputs, mode:)` drives the REPL with a pre-canned array of
# inputs and collects all output strings. Tests then assert on the collected
# output array.
#
# ## What we test
#
#  1.  Basic echo — input is echoed back
#  2.  Quit command — ":quit" ends the session
#  3.  Multiple inputs — all echoed in order
#  4.  Sync mode — :sync behaves the same as :async
#  5.  Async mode — :async (default) works correctly
#  6.  Banner contains "Parrot" — correct welcome text
#  7.  Line prompt contains parrot emoji — correct prompt character
#  8.  EOF (nil) exits gracefully — no crash on nil input
#  9.  Empty string echoed — blank input is not dropped
# 10.  Session ends on :quit with queued inputs remaining
# 11.  Parrot::Prompt#global_prompt content — direct unit test
# 12.  Parrot::Prompt#line_prompt format — direct unit test
# 13.  Multiple echoes accumulate before quit
# 14.  Output collected correctly — exact count of output calls
# 15.  Goodbye message / session end on :quit — loop does not continue

require "test_helper"
require "parrot"
require "coding_adventures_repl"

class TestParrot < Minitest::Test
  # --------------------------------------------------------------------------
  # Helper: run_parrot
  # --------------------------------------------------------------------------

  # run_parrot — drive the Parrot REPL with canned inputs and collect output.
  #
  # @param inputs [Array<String, nil>] lines to feed into the REPL.
  #   A nil element (or an exhausted array) signals EOF to the loop.
  # @param mode   [:async, :sync] evaluation strategy (default: :async)
  # @return [Array<String>] all strings passed to output_fn during the session.
  #
  # ## How the input queue works
  #
  # We dup the inputs array and shift elements one at a time. `Array#shift`
  # returns nil when the array is empty, which the loop treats as EOF (same
  # as the user pressing Ctrl-D). An explicit nil element also causes the
  # loop to stop.
  def run_parrot(inputs, mode: :async)
    output = []
    queue  = inputs.dup

    CodingAdventures::Repl.run_with_io(
      language:  CodingAdventures::Repl::EchoLanguage.new,
      prompt:    Parrot::Prompt.new,
      waiting:   CodingAdventures::Repl::SilentWaiting.new,
      input_fn:  -> { queue.shift },
      output_fn: ->(text) { output << text },
      mode:      mode
    )

    output
  end

  # --------------------------------------------------------------------------
  # Test 1: basic echo
  # --------------------------------------------------------------------------

  # The most fundamental behaviour: whatever the user types is echoed back.
  def test_echoes_basic_input
    out = run_parrot(["hello", ":quit"])

    # The output array contains prompt strings AND the echoed input.
    # We check that "hello" is present somewhere.
    assert_includes out, "hello"
  end

  # --------------------------------------------------------------------------
  # Test 2: quit ends the session
  # --------------------------------------------------------------------------

  # Typing ":quit" should end the loop without processing further input.
  def test_quit_ends_session
    out = run_parrot([":quit"])

    # The session ended cleanly (no exception). Verify "hello" was never echoed
    # since we did not provide it as input.
    refute_includes out, "hello"
  end

  # --------------------------------------------------------------------------
  # Test 3: multiple inputs echoed in order
  # --------------------------------------------------------------------------

  # Each input line should appear in the output in the same order it was typed.
  def test_multiple_inputs_echoed_in_order
    out = run_parrot(["alpha", "beta", "gamma", ":quit"])

    joined   = out.join
    alpha_i  = joined.index("alpha")
    beta_i   = joined.index("beta")
    gamma_i  = joined.index("gamma")

    # All three must be present.
    refute_nil alpha_i, "expected 'alpha' in output"
    refute_nil beta_i,  "expected 'beta' in output"
    refute_nil gamma_i, "expected 'gamma' in output"

    # They must appear in the order typed.
    assert alpha_i < beta_i,  "alpha must appear before beta"
    assert beta_i  < gamma_i, "beta must appear before gamma"
  end

  # --------------------------------------------------------------------------
  # Test 4: sync mode
  # --------------------------------------------------------------------------

  # :sync mode bypasses the Thread-based eval and calls the language directly.
  # The output should be identical to :async mode.
  def test_sync_mode_echoes_correctly
    out = run_parrot(["sync-input", ":quit"], mode: :sync)

    assert_includes out, "sync-input"
  end

  # --------------------------------------------------------------------------
  # Test 5: async mode
  # --------------------------------------------------------------------------

  # :async (the default) spawns a background Thread for each eval call.
  # The Parrot REPL uses this mode because EchoLanguage is fast.
  def test_async_mode_echoes_correctly
    out = run_parrot(["async-input", ":quit"], mode: :async)

    assert_includes out, "async-input"
  end

  # --------------------------------------------------------------------------
  # Test 6: banner contains "Parrot"
  # --------------------------------------------------------------------------

  # The global_prompt is printed before every input line. We check that the
  # word "Parrot" appears in the combined output.
  def test_banner_contains_parrot
    out = run_parrot([":quit"])

    assert_includes out.join, "Parrot"
  end

  # --------------------------------------------------------------------------
  # Test 7: line prompt contains parrot emoji
  # --------------------------------------------------------------------------

  # Direct unit test of Parrot::Prompt#line_prompt — no REPL loop needed.
  def test_line_prompt_contains_parrot_emoji
    prompt = Parrot::Prompt.new

    assert_includes prompt.line_prompt, "🦜"
  end

  # --------------------------------------------------------------------------
  # Test 8: EOF (nil input) exits gracefully
  # --------------------------------------------------------------------------

  # When input_fn returns nil, the loop should exit without raising an error.
  def test_eof_exits_gracefully
    # Passing only nil simulates the user pressing Ctrl-D immediately.
    out = run_parrot([nil])

    # The loop should have printed the prompt once before receiving nil.
    assert_includes out.join, "Parrot"
  end

  # --------------------------------------------------------------------------
  # Test 9: empty string is echoed
  # --------------------------------------------------------------------------

  # EchoLanguage returns [:ok, ""] for empty input. The loop should print "".
  # This verifies that blank lines are not silently dropped.
  def test_empty_string_is_echoed
    out = run_parrot(["", ":quit"])

    # The output array should contain an empty string (the echoed blank line).
    assert_includes out, ""
  end

  # --------------------------------------------------------------------------
  # Test 10: session ends on :quit with queued inputs remaining
  # --------------------------------------------------------------------------

  # Once ":quit" is processed, the loop must stop — it must NOT process
  # the "after-quit" input that follows in the queue.
  def test_quit_stops_processing_queued_inputs
    out = run_parrot([":quit", "after-quit"])

    refute_includes out, "after-quit"
  end

  # --------------------------------------------------------------------------
  # Test 11: Parrot::Prompt#global_prompt content
  # --------------------------------------------------------------------------

  # Direct unit test — verify the exact components of the banner string.
  def test_global_prompt_contains_expected_text
    prompt = Parrot::Prompt.new
    text   = prompt.global_prompt

    assert_includes text, "🦜"
    assert_includes text, "Parrot REPL"
    assert_includes text, ":quit"
  end

  # --------------------------------------------------------------------------
  # Test 12: Parrot::Prompt#line_prompt format
  # --------------------------------------------------------------------------

  # The line prompt should be non-empty, contain ">" as a visual separator,
  # and contain the parrot emoji.
  def test_line_prompt_format
    prompt = Parrot::Prompt.new
    text   = prompt.line_prompt

    refute_empty text
    assert_includes text, ">"
    assert_includes text, "🦜"
  end

  # --------------------------------------------------------------------------
  # Test 13: multiple echoes accumulate before quit
  # --------------------------------------------------------------------------

  # Three lines of input should each produce an echoed output entry.
  def test_multiple_echoes_accumulate
    out = run_parrot(["one", "two", "three", ":quit"])

    assert_includes out, "one"
    assert_includes out, "two"
    assert_includes out, "three"
  end

  # --------------------------------------------------------------------------
  # Test 14: output collected correctly — exact output call count
  # --------------------------------------------------------------------------

  # With "ping" and ":quit":
  #   - global_prompt before "ping"  → 1 output call
  #   - "ping" echoed                → 1 output call
  #   - global_prompt before ":quit" → 1 output call
  #   - ":quit" produces no output   → loop exits
  # Total: 3 output calls.
  def test_output_call_count_is_correct
    out = run_parrot(["ping", ":quit"], mode: :sync)

    assert_equal 3, out.length, "expected exactly 3 output calls"

    # The first and third outputs are the global prompt (banner).
    assert_includes out[0], "Parrot"
    # The second output is the echoed input.
    assert_equal "ping", out[1]
    # The third output is the second banner (before ":quit").
    assert_includes out[2], "Parrot"
  end

  # --------------------------------------------------------------------------
  # Test 15: loop does not continue after :quit
  # --------------------------------------------------------------------------

  # After ":quit" is received, the loop must break. We verify this by checking
  # that no additional prompt was printed after the quit line was consumed.
  # With inputs [":quit"], we expect exactly 1 output call (the first banner).
  def test_loop_ends_after_quit
    out = run_parrot([":quit"])

    # Only one output: the prompt printed before ":quit" was read.
    # The loop exits immediately on :quit without printing another prompt.
    assert_equal 1, out.length, "expected only the pre-quit prompt to be printed"
    assert_includes out[0], "Parrot"
  end
end
