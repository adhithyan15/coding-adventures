---
category: Testing & coverage
---

# A spec's guaranteed-by-construction coverage claim needs a test that enumerates it

VER04j section 2.7 said the signed edge domains "guarantee `max`, `max-1`,
`min`, `min+1`, `-1`, `0`, `1` are present, so each fault is catchable by
construction". Implementing the domain formula exactly as section 2.6 gives
(`wrap_S(E_U(w))` plus `-1..=-255`, with the specified sizes 527, 559, 623)
showed the sentence is false for signed `max-1` and `min+1`: `0x7ffe` and
`0x8001` are neither a power of two, nor `MAX ^ (1 << i)`, nor in `-255..=-1`.
The specification's *sizes* were right and its *guarantee* wrong; nothing
caught that until a test asked "is each boundary in the set?".

Do differently: when a spec (or a README) says a generated domain *contains*
something, write the test that enumerates the domain and asserts exactly that
list, before trusting the sentence for the design of a fault. Then correct
whichever of the sentence or the formula is wrong, in the spec, in the same
change. Here the fault that mattered (`off-by-one-range`) needs the *source*
type to contain `T.max + 1` and `T.min - 1`, which it does; the sentence was
rewritten to say that and the enumeration test pins it.
