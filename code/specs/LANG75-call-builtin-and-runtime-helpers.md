# LANG75 — Generic `call_builtin` lowering + runtime-helper expansion

**Status:** Draft — 2026-05-20
**Depends on:** LANG40, LANG41, LANG43, LANG44
**Unblocks:** BF07, NIB04, PL05, OCT02

## Motivation

Today the AOT backends (`aarch64-backend`, `x86_64-backend`) hard-code
exactly one runtime entry point: the `io_out` CIR opcode lowers to a
`CALL __twig_print_i64` and that's it.  The runtime archive
(`twig-aot/runtime/twig_runtime.c`) defines only that one function.

Every other LANG VM language we care about — Brainfuck, BASIC, Oct,
even later versions of Twig and Nib — wants more runtime helpers:

| Language | Needs |
|---|---|
| Brainfuck | `putchar(c)`, `getchar() -> c` (byte I/O on stdin/stdout) |
| Dartmouth BASIC | `print_string(s)`, `input_i64() -> n` (line-oriented I/O) |
| Oct        | `putchar`, `getchar`, plus Intel-8008-style port I/O (deferred) |
| Nib + Twig | `print_string(s)`, `print_f64(x)`, eventually `panic(msg)` |

Wiring each of these as a one-off CIR opcode (like `io_out` was) doesn't
scale.  LANG75 introduces a single generic `call_builtin "<name>", <args>`
CIR opcode that both backends lower to a `CALL` against an external
symbol, plus an enumerated set of helpers in the runtime archive.

## Non-goals

- **Variadic helpers** (e.g. printf-like formatters).  V1 has fixed
  signatures only.
- **Float helpers** (`print_f64`, etc.) — deferred until backends
  grow SSE2 support.
- **Memory allocation builtins** — covered separately by LANG76 (heap
  allocator).
- **Async / nonblocking I/O.**
- **Standalone twig-runtime ABI versioning** — the runtime archive is
  embedded in `twig-aot` and rebuilt every release; no on-disk ABI
  compatibility surface to worry about.

## New CIR opcode

```text
call_builtin <name>, <arg0>, <arg1>, …
```

| Field | Meaning |
|---|---|
| `name`  | Helper name without leading underscore (`putchar`, `getchar`, `print_string`, `input_i64`).  The backend prepends `__twig_` to form the linker symbol. |
| `args`  | Caller-supplied operands.  Loaded into the ABI's argument registers in order before the call.  Each helper has a fixed signature; passing the wrong number/type is a `BackendRefused`. |
| `dest`  | `None` for `void` helpers, `Some(var)` for returning ones.  Backend writes the return value (`RAX`/`X0`) into the dest slot after the call. |

### V1 helper table

| Name | C signature | Args (CIR) | Return |
|---|---|---|---|
| `print_i64` | `void __twig_print_i64(int64_t)` | `[i64]` | none |
| `putchar` | `void __twig_putchar(int32_t c)` | `[i32]` | none |
| `getchar` | `int32_t __twig_getchar(void)` | `[]` | `i32` (`-1` on EOF) |
| `print_string` | `void __twig_print_string(const char *s, int64_t len)` | `[ptr, i64]` | none |
| `input_i64` | `int64_t __twig_input_i64(void)` | `[]` | `i64` (parses one line; `0` on parse failure) |
| `exit` | `void __twig_exit(int32_t code)` (noreturn) | `[i32]` | none |

`io_out v` becomes sugar for `call_builtin "print_i64", v`; the
existing `io_out` opcode stays for backwards compatibility but is now
specified as that desugaring.

## Backend lowering

Both `x86_64-backend` and `aarch64-backend` add a `call_builtin`
dispatch that:

1. Validates the helper name against the table.  Unknown names →
   `BackendRefused`.
2. Validates the argument count + types against the helper's signature.
3. Loads each argument into the ABI's `i`th argument register using the
   existing per-ABI argument-register table (System V vs MS x64 for
   x86_64; AAPCS64 for aarch64).
4. Emits a `CALL rel32` (x86) / `BL rel26` (arm64) to symbol
   `__twig_<name>` with the same `PltRel32` external relocation kind
   the existing `io_out` lowering uses.
5. If the helper returns, stores `RAX` (or `X0`) into the dest slot.

The cross-function reloc patching from PR #3331 / aarch64's Pass 2
already handles same-module symbols, so `__twig_<name>` symbols
appearing in the runtime archive flow through unchanged to the
packager's `ExternBranchReloc` / `X86RelocRecord` list.

## Runtime archive changes

`twig-aot/runtime/twig_runtime.c` gains the helpers from the V1 table.
Each is a thin wrapper over `<stdio.h>` / `<stdlib.h>`:

```c
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>

void __twig_print_i64(int64_t v) { printf("%lld\n", (long long)v); fflush(stdout); }

void __twig_putchar(int32_t c) {
    fputc((unsigned char)c, stdout);
}

int32_t __twig_getchar(void) {
    int c = fgetc(stdin);
    return (int32_t)c;          // returns -1 (EOF) naturally
}

void __twig_print_string(const char *s, int64_t len) {
    if (s != NULL && len > 0) fwrite(s, 1, (size_t)len, stdout);
}

int64_t __twig_input_i64(void) {
    char buf[64];
    if (fgets(buf, sizeof(buf), stdin) == NULL) return 0;
    long long v = 0;
    sscanf(buf, "%lld", &v);
    return (int64_t)v;
}

__attribute__((noreturn)) void __twig_exit(int32_t code) { exit((int)code); }
```

All POSIX — same `cc -lc` / `link.exe libcmt.lib` link command line
the existing runtime uses.

## Tests

### Backend tests (per backend)

- `call_builtin_putchar_emits_correct_arg_marshal` — for both ABIs,
  the byte arg lands in the ABI's arg-0 register (`RDI`/`RCX`/`X0`)
  and the call site records a `PltRel32` reloc on `__twig_putchar`.
- `call_builtin_getchar_stores_eax_into_dest` — the dest slot receives
  the return value.
- `call_builtin_print_string_marshals_two_args` — pointer + len.
- `call_builtin_unknown_name_refuses` — `BackendRefused` not panic.

### Runtime integration tests

Per-host smoke tests in `twig-aot/tests/`:

- `linux_x86_64_putchar_writes_byte`: compile a tiny module that emits
  `call_builtin "putchar", 65` (`A`), link, run, capture stdout, assert
  it received the byte.
- Same for `windows_x86_64_putchar_writes_byte`.
- `getchar` round-trip: pipe a known byte into stdin, assert the
  program echoes it.

## Risk register

| Risk | Mitigation |
|---|---|
| ABI mismatch for the arg-0 register on Windows MS x64 (callee may expect RCX, we send via wrong reg) | Reuse the existing per-ABI `arg_regs()` table that already drives `io_out`. |
| `fputc` / `fgetc` blocking on a closed stdin causes hangs in tests | Tests use `Command` with a closed `stdin()` pipe; helpers return `-1`/`0` cleanly on EOF. |
| Helper name collision with user functions ("getchar" as a Twig function name?) | Backend always prepends `__twig_` so user code that defines `getchar` resolves to a separate symbol; documented in the spec. |
| Static analysis tools flag `__twig_input_i64`'s `sscanf` | Spec says "V1 is intentionally permissive — security-hardened input parsing is a follow-up." |
