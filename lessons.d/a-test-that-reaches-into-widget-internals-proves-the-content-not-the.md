---
category: Testing & coverage
---

# A test that reaches into widget internals proves the content, not the reachability

**What went wrong.** Mosaic's Flutter `HostNavigationSplit` shipped (#15652)
with a navigation pane that, at compact width, **no user could open**. It
reached main behind a green test literally named *"navigation split uses an
accessible drawer when compact"*.

That test established "accessible" like this:

```dart
final scaffold = tester.state<ScaffoldState>(find.byType(Scaffold));
scaffold.openDrawer();
await tester.pumpAndSettle();
expect(find.byType(Drawer), findsOneWidget);
expect(find.bySemanticsLabel(RegExp(r'^Projects(\n|$)')), findsOneWidget);
```

The test opens the drawer **itself**, by reaching into the widget's own state,
and then checks what is inside it. Everything it asserts is true. None of it is
about whether a person can get there — and `ScaffoldState.openDrawer()` is not
something a person can call.

Underneath, the emitted compact branch was `Scaffold(drawer:, body:)` with no
`AppBar`. Flutter draws the hamburger **only** from an `AppBar`, so the pane
opened on an edge-drag gesture and nothing else. Measured on the generated
fixture at 500px:

```
AppBar: 0   IconButton: 0   buttons: 0
actionable semantics nodes in the whole tree: 0
```

Zero. No visible control, nothing keyboard-focusable, nothing for a screen
reader to activate — in the primitive whose whole stated purpose is that a
navigation pane is a landmark a screen-reader user navigates by.

**What to do differently.**

- **Drive the UI through the same door the user does.** `tester.tap(finder)`,
  not `state.openDrawer()`. If a test needs privileged access to put the UI
  into the state it wants to assert on, that is evidence the state may be
  unreachable — the workaround IS the finding.
- **Name what you are actually asserting.** Had the test been called "the
  drawer's contents are correct once open" the gap would have been obvious in
  review. "Uses an accessible drawer" claimed something it never checked.
- **Count affordances, not markup.** `find.byType(Drawer)` says a Drawer
  exists. Walking the semantics tree and counting nodes carrying `tap` / `focus`
  says whether anyone can use it. For anything accessibility-shaped, assert on
  the semantics tree.
- **Compare against the platform's own control.** A stock
  `Scaffold(appBar:, drawer:)` reports `label="Open navigation menu"` AND
  `tooltip="…"` on its button. The first fix set only `tooltip`, leaving the
  label empty; measuring the platform's version is what caught it. Match the
  native shape rather than guessing at it.
- Same family as the Qt lesson: a check that never measures the thing the user
  experiences will pass for months. See
  [[a-text-assertion-on-generated-markup-cannot-see-that-the-markup-renders]].
