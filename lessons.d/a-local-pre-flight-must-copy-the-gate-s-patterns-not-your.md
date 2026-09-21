---
category: Testing & coverage
---

# A local pre-flight must copy the gate's patterns, not your memory of them

I keep a scratch pre-flight script that checks authored lessons before I run
the repo's real gates, because the full suite takes five minutes and the
book compile takes twenty. It is worth having. It is also only as good as
what I put in it.

Twice now it has been wrong in the same way.

**First**, it checked the duration field by reading `max_seconds` out of the
frontmatter. The gate uses `Math.max(declaredSeconds, computedSeconds)`, and
the computed figure is derived from the lesson's own word count. Checking the
declared value checks an author's estimate sitting next to the real number.
A lesson reached CI at 304s against a 300s ceiling.

**Second**, it had no standalone-book check at all. That gate forbids a
volume from pointing at the wider set, and I had been avoiding the literal
strings I remembered failing — `"the course"`, `"this course"`. Then I wrote
*"for the whole course of your Spanish"*, which contains neither, and which
the gate's actual regex matches:

```js
new RegExp(`\\b(?:the|this)\\s+(?:whole|entire|single|full)?\\s*(?:course|curriculum)\\b`, "gi")
```

The optional size word is right there. I had been carrying a paraphrase of
the rule in my head instead of the rule.

**The fix both times was the same: go and read the gate, then copy it.** For
the duration check I ported `estimateLessonDuration()` from `src/report.ts`
line for line into Python and **validated the port against the original on
eight existing lessons** before trusting it — all eight matched to the second,
and it then caught a 321s lesson after a late edit. For the prose check I
lifted the regexes out of `tests/standalone-book.test.ts` verbatim rather
than writing my own.

Two rules fall out of this:

1. **A mirror that has not been diffed against its original is a guess.** If
   you port a gate's logic, run both over real inputs and compare outputs
   before you rely on it. The port is cheap; the validation is what makes it
   worth anything.
2. **Never encode your memory of a rule.** Open the file that enforces it and
   copy the predicate. A paraphrase drifts in exactly the cases that matter —
   the ones you did not think of when you paraphrased.

The failure mode is quiet, which is what makes it expensive: a pre-flight that
passes everything tells you nothing, and it feels identical to a pre-flight
that is working.
