defmodule CodingAdventures.Repl.Loop do
  @moduledoc """
  The REPL loop — the engine that drives the read-eval-print cycle.

  ## What Is a REPL?

  REPL stands for **Read-Eval-Print Loop**. The name describes the four
  phases of every interactive programming session:

  1. **Read** — Display a prompt, read a line of input from the user.
  2. **Eval** — Hand that input to the language evaluator.
  3. **Print** — Display the result (or error) back to the user.
  4. **Loop** — Go back to step 1.

  ## Architecture: Three Plugins

  This loop is generic. It knows nothing about any particular language. All
  variable behaviour is injected through three pluggable modules:

  ```
  ┌─────────────────────────────────────────────────────┐
  │                     REPL Loop                       │
  │                                                     │
  │  input_fn ──► read ──► Language.eval ──► output_fn  │
  │                             │                       │
  │                        Task.async                   │
  │                             │                       │
  │                     Waiting plugin                  │
  │                    (tick while waiting)             │
  └─────────────────────────────────────────────────────┘
  ```

  - **Language** — maps input strings to results.
  - **Prompt** — controls what text appears before the cursor.
  - **Waiting** — animates while the evaluator is running.

  ## Async Evaluation with Task.async

  Evaluation happens asynchronously for two reasons:

  1. **Responsiveness** — If eval takes a long time (e.g. a network call,
     a slow computation), the main process can continue updating the
     waiting animation instead of freezing.

  2. **Isolation** — The eval runs in a separate process. If it crashes,
     the loop catches the failure and prints an error instead of dying.

  The pattern used here is:

      task = Task.async(fn -> Language.eval(input) end)

      result = poll_task(task, waiting, state)

  `poll_task/3` calls `Task.yield(task, tick_ms)` in a loop. Each yield
  either returns `{:ok, result}` (task done), `nil` (still running, tick),
  or we handle the task shutdown on quit.

  ## Error Handling

  Three failure modes are handled:

  1. **`{:error, msg}`** — The language returned an error. Print "ERROR: msg".
  2. **Exception in eval** — The task crashes. We catch it and print
     "ERROR: unexpected error: <reason>".
  3. **nil input (EOF)** — The input_fn returned nil (pipe closed or list
     exhausted). Treated as :quit.

  ## I/O Injection

  `input_fn` and `output_fn` are passed in rather than calling `IO.gets/1`
  and `IO.puts/1` directly. This decoupling is what makes the loop
  fully testable: in tests we pass a function over a pre-loaded list
  and capture output in a list too, without touching the real terminal.
  """

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Run the REPL loop until the language returns :quit or input is exhausted.

  ## Parameters

  - `language` — module implementing the Language behaviour.
  - `prompt` — module implementing the Prompt behaviour.
  - `waiting` — module implementing the Waiting behaviour.
  - `input_fn` — `(String.t() -> String.t() | nil)` called to read input.
    Receives the prompt string. Returns nil on EOF.
  - `output_fn` — `(String.t() -> :ok)` called to display output.
  - `opts` — keyword list of options:
    - `:mode` — `:async` (default) or `:sync`.
      In `:async` mode evaluation runs in a `Task` with waiting-plugin ticks.
      In `:sync` mode evaluation runs directly on the calling process (no
      Task, no waiting plugin), which simplifies testing and embedding.

  ## Returns

  `:ok` when the session ends.
  """
  def run(language, prompt, waiting, input_fn, output_fn, opts \\ []) do
    loop(language, prompt, waiting, input_fn, output_fn, opts)
  end

  @doc """
  Execute a single REPL step: show prompt, read input, eval, print result.

  Returns:
  - `{:continue, output}` — keep looping; `output` is what was printed
    (may be nil if nothing was printed).
  - `{:quit, nil}` — session should end.

  This function is exposed for testing individual steps in isolation without
  running the full loop. The full loop simply calls this repeatedly.

  ## Parameters

  - `language`, `prompt`, `waiting` — same as `run/6`.
  - `input` — the already-read input string (loop calls input_fn itself;
    step takes pre-read input so tests can provide it directly).
  - `input_fn` — passed through to sub-calls (not read here, but kept for
    API symmetry with loop).
  - `output_fn` — called with any output to display.
  - `opts` — same keyword list as `run/6` (`:mode` option).
  """
  def step(language, prompt, waiting, input, _input_fn, output_fn, opts \\ []) do
    do_step(language, prompt, waiting, input, output_fn, opts)
  end

  # ---------------------------------------------------------------------------
  # Main loop
  # ---------------------------------------------------------------------------

  # The loop is a simple tail-recursive function. Each iteration:
  # 1. Show the global prompt.
  # 2. Read a line of input.
  # 3. Handle nil (EOF → quit) or strip newline and eval.
  # 4. Check result: :quit → stop, {:continue, _} → recurse.
  defp loop(language, prompt, waiting, input_fn, output_fn, opts) do
    # Show the global prompt via the output_fn. We use IO.write-style (no
    # trailing newline) so the user's cursor is on the same line.
    output_fn.(prompt.global_prompt())

    # Read input. The input_fn receives the prompt string (some
    # implementations may use it, others ignore it).
    raw = input_fn.(prompt.global_prompt())

    case raw do
      nil ->
        # EOF — input stream exhausted (pipe closed, list empty). Treat
        # this exactly like :quit to avoid infinite loops.
        :ok

      :eof ->
        # IO.gets/1 returns the atom :eof when the underlying device signals
        # end-of-file (e.g. stdin closed via pipe, or ExUnit.CaptureIO given
        # an empty string). Treat identically to nil — end the session.
        :ok

      line ->
        # Strip the trailing newline that IO.gets includes. Without this,
        # every input string would end with "\n", which would propagate into
        # the language evaluator.
        input = String.trim_trailing(line, "\n")

        case do_step(language, prompt, waiting, input, output_fn, opts) do
          {:quit, _} -> :ok
          {:continue, _} -> loop(language, prompt, waiting, input_fn, output_fn, opts)
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Single step
  # ---------------------------------------------------------------------------

  # do_step/6 performs the Eval-Print part of one REPL cycle.
  #
  # The opts keyword list controls execution mode:
  #
  #   :async (default) — evaluation runs in a Task.async so the waiting
  #     plugin can animate while the language is busy.
  #
  #   :sync — evaluation runs directly on the calling process.  No Task,
  #     no waiting plugin calls.  Simpler and useful for tests and
  #     deterministic embeddings where async overhead is undesirable.
  defp do_step(language, _prompt, waiting, input, output_fn, opts) do
    mode = Keyword.get(opts, :mode, :async)

    result =
      case mode do
        :sync ->
          # Sync mode: call eval directly, no Task, no waiting plugin.
          # Exceptions are still caught so the loop survives a buggy language.
          try do
            language.eval(input)
          rescue
            e -> {:error, "unexpected error: #{Exception.message(e)}"}
          catch
            kind, value ->
              {:error, "unexpected error: #{inspect({kind, value})}"}
          end

        _ ->
          # Async mode (default): start the waiting animation, spawn a Task,
          # poll it while ticking the waiting plugin, then stop the animation.
          #
          # Start the waiting animation. The state is opaque to us; we just
          # pass it through to tick/1 and stop/1.
          wait_state = waiting.start()

          # Spawn the eval task. We wrap the language call in a try/rescue so
          # that an exception inside eval is caught and converted to an error
          # tuple rather than crashing the task (and thus the whole loop).
          task =
            Task.async(fn ->
              try do
                language.eval(input)
              rescue
                e -> {:error, "unexpected error: #{Exception.message(e)}"}
              catch
                # Catch throws too — some languages use throw for control flow.
                kind, value ->
                  {:error, "unexpected error: #{inspect({kind, value})}"}
              end
            end)

          # Poll until the task finishes, ticking the waiting plugin as we go.
          {async_result, final_wait_state} = poll_task(task, waiting, wait_state)

          # Always stop the waiting plugin, even on :quit or error, so it can
          # clean up any visual state (erase spinners, restore cursor, etc.).
          waiting.stop(final_wait_state)

          async_result
      end

    # Interpret the result and decide what to print and whether to continue.
    handle_result(result, output_fn)
  end

  # ---------------------------------------------------------------------------
  # Task polling with waiting ticks
  # ---------------------------------------------------------------------------

  # poll_task/3 drives the Task.yield loop.
  #
  # Task.yield(task, timeout) returns:
  #   {:ok, value} — the task finished with `value`
  #   {:exit, reason} — the task crashed with `reason`
  #   nil — timeout elapsed without a result; task is still running
  #
  # We use the timeout as our tick interval. On nil, we call waiting.tick/1
  # to advance the animation, then try again.
  defp poll_task(task, waiting, wait_state) do
    interval = waiting.tick_ms()

    case Task.yield(task, interval) do
      {:ok, result} ->
        # Task completed normally. Return result and current wait state.
        {result, wait_state}

      {:exit, reason} ->
        # Task process crashed. Convert to an error tuple so the caller
        # can display a message rather than propagating the crash.
        {{:error, "unexpected error: #{inspect(reason)}"}, wait_state}

      nil ->
        # Still running. Advance the waiting animation and poll again.
        new_wait_state = waiting.tick(wait_state)
        poll_task(task, waiting, new_wait_state)
    end
  end

  # ---------------------------------------------------------------------------
  # Result interpretation
  # ---------------------------------------------------------------------------

  # handle_result/2 translates the language's return value into:
  # - text to display (via output_fn), and
  # - a control signal ({:continue, _} or {:quit, nil}).
  #
  # This is the Print step of Read-Eval-Print-Loop.
  defp handle_result(:quit, _output_fn) do
    # The language signalled end-of-session. Do not print anything.
    {:quit, nil}
  end

  defp handle_result({:ok, nil}, _output_fn) do
    # Successful evaluation with no displayable value.
    # This is common for assignment statements, void functions, and side
    # effects. We silently continue. (Python's `x = 5` produces no output.)
    {:continue, nil}
  end

  defp handle_result({:ok, value}, output_fn) do
    # Successful evaluation. Print the value on its own line.
    output_fn.(value)
    {:continue, value}
  end

  defp handle_result({:error, message}, output_fn) do
    # Evaluation error. Print a prefixed error message.
    # The "ERROR: " prefix makes errors visually distinct from normal output
    # so the user can immediately identify something went wrong.
    output_fn.("ERROR: #{message}")
    {:continue, nil}
  end
end
