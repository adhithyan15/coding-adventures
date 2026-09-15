## 0.11.0 — 2026-06-27 — Literal string output (LANG-FULL AL4 on E4)

ALGOL 60 now has an executable literal-output foothold for strings. Undeclared
statement-position calls named `print` or `output` lower each string literal
actual to the shared E4 pair:

```text
str_const <temp>, "HI" : str
print_str <temp> : void
```

That keeps the backend story language-neutral: no ALGOL-specific code-gen hook,
just the same `str_const` + `print_str` path Dartmouth BASIC already proved on
all seven LANG backends. A program may still declare its own procedure named
`print`/`output`; user procedures win over the standard output fallback.

This slice is intentionally narrow and fail-closed. Non-literal arguments such
as `print(42)` or `print(s)` still produce an explicit unsupported-feature error
until full ALGOL string declarations/expressions land.

