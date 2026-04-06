# frozen_string_literal: true

# ============================================================================
# Loop — the core Read-Eval-Print loop implementation
# ============================================================================
#
# This is the heart of the REPL framework. The loop ties together all three
# pluggable interfaces:
#
#   Language  — evaluates user input (possibly slow; run on a background thread)
#   Prompt    — generates the ">" and "..." strings shown before the cursor
#   Waiting   — provides feedback while the evaluator is running asynchronously
#
# ## The REPL cycle (one iteration)
#
#   1. PRINT the prompt (global_prompt from the Prompt interface)
#   2. READ one line of input from input_fn
#   3. EVAL the input asynchronously on a new Thread
#      - The main thread polls with thread.join(tick_ms / 1000.0)
#      - While waiting, tick the Waiting interface each poll cycle
#   4. When the thread finishes:
#      - Stop the Waiting interface
#      - Handle the result: print output, print error, or exit the loop
#
# ## Async eval — why a thread per eval?
#
# Ruby's `Thread.new { ... }` gives us preemptive concurrency within a single
# process. The language backend might be slow (compile + run, network calls,
# long computation). Running eval on a background thread means:
#   - The main thread stays in control of I/O and animation
#   - We can poll for completion and drive the Waiting animation in lockstep
#   - Eventually, we could add timeout support (kill the thread after N seconds)
#
# The poll loop looks like this:
#
#   thread = Thread.new { language.eval(input) }
#   state  = waiting.start
#   until (result = thread.join(sleep_sec))
#     state = waiting.tick(state)
#   end
#   waiting.stop(state)
#   # result is the thread itself; fetch its return value:
#   outcome = thread.value
#
# `thread.join(timeout)` returns `nil` if the timeout expires before the
# thread finishes, or returns the thread itself if the thread completed.
# So `until thread.join(0.1)` means "keep looping while the thread is still
# running."
#
# ## Exception safety in the eval thread
#
# If the language backend raises an unhandled exception, we must not let it
# propagate silently or crash the REPL. The thread wraps the eval call in a
# begin/rescue block. If an exception occurs:
#   - The error message is returned as `[:error, e.message]`
#   - The REPL continues running (the user can try again)
#
# This mirrors how IRB handles exceptions: they're caught and printed, but the
# session lives on.
#
# ## I/O injection
#
# Rather than calling `$stdin.gets` and `$stdout.print` directly, the loop
# accepts:
#   input_fn  — a Proc that returns the next line of input (String), or nil
#               to signal EOF / quit (same effect as returning :quit from eval)
#   output_fn — a Proc that accepts a String and displays it
#
# I/O injection is essential for testing: tests can pass in arrays of
# pre-canned inputs and capture outputs without touching the terminal.

