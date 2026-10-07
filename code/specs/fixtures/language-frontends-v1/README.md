# Language frontend v1 fixtures

This is the language-neutral baseline for the twelve grammar-driven frontend
families missing as paired Dart and Swift lexer/parser packages. The governing
contract is [`language-frontend-neutral-fixtures.md`](../../language-frontend-neutral-fixtures.md).
It does not assert that any native package already passes.

`grammars.json` pins the exact bytes of all 132 canonical token and parser
grammars in these families (66 matched editions), including the versioned
`ecmascript` root under the JavaScript family. `cases.json` contains
one complete successful token stream and full generic AST per family. The
closed `schema.json` and `code/scripts/language_frontend_fixtures.py` reject
unknown fields, duplicate IDs, missing grammar editions, missing families,
stale grammar hashes, unsafe paths, invalid token references, and oversized
documents. Each AST leaf is a zero-based index into the non-EOF token list;
the EOF token itself remains mandatory as the last expected token.

The 12 baseline sources were selected from existing lexer/parser tests and
then independently projected through the canonical Python `GrammarLexer` and
`GrammarParser` using each case's explicit grammar edition. Python 3.12+ runs
the exact reference replay test in CI; the schema validator alone cannot tell
whether a well-shaped expected token or rule name is correct. The checked-in
projection, not an implementation under test, is the expected result. This
small common subset avoids contextual or host-specific wrapper transforms:
Excel references with ranges, mixed-case SQL/VHDL normalization, Verilog
preprocessing, Java/C# nested generic closers, and Python/Starlark indentation
blocks remain separate follow-up coverage. The simple cases do not establish
parity for those semantics.

Run from the repository root:

```bash
python code/scripts/language_frontend_fixtures.py
python3.13 code/scripts/tests/test_language_frontend_fixtures.py
```

Native consumers should parse `cases.json`, load the pinned grammar through
their own embedded/bundled grammar contract, compare every token and AST node,
and fail on any mismatch. They must not read the monorepo grammar files at
runtime or normalize an actual result by rewriting expected values or tree
structure. Additional cases require reference review and a corpus revision;
error outcomes require a new schema version.
