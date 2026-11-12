# Fixture: `simple-with`

A refusal fixture since CLOC44 slice 4 (CCR-075, #15860).

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | Top-level `with (o) { x(1 + 2); }` |
| `expected.stdout` | Empty: a refused input produces no output |
| `expected.stderr` | The pinned jar's diagnostic, byte for byte |
| `expected.exit` | The jar's exit status (`1`) |

The input contains a `with` statement, so `closure-compiler-v20260915` refuses
the input:

```text
tests/diff/simple-with/input/a.js:1:0: ERROR - [JSC_USE_OF_WITH] The with statement cannot be used in strict mode.
```

closurec refuses it identically. This fixture was originally written for
CLOC12.187, to prove `with` flowed through the pipeline without falling back to
WHITESPACE_ONLY. Under the CCR-075 decision (refuse what Closure refuses),
closurec matches upstream's refusal.
