## 2026-10-26 (A bare-name Text label reads the variable)

- `Text ( content: row )` inside `For ( each: slot: rows, as: row )` was
  blank on every row (backlog 6f1f). `emit_text` had no arm for a bare NAME
  (`LayoutPropValue::Keyword`), so it fell through to `Text(text = "")`,
  while React (`{row}`), HTML (`{{row}}`) and XAML (`{x:Bind Row}`) read the
  loop variable. It now lowers to the string template `Text(text = "$row")`.
- The template, not `.toString()`, because `Text(text = …)` takes only a
  `String` and the emitter cannot see the value's type: an `index:` binding
  is the `Double` shadow (it shows `0.0`, as SwiftUI's does), and
  `.toString()` on the commonest case, a `list<text>` element, draws a
  "Redundant call of conversion method" warning from `kotlinc`.
- A kebab-case binding is camelCased: `content: row-item` is `"$rowItem"`.
  Kotlin's scoping gives UI29 §3.4's "For wins": the `forEach` lambda
  parameter shadows a composable parameter of the same name. Outside a loop,
  a NAME that spells one of the component's own slots reads that slot.
- A NAME bound by neither fails `kotlinc` with "Unresolved reference", as
  React fails `tsc`. The moslayout validator still accepts it.
- A NAME whose camelCased spelling is not a Kotlin identifier is an
  `UnsafeSlotName` error, so `$name` can only interpolate that one variable.
- Visible change: CollectionActions' missing-media and unused-media lists
  (and EngramApp, which inlines them) now show each filename and asset id.
  The emitted Engram Compose project passes `gradle compileKotlin`.
- Tests: `tests/bare_name_text_reads_the_variable.rs` pins the loop binding,
  `index:` binding, own slot, slot-shadowing and `list<number>` cases through
  the real three-language pipeline. Unit tests pin the lowering and the
  unsafe-identifier refusal.
