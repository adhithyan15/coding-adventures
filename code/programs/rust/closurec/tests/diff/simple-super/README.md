# Fixture: `simple-super`

A refusal fixture since CLOC44 slice 3 (CCR-075, #15860).

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | Top-level `super.f(1 + 2);` |
| `expected.stdout` | Empty: a refused input produces no output |
| `expected.stderr` | The pinned jar's diagnostic, byte for byte |
| `expected.exit` | The jar's exit status (`1`) |

The `super` sits outside any method, so `closure-compiler-v20260915`
refuses the input:

```text
tests/diff/simple-super/input/a.js:1:0: ERROR - [JSC_INVALID_SUPER_ACCESS] 'super' may only be accessed within a method
```

closurec refuses it identically. This fixture was originally written for
CLOC12.166 PR2, to prove `super` flowed through the pipeline rather than
falling back to WHITESPACE_ONLY. Under the CCR-075 decision (refuse what Closure
refuses), closurec matches upstream's refusal. Valid `super` usage in methods
remains covered by bridge unit tests and ladder fixtures.

Regenerate `expected.stderr` and `expected.exit` from the jar, running from
the package root with the fixture's own `flags.txt`.
