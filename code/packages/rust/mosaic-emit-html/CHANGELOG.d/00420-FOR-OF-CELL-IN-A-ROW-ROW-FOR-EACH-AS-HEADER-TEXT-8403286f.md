- **For-of-cell in a Row** — `Row { For (each:…, as: header) { Text
  (content: header) } }` now lowers to
  `<tr><!-- mosaic-for each="cols" as="header" --><th>{{header}}</th><!-- /mosaic-for --></tr>`.
  The downstream template engine expands the loop to one cell per
  item rather than wrapping the entire iteration in a single cell.

