# REPL Framework (Elixir)

A pluggable Read-Eval-Print Loop framework with async evaluation, I/O injection, and three composable plugin interfaces.

## What It Does

This library provides the scaffolding for an interactive programming session — the loop that reads a line, evaluates it in some language, prints the result, and repeats. The loop itself knows nothing about any specific language; all behaviour is injected through three module-based plugins.

## The Three Plugins

| Plugin   | Behaviour                                  | Controls                              |
|----------|--------------------------------------------|---------------------------------------|
| Language | `CodingAdventures.Repl.Language`           | How a line of input is evaluated      |
| Prompt   | `CodingAdventures.Repl.Prompt`             | What text appears before the cursor   |
| Waiting  | `CodingAdventures.Repl.Waiting`            | What happens while eval is running    |

### Built-in Implementations

- **EchoLanguage** — echoes input back unchanged; `:quit` ends the session.
- **DefaultPrompt** — `"> "` for new lines, `"... "` for continuations.
- **SilentWaiting** — no-op; poll interval 100 ms.

## Usage

```elixir
alias CodingAdventures.Repl
alias CodingAdventures.Repl.{EchoLanguage, DefaultPrompt, SilentWaiting}

# Interactive session on the real terminal:
Repl.run(EchoLanguage, DefaultPrompt, SilentWaiting)
# > hello
# hello
# > :quit
```

## Implementing a Language

```elixir
defmodule MyLanguage do
  @behaviour CodingAdventures.Repl.Language

  @impl true
  def eval(":quit"), do: :quit
  def eval(input) do
    case evaluate(input) do
      {:ok, value}  -> {:ok, inspect(value)}
      {:error, msg} -> {:error, msg}
    end
  end
end
```

## Testing with Injected I/O

```elixir
{:ok, in_agent}  = Agent.start_link(fn -> ["hello", ":quit"] end)
{:ok, out_agent} = Agent.start_link(fn -> [] end)

input_fn  = fn _ -> Agent.get_and_update(in_agent, fn [] -> {nil, []}; [h|t] -> {h, t} end) end
output_fn = fn line -> Agent.update(out_agent, fn acc -> acc ++ [line] end) end

:ok = Repl.run_with_io(MyLanguage, DefaultPrompt, SilentWaiting, input_fn, output_fn)
IO.inspect Agent.get(out_agent, & &1)
```

## Async Evaluation

The loop evaluates each input via `Task.async`, keeping the main process free to tick the waiting plugin. Exceptions inside the language evaluator are caught and turned into `{:error, "unexpected error: ..."}` tuples — the loop never crashes.

## How It Fits in the Stack

This package sits above the language layers (lexers, parsers, evaluators) and provides the interactive shell glue. Any language that implements the three-value return contract can be dropped in as a plugin.
