### Fixed — more than half of SwiftUI's reported style drops were not drops

Engram's SwiftUI emission reported **40 style drops, 22 of them `gap`, and all
22 were false.** `$style.app-shell` was reported to drop `gap: 18` while the
emitted Swift opened `VStack(spacing: 18)` on that very part.

SwiftUI takes spacing at view-**construction** time, and the emitter has been
reading it from the part's own style for some time. The *reporter* had not
caught up: it derived drops by running the modifier chain and collecting what no
match arm handled, which is the right method for almost every property and blind
to this one. It now takes the layout, the way the Compose reporter already does
for `justify-content`/`align-items` (#14834).

Engram now reports 18, and no `gap`. `(SwiftUI, "gap")` is off
`ALLOWED_STYLE_DROPS` here and in the two packages that pinned it —
`mosaic-pkg-deck-stats` and `mosaic-pkg-collection-actions`, the only two of the
twelve that did. Which three were stale was measured by running all thirteen
gates, not inferred.

The entry's own comment already described the fix — "the container emitter
reading the part's style before emitting children" — which is what
`container_spacing` does. The mapping had landed and nothing brought anyone back
to the list, exactly as with the eleven stale pins found earlier; that is the
drift `no_pinned_style_drop_has_silently_been_fixed` exists to catch, and it
caught this one.

A gap a `Box`, `Stack` or `HostScroll` genuinely discards is still reported —
`container_spacing` refuses those by name, deliberately — as is a `Row` inside a
`HostTable`, which lowers to `HStack(spacing: 0)` to match
`border-collapse: collapse`.

