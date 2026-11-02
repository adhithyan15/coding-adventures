- **Timeline rows are mosaic-pkg-card Cards (Mosaic backlog item 4).** The
  timeline, including search results, no longer mounts the toolkit's
  `RecordList`. Each row is now:
  - a `mosaic-pkg-card` `Card` (title, excerpt, meta);
  - an *Open* button beside it, named `Open {title}` by the app's new
    `open-label` row field (`row[6]`);
  - the `★` next to that button;
  - the day's heading above the card, as before.

  This makes `Card` a real consumer for the first time (product program §5).
  A `HostButton` cannot wrap the card yet: its children are dropped on six of
  eight backends (EM-8), so the button sits beside it.

  Behaviour and accessibility are unchanged:
  - one tab stop per row, named after the entry;
  - `selected` on the open entry;
  - `-selected` parts for its highlight;
  - day headings keep the heading role.

  "On this day" stays a `RecordList`. Depends on `mosaic-pkg-card = "0.2.0"`.

  **Card's misfits are recorded and pinned.** Seven misfits are recorded in
  `journal-component-demand-audit-v1.md`. `native_complete_gate` pins Card's
  measured style drops in a new `CARD_STYLE_DROPS` list, which matches only on
  Card's own parts, so it cannot license the same property being dropped from
  one of Journal's parts:
  - `border-style` on SwiftUI, Qt, Flutter and XAML;
  - `padding-bottom` on Qt and Flutter;
  - `font-size` and `font-weight` on Flutter.

  The package stays native-complete, with zero capability degradations.

  **A Card row is taller (misfit M7), and the pane scrolls to it.** A Card
  row with its Open button is about 220px in the declared 1100×760 window,
  against about 124px for a `RecordList` row, so the second entry now starts
  below the fold. That is reachable because the pane scrolls on every native
  backend (JO-5). The native UI tests scroll to a row before they require it
  to be displayed.

  **Tests:**
  - `package_compiles` requires that the timeline `For` draws a `Card` bound
    to `row[2..4]` in both branches, with an Open button named by `row[6]`
    and nothing nested in it. It requires that `Card` resolves to
    `mosaic-pkg-card`, that the dependency is Card's current version, and
    that only the On this day `RecordList` remains.
  - The web tests open an entry by its button's `aria-label`, and a new test
    covers the card, button and badge layout.
  - Compose's `JournalUiTest` finds a row by `timeline-entry-open` and its
    `contentDescription`. Before every row or empty-state assertion it scrolls
    the pane into place: back to the top, then down with the `ScrollBy`
    semantics action, a bounded number of steps, until the node is displayed
    (`performScrollTo()` hangs, EM-11). It still requires every row to be
    displayed, not just to exist.
  - `JournalScrollUiTest` (Compose) and `journal_scroll_test.dart` (Flutter)
    find rows by the Open button's accessible name rather than its text,
    which is now "Open" on every row.
  - The Android and iPadOS tests match `timeline-entry-open` and the
    accessible name. Android reaches rows through semantics, and XCUITest's
    taps scroll on their own, so neither needs a scroll step.
