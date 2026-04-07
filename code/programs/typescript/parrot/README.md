# parrot — TypeScript

A demonstration program that runs a Parrot REPL using the
`@coding-adventures/repl` framework. The parrot echoes back everything you
type, decorated with parrot-themed prompts and banners.

## What it does

```
🦜 Parrot REPL
I repeat everything you say! Type :quit to exit.

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
  └── @coding-adventures/repl      ← the REPL framework
        ├── EchoLanguage           ← evaluator: echo input back
        ├── ParrotPrompt           ← prompt: parrot-themed strings
        └── SilentWaiting          ← waiting: no animation needed
```

The program's only unique contribution is `ParrotPrompt` — the personality
layer. The loop logic, echo behaviour, and quit handling all come from the
shared `@coding-adventures/repl` package.

## Usage

```bash
# Install dependencies
npm install

# Run the REPL interactively
npm start

# Run tests
npm test

# Run tests with coverage
npm run test:coverage
```

## Project layout

```
parrot/
  src/
    prompt.ts      # ParrotPrompt: implements the Prompt interface
    main.ts        # Entry point: wires I/O and starts the loop
  tests/
    parrot.test.ts # 15 unit tests using injected I/O
  BUILD            # Build script (Unix)
  BUILD_windows    # Build script (Windows)
  package.json
  tsconfig.json
  vitest.config.ts
```

## Implementation notes

- I/O is fully injected — tests never touch stdin or stdout.
- `main.ts` uses an async queue to bridge readline's event model to the
  Promise-based REPL loop.
- `terminal: false` prevents readline from double-echoing input.
- Sync and async modes are both tested.
