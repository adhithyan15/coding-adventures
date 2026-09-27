### Added — Row and Column honor `gap` after dynamic children (UI85, #14804)

Authored `gap` values on Flutter `Row` and `Column` parts now insert
horizontal or vertical `SizedBox` separators. The generated helper receives
the already evaluated child list, so `If` and `For` children do not leave
leading, trailing, or doubled spacing when they produce no widget.

The lowering preserves the authored `Row` or `Column`; it does not substitute
`Wrap` and therefore does not change flex behavior. `Stack` and incidental
multi-child `Box` columns remain unsupported for `gap` so the UI84 dropped
style reporter can identify those declarations instead of silently assigning
them incorrect layout semantics.

