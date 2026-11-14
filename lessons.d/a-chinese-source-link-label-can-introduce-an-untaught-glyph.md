---
category: Testing & coverage
---

# A Chinese source link label can introduce an untaught glyph

While narrowing `ZH-C03-si`'s floor-number example to a sourced Shanghai lift,
the new dictionary link label printed 死. The learner had not been taught that
character, so the script-closure test correctly reported new debt even though
the link itself was only a citation. The printed label was changed to the
already-taught romanization *sǐ*; the linked dictionary entry stayed exact.

Citation labels are learner-facing lesson text. When adding a source, check its
visible label for untaught script before generating books, and run the
track-specific script-closure measurement alongside the duration check. Do not
exempt source labels or loosen the closure gate to make a citation pass.
