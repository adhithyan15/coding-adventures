## HL-C378 — an English word spelled like a target-language headword is not a forward reference

Chapter 421 authored `el actor`. Spanish `forward-language` immediately went
**367 → 371**, and all four new entries point at lessons that were merged long
ago:

| lesson | the flagged use |
|---|---|
| `ES-C301-papel` | "the part an **actor** plays" |
| `ES-C302-esquina` | the same phrase, quoted back |
| `ES-C345-persona` | "an **actor's** mask" — the gloss of *persōna* itself |
| `ES-C345-repaso-persona-objeto` | "held up in front of an **actor's** face" |

Every one of those is ordinary English teaching prose. None of them asks the
learner to read a Spanish word early. The detector flags them because *actor*
is spelled identically in English and Spanish, and it has no way to tell which
language a token in the body belongs to.

`continuity.ts` already documents the opposite blind spot — a word the course
never teaches is invisible, because nothing marks it as target language. This
is the same missing signal producing a **false positive** instead of a false
negative, and it will fire again for every Spanish headword that is an English
homograph: *real*, *sin*, *once*, *mar*, *pan*, *ropa*, *vista*, *hora*.

**The four lessons were deliberately NOT reworded.** "An actor's mask" is the
standard rendering of *persōna*; replacing it with "a performer's mask" would
degrade real teaching prose to satisfy a detector, which is fixing the
measurement rather than the thing measured. The snapshot records the rise and
the changelog names the cause.

**The fix worth making, when someone touches `continuity.ts`:** a use is only a
forward reference when the token is marked as target language — inside emphasis,
a code span, a table cell in a target-language column, or matching the lesson's
own script. A bare English word in running prose should not qualify, however it
is spelled. Until then, treat a `forward-language` rise that coincides with a
new cognate headword as noise, and confirm by reading the cited lessons.
