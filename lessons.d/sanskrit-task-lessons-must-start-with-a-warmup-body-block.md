---
category: Repo policy / workflow reminders
---

# Sanskrit task lessons must start with a warmup body block

Two new Sanskrit retrieval lessons began directly with a `Guided Practice`
block. The lesson parser accepted them, focused corpus evidence passed, and the
book generator rendered them. The shared-spine integration gate then rejected
both with `first body block must be warmup`.

For Sanskrit task lessons, make `## Warm-up` the first body heading even when
the task is intentionally model-free. The warmup can prepare the learner to
retrieve familiar material without showing the target-language answer. Keep
the response cue in the later guided-practice block and the answer check after
that cue.
