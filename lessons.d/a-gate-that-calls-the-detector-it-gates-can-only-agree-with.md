---
category: Testing & coverage
---

# A gate that calls the detector it gates can only agree with it

**What went wrong.** `human-language-data` defers a scored question that needs a
pen instead of asking it at the wheel. The verdict came from
`activityPromptNeedsEyesOrHands`, which reused the PROSE writing detector, and
that detector is tuned to under-fire on purpose (in prose most writing verbs are
mentions). The corpus gate over the committed narration judged each asked
question with `activityPromptNeedsEyesOrHands` too. A security review found 51
questions still asked to drivers ("From bachchā, write the taught word.",
"Write is that so, without pinyin.") with the gate green. The gate was never
going to fail: whatever the detector missed when it wrote the narration, it
missed again when it checked it.

**Fix.** The prompt check now fails safe: any hand verb as a whole word defers
the question unless it is one of five closed, excused shapes. And a second gate
shares nothing with the generator but the committed output: a plain whole-word
scan for the verbs, with its own copy of the verb list and an exact
(lesson id, text) allowlist where each entry has a reason, and stale entries
fail.

**Do differently.** When a gate checks a generator's output, at least one of
its assertions must use a different and simpler method than the generator does.
A plain scan with an explicit allowlist works. Also match the detector's tuning
to what it reads: a check tuned not to cry wolf on prose is the wrong check for
a context where every match is a request. That is where to fail safe.
