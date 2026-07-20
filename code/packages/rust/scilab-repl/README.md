# Scilab REPL

The interactive REPL and the **`scilab` binary** for the Scilab language.
Wraps a persistent [`scilab-runtime`](../scilab-runtime) `Interpreter` and adds
console behaviours. Item **MA-10d**; sibling of `matlab-repl`/`maple-repl`.

## Usage

```sh
cargo run -p coding-adventures-scilab-repl --bin scilab
```

```
-->A = [1 2; 3 4];
-->sum(A(:, 1))
ans = 4
-->A($)
ans = 4
-->function y = square(x)
> y = x * x;
> endfunction
-->square(5)
ans = 25
-->quit
```

Lines **continue** across open brackets and an unterminated
`if`/`select`/`while`/`for` block (until its `end`) or `function` (until its
`endfunction`) — the `> ` prompt; `quit` or `exit` (or Ctrl-D) leaves. Errors
are shown, not fatal — the session continues.

## Testing

```sh
cargo test -p coding-adventures-scilab-repl
```
