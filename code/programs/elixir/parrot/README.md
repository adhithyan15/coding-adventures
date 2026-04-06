# Parrot (Elixir)

🦜 The world's simplest REPL — it repeats back whatever you type.

Parrot is a demonstration program for the
[`coding_adventures_repl`](../../packages/elixir/repl/) framework. It shows how
to wire the three pluggable components — Language, Prompt, and Waiting — into a
working interactive program.

---

## What It Does

```
🦜 Parrot REPL
I repeat everything you say! Type :quit to exit.

hello
hello
🦜 Parrot REPL
I repeat everything you say! Type :quit to exit.

how are you?
how are you?
🦜 Parrot REPL
I repeat everything you say! Type :quit to exit.

:quit
```

Everything you type is echoed back verbatim. Type `:quit` to end the session.

---

## How It Works

Three pluggable components are combined:

| Component        | Module               | Role                                          |
|------------------|----------------------|-----------------------------------------------|
| **Language**     | `EchoLanguage`       | Mirrors input back; `:quit` ends the session  |
| **Prompt**       | `Parrot.Prompt`      | Parrot-themed banner + `🦜 > ` line prompt    |
| **Waiting**      | `SilentWaiting`      | No spinner (EchoLanguage is instant)          |

The framework's `Loop.run/6` drives the read-eval-print cycle:

```
stdin ──► Loop.run ──► EchoLanguage.eval ──► stdout
               │
         Parrot.Prompt  (prints banner before each read)
         SilentWaiting  (does nothing — eval is instant)
```

---

## Running

### Interactive

```bash
cd code/programs/elixir/parrot
mix deps.get
mix run -e "Parrot.Main.main([])"
```

### As an Escript Binary

```bash
mix deps.get
mix escript.build
./parrot
```

### Tests

```bash
mix deps.get
mix test
```

---

## Project Structure

```
parrot/
├── lib/
│   └── parrot/
│       ├── main.ex     # Escript entry point; wires components to stdin/stdout
│       └── prompt.ex   # Parrot.Prompt: banner and line prompt strings
├── test/
│   ├── parrot_test.exs # 25+ ExUnit tests using I/O injection
│   └── test_helper.exs # ExUnit.start()
├── mix.exs             # Build manifest; escript config; path dep
├── BUILD               # Unix build script
├── BUILD_windows       # Windows build script
├── CHANGELOG.md
└── README.md           # This file
```

---

## Where It Fits in the Stack

```
code/programs/elixir/parrot/     ← you are here (demonstration program)
code/packages/elixir/repl/       ← REPL framework (Loop, EchoLanguage, etc.)
```

Parrot depends on the `repl` package but adds no new library functionality of
its own — its only purpose is to demonstrate how the framework is used.

---

## Design Notes

**Why does `global_prompt` print a full banner every cycle?**

The `CodingAdventures.Repl.Prompt` behaviour's `global_prompt/0` is called once
per REPL cycle (before each input read). Parrot uses this to show the banner on
every iteration, which is intentionally verbose for a demo. A production REPL
would track state and only show the banner on the first iteration.

**Why `SilentWaiting`?**

`EchoLanguage.eval/1` is a pure function — it returns instantly. There is no
"waiting" to animate. `SilentWaiting` is the correct plugin for this use case:
it implements the `Waiting` behaviour with no-ops.

**Why `IO.write` instead of `IO.puts`?**

`IO.puts/1` always appends a newline. `IO.write/1` writes exactly what you give
it. Since the prompt strings and echo results contain their own newlines where
needed, using `IO.write` gives precise control over whitespace.
