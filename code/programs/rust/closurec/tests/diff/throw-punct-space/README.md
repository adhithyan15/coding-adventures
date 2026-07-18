# throw-punct-space — `throw {a:1};` → `throw{a:1};`

Input: `throw {a:1};`

At SIMPLE, closurec emits no space between `throw` (or `return`) and an argument
that begins with punctuation — an object `{`, array `[`, string `"`, regex `/`,
template `` ` ``, or a `!`/`~`/`-`/`+` unary — because those tokenise cleanly
against the keyword. The space is kept only where a word token would fuse
(`throw x`, `throw 5`, `throw new C`, `throw void x`).

Expected (SIMPLE): `throw{a:1};` — byte-identical to the reference Closure
Compiler. The proof the pipeline ran (not a raw fallback) is the folded output;
the point of the fixture is the absence of the `throw`-space.

See `closure-emitter`'s `keyword_needs_space_before`.
