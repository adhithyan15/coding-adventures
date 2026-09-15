- `language/determiner-type-alias.adj` (new) — a sibling to the already-shipped
  `determiner-type.adj` (`determiner_type(type, description)`, article/demonstrative_determiner/
  distributive_determiner → their own defining sentences). That table's own header already
  reproduces, verbatim, the Grammarly sentence for the demonstrative determiner in full —
  "Demonstrative determiners, also known as demonstrative adjectives, communicate the placement of a
  noun in space or time." — but the type/description schema had room only for WHAT it does, not the
  parenthetical alias clause naming its alternate name. New `determiner_type_alias(type, alias)`
  table decodes that clause as its own row: demonstrative_determiner → demonstrative_adjective.
  Honest abstention on article, a real already-tabled determiner type whose own sentence states no
  alias. New e2e test file `facts_determinertypealias_e2e.rs` (3 tests: forward recall with citation,
  backward recall, honest abstention). No manifest objective, matching `determiner-type.adj`'s own
  precedent. Fifth slice from the language/ domain cleanup sweep.
