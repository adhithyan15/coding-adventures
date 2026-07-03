# minify_function_decl — function declaration round-trip (KNOWN DIVERGENCE — gap-030)

Input:

```
function f(){return 1;}
```

Upstream Closure v20240317 output (WHITESPACE_ONLY):

```
function f(){return 1};
```

Note the **two** transformations upstream applies:

1. The `;` after `return 1` is **dropped**. ASI lets the
   closing `}` terminate the return statement, so the inner
   `;` is redundant noise.
2. A `;` is **added** after the function declaration's closing
   `}`. This is upstream's syntactic-noise normalisation —
   even though the trailing `;` is a no-op at end-of-file, it
   keeps the function-declaration output shape predictable for
   concatenation cases.

closurec today emits:

```
function f(){return 1;}
```

— inner `;` preserved, no trailing `;`. The bytes differ from
upstream and the fixture is currently **IGNORED** pending
gap-030.

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central
`com.google.javascript:closure-compiler:v20240317`) by CLOC14.2
(PR pending).

Capture command:

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_function_decl/input/a.js
```

## What needs to change to flip this fixture to PASS

Two emitter changes (independent of each other):

1. **Drop redundant inner-`;`-before-`}`** in single-statement
   blocks where ASI would terminate anyway. Conservative
   trigger: emit `;` only if the next sibling is not the
   closing brace AND the current statement isn't already
   followed by a terminator.
2. **Emit trailing `;` after function-declaration's `}`** at
   top-level statement-list position.

Both are pure emitter changes — no AST work needed. Filed as
**gap-030** in `code/specs/CLOC12-gaps.md`.
