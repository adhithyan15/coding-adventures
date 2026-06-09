# minify_for_await_of

Captured from upstream Google Closure Compiler **v20240317**.
Pins that an `async function` containing a `for await(let x of arr) { use(x); }`
flattens the single-statement body AND drops the synthetic `;`
that would otherwise appear before the outer function `}`.

Currently **IGNORED** — closurec emits a stray `;` between
`use(x)` and the function-closing `}`. See `gap-049` in
`code/specs/CLOC12-gaps.md`. This is a general flattened-for-body
issue, not specific to `for-await-of`.
