---
category: Testing & coverage
---

# Rewording a lesson can push its narrated duration over the 300-second ceiling

**What went wrong.** A batch fixed stale pen-lift prose in 33 writing lessons.
Saying "without lifting, sweep on through the arch" takes more words than "lift
once", and two lessons that already sat near the limit grew past it: TE-S130
narrated at 301 s and UR-W07-pe at 316 s. The full human-language-data suite
caught them through its 300-second lesson-duration ceiling, but only after the
prose looked finished and every focused check had passed.

**Fix.** Trimmed the new wording in both lessons (298 s and 299 s), regenerated
the books, narration and modality records, and re-ran the full suite.

**Next time.** Before accepting reworded prose, run the duration check on every
lesson you touched and look at its headroom, not just pass or fail. A lesson
within a few seconds of 300 (TE-S128 is at 299 s) cannot absorb another
sentence; cut elsewhere in that lesson first.
