# Fixture: `simple-try-catch`

A refusal fixture since CLOC44 slice 2 (CCR-075, #15860).

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A function, plus a top-level `try`/`catch (e)`/`finally` whose `catch` handler holds a `return` |
| `expected.stdout` | Empty: a refused input produces no output |
| `expected.stderr` | The pinned jar's diagnostic, byte for byte |
| `expected.exit` | The jar's exit status (`1`) |

The `return` sits outside any function, so `closure-compiler-v20260915`
refuses the input:

```text
tests/diff/simple-try-catch/input/a.js:30:2: ERROR - [JSC_PARSE_ERROR] Parse error. return must be inside function
```

closurec refuses it identically. This fixture was written for CLOC19, to show
SIMPLE optimising through `try`/`catch`/`finally`. That coverage now lives in
the ladder's `try_catch` rungs and in `advanced-try-catch-rename`, whose
inputs are valid.

Regenerate `expected.stderr` and `expected.exit` from the jar, running from
the package root with the fixture's own `flags.txt`.
