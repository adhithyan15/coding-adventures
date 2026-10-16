---
category: Rust
---

# Literal geometry must retain intermediate rounding and incoming glyph frames

The first complete current-main diagram fixture migration7bec7277 compiled and
executed2001P/7F. One Ishikawa table rounded decimal112.04 instead of preserving
173-127*0.48's exact112.03999999999999; six Wardley full scenes still expected
old component centering/color, and four omitted annotation metadata keys.
The unfiltered full suite exposed behavior the focused field migration missed.

Retain the failed receipt and every whole scene/source/query assertion. Derive
literal floats from the declared arithmetic, including intermediate rounding.
Review complete current glyph origins/alignment/colors and metadata with the
new source semantics. Do not generate expectations from production layout output
or weaken whole equality to token presence or a floating tolerance.
