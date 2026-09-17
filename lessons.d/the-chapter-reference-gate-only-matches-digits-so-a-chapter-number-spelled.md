---
category: Repo policy / workflow reminders
---

# The chapter-reference gate only matches digits, so a chapter number spelled out in words rots past it silently

`tests/chapter-references.test.ts` counts cross-chapter prose references with
`/[Cc]hapter (\d+)/g`. That regex sees `chapter 42` and does **not** see
`chapter forty-two`. A draft of `ML-C79-valare` opened with *"Since chapter
forty-two you have owned five qualities"* and every gate passed, including the
one written specifically to stop that sentence.

The words rot exactly as fast as the digits do. When chapter 42 splits — and the
Spanish, French and German tracks are all in the file's own comments because
they did split — the prose points at the wrong place either way, and the only
difference is that nothing will fail when it happens.

**Do not write the number in any form.** Name the thing: *"the chapter that gave
you good, big and small"*, *"the short-answers chapter"*, *"when you first met
them"*. That is what the gate's own docstring asks for, and the gate can only
enforce half of it.

Worth remembering more generally: a gate's regex is the floor of a rule, not the
rule. Passing it is not evidence of complying with it.
