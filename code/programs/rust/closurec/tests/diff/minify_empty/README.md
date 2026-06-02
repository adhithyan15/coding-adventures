# minify_empty — empty input → empty output (KNOWN DIVERGENCE)

The simplest possible end-to-end fixture: zero bytes in. Under
`--compilation_level WHITESPACE_ONLY` the question is what
trailing bytes (if any) the compiler appends to a body it didn't
write.

**closurec today emits a single `\n` (0x0a)**. We don't yet have
a captured upstream golden for the empty-input case to know
whether upstream Closure emits the same `\n`, zero bytes, or
something else (some compilers emit a sourceMap comment header
even on empty input).

This fixture is in the **IGNORE_FIXTURES** list in
`tests/diff_minify.rs` pending a captured golden. When a real
upstream run lands the `expected.stdout` should be replaced
with the actual byte sequence and the fixture removed from the
ignore list.

The reason this case matters at all: a closurec user running it
on a generated source file that happens to be empty (e.g. a
build artefact for an empty input module) should get the same
exact output upstream Closure would produce — not a stray
newline byte that differs from upstream.
