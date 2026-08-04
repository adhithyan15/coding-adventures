# Changelog


- 0.2.0: `loadGrammar()` returns the grammar embedded at compile time in the generated `_Grammar.swift`; no longer reads `code/grammars/**` via `#filePath` at run time (works standalone; grammar source of truth unchanged).
- 0.1.0: Added Swift ALGOL 60 parser package backed by the versioned `algol/algol60.grammar` grammar.
