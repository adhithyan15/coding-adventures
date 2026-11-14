---
category: Testing & coverage
---

# A Warrant model written to cover the exhaustive domain must also cover everything the random generators draw, or the property and fuzz suites fail an honest subject

The first `zeros(n)` model of the WX-G fixture was an `if` ladder for `n <= 8`, because the
exhaustive `Bytes` domain holds buffers of 0 to 8 bytes. The exhaustive suites passed, then
the property and fuzz suites failed an *honest* subject: they draw random `Bytes` of up to 64
bytes, and the ladder's fall-through arm gave the wrong answer. The model was silently wrong
outside the domain it was written for, and nothing said so until the random suites ran.

Write a model that is correct for every length the generators can draw (here one block of
2^i zero bytes per set bit of `n`, correct to 255, and the generator draws at most 64), and
say in a comment where it stops being valid. Check the generator's range (`src.bounded(65)`
for `Bytes`), not just the enumerated set.
