# minify_fn_expr_iife

Captured from upstream Google Closure Compiler **v20240317**.
Pins that `(function(){return 42;}());` is normalized to
`(function(){return 42})();` — the IIFE call moves OUTSIDE
the wrapping parens.

Currently **IGNORED** — closurec preserves the source form.
See `gap-051` in `code/specs/CLOC12-gaps.md`.
