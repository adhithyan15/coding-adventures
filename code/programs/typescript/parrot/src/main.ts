/**
 * main.ts — the Parrot REPL entry point.
 *
 * This file wires together all the pieces of the Parrot REPL:
 *
 *   EchoLanguage  — the evaluator: echoes input back, quits on ":quit"
 *   ParrotPrompt  — the personality: parrot-themed banners and prompts
 *   SilentWaiting — the animation strategy: no spinner (echo is instant)
 *   runWithIo     — the REPL loop from @coding-adventures/repl
 *
 * Real stdin/stdout I/O is wired up here using Node.js `readline`. The
 * readline interface handles line buffering, so the user can use backspace
 * and arrow keys while typing.
 *
 * ## Why readline instead of process.stdin.on("data")?
 *
 * `process.stdin` in raw mode delivers individual keystrokes or chunks of
 * bytes. You'd need to handle line endings, backspace, and escape sequences
 * yourself. `readline.createInterface` handles all of that and delivers
 * complete lines, which is exactly what the REPL loop expects.
 *
 * ## Why `terminal: false`?
 *
 * Setting `terminal: false` prevents readline from echoing the user's input
 * back to stdout. Since we're running a demo REPL that echoes via EchoLanguage,
 * we don't want double echoing. In a real interactive REPL you'd typically
 * want `terminal: process.stdin.isTTY` to get proper editing support.
 *
 * ## The async queue pattern
 *
 * readline is event-driven: it fires "line" events when input arrives. The
 * REPL loop is Promise-based: it calls `inputFn()` and awaits the result.
 *
 * To bridge these two styles, we maintain a queue:
 *
 *   ┌──────────────┐   "line" event   ┌───────────┐
 *   │   readline   │ ────────────────▶ │   queue   │
 *   └──────────────┘                  └───────────┘
 *                                           │
 *                                    inputFn() pulls
 *                                           │
 *                                           ▼
 *                                     ┌──────────┐
 *                                     │ REPL loop │
 *                                     └──────────┘
 *
 * If readline delivers a line before inputFn is called, the line sits in the
 * queue. If inputFn is called before readline delivers a line, a resolver is
 * parked — and when the line arrives, the resolver is called immediately.
 */

import * as readline from "node:readline";
import { EchoLanguage, SilentWaiting, runWithIo } from "@coding-adventures/repl";
import { ParrotPrompt } from "./prompt.js";

/**
 * main — start the Parrot REPL reading from stdin and writing to stdout.
 *
 * This function is `async` because `runWithIo` returns a Promise that resolves
 * when the session ends (either `:quit` was typed or stdin reached EOF).
 */
async function main(): Promise<void> {
  // Create a readline interface that reads line-by-line from stdin.
  // `terminal: false` disables readline's built-in echo so EchoLanguage
  // controls all output.
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: false,
  });

  // The queue holds lines waiting to be consumed by inputFn.
  // `null` in the queue signals EOF (the "close" event from readline).
  const lines: Array<string | null> = [];

  // If inputFn is called while the queue is empty, it parks a resolver here.
  // When readline delivers the next line, the resolver is called to unblock
  // the awaiting inputFn.
  let resolve: (() => void) | null = null;

  // On each complete line, push it into the queue and wake any parked resolver.
  rl.on("line", (line) => {
    lines.push(line);
    if (resolve) { resolve(); resolve = null; }
  });

  // On EOF (Ctrl-D or pipe closed), push `null` to signal end-of-input.
  rl.on("close", () => {
    lines.push(null);
    if (resolve) { resolve(); resolve = null; }
  });

  /**
   * inputFn — async function that returns the next line or null on EOF.
   *
   * This is the bridge between readline's event model and the Promise-based
   * REPL loop. If a line is already queued, it returns immediately. Otherwise
   * it parks a Promise resolver and waits for readline to deliver the line.
   */
  const inputFn = async (): Promise<string | null> => {
    // Fast path: a line is already in the queue.
    if (lines.length > 0) return lines.shift()!;

    // Slow path: no line yet — park until readline delivers one.
    await new Promise<void>((r) => { resolve = r; });
    return lines.shift() ?? null;
  };

  // Run the REPL loop with all pieces wired together.
  // `runWithIo` handles the Read-Eval-Print cycle until quit or EOF.
  await runWithIo(
    new EchoLanguage(),       // evaluator: echoes input, quits on ":quit"
    new ParrotPrompt(),       // prompt: parrot-themed banners
    new SilentWaiting(),      // waiting: no animation (echo is instant)
    inputFn,                  // input: reads from stdin via readline
    (text: string) => process.stdout.write(text),  // output: writes to stdout
  );
}

// Top-level entry point. Any unhandled rejection is logged to stderr.
main().catch(console.error);
