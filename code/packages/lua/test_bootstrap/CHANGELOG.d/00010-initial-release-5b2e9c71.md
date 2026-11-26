## 0.1.0

- Initial Lua lane of the VER05 property-based testing engine (spec VER05o). It
  conforms to the shared `pbt-v1` corpus: 25 of 25 cases, with exact outputs,
  minimal witnesses and shrink step counts.
- `Int`: arbitrary-size integers over Lua's 64-bit integers (a native integer when
  the value fits, a private Big otherwise, normalized), so `u64`, `i128`, spans
  wider than 2^64 and 1,000-digit fixture integers are exact. Floats are never Ints.
- `Prng`: SplitMix64 and FNV-1a on wrapping words, an UNSIGNED divide/modulo for the
  rejection threshold `(2^64 - n) mod n`, `bounded(1)` consuming a word,
  `bounded_big`, `sub_seed`.
- Generator algebra with integrated shrinking (integer walk with floored negative
  bisection, sequence shrinker) and the hierarchical shrink-budget tree of VER05e
  4.1. The budget carrier is an explicit parameter, not ambient state: a property
  that yields, a coroutine closed mid-check and a coroutine holding a handle cannot
  corrupt another check, and a closed or stale handle is spent.
- Honest runner: zero cases is never a pass, the witness is re-verified, an unset CI
  mode is CI and carries no seed or witness, a property holds only on exactly `true`,
  errors are any Lua value and are classified by `type()` alone.
- `Canon`: total, bounded rendering that never runs a user metamethod (`rawget`,
  `rawlen`, `next` only), with the Elixir lane's escape set; cycles, depth 16, 10,000
  nodes and 1 MiB are enforced while traversing.
- Strict JSON reader, TFV v1 validator, corpus evaluator with per-operation negative
  controls, and sabotage hooks.
- Tests: 281 busted tests, including Python-reference vectors for `Int` and unsigned
  division, the pinned rejection-draw, `2^63 + 1`, `bounded(1)` and floored-bisection
  tests, coroutine and hostile-value tests, and instruction-hook bounds so a
  regression fails instead of hanging.
- Coverage measured with luacov (`LUA_COVERAGE=1 busted . --pattern=test_ --coverage`):
  97.2% of executable lines of `src` (2157 of 2219). The tests skip their instruction-count
  bound under `LUA_COVERAGE` because luacov and a chained hook misattribute lines;
  `H.hard_limited` keeps the bound for the two tests that really loop.
