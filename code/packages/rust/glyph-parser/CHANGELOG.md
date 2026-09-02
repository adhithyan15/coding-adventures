# Changelog

## Unreleased

- Initial implementation of FNT02: glyph outlines from `glyf` and `loca`.
  - `loca` in both short and long formats.
  - Simple glyphs: run-length flags, short/same-or-positive delta packing, and
    the implied on-curve points TrueType omits between consecutive controls.
  - Composite glyphs flattened recursively: plain offsets, uniform scale, x/y
    scale, 2x2 transforms, nesting, and anchor-point placement.
  - Malformed fonts are refused rather than trusted: a self-referential
    composite hits a depth limit instead of exhausting the stack, and a total
    component budget bounds the work a single glyph can demand. The depth cap
    alone does not: a tree nine levels deep and eight wide stays inside it
    while asking for 8^9 -- over 134 million -- component resolutions from a
    few hundred bytes. A test builds exactly that font and asserts it is
    refused in milliseconds.
- Verified against fontTools over **every glyph** in Inter and two Noto faces
  (4,161 glyphs, 2,104 of them composite, both `loca` formats), plus a
  synthetic font for the composite features no shipping font in this repository
  uses. The oracle was mutation-checked: inverting the `loca` halving, reading
  the wrong delta flag bit, and rounding half away from zero each make it fail.
