---
category: Testing & coverage
---

# Power ISA slash fields are ignored, not must-zero decode guards

**What went wrong.** A decoder treated fields printed as `/` in Power ISA
instruction diagrams as reserved bits that made a nonzero encoding illegal.
That rejected valid Microwatt aliases including record-bit variants of
`popcnt*`/`prty*`, nonzero `mfcr` FXM, high-multiply's unused OE position, and
operand fields on synchronization instructions. It also hid the distinct,
recognized L=1 `mfocrf`/`mtocrf` forms behind `IllegalInstruction`.

**Fix.** Apply the normative Book I section 1.3.3 rule: processors ignore
slash-marked fields, while reserved *values in defined fields* remain invalid.
Audit classification per bit, execute nonzero slash-field words on the pinned
RTL, and keep excluded but recognized instruction forms typed and atomic.

**Next time.** Do not infer decode validity from diagram typography alone.
Transcribe the architecture's field-rule prose first, confirm selector inputs
in the pinned RTL, then add per-bit classification invariance and real-core
witnesses before calling a decode surface exhaustive.
