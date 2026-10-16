---
category: Rust
---

# Timeline title fixtures must respect hash comment boundaries

A new whole-IR title-decoy fixture expected quoted #badbad to remain in a timeline
title. The unchanged TITLE_STATEMENT regex excludes # even inside apparent JSON
quotes; COMMENT then owns the suffix. Parser1cdb passed550 tests but this one
expected-title assertion failed; strict did not run. Preserve the complete old
source and all configuration/IR assertions, correct only the title to its
independently declared grammar prefix, and add a hash-free red-color decoy whose
entire title is semantic data. Do not change the grammar, infer an oracle from
actual production output, or relabel the first combined receipt as accepted.
