### Changed — Spanish's `sequence` numbers are renumbered on a clean 10-spaced run

- Every one of Spanish's 148 sequenced lessons is renumbered to **10, 20, 30 … 1680**,
  in the same reading order it already had. **Relative order is unchanged**, and so
  is every measurement derived from it: forward prerequisites 5, forward reviews 6,
  forward references 99, atoms 199/102. Byte-identical answers, different integers.
- **Why it needed doing.** HL09 step 2 had to fit chapters 7–18 into the 129 integers
  between 510 (chapter 6's end) and 640 (chapter 19's start), because chapters 19–33
  were already sequenced at 640–845. That forced a spacing of **2**. Gap census before:
  51 gaps of 2, 33 of 5, and a scattering of 3s and 4s. After: **147 gaps, every one of
  them 10.**
- **Chapter 7 now has room.** Its six lessons are still unsequenced pending the owner's
  ruling on their order, but the renumbering reserves **210 numbers — 21 slots — between
  chapter 6 and chapter 8** for six lessons plus the splits they will need. Previously
  the gap was 10, which would have forced a second renumber the moment chapter 7 landed.
- Safe by construction, and verified rather than assumed: the security review of #10047
  confirmed **nothing consumes a sequence's absolute value** — every comparison in
  `ramp.ts`, `book.ts`, `modality.ts`, `hash.ts` is relative, and the only absolute
  predicate is `curriculum.ts`'s `Number.isInteger(sequence) && sequence > 0`. The values
  are persisted verbatim into three generated artifacts, so this is a regeneration event,
  and the byte-exact `--check` CLIs fail loudly rather than silently on a stale one.
- Diff shape confirms the claim: lesson files changed **only** in their `sequence:` line,
  and the 21 regenerated book chapters changed **only** in their `canonical-source-hash`
  comment. No rendered content moved.

