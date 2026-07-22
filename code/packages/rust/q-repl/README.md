# Q REPL

The interactive REPL and the **`q` binary** for the Q language. Wraps a
persistent [`q-runtime`](../q-runtime) `Interpreter` and adds console
behaviours. Item **MA-11d**; sibling of `j-repl`/`apl-repl`/`matlab-repl`/
`s-repl`/`r-repl`.

## Usage

```sh
cargo run -p coding-adventures-q-repl --bin q
```

```
Q (on array-runtime) — type quit to exit.
>> f:{[x;y] x+y}
>> f 2 3
5
>> +/1 2 3 4
10
>> !5
0 1 2 3 4
>> 2*3+4
14
>> quit
```

A multi-line function literal is typed one line at a time, exactly the way
a real interactive session works:

```
>> f:{[x;y]
...  x+y}
>> 2 f 3
5
```

## Lines continue across an open `(`, `{`, or `[` — the one genuinely new
## concern beyond J's/APL's plain paren-balance scanner

Read `j-repl`'s own module doc comment first: J/APL have no user-defined
block construct at all, so their continuation scanner reduces to plain
paren-balance tracking. Q is different — it has a real one, the function
literal `{[x;y] stmt; stmt}` (MA11 §2/§3 bullet 1) — which can legitimately
span several physical lines in an interactive session. This crate's
scanner tracks three *independent* running depths (parens, braces,
brackets), so a mismatched `{)` correctly stays "incomplete" on the
strength of the still-open brace rather than two mismatched counts
happening to cancel out. It delegates to the *real* Q lexer
(`coding_adventures_q_lexer::try_tokenize_q`) to count bracket tokens,
rather than hand-rolling a character scan — Q's `/`-comment marker (MA11
§3 bullet 2) is an ordinary character that could contain an unbalanced
bracket inside comment text, and the real lexer's own pre-tokenize hook
already strips comments correctly, so a stray `(` inside a comment can
never fool this scanner.

`quit`/`exit` (or Ctrl-D) leaves. Errors are shown, not fatal — the session
continues.

## Testing

```sh
cargo test -p coding-adventures-q-repl
```
