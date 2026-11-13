---
category: Testing & coverage
---

# A Warrant claim must be total: a filter on a binder starves the property suite, so map arbitrary values onto the domain instead

**What went wrong.** The first draft of `logic-gates.warrant` restricted binders with refinements (`n >= 2 and n <= 8`, `shr(k, n) == 0u8`, `isjunk(j)`). The exhaustive suite filtered them fine, but the generated random property suite retries a refinement 100 times per sample and then fails with `property-exhausted`: "filter precondition rejected all candidates". A Nat in 2..=8 is 11% of the Nat domain and a U8 that is one of four chosen junk values is 1.6%, so the suite failed at a fixed seed. Separately, `Bytes` cannot be indexed in an evidence body and `take`, `drop` and `replicate` are not supported by the evidence generator yet (W0701), so a variable-length vector cannot be sliced either.

**Fix.** Make every claim total. Map an arbitrary Nat onto a width (`wd(n) = 2 + n mod 7`), an arbitrary U8 onto the set of non-bit values (`junk(x) = 2 + x mod 254`, which covers all 254 of them), and use `Bool` binders for bits. Build each vector from an integer by concatenating `be_bytes(bit_and(shr(w, i), 1u8))`, and each expected output by concatenating blocks of zeros built by doubling. Keep refinements only where the refined set is most of the domain (`a > 1u8 or b > 1u8` over U8 x U8).

**Do differently.** Before writing a refinement, ask what fraction of the generator's domain it admits; below about 10% rewrite the claim as a total map. When a claim fails only in the property or fuzz suite, suspect the filter, not the implementation. Check which built-ins the evidence generator supports (not just the type checker) before designing a model around them.
