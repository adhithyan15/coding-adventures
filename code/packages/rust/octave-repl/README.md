# Octave REPL

The interactive REPL and the **`octave` binary** for GNU Octave. Wraps a
persistent [`octave-runtime`](../octave-runtime) `Interpreter`. Item **MA-3e**;
sibling of `matlab-repl`.

## Usage

```sh
cargo run -p coding-adventures-octave-repl --bin octave
```

```octave
octave> A = [1 2; 3 4];
octave> for i = 1:3
...   A = A + 1;
... endfor
octave> A(1,1) != 4
ans = 0
octave> quit
```

Lines continue across open brackets and unterminated blocks — recognising both
`end` and Octave's `endif`/`endfor`/`endwhile`/`endfunction`/`endswitch`/
`end_try_catch` terminators — with `#`/`%` comments skipped. `quit`/`exit`/Ctrl-D
leaves; errors are shown, not fatal.

## Testing

```sh
cargo test -p coding-adventures-octave-repl
```
