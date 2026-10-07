---
category: Security boundaries
---

# Charge metadata queue work before growing pending storage and sort only after payload validation

The independent CV02 review instrumented a public mutation call containing
100,000 metadata keys with max_work=2. The operation ultimately returned a work
error, but first collected and sorted a 1.6 MiB vector of borrowed fields.
The configured work allowance had not protected that intermediate operation.

Payload accounting now iterates borrowed map fields without sorting. It checks
keys before enqueueing child values and charges array/object queue work before
growing pending storage. Canonical encoding sorts only after full payload
validation, with collection work charged before materialization. Iterative
disposal of already-owned rejected arguments remains a separate unavoidable
cleanup cost, not a reason to allow uncharged validation buffers.
