---
category: Testing & coverage
---

# Do not discard an entire line when metadata shares that line with scored content

The first CodeQL fix filtered every line beginning with speaker metadata, but those lines also contained the scored utterance, so the listening word counts fell below their bands. Remove only the finite set of exact metadata tokens with `replaceAll`, preserving every character of adjacent scored speech, and rerun the semantic source-band test after changing counting helpers.
