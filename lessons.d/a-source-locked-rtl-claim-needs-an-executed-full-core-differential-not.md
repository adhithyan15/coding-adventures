---
category: Testing & coverage
---

# A source-locked RTL claim needs an executed full-core differential, not line-reading alone

**What went wrong.** A Power ISA implementation was reviewed against pinned
Microwatt source lines and extensive hand-authored tests, but three exact RTL
behaviors were still transcribed incorrectly. The `sradi` decoder used divided
nine-bit-looking selector values instead of Microwatt's ten-bit 826/827
selectors. `rlwimi` rebuilt a 64-bit destination from the low word and lost the
old upper half. `mullw` was treated as a sign-extended low word even though the
RTL returns the complete signed 32-by-32 product and separately computes
word-width overflow. All three choices looked locally plausible and passed the
source-derived suite.

**Fix.** Build and execute the complete pinned Microwatt core with an official,
checksum-verified GHDL release. Seed every architecturally visible register,
run one instruction followed by the RTL's exact ATTN convention, and export
complete GPR/CR/XER/LR/CTR state plus bounded-memory writes and hashes. Commit
the generated corpus with source, tool, harness, generator, seed, row-count,
and content hashes, then consume it independently through both a
manifest/schema validator and the production functional simulator.

**Next time.** Treat reading RTL as specification discovery, not execution
evidence. Before claiming a source-locked hardware profile complete, run a
real pinned core over every decode family and each implementation-sensitive
boundary. Include nonzero upper destination halves, selector aliases, record
forms, and overflow-width edges: these are exactly where a plausible manual
translation can agree with itself while disagreeing with the hardware.
