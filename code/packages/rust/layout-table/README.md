# layout-table

`layout-table` implements a reusable, host-neutral CSS table formatting
context over `layout-ir`. It normalizes anonymous rows and cells, preserves
header/body/footer ordering, places row and column spans, and resolves fixed or
intrinsic column widths before delegating each cell subtree to shared layout.

The typed `ext["table"]` boundary keeps CSS and HTML attribute computation out
of the geometry engine. Separate and collapsed border models, caption side,
column hints, and cell vertical alignment therefore behave identically in all
Venture hosts.

Spec: [`UI42-layout-table`](../../../specs/UI42-layout-table.md).
