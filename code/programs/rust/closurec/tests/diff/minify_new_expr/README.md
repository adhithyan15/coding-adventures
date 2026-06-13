# minify_new_expr

Captured from upstream Google Closure Compiler **v20240317**.
Pins that `new Foo()` (empty argument list) drops the parens
to `new Foo`.

Currently **IGNORED** — closurec emits `var x=new Foo();`.
See `gap-050` in `code/specs/CLOC12-gaps.md`.
