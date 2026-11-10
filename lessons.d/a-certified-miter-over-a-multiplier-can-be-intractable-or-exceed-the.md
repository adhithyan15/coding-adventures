---
category: Rust
---

# A certified miter over a multiplier can be intractable or exceed the checker's work limit, so size test miters by measurement

While writing the bit-blaster faithfulness tests, the first draft proved
multiplier identities (`a*b == b*a`, `a*(b+c) == a*b + a*c`) at width 8 in a
debug build. One test ran for many minutes and then failed with
`drat_check::Rejection::ResourceLimit`: the proof was not rejected as wrong,
the checker gave up because the work exceeded its default `max_work`
(2^32 literal visits, a safety bound, not a verdict).

Measured afterwards (release build, `a*b == b*a`): the checker's work grows
about 10x per bit of width: 1.0e6 at 5 bits, 1.1e7 at 6, 1.2e8 at 7. The
three-variable distributivity miter is far worse (4.3e6 at 4 bits, 5.7e7 at
5), so it would pass the limit by 8 bits. Debug builds are roughly an order
of magnitude slower still. I did not
re-run width 8 to attribute the stall to a single identity; the sizes
above are what was measured.

What to do: size every certified miter by measurement. In this crate
multiplier identities stay at width <= 5 (three-operand ones <= 3) and
division identities at <= 4 in the default test tier; wider ones belong in
an `#[ignore]` tier or call `check_with_limits` with an explicit
`max_work` and say so. Never loosen the library's default limit to make a
test pass: a `ResourceLimit` rejection means "too big", and the tactic that
reports UNSAT only after the kernel accepts correctly degrades it to
Unknown.
