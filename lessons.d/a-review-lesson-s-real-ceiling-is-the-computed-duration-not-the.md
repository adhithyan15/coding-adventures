---
category: Repo policy / workflow reminders
---

# A review lesson's real ceiling is the computed duration, not the declared max_seconds

Three of HL-C439's five Hindi retrieval pages failed `validate` at 495s, 502s
and 358s against a 300-second error threshold, with `max_seconds` declared at
240 and 250. Lowering the declaration would have done nothing:

```
effectiveSeconds = max(declaredSeconds, computedSeconds)
computedSeconds  = ceil((words/2 + 8*prompts + 4*repeatCues + pauseSeconds) * 1.15)
```

The declaration is a floor, never a cap. Only the prose moves the number.

Two of those terms are easy to trip blind.

**A repeat cue is any of `repeat | again | twice | three times`, anywhere, at 4
seconds each.** A Hindi farewell page pays for its own gloss, because **फिर**
means *again*.

**A prompt is any line containing `?` OR beginning with one of twenty
imperatives** — `say, repeat, answer, choose, write, read, translate, recall,
practice, produce, ask, respond, point, cover, rebuild, listen, speak, try` —
counted after the list marker is stripped, at 8 seconds each. That is a
per-LINE test on wrapped prose, so a paragraph that happens to break before
`Recall starts holding...` or `point.` costs 8 seconds for a line that asks
nothing. Reflowing one sentence moved a lesson by 9 seconds, and rewording
`recall` to `memory` moved another by the same.

**The practical budget for a retrieval page is about 440 words** with three
prompt lines and one repeat cue. Write to that budget; trimming into it took
six passes per lesson.

Measure with `estimateLessonDuration` from `dist/report.js` directly rather
than re-running `validate`, which takes 30 seconds to say the same thing.
