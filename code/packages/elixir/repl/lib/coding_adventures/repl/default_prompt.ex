defmodule CodingAdventures.Repl.DefaultPrompt do
  @moduledoc """
  DefaultPrompt — the conventional two-character shell prompt.

  ## The Conventions

  Two-character prompts (`> ` and `... `) are deeply ingrained in interactive
  programming culture:

  - `> ` is used by Node.js, Ruby's irb, Elixir's iex (sort of), and
    countless unix shells in their simplest form.
  - `... ` is used by Python, Ruby's irb, and others to signal that the
    previous line opened a block that hasn't been closed yet.

  These prompts are short enough to not crowd the code the user types, and
  distinctive enough that the eye immediately finds the input boundary.

  ## When to Replace This

  - **Branded REPLs** — `myapp> ` instead of `> `
  - **Coloured prompts** — wrap in ANSI escape codes for green or bold text
  - **Stateful prompts** — show the current namespace, module, or scope
  - **Contextual prompts** — different prompt inside a function definition
    vs. at the top level

  Swap this module for any other implementation of the Prompt behaviour and
  the loop will use it transparently.
  """

  @behaviour CodingAdventures.Repl.Prompt

  # "> " is the canonical "ready for input" signal.
  # The trailing space keeps the cursor visually separate from what is typed.
  @impl true
  def global_prompt(), do: "> "

  # "... " signals continuation — the expression is incomplete and the user
  # should keep typing. The four characters match the width of "> " plus one
  # extra dot, which aligns neatly with the line above.
  @impl true
  def line_prompt(), do: "... "
end
