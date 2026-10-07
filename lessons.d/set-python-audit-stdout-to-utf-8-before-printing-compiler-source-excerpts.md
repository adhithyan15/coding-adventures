---
category: Python
---

# Set Python audit stdout to UTF-8 before printing compiler source excerpts

A Windows audit script read source as UTF-8 but printed it through Python's
CP1252 stdout. A Unicode arrow raised UnicodeEncodeError halfway through the
read-only audit, so the remaining files were never inspected. Configure
`sys.stdout.reconfigure(encoding="utf-8")` before printing, or emit escaped
JSON. UTF-8 file decoding alone does not configure terminal output.

The first lesson-creation attempt also used an invented category `tooling`.
The CLI rejected it. Read `lessons.d/_meta.md` and use its exact category names;
this lesson uses `Python`. Rust filename globs must likewise be passed to rg's
`-g` option, not as literal PowerShell path arguments.
