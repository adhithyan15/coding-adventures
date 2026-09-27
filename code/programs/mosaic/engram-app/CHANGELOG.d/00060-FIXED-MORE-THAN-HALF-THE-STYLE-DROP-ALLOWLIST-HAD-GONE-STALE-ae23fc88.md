### Fixed — more than half the style-drop allowlist had gone stale

`ALLOWED_STYLE_DROPS` pins the style properties each backend is currently
allowed to drop. The gate around it was one-directional: it catches a **new**
drop, and could not catch a pinned one that had **stopped** happening.

That asymmetry is not cosmetic. An entry whose gap was fixed stays on the list,
and from then on it is a standing licence — a later regression that re-drops the
same property on the same backend is allowlisted and passes green, which is
precisely the state this file exists to prevent.

The existing `seen_any` backstop does not cover it. It asks whether *any* drop
was reported, so it stays green while twenty of twenty-one entries rot.

`no_pinned_style_drop_has_silently_been_fixed` now requires every pinned pair to
still be observed. **On its first run it found 11 of the 21 stale:**

| backend | properties no longer dropped |
| --- | --- |
| Compose | `max-width`, `justify-content`, `align-items`, `align`, `border-bottom-width`, `border-bottom-color`, `flex-wrap` |
| XAML | `border-bottom-width`, `border-bottom-color` |
| SwiftUI | `border-bottom-width`, `border-bottom-color` |

Spot-checked against the emitters rather than taken from the test: Compose's
`max-width` is `Modifier.widthIn`, tagged `#14833` in `mosaic-emit-compose`, and
XAML's per-side borders lower to `BorderThickness` with their own tests. The
gaps were real and were closed; nothing brought anyone back to this list,
because fixing an emitter does not touch this file.

All eleven are deleted. The ten that remain are genuinely still dropped, and
most of them are the entries that need no mapping at all — `border-bottom-style`
is `solid` on every backend that has only solid strokes.

No production code changed. The eleven mappings this records were landed by
other work; what changed here is that the ledger now matches reality, and cannot
drift from it silently again.

