# Fixture: `simple-fold-replace`

End-to-end oracle for string-literal `replace` / `replaceAll` folding at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `"a-b-c".replaceAll("-","_")` and `"aXbXc".replace("X","-")` |
| `expected.stdout` | The folded output: `var a="a_b_c";var b="a-bXc";report(a,b);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose `constant-fold`
pass folds the string-pattern / string-replacement overload of
`String#replace` and `String#replaceAll` (ECMAScript §22.1.3.19 / §22.1.3.20)
on string literals to a single string literal. `replace` substitutes the
**first** match (`"aXbXc".replace("X","-")` → `"a-bXc"`); `replaceAll`
substitutes **every** match (`"a-b-c".replaceAll("-","_")` → `"a_b_c"`).

The string overload matches the search string **literally** — no regex — so a
`.` is a literal dot. Both operands are valid strings, so the substitution
yields valid UTF-16 (no surrogate pair is split). The fold declines (leaving
the call for the runtime) when the replacement contains `$` (V8 expands `$$` /
`$&` / `` $` `` / `$'` / `$n` substitution patterns) or the search string is
empty (V8 inserts at every code-unit boundary); a non-string argument or a
non-literal receiver also passes through. The same input under
`WHITESPACE_ONLY` keeps the calls unfolded.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-replace/input/a.js \
    > tests/diff/simple-fold-replace/expected.stdout
```
