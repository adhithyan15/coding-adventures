---
category: Mosaic compiler pipeline
---

# A HostNavigationSplit nested in a vertical HostScroll crashes Compose and collapses SwiftUI to zero height, while every package gate stays green

**What went wrong.** PR-6 of the Mosaic backlog put `mosaic-pkg-notes` on
`HostNavigationSplit`: its `Row [ notes-body ]` became the split, with the
list as the pane, a wrapper `Column` as the detail, `pane-width : 220` and
`collapse : never`. The package's own native gate improved exactly as the
editor convergence audit predicted. It went from 10 style drops per theme to
2, with zero capability and zero behaviour degradations, and every package
test passed.

The app was broken anyway. Trestle mounts Notes inside its vertical
`HostScroll [ content-scroll ]`, and two native lowerings of the split need a
bounded height:

- **Compose.** The split lowers to `NavigationSuiteScaffoldLayout`, whose
  measure policy calls `layout(constraints.maxWidth, constraints.maxHeight)`
  (read from the bytecode of `material3-adaptive-navigation-suite-desktop`
  1.9.0). Under `verticalScroll`, `maxHeight` is `Constraints.Infinity`. The
  emitted TaskApp's `TaskAppScreenshots` harness rendered Board, Sheet and
  Calendar, then failed on the Notes tab with
  `IllegalStateException: Size(984 x 2147483647) is out of range`. The
  unmodified Notes rendered that tab.
- **SwiftUI.** A probe hosting `NavigationSplitView` in a vertical
  `ScrollView` on macOS measured the split at **0 points tall**. The same
  view without the `ScrollView` measured 776. `HostScroll` lowers to a bare
  `ScrollView`, so Notes would draw nothing.

CI would not have caught either failure. No native UI test opens Trestle's
Notes view. `TaskAppScreenshots` does open it, but CI does not run that
harness, and `TaskAppUiTest` never visits Notes. Every UI29-6 emitter fixture
puts the split at the component root, where the window bounds it.

**Do differently.** `HostNavigationSplit` is a window-shell container (UI29-6
§5: "the shell IS the split"). Before moving a nested consumer onto it, check
whether any ancestor is a vertical `HostScroll`
(`grep -n HostScroll <app>.mll`). If one is, the lowerings need a
bounded-height story first. Then run the app's Compose screenshot harness
on the view, as `a-component-swap-that-passes-every-compile-gate-and-web-test-can`
describes, and do not trust the package gate alone. A drop count measures
which styles were lowered. It says nothing about whether the result has a
size.