module CodingAdventures
  module Repl
    # Loop encapsulates the Read-Eval-Print cycle.
    #
    # Construct with all required collaborators, then call `run` to start
    # the interactive session.
    #
    # Two evaluation modes are supported via the `mode:` keyword:
    #
    #   :async (default) — the language backend runs on a background Thread.
    #     The main thread polls with thread.join(tick_ms / 1000.0) and drives
    #     the Waiting animation each poll cycle. Requires a non-nil `waiting`.
    #
    #   :sync — the language backend is called directly in a begin/rescue on
    #     the current thread. No Thread is spawned, the Waiting interface is
    #     never touched, and `waiting` may be nil. Use this for batch/scripted
    #     evaluation or in contexts where threads are undesirable.
    class Loop
      # @param language  [#eval]          the language backend
      # @param prompt    [#global_prompt] the prompt generator
      # @param waiting   [#start,#tick,#tick_ms,#stop] the waiting strategy
      #                  (may be nil when mode: :sync)
      # @param input_fn  [Proc] called with no args; returns String or nil
      # @param output_fn [Proc] called with a String to display
      # @param mode      [:async, :sync]  evaluation strategy (default: :async)
      def initialize(language:, prompt:, waiting:, input_fn:, output_fn:, mode: :async)
        @language  = language
        @prompt    = prompt
        @waiting   = waiting
        @input_fn  = input_fn
        @output_fn = output_fn
        @mode      = mode
      end

      # Run the REPL loop until the user quits or input_fn returns nil.
      #
      # This method blocks the calling thread until the session ends.
      #
      # @return [nil]
      def run
        loop do
          # ── PRINT the prompt ───────────────────────────────────────────────
          # We use global_prompt here; a multi-line language could switch to
          # line_prompt when it detects an incomplete expression, but that is
          # left as an extension point for subclasses or wrappers.
          @output_fn.call(@prompt.global_prompt)

          # ── READ ───────────────────────────────────────────────────────────
          # Call the input function. If it returns nil, treat as EOF/quit —
          # the user closed the input stream (Ctrl-D) or the input array ran
          # out in tests.
          input = @input_fn.call
          break if input.nil?

          # Strip the trailing newline that `gets` includes. Input functions
          # in tests may or may not include it, so we chomp defensively.
          input = input.chomp

          # ── EVAL ───────────────────────────────────────────────────────────
          # Route to the appropriate evaluation strategy.
          outcome = if @mode == :sync
            eval_sync(input)
          else
            eval_async(input)
          end

          # ── PRINT / handle the outcome ────────────────────────────────────
          case outcome
          in :quit
            # The language backend (or EchoLanguage on ":quit") asked us to
            # end the session. Break out of the outer loop.
            break
          in [:ok, nil]
            # Successful eval with no output (e.g., an assignment expression).
            # Print nothing — this mirrors IRB's behaviour for side-effect-only
            # expressions.
            nil
          in [:ok, output]
            # Successful eval with output. Print the result string.
            @output_fn.call(output)
          in [:error, message]
            # The eval failed (either the language returned [:error, ...] or
            # we rescued an exception). Print the error and continue — the
            # session is still alive.
            @output_fn.call("Error: #{message}")
          end
        end

        nil
      end

      private

      # Evaluate in sync mode: call language.eval directly on the current
      # thread, inside a begin/rescue. No Thread is spawned, and the Waiting
      # interface is completely bypassed.
      #
      # This is simpler and has less overhead than async mode. It is suitable
      # for scripted evaluation, testing, or any context where the eval is
      # known to be fast or threading is undesirable.
      #
      # @param input [String] the chomped user input
      # @return [:quit, Array] the eval outcome
      def eval_sync(input)
        begin
          @language.eval(input)
        rescue => e
          # Convert any unexpected exception into an error result, just as
          # the async path does. The REPL session continues.
          [:error, e.message]
        end
      end

      # Evaluate in async mode: spawn a Thread for the eval call and drive
      # the Waiting animation while polling for completion.
      #
      # Spawning the eval on a background thread keeps the main thread free
      # to update the spinner / progress indicator. This is the original
      # behaviour and remains the default.
      #
      # Exception safety: the thread body wraps eval in begin/rescue so that
      # any unhandled exception from the language backend is captured and
      # converted into an [:error, message] result rather than being silently
      # swallowed (Ruby's default for threads without abort_on_exception).
      #
      # @param input [String] the chomped user input
      # @return [:quit, Array] the eval outcome
      def eval_async(input)
        # How long to sleep between polls, in seconds.
        sleep_sec = @waiting.tick_ms / 1000.0

        # Spawn a new thread for the eval call. This keeps the main thread
        # free to drive the Waiting animation.
        thread = Thread.new do
          begin
            @language.eval(input)
          rescue => e
            # Convert any unexpected exception into an error result.
            # The REPL session continues — the user is informed and can retry.
            [:error, e.message]
          end
        end

        # ── WAIT (poll loop) ─────────────────────────────────────────────────
        # Drive the Waiting animation while the eval thread is running.
        # `thread.join(sleep_sec)` returns nil if the thread is still alive,
        # or returns the thread object if it has finished.
        state = @waiting.start
        state = @waiting.tick(state) until thread.join(sleep_sec)

        # ── STOP waiting ────────────────────────────────────────────────────
        # The eval thread is done. Let the Waiting impl clean up (e.g.,
        # erase a spinner line from the terminal).
        @waiting.stop(state)

        # `thread.value` returns the last expression from the thread block —
        # our `[:ok, x]`, `[:error, msg]`, or `:quit`.
        thread.value
      end
    end
  end
end
