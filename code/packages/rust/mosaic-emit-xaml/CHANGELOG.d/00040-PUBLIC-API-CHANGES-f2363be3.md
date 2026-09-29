### Public API changes

- `XamlEmitResult::for_view_models` now populated (was always empty
  in PR-1).
- `XamlEmitResult::if_helpers` remains an empty `Vec` (helpers inline
  into `code_behind`; see Deviation above).
- No breaking changes to consumers.

