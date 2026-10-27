# Layer 07an1 — Cortex-M APSR MSR Mask Repair

## 1. Status and authority

Independent review of the Cortex-M7 gate specification found that the merged
Cortex-M3 functional/gate decoders and Cortex-M7 functional wrapper collapse
three architecturally distinct APSR writes. The normative sources are the
*Armv7-M Architecture Reference Manual* and Arm's *Cortex-M7 Devices Generic
User Guide*. This repair precedes the Cortex-M7 gate implementation.

The second halfword of the admitted MSR (register) encoding contains a two-bit
mask in bits 11:10 and SYSm in bits 7:0. For APSR-family SYSm values 0 through
3 on a core with the DSP extension, the supported masks are:

| Encoding | Mask | Required write |
|---|---:|---|
| `APSR_g` | `01` | GE[3:0] only |
| `APSR_nzcvq` | `10` | N, Z, C, V, and Q only |
| `APSR_nzcvqg` | `11` | both groups |

Mask `00` is reserved. On a core without the DSP extension, setting mask bit
zero (the G group) is architecturally UNPREDICTABLE and is rejected by this
checked boundary. Non-APSR special-register writes retain their existing
bounded contract and require the established `10` encoding. A reserved mask,
unavailable-extension mask, or reserved-field neighbor returns the existing
typed unknown-instruction error without mutation.

## 2. Cortex-M3 behavior

Cortex-M3 does not have the DSP extension. Its functional and gate decoders
must therefore classify the masks as follows:

- an NZCVQ-only write updates N/Z/C/V/Q from source bits 31:27 exactly as
  before;
- G-only and combined NZCVQ+G writes return the typed unknown-instruction
  error because mask bit zero names an unavailable extension; and
- no mask changes any unrelated special register, mode, memory, or lifecycle
  state.

Both production decoders must derive the mask from the actual second halfword.
The existing two-argument `msr(rn, sysm)` encoder remains source-compatible and
emits `APSR_nzcvq`/mask `10`; a new `ApsrMask` enum exposes exactly the three
nonzero architectural forms without silently truncating arbitrary integers.
Encoding a G form is useful to tests and downstream DSP cores but does not make
that instruction executable on Cortex-M3.

The Cortex-M3 gate result must remain exact functional lockstep. Its mask
classification and write-enable selection flow through repository gate
comparators and mux/write-enable paths rather than host equality standing in
for architectural routing.

## 3. Cortex-M7 behavior

Cortex-M7 owns GE[3:0] beside the inherited Cortex-M3 N/Z/C/V/Q state. On an
executed APSR-family MSR:

- mask `01` copies source bits 19:16 to GE and preserves inherited APSR flags;
- mask `10` copies source bits 31:27 to inherited APSR and preserves GE; and
- mask `11` performs both updates from the same source value.

Every admitted APSR-family MRS composite read continues to return GE in bits
19:16 together with the inherited view selected by SYSm. IT-condition failure
skips the complete MRS/MSR effect. Reserved mask and operand encodings are
rejected before IT evaluation where the architecture requires encoding
validation.

The functional wrapper delegates mask `10` and MRS to the corrected Cortex-M3
transition, but must recognize masks `01` and `11` before that non-DSP decoder
rejects them. Those DSP-mask forms execute through one fallible complete-state
candidate and one commit so failure rollback restores the prior core plus GE
state. The later gate package must expose or add a fallible, noncommitting,
preallocated inherited transition so M3 candidate state and GE DFFs clock in
one composite commit.

## 4. Regression and completion gate

This repair is complete only when it provides:

1. exact encoding/classification witnesses for masks 00, 01, 10, and 11 over
   every admitted APSR-family SYSm alias and reserved neighbors;
2. Cortex-M3 functional and gate witnesses proving NZCVQ-only updates, typed
   atomic rejection of mask 00, G-only, and combined forms, and exact
   differential traces/state;
3. Cortex-M7 witnesses proving G-only, NZCVQ-only, and combined group isolation,
   every MRS composite alias, false-IT preservation, restore, and allocation-
   failure rollback;
4. source-compatible default encoders plus typed explicit-mask encoders whose
   output is asserted byte-for-byte and cannot alias out-of-range integers;
5. updated functional/gate/M7 documentation, changelogs, and the expansion
   backlog prerequisite; and
6. release tests, package-scoped Rustfmt, strict Clippy, rustdoc, affected BUILD
   integration, lessons validation, coverage evidence, latest-main ancestry,
   and independent exact-head review before publication.
