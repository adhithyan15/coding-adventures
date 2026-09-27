- **For-of-Row in a section** — `HostTableBody { For (each:…, as: row)
  { Row { … } } }` now lowers to
  `<tbody><!-- mosaic-for each="rows" as="row" --><tr>…</tr><!-- /mosaic-for --></tbody>`,
  where the inner `<tr>` flows through `emit_table_row` so cells
  emit as native `<th>`/`<td>` (not the flex-`<div>` the generic
  walker would have produced).

