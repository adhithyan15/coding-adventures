### Changed — Qt no longer drops `flex-grow`

The Qt emitter now lowers `flex-grow` on a direct `Row` / `Column` child to
`Layout.fillWidth` / `Layout.fillHeight` (UI21 §2.2). Engram's one Qt
occurrence, `deck-option-button`, is such a child, so its Qt drop report
falls from 40 to 38 (one per layout variant). The `(Qt, "flex-grow")` pin
left `ALLOWED_STYLE_DROPS` in `tests/native_complete_gate.rs`; the inverse
ratchet would otherwise fail on the now-stale entry.

The same change binds the `app-shell` `Column` to the window on Qt: the
wrapper and its content layout are anchored, the padding becomes anchor
margins, and the root `Item` reads the shell's implicit size. With no child
growing into the height yet, the shell keeps its content height. That is
what makes EN-6's `content-scroll` (#18477) scroll on Qt once it lands.
