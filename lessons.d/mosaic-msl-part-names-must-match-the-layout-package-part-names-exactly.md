# Mosaic .msl part names must match the layout/package part names exactly (silent style drop)

A stylesheet (`.msl`) styles named `part`s; the emitter matches each `part X {...}`
block to a part of the same name in the layout (`.mll`) / resolved package composition.
If a part name doesn't match, the emitter **silently drops** those styles — the element
renders with NO styling, no error.

`Grid.light.msl` was authored against the legacy monolithic-Grid naming
(`sheet/cell`, `sheet/header-cell`, …) removed in U29-X1, while the shipped
`mosaic-pkg-grid` composition (and `Grid.dark.msl`) use flat names (`cell`,
`header-cell`, `header-row`, `data-row`). Result: the light-theme grid cells had
no borders/background → **no gridlines**. It stayed hidden because the light theme
was **dead code** (rendered by no demo) until the web dark/light switcher (PR #7338)
finally exercised it.

Lessons:
- When you light up a previously-dead variant/theme, VERIFY IT ACTUALLY RENDERS
  (drive it), don't assume parity with the working variant — the parallel file may
  have drifted.
- Keep sibling stylesheets' `part` names identical to each other and to the
  layout/package parts. A quick guard: `diff <(grep '^\s*part ' X.dark.msl) <(grep '^\s*part ' X.light.msl)`.
- Silent-drop-on-mismatch is a sharp edge in the emitter; a warning for unmatched
  `part` blocks would have caught this at build time (possible follow-up).

Discovered: 2026-07-02, light-theme grid had no gridlines after PR #7338 shipped the switcher.
