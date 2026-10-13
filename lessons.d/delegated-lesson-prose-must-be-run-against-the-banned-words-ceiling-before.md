---
category: Testing & coverage
---

# Delegated lesson prose must be run against the banned-words ceiling before it is accepted

**What went wrong.** A batch of chapter-payoff fixes was split across four
sub-agents, each adding recall prompts to lesson prose. Each agent ran the tests
for its own tracks: corpus, chapters, literal-markup and integration. None ran
`tests/banned-words.test.ts`, which pins a corpus-wide ceiling on "just",
"simply", "obviously" and "as you know" in learner-facing prose. Four new
occurrences went in, in Hindi, Persian, Russian and Urdu, such as "the pieces
you just wrote" and "the formula simply means *goodbye*". Only the full suite
caught them, and only after the batch was committed.

**Fix.** Removed the four words. The prose reads the same or better without
them.

**Next time.** When a brief asks another agent for lesson prose, name the
banned-words test in its validation list. Corpus-wide ceilings are invisible
to per-track test runs: any test that counts across all tracks must be run
by whoever writes prose, not left for the final full-suite pass.
