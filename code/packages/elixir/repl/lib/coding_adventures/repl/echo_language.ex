defmodule CodingAdventures.Repl.EchoLanguage do
  @moduledoc """
  EchoLanguage — the simplest possible Language implementation.

  ## What It Does

  EchoLanguage mirrors the user's input back as the result, unchanged.
  It is the "hello world" of language plugins:

      > hello
      hello
      > 42
      42
      > :quit
      (session ends)

  ## Why Does This Exist?

  1. **Testing** — The REPL loop is independent of any real language. To test
     the loop's plumbing (I/O injection, waiting integration, error handling)
     without a real evaluator, we need a trivial stand-in. EchoLanguage fills
     that role perfectly.

  2. **Documentation** — It shows exactly what the Language behaviour contract
     looks like when implemented. A reader new to the codebase can understand
     the interface by reading 10 lines of code here before moving on to a
     more complex implementation.

  3. **Demos** — A standalone REPL demo that "works" without any language
     installed is useful for presentations and early integration testing.

  ## The Quit Convention

  The string `":quit"` is treated as a special sentinel that signals the user
  wants to end the session. This mirrors the convention used in many
  terminal-based REPLs. The exact sentinel is intentionally a plain string —
  there is no magic keyword parsing in the loop itself.

  If you implement your own language and want a different quit sequence
  (`:exit`, `quit()`, `\\q`, etc.), simply return `:quit` for that input.
  """

  @behaviour CodingAdventures.Repl.Language

  # ---------------------------------------------------------------------------
  # eval/1 — the sole required callback
  # ---------------------------------------------------------------------------

  @impl true
  @doc """
  Echo the input back as the result.

  Special cases:
  - `":quit"` → `:quit` — ends the session
  - anything else → `{:ok, input}` — reflects the text back unchanged
  """
  def eval(":quit"), do: :quit

  def eval(input), do: {:ok, input}
end
