## [Unreleased] — preserve checked layout across title margin import

Retain the checked wrapper, constraint indexes and padding/enclosure guards
while applying the incoming title-height expression inside supported layout.
Include caller-owned titleTopMargin in finite inclusive0..=65536 admission.
Preserve both default config literals and all original assertions.

Executed public red:40invalid cells fail exact typed admission;80 complete
layout/config/input translation cells pass. The guard then passes both tests.
Closed main import proof reconstructs4007pure/7overlapping objects, including
the single explicit title-height conflict resolution. Final full affected
suites/strict and independent review remain required. Miri DEFERRED/PENDING.
