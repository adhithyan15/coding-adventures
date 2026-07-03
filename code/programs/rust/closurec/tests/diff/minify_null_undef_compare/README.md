# minify_null_undef_compare

Captured from upstream Google Closure Compiler **v20240317**.
Pins that `var t = (x == null);` strips outer parens to `var t=x==null;`.

Currently **IGNORED** — closurec preserves the source parens. See `gap-053`.
