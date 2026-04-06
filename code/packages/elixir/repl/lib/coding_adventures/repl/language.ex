defmodule CodingAdventures.Repl.Language do
  @moduledoc """
  The Language behaviour — the heart of the REPL's pluggability.

  ## What Is a Language?

  In a REPL, a *language* is anything that can take a string of text and
  produce a result. That result is one of three things:

  1. `{:ok, String.t()}` — evaluation succeeded and there is a value to show.
  2. `{:ok, nil}` — evaluation succeeded but there is nothing to display
     (e.g. an assignment that produces no value, like `x = 5` in Python).
  3. `{:error, String.t()}` — evaluation failed with a message to display.
  4. `:quit` — the user wants to exit the session.

  ## Why a Behaviour?

  Using a behaviour (Elixir's equivalent of a protocol or interface) lets the
  REPL loop remain completely agnostic about the language it is evaluating.
  You can plug in:

  - A trivial echo language (for testing and demos)
  - A mathematical expression evaluator
  - A full Elixir interpreter
  - A Brainfuck engine
  - Anything else that can map `String.t() → result`

  The loop only knows about this contract. Nothing more.

  ## Implementing a Language

  ```elixir
  defmodule MyMathLanguage do
    @behaviour CodingAdventures.Repl.Language

    @impl true
    def eval(":quit"), do: :quit
    def eval(input) do
      case Integer.parse(input) do
        {n, ""} -> {:ok, Integer.to_string(n * 2)}
        _       -> {:error, "not a number: \#{input}"}
      end
    end
  end
  ```
  """

  # ---------------------------------------------------------------------------
  # Callback definition
  #
  # eval/1 receives the raw input string (already stripped of its trailing
  # newline by the loop). It must return one of the four tagged values above.
  # ---------------------------------------------------------------------------

  @doc """
  Evaluate a single line of input in this language.

  ## Parameters

  - `input` — the raw text the user typed, newline already stripped.

  ## Returns

  - `{:ok, value}` — success; `value` is a printable string or `nil`.
  - `{:error, message}` — failure; `message` describes what went wrong.
  - `:quit` — the user requested an end to the session.
  """
  @callback eval(input :: String.t()) ::
              {:ok, String.t() | nil}
              | {:error, String.t()}
              | :quit
end
