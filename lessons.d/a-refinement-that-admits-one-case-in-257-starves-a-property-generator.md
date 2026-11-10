---
category: Testing & coverage
---

# A refinement that admits one case in 257 starves a property generator that retries 100 times

Writing gf256 claims such as "for every `a` and every `x` with `a * x == 1`,
`inverse(a) == Ok(x)`" gave bounded-exhaustive suites that are perfectly fine
(65,536 tuples enumerated, 255 admitted) and property suites that failed with
`property-exhausted: filter precondition rejected all candidates`: a uniform
`U8 x U8` draw satisfies the refinement one time in 257, and the generator gives
up after 100 retries. The same thing happened to claims over a `U64` exponent
restricted to `e < 255` (random 64-bit words are almost never that small) and
to a two-exponent distinctness claim whose second draw is mostly "identical to
the first".

**Do instead:**

* state a claim so its refinement admits a large share of the generator's
  distribution: replace the second binder by a function of the first computed
  by an independent model (`inverse(a) == Ok(ginv(a))` over `a != 0`, with
  `ginv` built from the model multiplication) instead of searching for it;
* make the generator draw the values the refinement keeps (small `U64` values
  often, and a small partner for a small value) rather than loosening the retry
  limit, and gate that change per document so other packages' generated files
  stay byte-identical;
* a refinement may use models only, never operations: an admission predicate
  evaluated by a faulted adapter changes which cases a negative control admits.
  The generator refuses an operation inside a refinement.

The manifest records `discarded` per suite precisely so a refinement that
admits almost nothing is visible rather than passing vacuously.
