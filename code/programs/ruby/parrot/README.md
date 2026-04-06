# parrot — Ruby

A demonstration program that runs a Parrot REPL using the
`coding_adventures_repl` gem. The parrot echoes back everything you type,
decorated with parrot-themed prompts and banners.

## What it does

```
🦜 Parrot REPL
I repeat everything you say! Type :quit to exit.

hello
🦜 Parrot REPL
I repeat everything you say! Type :quit to exit.

:quit
```

Every line you type is echoed back unchanged. Type `:quit` to exit.

## How it fits in the stack

```
parrot (this program)
  └── coding_adventures_repl      ← the REPL framework gem
        ├── EchoLanguage           ← evaluator: echo input back
        ├── Parrot::Prompt         ← prompt: parrot-themed strings
        └── SilentWaiting          ← waiting: no animation needed
```

The program's only unique contribution is `Parrot::Prompt` — the personality
layer. The loop logic, echo behaviour, and quit handling all come from the
shared `coding_adventures_repl` gem.

## Usage

```bash
# Install dependencies
bundle install

# Run the REPL interactively
ruby bin/parrot

# Run tests
bundle exec rake test

# Run tests directly
bundle exec ruby -Ilib -Itest test/test_parrot.rb
```

## Project layout

```
parrot/
  bin/
    parrot         # Executable entry point
  lib/
    parrot.rb      # Parrot module: wires the REPL together
    parrot/
      prompt.rb    # Parrot::Prompt: implements the Prompt interface
  test/
    test_helper.rb # Shared Minitest setup
    test_parrot.rb # 15 unit tests using injected I/O
  BUILD            # Build script (Unix)
  BUILD_windows    # Build script (Windows)
  Gemfile
  Rakefile
```

## Implementation notes

- I/O is fully injected — tests never touch `$stdin` or `$stdout`.
- `Parrot::Prompt` includes `CodingAdventures::Repl::Prompt` (a module, not a
  class), overriding both `global_prompt` and `line_prompt`.
- `Parrot.run` uses `$stdin.gets&.chomp` — the safe navigation operator `&.`
  ensures nil is returned (not an error) on EOF.
- Both `:sync` and `:async` evaluation modes are tested.
