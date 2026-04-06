defmodule Parrot.Main do
  @moduledoc """
  Parrot REPL — the world's simplest REPL.

  Whatever you type, it echoes back. Type :quit to exit.

  ## What This Program Does

  Parrot is a demonstration program for the `CodingAdventures.Repl` framework.
  It wires together three pluggable components and connects them to stdin/stdout:

  ```
  stdin  ──► Loop.run ──► EchoLanguage.eval ──► stdout
                 │
           Parrot.Prompt          (banner + "🦜 > " prompt)
           SilentWaiting          (no spinner, pure silence)
  ```

  ## Why This Exists

  Every REPL framework needs a "hello world" — a minimal, working program that
  demonstrates how all the pieces fit together. Parrot is that program.

  It is intentionally trivial: EchoLanguage never computes anything, it just
  mirrors the input. This means every line of complexity in this file is
  framework plumbing, not language logic — exactly what a demo should show.

  ## How the Framework Works (Brief Tour)

  1. `Loop.run/6` starts the read-eval-print loop.
  2. Before each iteration it calls `Parrot.Prompt.global_prompt/0` and
     writes the result via `output_fn`.
  3. It then calls `input_fn` to read the next line from the user.
  4. The line is passed to `EchoLanguage.eval/1`:
     - `":quit"` → returns `:quit` → loop ends.
     - anything else → returns `{:ok, text}` → `output_fn.(text)` is called.
  5. Go back to step 2.

  ## Running the Program

  Build the escript:

      mix deps.get
      mix escript.build

  Run it:

      ./parrot

  Or:

      mix run -e "Parrot.Main.main([])"

  ## I/O Plumbing

  `IO.gets("")` reads a line from stdin including the trailing newline.
  The loop strips that newline before passing the text to the language evaluator.

  `IO.write/1` writes to stdout without adding a newline — the loop handles
  newlines itself (the `{:ok, value}` result is written directly, and the
  prompt strings already contain the newlines they need).
  """

  alias CodingAdventures.Repl.{EchoLanguage, SilentWaiting, Loop}

  @doc """
  The escript entry point. Called with the command-line arguments list.

  For Parrot we don't use any CLI arguments — the REPL starts immediately.
  The `_args` parameter acknowledges the argument list without binding it to
  a name we'd then never use (the leading underscore silences the compiler
  warning about an unused variable).

  ## Flow

      main([]) → Loop.run(...) → reads lines until :quit or EOF → :ok
  """
  def main(_args) do
    Loop.run(
      # language: the evaluator. EchoLanguage mirrors input back unchanged,
      # except ":quit" which returns :quit to end the session.
      EchoLanguage,

      # prompt: controls the text shown before each input line. Our parrot-
      # themed module shows the 🦜 banner and "🦜 > " continuation prompt.
      Parrot.Prompt,

      # waiting: drives animation while eval runs. SilentWaiting does nothing
      # (no spinner, no output) — appropriate for EchoLanguage which is instant.
      SilentWaiting,

      # input_fn: reads one line from stdin. IO.gets/1 blocks until the user
      # presses Enter, then returns the line including the trailing "\n".
      # We pass "" as the prompt argument because Loop already printed the
      # prompt via output_fn — we don't want it doubled.
      # String.trim_trailing strips the "\n" so the language gets clean input.
      fn -> IO.gets("") |> String.trim_trailing("\n") end,

      # output_fn: writes one string to stdout. IO.write/1 does not add a
      # newline, which is correct because:
      # - Prompt strings ("🦜 Parrot REPL\n...") contain their own newlines.
      # - Echo results are passed through as-is; the loop will add context
      #   if needed, but for EchoLanguage the value is just the input string.
      fn text -> IO.write(text) end
    )
  end
end
