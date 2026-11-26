## 0.1.0

- Initial Dart lane of the lisp-parser identity: `parseLisp` /
  `tryParseLisp` / `parseLispTokens` / `tryParseLispTokens` over the
  compiled `lisp.grammar` grammar (`lib/src/_grammar.dart`, generated
  with the in-lane emitter), tokenizing through the Dart lisp-lexer
  lane and lowering onto the typed `SExpr` forest (`SAtom` / `SList`
  / `SDottedPair` / `SQuoted`) with the reference's 48-level nesting
  pre-scan (`maxNestingDepth`). All failures throw the message-only
  `LispParseError`, and the nullable twins return `null` for any
  failure.
- Tests mirror the reference suite (all 32 tests, with the 48/49
  boundary re-measured on this engine), plus white-box extractor
  tests pinning every fail-closed arm, value-semantics tests, 8
  seeded property tests over the in-lane `pbt` package, and a
  2,000-iteration hostile fuzz campaign over the in-lane `fuzz`
  package; 100% line coverage, 39/40 functions (residue: the
  `AtomKind` synthetic enum constructor, uncoverable by any test —
  merged-lane precedent).
