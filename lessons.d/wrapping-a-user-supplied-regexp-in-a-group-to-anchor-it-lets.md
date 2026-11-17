---
category: Testing & coverage
---

# Wrapping a user-supplied regexp in a group to anchor it lets a stray closing paren escape the wrapper

The Go grammar lexer anchored each token pattern as `^pattern`. To avoid the
alternation trap (`^a|b` anchors only the first branch) and to stop the regexp
engine scanning ahead, the compiled lexer switched to `^(?:pattern)`. The
fuzzer found the catch within seconds: the pattern `)*0|0(` is invalid after a
bare `^`, but inside the wrapper the `)` closes the wrapper and the `(` opens a
new group, so the combination is a *valid* regexp with a different meaning. The
new code accepted a grammar the old code rejected, and matched at positions the
old anchor would not.

Fix: validate the pattern in the old, bare shape first (`regexp/syntax.Parse("^"
+ pattern)`) and only then build the wrapped form. Use the wrapped form for
speed, never as the arbiter of validity.

Differently next time: when an optimisation changes how a user string is
embedded in a larger expression, the equivalence test must also cover
*malformed* inputs, and a coverage-guided fuzzer against the old behaviour is
the cheapest way to find them (`FuzzMatcherMatchesRegexp` in
`code/packages/go/lexer`).
