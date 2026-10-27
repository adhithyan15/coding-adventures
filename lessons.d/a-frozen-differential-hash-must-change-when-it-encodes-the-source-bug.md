---
category: Testing & coverage
---

# A frozen differential hash must change when it encodes the source bug being repaired

The PowerPC 601 status-authority specification initially required both the
official six-bit `sraw` carry edge and byte-for-byte preservation of all 239
legacy Python-oracle hashes. One frozen vector used source `0x80000000` with
count 63. The legacy model clamped the count to 31 and cleared CA; the official
manual requires counts 32 through 63 to copy the source sign into CA. Correcting
the architecture therefore necessarily changed that vector's full-state hash.

When a differential corpus contains the behavior under repair, preserve its
shape and every unrelated expectation, but source-correct the affected vector
and state the exception explicitly. A blanket "all hashes unchanged" promise
is valid only after checking that no corpus vector crosses the repaired edge.
Keep a new primary-source-derived test independent of the legacy oracle so the
updated hash cannot become circular evidence for the fix.
