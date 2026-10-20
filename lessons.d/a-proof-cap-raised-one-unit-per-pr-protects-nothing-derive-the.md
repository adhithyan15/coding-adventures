---
category: Compiler / VM / language pipeline
---

# A proof cap raised one unit per PR protects nothing; derive the bound from the property it guards

**What went wrong.** The ALGOL compiler's bounded `sign`-rooted transform
accepted additive offsets through a hand-written `match` of `1.0 => 1`,
`2.0 => 2`, … plus a `count <= N` check. Between 2026-10-14 and 2026-10-18,
forty-four consecutive PRs each raised `N` by one, added one more match arm,
one more matrix row, version bumps and spec wording. None of them asked what
the cap protected. It protected nothing: the outer runtime-real selector rule
already accepted finite static terms of any size, and `exp` chains under the
same proof could already overflow at an offset of one.

**The fix.** One named constant derived from the actual invariant. The proof
needs every path value to stay an exact binary64 integer. Since `sign(x)` is
`-1`, `0` or `1`, a checked total of `2^53 - 1` whole units is the real
bound (`MAX_SIGN_OFFSET_UNITS`). The tests now pin the edges: `2^53 - 1` is
accepted, while `2^53`, `2^53 - 1 + 1`, `0.5` and variable terms are rejected.
A matrix row at the maximum also showed that the portable real formatter
switches to six-digit scientific form at that magnitude. That is portable but
not exact, which is worth knowing before claiming "exact output".

**Next time.** If a change only bumps a literal limit, stop and write down the
property the limit guards. Then either derive the limit from that property, or
show that removing it breaks a test. If you are about to open the second PR
that changes the same constant by one, the loop itself is the bug.
