---
category: Rust
---

# Automatic merge success does not preserve coordinate-relative suffix offsets

The first integrated focus run compiled and passed the three native boundary
controls, but failed three Mermaid controls. An incoming component-suffix parse
had automatically merged a relative coordinate index into a slice of the
original quoted-label string. The ordinary node reader also searched for a
bracket inside a quoted label, breaking a quoted pipeline parent lookup.

Keep the active-quote label reader, then carry the coordinate remainder into
the suffix parser. Retain invalid-suffix refusals while admitting the incoming
label-offset grammar. Automatic Git resolution is not semantic acceptance.

The new annotation-theme test itself initially placed dialect colors directly
under themeVariables. The established and incoming dialect contract requires
themeVariables.wardley. Correct that source before changing production theme
authority, and preserve the original failed receipt as a mixed failure.
