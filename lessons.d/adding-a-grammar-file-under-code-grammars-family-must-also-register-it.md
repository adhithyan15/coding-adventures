---
category: CI & GitHub Actions
---

# Adding a grammar file under code/grammars/<family> must also register it in the language-frontend fixture ledger

**What went wrong.** LANG84 R0 (#17821) added per-release Ruby pairs
(`ruby0.49`, `ruby0.50`, `ruby3.3.5`–`ruby3.3.7`). CI's "Repo-wide metadata
contracts" job then failed in "Verify language frontend neutral fixtures"
with `ValueError: grammar roster omits or adds a canonical file`.

`code/scripts/language_frontend_fixtures.py` globs every `*.tokens` and
`*.grammar` file for the families it tracks (csharp, excel, java, javascript
via `ecmascript/`, lattice, python, ruby, sql, starlark, typescript, verilog,
vhdl). It requires each one to appear in
`code/specs/fixtures/language-frontends-v1/grammars.json` with its exact
sha256 digest. The job never ran while the PR was stacked on another branch,
so the failure only showed after it was retargeted to main.

**The fix.** Register the new pairs in the ledger with their digests, then
run `python3 code/scripts/language_frontend_fixtures.py` and its unit tests
locally.

**Next time.**
- When a change adds, renames or edits a grammar file in one of those
  families, update `grammars.json` in the same commit. A content change also
  changes the digest.
- Before pushing, run the whole "Verify repo-wide metadata contracts" step
  locally, not just the checker for the files you touched.
- The ledger caps entries at `MAX_GRAMMARS = 256`. Every-release inventories
  (LANG84) will cross it, so plan how the ledger scales before bulk-adding
  release pairs.
