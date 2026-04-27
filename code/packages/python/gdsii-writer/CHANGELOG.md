# Changelog

## [0.1.0] — Unreleased

### Added
- `GdsWriter`: streaming binary writer; context-manager support.
- Library wrappers: `write_header()` (HEADER/BGNLIB/LIBNAME/UNITS) and `write_footer()` (ENDLIB).
- Cell wrappers: `begin_structure(name)` / `end_structure()`.
- Element emitters:
  - `boundary(layer, datatype, points)` — auto-closes polygon if last != first.
  - `path(layer, datatype, width, points)` — wire.
  - `sref(name, x, y, angle=0, mag=1, reflect=False)` — instance, with STRANS/MAG/ANGLE when needed.
  - `text(layer, text_type, x, y, text)` — pin label.
- 8-byte fixed-point real conversion per Calma spec (signed fraction + 7-bit excess-64 base-16 exponent).
- User-unit / DB-unit handling (default: 1 µm user, 1 nm DB).

### Out of scope (v0.2.0)
- AREF (array reference) — emit as multiple SREFs for now.
- GDSII reader (parsing).
- Properties (PROPATTR/PROPVALUE).
- BOX records (deprecated).
- OASIS (next-gen format).
