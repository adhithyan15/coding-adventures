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
- **Card's misfits are recorded and pinned.** Six misfits are recorded in
  `journal-component-demand-audit-v1.md`. `native_complete_gate` pins Card's
  measured style drops in a new `CARD_STYLE_DROPS` list, which matches only on
  Card's own parts, so it cannot license the same property being dropped from
  one of Journal's parts:
  - `border-style` on SwiftUI, Qt, Flutter and XAML;
  - `padding-bottom` on Qt and Flutter;
  - `font-size` and `font-weight` on Flutter.

  The package stays native-complete, with zero capability degradations.
- **Tests.**
  - `package_compiles` requires that the timeline `For` draws a `Card` bound
    to `row[2..4]` in both branches, with an Open button named by `row[6]`
    and nothing nested in it. It requires that `Card` resolves to
    `mosaic-pkg-card`, that the dependency is Card's current version, and
    that only the On this day `RecordList` remains.
  - The web tests open an entry by its button's `aria-label`, and a new test
    covers the card, button and badge layout.
