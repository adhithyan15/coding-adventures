# cas-pattern-matching

Pure TypeScript structural pattern matching and rewriting over symbolic IR.

Patterns are ordinary `IRNode` trees using sentinel heads:

- `Blank()`
- `Blank(T)`
- `Pattern(name, inner)`
- `Rule(lhs, rhs)`
- `RuleDelayed(lhs, rhs)`

`MatchDeclareContext` ports the MACSYMA-style declaration layer: declared
symbols compile into `Pattern(name, Blank(...))` nodes with predicate-derived
constraints. `RuleStore` provides a small named-rule registry for storing and
retrieving compiled rules before `applyRule` or `rewrite`.
