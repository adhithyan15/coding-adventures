### Added — `For` lowering (spec §6.1)

- `For (each: <expr>, as: <name>, index: <name>?) { ... }` now lowers
  to `<ItemsRepeater ItemsSource="{x:Bind ...}">` with an
  `<ItemsRepeater.ItemTemplate>` containing a `<DataTemplate
  x:DataType="local:{Component}_{AsName}Vm">`.
- One `RowVm` C# record is generated per `For` block and surfaced as
  an `EmittedFile` in `XamlEmitResult::for_view_models`. The record
  shape is
  `public sealed record {Component}_{AsName}Vm(ElementType ElementProperty[, int Index]);`.
- RowVms dedupe within a component: two `For` blocks binding the same
  `as:` name share one generated record.
- The element type is derived from the iterated slot's declared
  mosmodel type (`list<text>` → `string`, `list<number>` → `double`,
  `list<bool>` → `bool`, etc.). Expressions like `row.cells` that
  don't resolve to a typed slot default to `object`.
- `For`'s bound name and optional index are pushed into the
  `EmitContext::for_scope` for the duration of the body walk; nested
  `For` blocks are supported, innermost shadowing outermost.

