---
category: Testing & coverage
---

# A piece cap does not bound a sequence strip's height once its groups are whole words

**Context:** human-language-data and script-ductus, drawing Devanagari
phrases (मम नाम) as a sequence strip of composed words, each word one group.

**What happened:** the design capped a phrase like any sequence strip, at
`MAX_SEQUENCE_PIECES` (10) letters and signs, assuming that kept figures
under the 1,801-unit tallest printed strip. Measuring the worst case showed
it does not: a group is a whole word, every word wraps its frames at six to
a row, and two-letter words of the letters with the most movements (औइ, औझ,
धऋ, औब) wrap to three rows each. Four such words fit in ten pieces and print
2,048 units tall; three print 1,571.

**Fix:** a second cap, `MAX_PHRASE_WORDS` (3), with a test that composes the
worst words and renders the strip. The test composes each word once (each
composition runs the ink checks, about 0.7 s) and lives under the filmstrip
ledger test's existing per-test budget; written naively in the headline-word
suite it took 6.7 s and failed vitest's 5 s default.

**Do differently:** when a strip's groups change from letters to something
bigger, re-measure height with the worst real entries before trusting an
existing cap, and keep the measurement as a test. Compose expensive fixtures
once per test.
