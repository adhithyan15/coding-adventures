---
category: Testing & coverage
---

# A fixture normalizer must not rewrite upstream expectations to match current output

`html-lexer`'s `normalize_html5lib_fixtures.py` lowers upstream html5lib
tokenizer tests into our fixture schema. For the four "End tag closing RCDATA
or RAWTEXT" cases (`foo</xmp ` and `foo</xmp/` at end of file), it replaced
upstream's expected output (`foo`, `eof-in-tag`) with what our lexer produced
(`foo</xmp `, no error). The conformance suite then passed while the lexer
decided "appropriate end tag" in the wrong place, and the defect only surfaced
as 31 tree-construction failures in `html-tree-builder`.

Fix: the lexer decides at whitespace or `/` as the specification does, and the
normalizer keeps upstream's tokens and diagnostics.

Do differently: a normalizer may translate names and formats (token spellings,
diagnostic code aliases), never outcomes. When our output disagrees with an
upstream fixture, list the case as a known failure or skip it with a reason;
do not make the fixture agree.
