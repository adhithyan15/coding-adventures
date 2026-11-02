---
category: Compiler / VM / language pipeline
---

# Callers append code that calls an emitted program's functions by name, so a backend cannot scope its top-level functions

While fixing issue #18466, the JavaScript backend's user code was first
wrapped in a `{ … }` block.  In strict mode a function declaration in a
block is scoped to it, so a user `(define (Map x) x)` could no longer be
hoisted over the inlined runtime and replace its `Map`.  The backend's own
suite passed.  `java-to-semantic-ir`'s `loop_control_java_execution` then
failed with `ReferenceError: probe is not defined`.  That test writes the
artifact and appends `console.log(probe());`.  The Python tests in the same
crate append `print(probe())` the same way.

Fix: the declarations stay at top level, and `sanitize_ident` reserves the
JavaScript and Node globals instead (`JS_GLOBALS`, with a unit test that
asks `node` for its real global object).

**An emitted artifact's top-level function names are an interface: other
crates splice code after the artifact and call them by name.**  Before
changing how a backend scopes or names top-level functions, grep the
workspace for writers that append to an artifact (`epilogue`,
`writeln!(file, …)` after `artifact.source`).  Then run those crates'
tests, not only the backend's own.  The C and Ruby backends' `sir_fn_`
function prefix was checked this way: no caller appends to C or Ruby
artifacts.
