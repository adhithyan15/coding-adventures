## 0.1.1

- Author-supplied tables (`Options.new`, `one_of`, `frequency`, `tuple`, `Hooks.with`) are
  read with `next`/`rawget`/`rawlen`; no `__pairs`, `__index` or `__len` fires.
- Every module captures the standard functions it uses at load and never calls
  `s:method()`, and `Runner.check` runs under `pcall`: a property that patches the string
  metatable or `string.*`/`table.*`/`utf8.*` still gets an honest verdict.
- `Int.parse` caps digits itself (default 1000); `Int.shl`/`Int.pow2` refuse results over
  2^20 bits; the runner copies a Big witness before shrinking.
- Canon sorts with an explicit byte comparator and no longer queries the locale.
- Strict JSON flushes its string parts in chunks (8 MiB of escapes no longer builds a
  millions-entry table).
- Documented: the budget is cooperative, not a security boundary (spec section 4 and README).
