---
category: Testing & coverage
---

# A historical geometry oracle must use the retired constructor

The timeline migration's independent numeric advice and root fixture assumed
the old period width had a minimum24 clamp. Reading the immutable22638c8b
constructor instead showed raw `width - spine_x - 52`: the old padding1000,
margin50,canvas640 layout had width-568. The24-wide payload is a useful forged
inner-zero control, but it does not preserve that old outer-negative refusal.
Keep both full payloads, label them accurately, and preserve the original
expression order even when an alternative happens to produce identical bits
at the inline precision witness. The earlier model/report remains historical.

Run835f9767 passed258 affected tests and two empty doctests, but its painter
strict check failed on Clippy some_filter in a new expected-Scene helper.
Its aggregate acceptance remains false. Replace Some(...).filter with the
equivalent then_some expression without an allowance; rerun affected suites
and both strict checks on the corrected final source. Independent equations,
test success and review advice each have limits; none substitutes for the
actual old source or the final warnings-denied command.
