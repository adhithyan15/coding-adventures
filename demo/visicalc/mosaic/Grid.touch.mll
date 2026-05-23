// Grid.touch.mll — touch / mobile layout for the spreadsheet grid (UI30).
//
// The desktop layout pins the column-header row at the top of the
// scroll viewport via `sticky-header: true` — when the user scrolls
// the body, the headers stay visible. This works well on a desktop
// because there's plenty of vertical real estate; the sticky band
// only costs ~24 px out of a typically ~600 px viewport.
//
// On a phone the same band steals ~5-10% of the visible viewport
// for every scroll position, and worse — the horizontal-scroll pattern
// that lets desktop users see distant columns doesn't work as well
// on touch, where users naturally swipe through cells. The touch
// variant therefore drops sticky-header: false (= the default), so
// the header row scrolls away normally and the user gets the full
// viewport back as they scroll.
//
// What the touch variant changes vs. .desktop.mll:
//
//   1. sticky-header: true  →  sticky-header: false
//      (See above. The kernel Grid primitive treats `false` as the
//      default, so we could simply omit the prop — we make it
//      explicit here for readability + diff legibility.)
//   2. (everything else identical)
//
// The interface (Grid.mil) is unchanged — same 9 slots, same
// onNavigate emit. Tap-target sizing for individual cells lives in
// the .msl (a follow-up could add a Grid.touch.dark.msl that bumps
// row height to ≥44 px per Apple HIG); for v1 of the touch
// variant the .desktop.msl's styling is reused.

layout Grid {
  Grid [sheet] (
    headers:       slot: column-headers,
    rows:          slot: viewport-rows,
    column-widths: slot: column-widths,
    selected-row:  slot: selected-row,
    selected-col:  slot: selected-col,
    edit-row:      slot: edit-row,
    edit-col:      slot: edit-col,
    sticky-header: false,
    total-height:  slot: total-height,
    onNavigate:    emit: onNavigate
  )
}
