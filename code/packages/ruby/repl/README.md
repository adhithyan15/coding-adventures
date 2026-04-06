# coding_adventures_repl

**A pluggable REPL framework with async eval and I/O injection** — part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) computing stack.

## What is this?

A Read-Eval-Print Loop (REPL) is the interactive shell at the heart of every interpreted language: IRB for Ruby, the Python shell, Node.js REPL, `psql` for PostgreSQL. The user types input, the language evaluates it, the result is printed, and the cycle repeats.

This gem provides a framework for building REPLs. You plug in three interfaces:

1. **Language** — how to evaluate user input
2. **Prompt** — what to show before the cursor
3. **Waiting** — what to show while evaluation is in progress (async)

The framework handles the loop mechanics: threading, polling, I/O, and error handling.

## Where it fits in the stack

```
REPL Framework  <-- YOU ARE HERE
    |
    v
Language backends (plug in any evaluator)
```

**Depends on:** Nothing — Ruby stdlib only (`thread` is built in).

## Installation

```ruby
# In your Gemfile
gem "coding_adventures_repl"
```

## Quick start

```ruby
require "coding_adventures_repl"

# Run with defaults: EchoLanguage, DefaultPrompt, SilentWaiting, real stdin/stdout
CodingAdventures::Repl.run
# > hello
# hello
# > :quit
# (exits)
```

## Plugging in your own language

```ruby
class MyLanguage
  include CodingAdventures::Repl::Language

  def eval(input)
    return :quit if input == "exit"
    result = my_interpreter.run(input)
    [:ok, result.to_s]
  rescue => e
    [:error, e.message]
  end
end

CodingAdventures::Repl.run(language: MyLanguage.new)
```

## The three interfaces

### Language

```ruby
# eval(input) must return one of:
[:ok, "result string"]  # success with output
[:ok, nil]              # success with no output (e.g., an assignment)
[:error, "message"]     # failure
:quit                   # end the session
```

### Prompt

```ruby
class MyPrompt
  include CodingAdventures::Repl::Prompt

  def global_prompt = "myrepl> "
  def line_prompt   = "....> "
end
```

### Waiting

```ruby
class SpinnerWaiting
  include CodingAdventures::Repl::Waiting

  FRAMES = %w[| / - \\]

  def start   = 0
  def tick_ms = 80

  def tick(i)
    print "\r#{FRAMES[i % FRAMES.size]}"
    i + 1
  end

  def stop(_i)
    print "\r"
    nil
  end
end
```

## I/O injection for testing

```ruby
inputs  = ["hello", ":quit"]
outputs = []

CodingAdventures::Repl.run_with_io(
  language:  CodingAdventures::Repl::EchoLanguage.new,
  prompt:    CodingAdventures::Repl::DefaultPrompt.new,
  waiting:   CodingAdventures::Repl::SilentWaiting.new,
  input_fn:  -> { inputs.shift },
  output_fn: ->(s) { outputs << s }
)

outputs # => ["> ", "hello", "> "]
```

## Async eval

Evaluation runs on a background `Thread`. The main thread polls with:

```ruby
thread.join(tick_ms / 1000.0)
```

If the thread is still running, `join` returns `nil` and the waiting animation ticks. When the thread finishes, `join` returns the thread and the loop continues.

If the language backend raises an unhandled exception, it is caught inside the thread and converted to `[:error, e.message]`. The session survives backend bugs.

## Built-in implementations

| Class | Interface | Behaviour |
|-------|-----------|-----------|
| `EchoLanguage` | Language | Echoes input; `:quit` on `":quit"` |
| `DefaultPrompt` | Prompt | `"> "` and `"... "` |
| `SilentWaiting` | Waiting | No-op; polls every 100ms |

## Development

```bash
bundle install
bundle exec rake test
```

## License

MIT
