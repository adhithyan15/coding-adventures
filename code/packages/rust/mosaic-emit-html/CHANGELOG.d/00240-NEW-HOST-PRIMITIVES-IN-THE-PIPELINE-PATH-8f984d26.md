- New host primitives in the pipeline path:
  - `Stack` → `<div style="position: relative">` (children layered)
  - `HostInput` → `<input type="text" value="{{value}}" ...>` with
    optional `value` / `placeholder` / `read-only` props (slot ref,
    string literal, or keyword `true`/`false` per prop)
  - `HostButton` → `<button>{{label}}</button>` with optional
    `disabled` keyword/slot-ref handling
  - `HostScroll` → `<div style="overflow: auto">`
  - `HostTable` → semantic `<table>` with sub-tags
    `HostTableColGroup` (`<colgroup>` + `<col>` children),
    `HostTableHead` (`<thead>` with `<tr>`/`<th>`),
    `HostTableBody` (`<tbody>` with `<tr>`/`<td>`),
    `HostTableFoot` (`<tfoot>` with `<tr>`/`<td>`)
