---
category: Testing & coverage
---

# A verb scan cannot see a question whose answer is written; read what the activity accepts

**What went wrong.** `human-language-data` defers a scored question a driver
cannot answer aloud. After two rounds the check was a fail-safe scan for hand
verbs (write, copy, trace, point …) with a closed list of excuses, and an
independent gate scanned the committed narration for the same verbs. A second
security review still found eleven questions asked at the wheel in kept sections
of core-drivable lessons. Most named no hand verb at all: "Refuse an offer and
then leave, in two written lines.", "Refuse in one word, in Devanagari.", "Which
already-taught Gurmukhi consonant is na: ਨ or ਸ?", "Which taught character is
the Level 1 word female or woman?" (it accepts only 女). The request was in the
FORM of the answer, and both the generator and the gate looked only for a verb.
Four excuses were also wider than their examples: "Do it again and copy the
sign" passed as a third-person question because it opened with "Do", and "Copy
the tone marks" passed as imitating a sound.

**Fix.** The narration now judges the whole activity, not just its prompt
(`activityNeedsEyesOrHands`). It defers a question whose prompt asks for a
written form or names a script as the answer's form, one that offers a choice
between unromanized glyphs, one whose every accepted answer is script, and one
that grades a `-SCRIPT-` atom with a script answer. Each excuse is tied to its
real shape: a question must open a sentence that ends in "?", and a "mark" is
not a sound. The independent gate reads the same structural facts straight
from the committed JSON, using its own literals.

**Do differently.** When a check guards what a learner is asked to produce, look
at the expected answer as well as the instruction. The data already said what a
voice could not do: every accepted answer was in a script. For each excuse in a
fail-safe check, write the closest request that would get through it and add it
as a control. "Do …" fits a question shape and an imperative alike.
