---
category: Testing & coverage
---

# Warrant models cannot call operations and take, drop, slice and replicate are checked but not yet generated, so claims about a stream's parts must be built, not cut

**What went wrong.** The first draft of `lzw.warrant` wrote the obvious claims about `compress`'s output: "its first four bytes are the length" (`take(compress(d), 4)`), "the declared length cuts the output" (`decompress(concat(be_bytes(n), drop(compress(d), 4)))`) and runs built with `to_bytes(replicate(x, n))`. Each failed at generation, one at a time: `W0301 ModelCallsOperation("compress")` (a `model` is pure and may not call an operation, so the helper `codes(d) = drop(compress(d), 4)` is illegal), then `W0701 UnsupportedTarget("call to take")` and the same for `replicate`. The WX-D1 sequence functions type-check and the ground evaluator runs them, but the Rust code generator does not emit them yet (VER04k). Two further limits showed up the same way: a binder domain is a fixed finite set whose product must fit the exhaustive budget (a `Bytes` binder times a `U16` times a `U32` is `W0706`, a hard error, not a demotion), and `ok-as-err` does not apply to a `Result<Bytes, E>`.

**Fix.** Build the stream instead of cutting it. Known streams are hex literals from an independent implementation; families are built with `concat` (doubling a 279-byte decompression-bomb unit ten times gives a 285 KB stream); runs are `concat` doublings of `be_bytes(x)`; the header and code packing are modelled arithmetically from `le_bytes`/`shl`/`bit_or`; and "the declared length is an upper bound" is stated over a literal code stream with an if-ladder model of the expected prefix.

**Do differently.** Before designing claims, write the smallest document that uses each construct you plan to need and run `gen_evidence` on it: the checker accepts more than the generator emits. Keep every binder product under the budget by choosing narrower types (`U8`, `U16`) and a fixed literal for the rest, and put expensive domains (`U32`, `Bytes`) only in claims where they are the only binder.
