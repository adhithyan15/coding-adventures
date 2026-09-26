### Changed - an empty Study screen is the toolkit's EmptyState (#15440)

With no card queued, the Study screen used to draw the review card anyway: its
prompt read "No cards queued", a "Reveal answer" button sat under it, and the
five review actions (Undo, Bury card, Bury siblings, Suspend, Mark) stayed on
screen with nothing to act on. Now `pkg::mosaic-pkg-toolkit::EmptyState`
replaces the card and its actions, with a button back to the deck list
(`onShowDecks`, an existing event, so the adapter's 89-event contract is
unchanged). Session progress stays above it.

- New slots: `study-empty` (bool, default false), `study-empty-title`,
  `study-empty-message`, `study-empty-action-label`.
- Both `EngramApp.mll` and `EngramApp.touch.mll` change identically.
- The toolkit dependency moves to 0.14.0, which adds `EmptyState`.
- `package_compiles.rs` asserts both layouts use the toolkit component, and the
  XAML needle for the answer region takes the review card's new depth (it is
  now inside the `Else`).

