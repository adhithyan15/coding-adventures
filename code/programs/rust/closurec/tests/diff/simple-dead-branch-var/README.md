# simple-dead-branch-var

Locks the byte output of SIMPLE-level dead-branch hoisted-`var` extraction
(`closure-pass-fold-control-flow`), a **miscompile** fix. A statically-dead `if`
branch is removed, but a `var` inside it still hoists to the enclosing function
scope; dropping it would flip a later read from a declared-`undefined` binding to
a `ReferenceError`. The binding is extracted (initializer stripped, since the
branch never runs) before the taken code:

```js
if (false) { var z = compute(); } else use();  ->  var z; use();
```

Verified byte-identical to the reference Closure Compiler
(`closure-compiler-v20260712.jar`, SIMPLE, `--language_out NO_TRANSPILE`). See
`input/a.js` and `expected.stdout`.
