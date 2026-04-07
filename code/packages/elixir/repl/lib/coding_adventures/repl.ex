defmodule CodingAdventures.Repl do
  @moduledoc """
  REPL Framework — a pluggable Read-Eval-Print Loop for any language.

  ## Overview

  This module is the public façade for the REPL framework. It provides three
  entry points:

  1. `run/4` — start an interactive session using the real terminal.
  2. `run_with_io/5` — start a session with injected I/O (for testing).
  3. `step/6` — execute a single REPL iteration (for fine-grained testing).

  ## The Three Plugins

  A REPL session is configured by three pluggable modules, each implementing
  a behaviour:

  | Plugin   | Behaviour                         | Controls                      |
  |----------|-----------------------------------|-------------------------------|
  | Language | `CodingAdventures.Repl.Language`  | How input is evaluated        |
  | Prompt   | `CodingAdventures.Repl.Prompt`    | What text appears as a prompt |
  | Waiting  | `CodingAdventures.Repl.Waiting`   | What happens during eval      |

  ## Quick Start

  ```elixir
  # An interactive echo REPL on the real terminal:
  alias CodingAdventures.Repl
  alias CodingAdventures.Repl.{EchoLanguage, DefaultPrompt, SilentWaiting}

  Repl.run(EchoLanguage, DefaultPrompt, SilentWaiting)
  # > hello
  # hello
  # > :quit
  # (session ends)
  ```

  ## Testing with Injected I/O

  ```elixir
  # Capture output without touching the terminal:
  outputs = []
  inputs = ["hello", "world", ":quit"]

  input_agent = spawn_link(fn -> input_agent_loop(inputs) end)
  input_fn = fn _prompt -> send(input_agent, {:get, self()}); receive do {:line, l} -> l end end
  output_fn = fn line -> send(self(), {:output, line}) end

  Repl.run_with_io(EchoLanguage, DefaultPrompt, SilentWaiting, input_fn, output_fn)
  ```

  Or use the convenience helper that handles all the plumbing (see tests).

  ## Design Goals

  1. **No assumptions about the language** — The loop never inspects the
     semantics of what it evaluates. It only cares about the four-value
     return contract.

  2. **Testable without a terminal** — I/O injection (`input_fn`/`output_fn`)
     means the entire loop can be exercised in unit tests with no mocking
     framework required.

  3. **Composable** — Language, Prompt, and Waiting are independent. You can
     swap any one without touching the others.

  4. **Crash-safe** — Exceptions inside the language evaluator are caught by
     the loop and converted to error messages. The REPL never crashes.
  """

  alias CodingAdventures.Repl.Loop

  # ---------------------------------------------------------------------------
  # run/4 — interactive session on the real terminal
  # ---------------------------------------------------------------------------

  @doc """
  Start an interactive REPL session using the real terminal for I/O.

  This is the entry point for end users running the REPL at a command line.
  It wires up `IO.gets/1` for input and a custom output function that uses
  `IO.write/1` for prompts and `IO.puts/1` for results.

  ## Parameters

  - `language` — module implementing `CodingAdventures.Repl.Language`.
  - `prompt` — module implementing `CodingAdventures.Repl.Prompt`.
  - `waiting` — module implementing `CodingAdventures.Repl.Waiting`.
  - `opts` — keyword list, currently unused (reserved for future options
    such as history, colour, and multi-line editing).

  ## Returns

  `:ok` when the session ends (user typed `:quit` or EOF).

  ## Example

      Repl.run(MyLanguage, DefaultPrompt, SilentWaiting)
  """
  def run(language, prompt, waiting, _opts \\ []) do
    # IO.gets/1 reads a line and includes the trailing newline. The loop
    # strips it before passing to the language evaluator.
    #
    # We pass an empty string to IO.gets because the loop calls output_fn
    # with the prompt string first — this lets output_fn decide whether to
    # use IO.write (no newline, so cursor stays on same line) or IO.puts.
    input_fn = fn _prompt_str -> IO.gets("") end

    # For terminal use: prompts are written with IO.write (no trailing newline
    # so the cursor stays on the prompt line); results use IO.puts (newline
    # appended). We detect a prompt by checking if it ends with a space and
    # has no embedded newline — conventional prompt strings like "> " and
    # "... " fit this pattern.
    output_fn = &terminal_output/1

    Loop.run(language, prompt, waiting, input_fn, output_fn)
  end

  # terminal_output/1 is a named function (rather than an anonymous closure)
  # so that it can be tested in isolation via ExUnit.CaptureIO.
  #
  # Convention: a string that ends with a space and has no embedded newline
  # is treated as a prompt → IO.write (no newline appended).
  # Everything else is a result → IO.puts (newline appended).
  @doc false
  def terminal_output(text) do
    if String.ends_with?(text, " ") and not String.contains?(text, "\n") do
      IO.write(text)
    else
      IO.puts(text)
    end
  end

  # ---------------------------------------------------------------------------
  # run_with_io/5 — testable session with injected I/O
  # ---------------------------------------------------------------------------

  @doc """
  Start a REPL session with injected I/O functions.

  This is the primary entry point for tests and programmatic use. By passing
  custom `input_fn` and `output_fn`, the caller has full control over what the
  REPL reads and where it writes.

  ## Parameters

  - `language` — module implementing `CodingAdventures.Repl.Language`.
  - `prompt` — module implementing `CodingAdventures.Repl.Prompt`.
  - `waiting` — module implementing `CodingAdventures.Repl.Waiting`.
  - `input_fn` — `(String.t() -> String.t() | nil)`. Called to read each
    line. Receives the current prompt string. Return `nil` to signal EOF
    (treated as `:quit`).
  - `output_fn` — `(String.t() -> any())`. Called for every prompt and
    every result line. Return value is ignored.
  - `opts` — keyword list forwarded to `Loop.run/6`.  Supported keys:
    - `:mode` — `:async` (default) or `:sync`.  See `Loop.run/6`.

  ## Returns

  `:ok` when the session ends.

  ## Example

      # Drive the REPL from a list of pre-prepared inputs:
      lines = ["hello", "world", ":quit"]
      ref = make_ref()
      {:ok, agent} = Agent.start_link(fn -> lines end)
      input_fn = fn _p ->
        Agent.get_and_update(agent, fn
          [] -> {nil, []}
          [h | t] -> {h, t}
        end)
      end

      outputs = []
      {:ok, out_agent} = Agent.start_link(fn -> [] end)
      output_fn = fn line ->
        Agent.update(out_agent, fn acc -> acc ++ [line] end)
      end

      :ok = Repl.run_with_io(EchoLanguage, DefaultPrompt, SilentWaiting,
                              input_fn, output_fn)
      results = Agent.get(out_agent, & &1)
  """
  def run_with_io(language, prompt, waiting, input_fn, output_fn, opts \\ []) do
    Loop.run(language, prompt, waiting, input_fn, output_fn, opts)
  end

  # ---------------------------------------------------------------------------
  # step/6 — execute one REPL iteration
  # ---------------------------------------------------------------------------

  @doc """
  Execute a single REPL step with a pre-read input string.

  Useful for unit tests that want to test specific inputs without running
  the full loop. The prompt is shown, eval is called, output is emitted,
  and the result signal is returned.

  ## Parameters

  - `language`, `prompt`, `waiting` — same as `run/4`.
  - `input` — the raw input string (as if the user typed it; no newline needed).
  - `input_fn` — not called by `step/6` itself (the input is pre-provided),
    but passed through for API symmetry.
  - `output_fn` — called with any output the step produces.
  - `opts` — keyword list forwarded to `Loop.step/7`.  Supported keys:
    - `:mode` — `:async` (default) or `:sync`.

  ## Returns

  - `{:continue, value_or_nil}` — step completed; keep going.
  - `{:quit, nil}` — step produced a quit signal.
  """
  def step(language, prompt, waiting, input, input_fn, output_fn, opts \\ []) do
    Loop.step(language, prompt, waiting, input, input_fn, output_fn, opts)
  end
end
