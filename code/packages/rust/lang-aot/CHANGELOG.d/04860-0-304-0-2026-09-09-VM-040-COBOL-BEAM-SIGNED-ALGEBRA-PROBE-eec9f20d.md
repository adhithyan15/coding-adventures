## 0.304.0 — 2026-09-09 — VM-040 COBOL BEAM signed/algebra probe

Declared `Beam` for the next four COBOL-60 matrix rows: signed numeric
overpunch (`SUBTRACT` into a `S9(2)` receiver, DISPLAY overpunches the
negative units digit), alphanumeric MOVE + comparison (a truncating MOVE
followed by a `GREATER` string compare), `COMPUTE` exponentiation (a constant
integer exponent unrolled to `mul`), and nested `COMPUTE` division (`A / B +
C` at the scale-12 intermediate precision the oracle uses). New test
`portable_text_stdout_cobol_beam_signed_and_algebra` runs all four on real
`erl`; COBOL now declares 12 of its 58 rows on `Beam` (four literal/output,
four control/rounding, four signed/algebra), 418 total declared cells.

This probe found two real `iir-to-beam`/`ir-to-beam` defects — not lang-aot
bugs — fixed as part of this slice (see those crates' changelogs for detail):
the alphanumeric MOVE's truncating slice needed `str_slice` support
(`iir-to-beam` 0.9.0), and the nested-division row exposed a large-positive-
integer sign-encoding bug in the underlying BEAM bytecode encoder
(`ir-to-beam` 0.3.0) that silently corrupted a COBOL `COMPUTE`'s scale-12
intermediate into a negative number. All 12 Oct, 26 Nib, and the eight
already-declared COBOL BEAM programs pass again after both fixes.

