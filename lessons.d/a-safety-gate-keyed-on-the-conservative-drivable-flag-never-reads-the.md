---
category: Testing & coverage
---

# A safety gate keyed on the conservative drivable flag never reads the lessons announced by their core

**What went wrong.** Ten passes of issue #12070 built detectors for steps a
driver must not be told to do (read script, write, point, cover the page,
gesture) and a test, `drivable-writing-cues.test.ts`, that demanded zero of
them in every lesson the manifest marks `drivable` — `modality === "voice"`.
Meanwhile block-level modality had added a second flag, `coreDrivable`
(`coreModality === "voice"`), and the narration announces a lesson by THAT one:
"you can do this one in the car". A lesson with a detachable letters or
writing section is `sight` or `pen` in full, so it is never `drivable`, so the
gate never read it — but its core was announced, and its wrap-up said
"[PAUSE 3s] Read **नमस्ते**." with no stop guard. A security review found 166
such steps in 132 lessons. Every test was green, because each one asserted
exactly what it scanned.

**Fix.** The detectors moved into `src/drivable-instructions.ts` and became a
modality rule (`eyes-or-hands-step`), so the CLASSIFIER cannot call such a
core drivable; `tests/drivable-instructions.test.ts` gates the committed
manifest's `coreDrivable` lessons at zero, with a synthetic non-vacuity check
that undoes a deferred `[YOU READ: …]` in real lessons and expects the core to
flip.

**Do differently.** When a model grows a second, more permissive flag that a
consumer acts on, list every gate keyed on the old flag and ask which one the
CONSUMER reads. A safety check belongs on the flag that makes the promise to
the user, and better still in the rule that computes the flag, so no consumer
can read an answer the check never saw.
