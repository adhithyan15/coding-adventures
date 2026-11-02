---
category: Mosaic compiler pipeline
---

# A Qt layout given spare room with no growing child spreads it between the children, so bind a layout's main axis only when a child fills it

**What happened.** UI21 §2.2 (the EN-6 follow-up to #18477) binds a `Column`
layout root to the window, so that a `HostScroll` with `flex-grow` has a
bounded height to scroll within. The obvious lowering is
`anchors.fill: parent` on every `Column` root, which is also what the
original UI21 table said. It is wrong for any shell that has no child
growing into the extra height.

CSS packs a column's children at the start and leaves the surplus at the
end. A Qt `ColumnLayout` taller than its content spreads the surplus
*between* its children. Measured with the real Qt runtime (PySide6 6.12,
offscreen): three short children of an anchored 400px `ColumnLayout` landed
at y = 50, 174 and 314. TaskApp's unfixed main column, which the split
anchors, shows the same thing at 800px, with children at y = 119, 380, 446
and 642. A `RowLayout` is worse on its cross axis: it centres its children
vertically, so a window-tall `Row` moves everything to the middle.

Two related traps:

- A non-filling child of a Qt layout is fixed at its preferred size. The
  layout does not shrink it to fit, as CSS `flex-shrink: 1` would. So a
  `ScrollView` without `Layout.fillHeight` stays as tall as its content, even
  inside a bounded column.
- A `ScrollView` whose content extent is pinned to its own size
  (`contentWidth: availableWidth`) has an implicit width of 0. In a layout,
  without `Layout.fillWidth`, it is laid out zero pixels wide.

**Fix.** A `Column` root binds its width always, and its height only when a
direct child fills that axis (`Layout.fillHeight`, from a lowered
`flex-grow`, a `Spacer` or a `HostSurface`). A `Row` binds its width only
when a child grows, and never its height. The root `Item` then reads the
layout's *implicit* size rather than `childrenRect`, because `childrenRect`
would loop through the anchors when a consumer embeds the component.

**Do differently.** Before you bind a Qt layout's extent to its parent,
decide what will absorb the surplus. If nothing will, leave that axis
unbound. Measure it in the real runtime rather than reasoning from CSS. A
throwaway PySide6 venv is enough:
`pip install PySide6`, then load the QML in a `QQuickView` under
`QT_QPA_PLATFORM=offscreen` and read the item geometry. Binding loops go to
stderr, not to `QQmlEngine::warnings`, so capture both.
