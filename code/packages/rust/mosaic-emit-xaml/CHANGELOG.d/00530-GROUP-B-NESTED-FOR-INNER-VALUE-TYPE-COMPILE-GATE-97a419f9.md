### Group B — nested-For inner value type (compile gate)

The inner `For (each: row, as: v)` (UI29 §3.4, `each:` referencing
the outer For's `as:` binding) inferred the cell value type as
`IReadOnlyList<string>` instead of `string`, because `emit_for`'s
Keyword arm used the enclosing binding's `element_type` verbatim
(that is the type of `row` ITSELF). The cell then bound a `string`
`<TextBlock Text="{x:Bind V}"/>` to a list field — a `dotnet build`
blocker. Fixed by peeling exactly one `List<>` level
(`inner_type_of_list(outer_type)`), so the inner value VM
(`Grid_VVm`) types `V` as `string` while the outer `Grid_RowVm`
keeps `IReadOnlyList<string> Row`.

Test: `group_b_inner_value_vm_field_is_string_not_list`.

