- **The pane scrolls on every backend (JO-5).** The pane's `Column [ timeline ]`
  is wrapped, from outside, in a vertical `HostScroll [ timeline-scroll ]`, as
  TaskApp's view area is (#14734). Before this, no native backend scrolled the
  pane. In the declared 1100×760 window its controls end about 500px down, so
  every entry past roughly the second was unreachable except through search.
  The web never showed it, because a browser page scrolls.

  The pane now lowers to:
  - `ScrollView` on SwiftUI;
  - `verticalScroll` on Compose;
  - `SingleChildScrollView` on Flutter;
  - `ScrollView` on Qt;
  - `ScrollViewer` on XAML;
  - `overflow-y: auto` on the web.

  The package stays native-complete with no new style drops.

  **Tests:**
  - `package_compiles::the_pane_scrolls` requires that the split's pane is
    `HostScroll [ timeline-scroll ]` wrapping exactly `Column [ timeline ]`.
  - `native_complete_gate::every_native_backend_scrolls_the_pane` builds each
    native backend, and requires its scroll container in the generated source
    before the pane's first control.
