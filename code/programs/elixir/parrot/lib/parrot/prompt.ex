defmodule Parrot.Prompt do
  @moduledoc """
  ParrotPrompt — the parrot-themed prompt for the Parrot REPL.

  ## What Is a Prompt?

  A prompt is the short string printed before the cursor to say "I'm ready;
  type something."  The `CodingAdventures.Repl.Prompt` behaviour requires two
  callbacks:

  - `global_prompt/0` — shown at the start of each fresh input cycle.
    This is a good place for a banner or greeting.
  - `line_prompt/0` — shown on continuation lines (e.g. inside a multi-line
    block). Parrot doesn't do multi-line input, but the framework requires this
    callback, so we give it a consistent parrot style.

  ## Why a Separate Module?

  Separating the prompt from the main entry point follows the Single
  Responsibility Principle: the prompt knows how to describe the REPL to the
  user; Main knows how to wire everything together. This also makes the prompt
  independently testable — we can confirm the text contains "Parrot" and the
  parrot emoji without running the full REPL loop.

  ## Parrot Theme

  The 🦜 emoji is used throughout to reinforce the parrot metaphor. A parrot
  repeats what it hears — and so does this REPL. The global_prompt is shown
  exactly once at the top of each session cycle (before the first input read),
  so it acts as a banner and a per-line prompt rolled into one.

  In the framework's `run_with_io` loop:

      output_fn.(prompt.global_prompt())   ← called before every input read
      line = input_fn.(...)
      ...
      output_fn.(value)                    ← called with the echo result

  So `global_prompt()` doubles as the per-line prompt. We include a brief
  explanation of the REPL in the first call and keep subsequent calls as a
  short "🦜 > " so the output isn't flooded with banners.

  However, because the framework calls `global_prompt/0` on every iteration,
  we return the full multi-line banner every time. This is intentional for
  simplicity — the demo is meant to be illustrative, not production-polished.
  A real application might track iteration count and only show the banner once.
  """

  @behaviour CodingAdventures.Repl.Prompt

  # ---------------------------------------------------------------------------
  # global_prompt/0
  # ---------------------------------------------------------------------------

  @doc """
  The banner/prompt shown at the start of each REPL cycle.

  Returns a multi-line string:

      🦜 Parrot REPL
      I repeat everything you say! Type :quit to exit.

  The double newline at the end creates a blank line between the banner and
  the user's cursor, giving the interface a little breathing room.

  The `@impl true` annotation tells the Elixir compiler that this function
  is intentionally satisfying the `global_prompt/0` callback from the Prompt
  behaviour. If the behaviour ever changes its callback signature, the compiler
  will warn us here rather than letting a mismatch go undetected.
  """
  @impl true
  def global_prompt() do
    "🦜 Parrot REPL\nI repeat everything you say! Type :quit to exit.\n\n"
  end

  # ---------------------------------------------------------------------------
  # line_prompt/0
  # ---------------------------------------------------------------------------

  @doc """
  The continuation prompt shown on follow-up lines of a multi-line input.

  Parrot uses EchoLanguage, which is always single-line, so this prompt is
  never shown in practice. We implement it anyway because:

  1. The `CodingAdventures.Repl.Prompt` behaviour requires it.
  2. If someone wires Parrot.Prompt to a different language in the future,
     they'll get a sensible continuation prompt automatically.

  The parrot emoji keeps the brand consistent across both prompt types.
  """
  @impl true
  def line_prompt() do
    "🦜 > "
  end
end
