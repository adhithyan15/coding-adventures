### Group C — per-column fixed widths

The per-column cell loop's value VM (`Grid_VVm`) now carries a
`double Width` field, and the generated cell element binds
`Width="{x:Bind Width}"` (injected via
`inject_attr_into_first_element`). The host-side VM-builder that
POPULATES the width (zipping cell value + column index → width) is
host code the emitter doesn't generate — a `<remarks>` doc comment
in the generated value-VM `.cs` tells the Windows dev exactly how
(`new Grid_VVm(value, col, ColumnWidths[col])`).

Tests: `group_c_value_vm_carries_width_and_cell_binds_it`.

