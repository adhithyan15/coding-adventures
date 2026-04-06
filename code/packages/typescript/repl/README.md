# @coding-adventures/repl

A minimal, fully-typed **REPL (Read-Eval-Print Loop) framework** for TypeScript.

A REPL is the interactive shell that appears when you type `node`, `python3`,
`iex`, or `ghci` at a terminal. This package provides the loop machinery —
prompt, waiting animation, error handling, and I/O injection — so you can focus
on building the evaluator itself.

## How it fits in the stack

```
┌─────────────────────────────────────────────────────────────┐
│                    Your Application                         │
│  (implements Language, optionally Prompt and Waiting)       │
└───────────────────────────┬─────────────────────────────────┘
                            │ uses
┌───────────────────────────▼─────────────────────────────────┐
│              @coding-adventures/repl                        │
│  runWithIo / run — the loop, prompt, waiting, error guard   │
└─────────────────────────────────────────────────────────────┘
```

## Core interfaces

```typescript
// What the evaluator returns for each input
type EvalResult =
  | { tag: "ok"; output: string | null }   // success; null = no output
  | { tag: "error"; message: string }       // eval failed
  | { tag: "quit" };                        // end the session

// The evaluator — one method, async
interface Language {
  eval(input: string): Promise<EvalResult>;
}

// Prompt strings shown to the user
interface Prompt {
  globalPrompt(): string;   // "> "
  linePrompt(): string;     // "... "
}

// Animation shown while eval is running
interface Waiting {
  start(): unknown;
  tick(state: unknown): unknown;
  tickMs(): number;
  stop(state: unknown): void;
}
```

## Usage — interactive terminal

```typescript
import { run, EchoLanguage } from "@coding-adventures/repl";

// Start an interactive echo REPL in the terminal.
// Type anything to see it echoed back. Type :quit to exit.
await run(new EchoLanguage());
```

## Usage — injected I/O (for testing and embedding)

```typescript
import {
  runWithIo,
  EchoLanguage,
  DefaultPrompt,
  SilentWaiting,
} from "@coding-adventures/repl";

const outputs: string[] = [];
const inputs = ["hello", "world", ":quit"];
let i = 0;

await runWithIo(
  new EchoLanguage(),
  new DefaultPrompt(),
  new SilentWaiting(),
  async () => inputs[i++] ?? null,   // InputFn
  (s) => outputs.push(s),           // OutputFn
);

// outputs includes: "> ", "hello", "> ", "world", "> "
```

## Implementing your own Language

```typescript
import type { Language, EvalResult } from "@coding-adventures/repl";

class CalculatorLanguage implements Language {
  async eval(input: string): Promise<EvalResult> {
    if (input.trim() === "exit") return { tag: "quit" };
    try {
      // Never use eval() in production — this is just for illustration.
      const result = Function(`"use strict"; return (${input})`)();
      return { tag: "ok", output: String(result) };
    } catch (e) {
      return { tag: "error", message: (e as Error).message };
    }
  }
}
```

## Built-in implementations

| Class            | Interface  | Description                                    |
|------------------|------------|------------------------------------------------|
| `EchoLanguage`   | `Language` | Echoes input back; `:quit` exits               |
| `DefaultPrompt`  | `Prompt`   | `"> "` global, `"... "` line                   |
| `SilentWaiting`  | `Waiting`  | No-op animation; safe for tests and CI         |

## Loop behaviour

The main loop (`runWithIo`) terminates when either:
- `inputFn()` returns `null` (EOF / Ctrl-D), or
- `language.eval()` returns `{tag: "quit"}`.

Output per EvalResult:

| Result                        | Output                    |
|-------------------------------|---------------------------|
| `{tag: "ok", output: "foo"}`  | `"foo"`                   |
| `{tag: "ok", output: null}`   | *(nothing)*               |
| `{tag: "error", message: "x"}`| `"ERROR: x"`              |
| `{tag: "quit"}`               | *(nothing — loop exits)*  |

If `language.eval()` throws (unexpected exception), the loop catches the
rejection and formats it as an `error` result, then continues.

## Running tests

```sh
npm install
npm test
npm run test:coverage
```
