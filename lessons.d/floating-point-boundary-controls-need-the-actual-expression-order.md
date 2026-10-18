---
category: Testing & coverage
---

# Floating-point boundary controls need the actual expression order

The unexecuted timeline event-card tests initially used the wrong rounded
offsets at padding2^55 and mislabeled x2^60/width80 as an inner-frame witness.
Independent review caught both: successive additions produce periody+72,
linky+120, eventy+136 and spiney+64; the isolated width witness needs x2^59.
An arrow at exactly x2^56 also straddles two ULP sizes and remains valid, while
shifting it32 units makes its base corners coincide. These were fixture errors,
not production failures or executed acceptance.

Model each consumed operation in order, assert the represented outer/inner
endpoints before testing admission, and distinguish power-of-two boundaries
from nearby same-binade coordinates. Preserve an exact typed positive control
alongside each collapse/refusal. A failed patch context also leaves the entire
patch unapplied; inspect the corrected diff before committing.

The migration helper also refused twice before source mutation: a one-line
old anchor had two different replacement newline encodings, then a short width
replacement was ambiguous during inversion after other insertions. Select the
physical newline encoding once and include the unchanged coordinate context in
both inverse anchors. Preserve the refused helper versions and write the whole
candidate/recipe before applying any file.
