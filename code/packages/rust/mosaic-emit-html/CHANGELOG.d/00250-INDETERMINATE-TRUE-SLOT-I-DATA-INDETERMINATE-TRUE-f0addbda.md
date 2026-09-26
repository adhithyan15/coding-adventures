- `indeterminate: true` / `slot: i` → `data-indeterminate="true"` /
  `data-indeterminate="{{i}}"`. There is no HTML attribute for
  `indeterminate` — it's a JS DOM property — so the marker lets the
  host's hydration script set `el.indeterminate = …` imperatively.

10 new tests cover: bare inputs, `checked: true` keyword, `checked:
slot` data marker, label wrapping, `onToggle` data marker,
`indeterminate` data marker, bare radio, `group:` → `name=`,
`value:` → `value=`, `onSelect:` data marker.

## 0.3.0 — 2026-05-19

U29-K-html — UI29 kernel primitives in the pipeline emitter. Extends the
three-language pipeline path (added in 0.2.0) with the seven UI29 kernel
primitives beyond the original Box/Row/Column set.

