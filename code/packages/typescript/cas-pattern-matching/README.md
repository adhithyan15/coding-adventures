# cas-pattern-matching

Pure TypeScript structural pattern matching and rewriting over symbolic IR.

Patterns are ordinary `IRNode` trees using sentinel heads:

- `Blank()`
- `Blank(T)`
- `Pattern(name, inner)`
- `Rule(lhs, rhs)`
- `RuleDelayed(lhs, rhs)`
