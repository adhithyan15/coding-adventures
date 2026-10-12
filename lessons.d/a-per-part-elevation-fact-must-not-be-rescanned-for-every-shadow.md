---
category: Rust
---

# A per-part elevation fact must not be rescanned for every shadow drop

An imported SwiftUI reporter put a supported-elevation search inside the loop
over shadow drops. Valid authored styles preserve repeated declarations, so
32 shadows made 1,024 predicate visits without elevation, or 1,056 with a
supported elevation at the end. Complete native suites passed but independent
review correctly withheld acceptance for this work amplification.

Compute the same fact once per expanded part and reuse its Boolean. Instrument
the real predicate with a private generic callback, using a no-op through the
public API. The actual red reported one semantic control passing and both work
controls failing; keep the negative receipt. Green controls count exactly N or
N+1 visits at 32 and 128 declarations, while comparing complete public report
fields and order. Preserve inset/standalone behavior and multi-part locality.
Use public IR for unsupported elevation, because the authored compiler rejects
that enum value; do not claim invalid source is a valid authored resource case.
