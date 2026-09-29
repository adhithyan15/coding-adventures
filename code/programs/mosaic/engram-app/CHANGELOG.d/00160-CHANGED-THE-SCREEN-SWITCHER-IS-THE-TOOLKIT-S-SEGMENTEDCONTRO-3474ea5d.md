### Changed - the screen switcher is the toolkit's SegmentedControl (#14063)

The header's six screen buttons were written out inline as six If/Else
`HostButton` pairs, twelve parts in all, in both the desktop and the touch
layout. They are now one `pkg::mosaic-pkg-toolkit::SegmentedControl`:

- desktop passes `vertical : false`;
- touch passes `vertical : true`, since touch has always stacked the switcher
  to fit a phone width (#15432).

Engram is the control's first consumer outside the toolkit. TaskApp is next.

- **Interface:**
  - New slots `nav-options` (one `[label, accessible-name]` row per screen)
    and `nav-selected-index`.
  - New `emit onShowScreen ( index : number )`, so there are now 89 events.
  - The six `onShow*` events stay: they name a screen without an index, which
    is what a shortcut or a deep link wants.
- **Dependency:** `mosaic-pkg-toolkit = "0.13.0"`.
- **Styles:** the twelve `nav-*-button` / `nav-*-active` parts are gone from
  both themes. The switcher now takes the toolkit's look. That drops
  Engram's 6px radius and bold labels on those buttons; restyling a
  dependency from the app is a theming question, not settled here.
- **Screen-reader naming:** the selected screen is announced through its
  name ("Study, selected"), because `HostButton` has no selected state yet
  (#15420).
- **Tests:**
  - The event-count pin is 89.
  - The light-theme test used to look for `#1e40af`, which only the removed
    nav parts set in this stylesheet and dependency packages still use. It
    now pins the app header's own light style.

