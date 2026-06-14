# minify_for_body_inner_close

Pins gap-049 — when gap-032 flattens a single-statement block that closes
immediately before an outer `}`, the inlined trailing `;` is dropped.

Input `async function f(){for await(var v of a){a;}}`:
- The `for await(…){a;}` body is a single-statement block eligible for
  gap-032 flatten: `{a;}` → `a`.
- The closing `}` of the `for` body is immediately followed by the function
  body `}`. In that position, the trailing `;` that normally terminates the
  inlined statement would be redundant (Rule A would have dropped a source `;`
  in this slot). gap-032 detects `next_after_close == "}"` and sets
  `drop_trailing_semi = true`, so `emit_end = close_idx - 1` (omitting the `;`).
- Result: `async function f(){for await(var v of a)a};` — no `;` between `a`
  and `}`, matching upstream Closure v20240317 byte-for-byte.

Without this fix, closurec would emit `…for await(var v of a)a;};` — one
extra byte compared to upstream.
