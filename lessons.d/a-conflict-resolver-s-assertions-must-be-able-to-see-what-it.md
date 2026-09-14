---
category: Testing & coverage
---

# A conflict resolver's assertions must be able to see what it discarded

2026-09-13.

A script resolved an additive CHANGELOG conflict by extracting lines starting
with `- ` from each side and concatenating them. That is right for a file of
one-line entries, and it silently ate a 90-line prose section — `###` headings,
paragraphs and a code fence — from a file whose newer entries have that shape.

It reported success. Its assertions checked that every *bullet* survived, and a
bullet filter cannot see a paragraph it never collected. The check and the
transform shared the same blind spot, so agreement between them proved nothing.

`resolve_changelog_generic.py` in the scratchpad is the replacement. It keeps
each side VERBATIM and checks the result two ways that a dropped paragraph
cannot satisfy:

- every non-blank line of both sides appears in the output, compared line by
  line rather than through a filter;
- the output is no shorter than the conflicted file minus the marker lines.

The general rule: when a transform selects a subset, do not verify it with the
same selector. Verify against the whole input — by length, by line membership,
by anything the selector is not.

This was caught by eye, on a file that was about to be pushed. The version that
merges is the version nobody re-read.
