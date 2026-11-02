---
category: Mosaic compiler pipeline
---

# A vertical HostScroll does not scroll on Qt when the shell is a Column, because the Qt emitter never bounds a Column root

**What went wrong.** EN-6 wrapped Engram's screens in a vertical
`HostScroll [ content-scroll ]` with `flex-grow : 1`, as the last child of the
`Column [ app-shell ]`. That fixed reachability on SwiftUI, Compose, Flutter
and XAML, and every gate passed on Qt too. But the generated QML cannot
scroll, for two reasons:

- The shell `Column` lowers to a `Rectangle` sized by `childrenRect`. Inside
  it, a `ColumnLayout` is placed with `x`/`y`, with no anchors and no height.
  `QQuickView::SizeRootObjectToView` sizes the root `Item` to the window, but
  nothing passes that size down, so the whole tree is as tall as its content.
- `flex-grow` is not lowered on Qt (`qt_drop_reason`), so the `ScrollView`
  gets no `Layout.fillHeight`. In a `ColumnLayout` it is fixed at its
  preferred height, which for a `ScrollView` is the content's height.

A viewport as tall as its content has nothing to scroll. The Qt style-drop
report did not even list `flex-grow` for `content-scroll`: Engram's Qt drop
count stayed at 40.

Journal (JO-5) and TaskApp (#14734) never hit this. Their shell is a
`HostNavigationSplit`, which the Qt emitter anchors to fill its parent
(`anchors.fill: parent` at depth 1, #15833).

**Do differently.** Before you claim that a `HostScroll` scrolls on Qt, read
the emitted QML from the root down to the `ScrollView`. Every ancestor must get
a bounded height: through `anchors.fill`, or through `Layout.fillHeight` inside
a layout. Do not trust the style-drop report for this. The emitter-side fix is
to bound a `Column` root to the window, and to lower `flex-grow` to
`Layout.fillHeight` / `fillWidth` for a child of a `ColumnLayout` /
`RowLayout`. Until then, a Column-shell app does not scroll on Qt.
