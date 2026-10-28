### Fixed — the screens scroll on every native backend except Qt (EN-6)

No Engram layout had a scroll container, so on a native backend whatever a
screen laid out past the window's bottom edge could not be reached. Engram
declares no window, so Compose Desktop opens it at 800×600. There, on
`main`, with an empty collection:

- Options' note type editor was off screen;
- Decks' collection buttons were off screen (Import, Export, Add note, …);
- on Flutter, Decks overflowed the window by 344px.

The web never showed it, because a browser page scrolls.

In both layouts the shell's last child is now a vertical
`HostScroll [ content-scroll ]` (`flex-grow : 1`). It wraps a
`Column [ content ]` (`gap : 18`) that holds all six screens, as TaskApp's
view area (#14734) and Journal's pane (JO-5) do. On desktop, the header and
host status stay above the scroll, so the switcher never scrolls away. On
touch, they scroll too: the stacked header is taller than a phone in
landscape, where on Flutter every screen overflowed, even Stats by 234px.

The screens lower to `ScrollView` on SwiftUI, `verticalScroll` on Compose,
`SingleChildScrollView` on Flutter, `ScrollViewer` on XAML, and
`overflow-y: auto` on the web. Qt emits a `ScrollView` but does not scroll
yet, because its emitter sizes a `Column` shell to its content and drops
`flex-grow`.

Native-complete is unchanged: zero capability degradations on every backend.
SwiftUI reports two more `flex-grow` drops (the scroll, once per layout), a
property already pinned. At 1280×900 the Compose renders of Decks, Study,
Browse, Add and Stats are byte-identical to `main`'s.

Tests:

- `package_compiles::the_screens_scroll` pins the layout in both files.
- `native_complete_gate::every_native_backend_scrolls_the_screens` requires
  each native backend's scroll container in both generated layouts.
- New `conformance/compose/EngramScrollUiTest.kt`, run by CI's Compose lane.
  At 800×600 it opens Options, requires the New type button to be off screen,
  scrolls (`ScrollBy`) until it is displayed, clicks it, and requires the new
  note type's id to appear and the switcher to still be displayed. It fails on
  `main`: no node scrolls.
- New `conformance/flutter/engram_scroll_test.dart`, run by CI's Flutter lane.
  At 800×600 it requires Decks to lay out with no exception, and its last
  button to start below the window. It drags until the button is inside the
  window, and requires the button to be hit-testable. It fails on `main`:
  Decks overflows by 344px.
- `EngramScreenshots` renders every screen at 800×600 as well, and now clicks
  Decks rather than assuming the first screen. The runtime saves the current
  screen, so the second pass would otherwise start where the first one ended.
