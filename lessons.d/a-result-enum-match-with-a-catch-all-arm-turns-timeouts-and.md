---
category: Testing & coverage
---

# A result-enum match with a catch-all arm turns timeouts and out-of-memory into a clean report

`fuzz_target` in `code/packages/rust/test-bootstrap/src/fuzz.rs` matched each
target execution as `Pass`, `Crash`, and `_ => {}`. `Timeout` and `Oom` fell
into the last arm, and `FuzzReport` had nowhere to put them, so a target that
timed out on every input produced `crashes == []`, exactly what a clean target
produces. Every consumer that asked "were there crashes?" read it as "no
problems found".

The same shape appeared three more times in the same audit: `check_property`
with `num_tests == 0` returned `Pass` having run nothing, a declared hard
benchmark budget with no counter attached reported `Pass`, and a shrinker that
returned a non-failing value was reported as the minimal counterexample.

What to do differently:

- Match result enums exhaustively, with no `_` arm, so adding or ignoring a
  variant is a compile error rather than a silent drop.
- Give every outcome somewhere to land in the report (a count is enough), and
  expose one honest predicate (`FuzzReport::is_clean()`) instead of leaving
  callers to infer success from the absence of one field.
- "Nothing happened" is not "nothing was found": a run with zero iterations,
  zero cases, or an unmeasured budget must say so, and must not read as a pass.
- Re-verify anything a caller supplies that you then report (a shrunk witness)
  before reporting it.
