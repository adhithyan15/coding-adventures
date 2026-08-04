# J REPL

The interactive REPL and the **`j` binary** for the J language. Wraps a
persistent [`j-runtime`](../j-runtime) `Interpreter` and adds console
behaviours. Item **MA-6d**; sibling of `apl-repl`/`matlab-repl`/`s-repl`/
`r-repl`.

## Usage

```sh
cargo run -p coding-adventures-j-repl --bin j
```

```
J (on array-runtime) — type quit to exit.
>> A=.2 3$1 2 3 4 5 6
>> A
1 2 3
4 5 6
>> +/A
6 15
>> i.5
0 1 2 3 4
>> 2*3+4
14
>> quit
```

Lines **continue** across an open `(` (the `... ` prompt) — J has no block
keywords and no string literal type, so (exactly like `apl-repl`'s own
scanner) this one reduces to plain paren-balance tracking. `NB.` comments
need no REPL-level handling at all — they're stripped entirely by the
lexer's skip pattern before this crate ever sees a token. `quit`/`exit` (or
Ctrl-D) leaves. Errors are shown, not fatal — the session continues.

## Testing

```sh
cargo test -p coding-adventures-j-repl
```
