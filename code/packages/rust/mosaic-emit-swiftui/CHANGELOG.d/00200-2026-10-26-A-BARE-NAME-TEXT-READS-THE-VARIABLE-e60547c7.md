## 2026-10-26 (A bare-name Text label reads the variable)

`Text ( content: row )` inside `For ( each: slot: rows, as: row )` showed the
word `row` on every row (backlog 6f1f). `swift_text_expression` quoted a bare
NAME (`LayoutPropValue::Keyword`) as `Text("row")`, while React (`{row}`),
HTML (`{{row}}`) and XAML (`{x:Bind Row}`) read the loop variable. It now
lowers to `_mosaicText(row)`, as every other SwiftUI prop that takes a bare
NAME already did (button labels, placeholders, accessibility labels).

- A kebab-case binding is camelCased: `content: row-item` is
  `_mosaicText(rowItem)`. An `index:` binding reads its `Double` shadow, so
  it shows `0.0`, as `content: ( i )` already did.
- Swift's scoping gives UI29 §3.4's "For wins": the `ForEach` closure
  parameter shadows a component property of the same name. Outside a loop, a
  NAME that spells one of the component's own slots reads that slot, as
  React's destructured prop does.
- A NAME bound by neither no longer ships as its own spelling. It fails
  `swiftc` with "cannot find '<name>' in scope", as React fails `tsc`. The
  moslayout validator still accepts it.
- A NAME whose camelCased spelling is not a Swift identifier is an
  `UnsafeSlotName` error.
- Visible change: CollectionActions' missing-media and unused-media lists (and
  EngramApp, which inlines them) now show each filename and asset id.
- Tests: `tests/bare_name_text_reads_the_variable.rs` pins the loop binding,
  `index:` binding, own slot and slot-shadowing cases, and runs
  `swiftc -typecheck` on the emitted file on macOS. A unit test pins the
  unsafe-identifier refusal.
