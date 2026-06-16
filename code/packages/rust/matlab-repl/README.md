# MATLAB REPL

The interactive REPL and the **`matlab` binary** for the MATLAB language. Wraps a
persistent [`matlab-runtime`](../matlab-runtime) `Interpreter` and adds console
behaviours. Item **MA-3d**; sibling of `s-repl`/`r-repl`.

## Usage

```sh
cargo run -p coding-adventures-matlab-repl --bin matlab
```

```matlab
>> A = [1 2; 3 4];
>> A * A          % matrix product, executed through the array-runtime planner
ans =

 7  10
15  22

>> sum(A(:, 1))
ans = 4
>> quit
```

Lines **continue** across open brackets and unterminated
`if`/`for`/`while`/`switch`/`try`/`function` blocks (the `... ` prompt); `quit`
or `exit` (or Ctrl-D) leaves. Errors are shown, not fatal — the session
continues.

## Testing

```sh
cargo test -p coding-adventures-matlab-repl
```
