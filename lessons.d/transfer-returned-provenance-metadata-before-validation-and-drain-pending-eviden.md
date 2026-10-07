---
category: Security boundaries
---

# Transfer returned provenance metadata before validation and drain pending evidence iteratively

The second CV02 security review found that the scheduler cloned a pass's
returned metadata before calling checked recording. A 100,000-key payload with
max_events=0 caused a 7.47 MiB copy before rejection. A small-stack child
process reproduced an abort for 65,536-level metadata, during cloning or when
an early error recursively dropped the remaining returned events.

Consume contributions by value. The checked log owns and safely disposes each
rejected argument; the scheduler explicitly drains remaining metadata through
dispose_metadata on an error or absent program identity. Keep the public
Contribution type free of Drop so existing callers can still move its fields.
Validation after a recursive clone is too late to protect an evidence boundary.
