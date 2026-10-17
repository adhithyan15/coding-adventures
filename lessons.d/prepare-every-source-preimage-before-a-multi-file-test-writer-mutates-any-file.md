---
category: Security boundaries
---

# Prepare every source preimage before a multi-file test writer mutates any file

CLOC40's unexecuted CLI test writer checked clean status and released review
holds, but did not pin the exact source HEAD or both physical input hashes. It
also wrote the first file before reading and validating the second. Independent
review found that a moved clean checkout or second-file anchor failure could
leave a partial, mismatched test edit. No source writes occurred before repair.

Pin the exact current HEAD and both source preimages, validate every anchor and
build all proposals in memory, then recheck the preimages and save a write plan
before mutation. Keep the writer separate from native execution and record its
actual execution rather than treating a prepared helper as completed evidence.

Use a saved UTF-8 Python helper for source edits containing Rust quotes. A prior
PowerShell Python one-liner lost nested quotation marks and failed to parse;
that failure occurred before any file write and was corrected before tests.

The later unexecuted docs writer also needed a stronger review-verdict guard:
searching only for PASSED could accept NOT PASSED and did not require the full
execution HEAD in the review opening. Require the positive SECURITY REVIEW
PASSED marker, reject negative header markers, and bind the exact full source
HEAD within the opening review text in addition to path and fingerprint checks.
