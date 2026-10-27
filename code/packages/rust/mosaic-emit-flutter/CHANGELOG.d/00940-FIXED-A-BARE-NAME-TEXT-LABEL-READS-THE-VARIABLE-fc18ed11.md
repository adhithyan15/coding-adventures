### Fixed — a bare-name Text label reads the variable

`Text ( content: row )` inside `For ( each: slot: rows, as: row )` was
blank on every row (backlog 6f1f). `emit_text` had no arm for a bare NAME
(`LayoutPropValue::Keyword`), so it fell through to `const Text("")`, and
because a `For` chooses its closure parameter by scanning the emitted body
for the binding (`dart_source_references_identifier`), the loop also threw
the binding away: `rows.map((_) => const Text(""))`. React (`{row}`), HTML
(`{{row}}`) and XAML (`{x:Bind Row}`) read the loop variable.

- The NAME now lowers to `Text(row.toString())`. Naming the binding is what
  makes the loop bind `(row)` again, or `final i = entry.key;` for an
  `index:` binding; the reference detector needed no change of its own.
- `.toString()` because `Text` takes a `String`: an `index:` binding is an
  `int` (it shows `0`) and a `list<number>` element a `double`. It matches
  `drag_text_prop_expression`, and unlike `'$row'` it draws no
  `unnecessary_string_interpolations` lint when the value is a `String`. The
  widget is never `const`.
- A kebab-case binding is camelCased: `content: row-item` is
  `rowItem.toString()`. Dart's scoping gives UI29 §3.4's "For wins": the
  closure parameter shadows a widget field of the same name. Outside a loop,
  a NAME that spells one of the component's own slots reads that slot.
- A NAME bound by neither fails `dart analyze` with "Undefined name", as
  React fails `tsc`. The moslayout validator still accepts it.
- A NAME that is not a Dart identifier (or is a reserved word) is an
  `UnsafeSlotName` error, via `validate_slot_or_field_name`.
- Visible change: CollectionActions' missing-media and unused-media lists
  (and EngramApp, which inlines them) now show each filename and asset id.
  The emitted CollectionActions project passes `flutter analyze`.
- Tests: `tests/bare_name_text_reads_the_variable.rs` pins the loop binding,
  `index:` binding, own slot, slot-shadowing and `list<number>` cases through
  the real three-language pipeline. Unit tests pin both `For` lowerings
  (spread and standalone) and the unsafe-identifier refusal.
