# APL REPL

The interactive REPL and the **`apl` binary** for the APL language. Wraps a
persistent [`apl-runtime`](../apl-runtime) `Interpreter` and adds console
behaviours. Item **MA-4e**; sibling of `matlab-repl`/`s-repl`/`r-repl`.

## Usage

```sh
cargo run -p coding-adventures-apl-repl --bin apl
```

```
APL (on array-runtime) — type quit to exit.
>> A←2 3⍴1 2 3 4 5 6
>> A
1 2 3
4 5 6
>> +/A
6 15
>> 2×3+4
14
>> quit
```

Lines **continue** across an open `(` (the `... ` prompt) — APL has no block
keywords and no string literal type, so unlike `matlab-repl`'s scanner (which
also tracks `if`/`for`/`while`/... and `"`-strings) this one reduces to plain
paren-balance tracking. `quit`/`exit` (or Ctrl-D) leaves. Errors are shown,
not fatal — the session continues.

## Testing

```sh
cargo test -p coding-adventures-apl-repl
```
