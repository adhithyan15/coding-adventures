# minify_minimal_var — minimal already-minified statement

Input is a single `var x=1;` already in its minimal whitespace
form, with a single trailing newline. Under
`--compilation_level WHITESPACE_ONLY` upstream Closure leaves it
verbatim plus a trailing newline.

This is the simplest fixture that exercises:

- The closurec CLI (`--js` + `--compilation_level`)
- The lex/parse/emit round-trip
- The trailing-newline contract (LF, single, at end of output)

If this test ever fails the regression is likely in the emitter
(adding/removing whitespace or trailing bytes) or the trailing-
newline policy in the closurec main loop.

## Provenance

Captured from upstream Google Closure Compiler **v20240317**
(downloaded from Maven Central
`com.google.javascript:closure-compiler:v20240317`).

Capture command:

```
java -jar closure-compiler-v20240317.jar \
  --compilation_level WHITESPACE_ONLY \
  --js tests/diff/minify_minimal_var/input/a.js
```

Captured by CLOC14.1 (PR pending). The previous hand-traced
golden was confirmed byte-identical to the real upstream
capture — the WHITESPACE_ONLY contract on this minimal input
is constrained enough that hand-tracing produced the right
bytes. This commit replaces the provenance note with the real
capture details so the test no longer needs the "hand-traced"
caveat.
