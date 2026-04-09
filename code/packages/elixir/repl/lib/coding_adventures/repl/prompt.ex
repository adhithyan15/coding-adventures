defmodule CodingAdventures.Repl.Prompt do
  @moduledoc """
  The Prompt behaviour — controls what the REPL displays to the user.

  ## Two Kinds of Prompt

  Classic interactive shells distinguish between two prompts:

  1. **Global prompt** — shown at the start of a new expression. This is the
     familiar `> ` or `iex> ` that you see when the REPL is waiting for fresh
     input. It signals "I am ready; give me something new."

  2. **Line prompt** — shown on continuation lines when an expression spans
     multiple input lines. In Python you see `... ` when you open a block;
     in iex you see `...> `. It signals "I got your first line; keep going."

  ## Why Two Callbacks?

  Because the loop needs to know which one to show without reaching into the
  prompt module's internals. The caller decides based on context — for now
  the framework always shows the global prompt (single-line input model), but
  multi-line language implementations can use line_prompt themselves when they
  detect an incomplete expression.

  ## Implementing a Prompt

  ```elixir
  defmodule MyPrompt do
    @behaviour CodingAdventures.Repl.Prompt

    @impl true
    def global_prompt(), do: "calc> "

    @impl true
    def line_prompt(), do: "  ... "
  end
  ```
  """

  @doc """
  The prompt shown at the start of each new input line.

  Should be short (a few characters). Convention is to end with a space so
  the cursor appears one character after the prompt.
  """
  @callback global_prompt() :: String.t()

  @doc """
  The prompt shown on continuation lines of a multi-line expression.

  Typically indented to align with the text after the global prompt, so the
  user can visually see that the input is continuing.
  """
  @callback line_prompt() :: String.t()
end
