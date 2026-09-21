---
category: Cross-platform & Windows BUILD_windows
---

# A text assertion on generated markup cannot see that the markup renders nothing

**What went wrong.** Mosaic's Qt `HostNavigationSplit` lowering (#15634) shipped
laying out to **0 × 0 at every window size**. A Qt app built on it showed an
empty window. It stayed that way through review, CI and a merge (#15833).

Measured in a 1200 × 600 window: root component item **1200**, the `SplitView`
inside it **0**, both panes **0**.

The cause was one inert pair of lines:

```qml
Item {              // the generated component root — a plain Item
    SplitView {
        Layout.fillWidth: true      // QtQuick.Layouts attached property
        Layout.fillHeight: true     // — inert outside a RowLayout/ColumnLayout
```

QML accepts an attached property anywhere and ignores it where it means
nothing, with **no warning**. `SplitView` also derives no implicit size from
its panes, unlike `RowLayout`. So the split had no size from any source.

**Why four layers of checking all missed it.**

| check | what it actually proved |
| --- | --- |
| emitter unit tests | `out.contains("SplitView {")`, `out.contains("SplitView.preferredWidth: 236")` — the *string* is present |
| CI | `grep -q 'SplitView {'` — same |
| CI | `cmake --build` — a zero-size component compiles perfectly |
| degradation report | `nativeComplete: true`, one note saying the *collapse* was static |

Every one of them is satisfied by markup that draws nothing. Not one asked how
wide the thing was.

**What to do differently.**

- **For a rendering backend, assert rendered geometry, not emitted text.** A
  generated-markup substring proves the emitter wrote a string. It cannot
  distinguish a working component from an invisible one. The fix added
  `code/scripts/qt-navigation-split-render-check.qml`, which loads the
  generated QML at a known window size and measures it — it passes on the
  fixed output and fails on the old one, which is the only evidence that a
  regression test is worth having.
- **A build that succeeds is not a component that appears.** `cmake --build`,
  `dotnet build`, `flutter analyze` all pass on output that renders nothing.
  Treat "it compiles" as the floor, never the check.
- **Suspect silently-inert declarative properties.** `Layout.*` outside a
  Layout, `anchors.*` inside one, a CSS property on a `display:` that ignores
  it. These fail without a diagnostic, so only a measurement finds them.
- **Read the degradation report as a claim to verify, not as the state of the
  world.** It said the one problem was adaptive collapse. The real problem was
  that nothing rendered — a defect the report had no vocabulary for and no way
  to notice.
