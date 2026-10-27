---
category: Compiler / VM / language pipeline
---

# A mechanical edit to content-hashed generated tables must be followed by regenerating every stem a test compares

**What went wrong.** #18158 added `guard:` / `conditions:` to 63 committed
`_grammar*.rs` files with an anchored insertion script, instead of
regenerating them, to avoid unrelated drift (#14202). That is safe for most
tables, but the release-family lexers (ruby, python, perl) name each release
module `g_<hash of the generated text>`. The new field changes the text, so
it changes every module name. The PR regenerated the stems CI checks
(mosmodel, moslayout, mosstyle, macrooct, nib, ruby, python) and missed
perl, whose check is not in that workflow job. It is a unit test in the
grammar-tools program instead:
`tests::committed_perl_release_tables_match_regeneration`. So
`build (shard-5-of-15)` failed. The same shard also failed
`spreadsheet-wasm`'s `build-wasm.sh --check`, because the lexer and
grammar-tools crates sit in its wasm dependency tree. That is the older
lesson `a-change-to-the-generic-lexer-crate-must-also-rebuild-spreadsheet-wasm`,
not applied.

**Fix.** Regenerated perl with the branch's grammar-tools: the diff is only
the five module names. Rebuilt the `.wasm` and the VisiCalc bundle.

**Next time.** After any mechanical edit to generated tables, find every
place that compares a table with its regeneration:
`grep -rn "match_regeneration\|generate-rust-compiled-grammars" code/programs/rust/grammar-tools/src .github/workflows`.
Regenerate each of those stems, and expect no diff beyond the hash names.
Before pushing a change to `lexer`, `parser` or `grammar-tools`, run
`bash code/packages/rust/spreadsheet-wasm/build-wasm.sh --check`.
