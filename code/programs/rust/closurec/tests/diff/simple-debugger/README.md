# Fixture: `simple-debugger`

End-to-end oracle for `--compilation_level SIMPLE` across a `debugger;`
statement. CLOC21 made it representable; CLOC24 then stripped it, and
**CCR-053 undid that strip** because it was never upstream's behaviour.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A single-use function and foldable arithmetic surrounding a `debugger;` statement |
| `expected.stdout` | The optimized output (see below) |

```text
function log(p){report(p)}log(1);var x=3;debugger;use(x);
```

What this proves — none of which was reachable before CLOC21 (any `debugger`
forced a WHITESPACE_ONLY fallback):

* **Constant folding** — `1 + 2` ⇒ `3`. This is the signal that the typed
  pipeline ran rather than the WHITESPACE_ONLY fallback, which emits
  `var x=1+2;` verbatim.
* **`debugger;` preserved** — measured against the pinned oracle, upstream
  Closure keeps a reachable `debugger` at SIMPLE, and removes it only as
  collateral when the enclosing statement goes anyway (after a `return` or
  `throw`, or inside `if (false) { … }`). At ADVANCED the rule is narrower
  rather than absent: upstream also eliminates a call whose body is *only* a
  `debugger`, which is call-elimination treating the body as pure rather than
  a `debugger` sweep, and which we do not do. It is also not
  effect-free: it breaks into an attached debugger, so removing it changes
  observable behaviour.
* **`log` kept** — SIMPLE is open-world and never inlines or deletes an
  observable top-level name, because another script sharing the page could
  call it.

**This fixture does not match upstream byte-for-byte.** Upstream renames the
parameter (`function log(a)`); we do not. That is CCR-022, and it is the only
remaining difference on this input.

Two claims that were in this file and were wrong, recorded so they are not
reintroduced: that `log` is inlined to `report(1)` and its declaration deleted
(it is not — that is ADVANCED, closed-world, and the sibling `input/a.js`
always said so), and that stripping `debugger` matched upstream "exactly".

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-debugger/input/a.js \
    > tests/diff/simple-debugger/expected.stdout
```
