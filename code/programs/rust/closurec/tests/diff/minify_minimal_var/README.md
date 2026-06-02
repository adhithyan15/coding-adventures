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

Captured by hand-tracing the WHITESPACE_ONLY contract (no real
transformation runs; the emitter just rewrites the AST verbatim).
A real upstream-Closure-captured golden should replace this when
a fresh capture run lands — but the byte sequence is so
constrained here that the hand-traced version is correct.
