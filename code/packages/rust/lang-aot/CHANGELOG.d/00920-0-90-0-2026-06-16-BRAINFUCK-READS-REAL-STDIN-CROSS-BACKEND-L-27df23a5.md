## 0.90.0 — 2026-06-16 — Brainfuck reads real stdin cross-backend (LANG-FULL B1-stdin)

The matrix proved every backend can *write* output (`.`); two new programs prove every
backend can *read* input (`,`):

- `,+.` — read one byte from real stdin, `+` (increment), print: input `"A"` (65) →
  output `"B"` (66). The output depends on **both** the input and a computation on it —
  not a constant, not a bare echo.
- `,.,.` — read a byte and print it, twice: input `"Hi"` → output `"Hi"`. Proves
  *repeated* reads advance through the input stream (the second `,` sees `'i'`, not `'H'`).

Both run on **all 7 backends** (native/LLVM/WASM/JVM/CLR/VM/JIT — verified locally with
every toolchain present). Wiring (test-harness only — every backend already *compiled*
`,`→`getchar`; it had simply never been fed input):

- **native / LLVM / JVM / CLR** read from their real process stdin (libc `getchar`,
  `System.in`, `Console.Read`). New `output_with_stdin` helper spawns the child with a
  piped stdin, writes the program's input, and closes the pipe (→ EOF).
- **WASM / VM / JIT** are in-process: their `getchar` host now drains a per-program byte
  buffer seeded from `program_stdin` (empty for every non-stdin program, so the first
  read is EOF — unchanged behaviour).

Both programs read **exactly** the bytes supplied and never read past EOF, so they
terminate identically on every backend **regardless** of the divergent `getchar`-EOF
convention (JVM/VM/JIT return `0`; libc/`Console.Read`/wasm return `-1` → the u8 cell
wraps to 255). The classic cat `,[.,]` would loop forever on the `-1` backends, so
normalising EOF across backends is tracked as a separate item. No backend/frontend crate
changed.

