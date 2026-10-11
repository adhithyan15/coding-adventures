---
category: TypeScript / JavaScript
---

# An optional token in front of a negative lookahead lets the regex engine backtrack around the lookahead

**What went wrong.** The heading step pattern in `drivable-instructions.ts`
ended `…VERB(?:\*\*)?(?! (?:is|was|means)\b)`, meaning "a bold verb is fine,
but not when a copula follows". On `**Write** is Old English`, the engine first
let `(?:\*\*)?` consume the closing `**`, saw " is" and failed the lookahead,
then backtracked: the optional group matched nothing, the lookahead now saw
`** is`, which is not " is", and the match succeeded. The exclusion silently
never applied to the one shape it was written for. A unit control caught it.

**Fix.** Do not consume the optional token; look past it inside the lookahead:
`(?!\*{0,2} (?:is|was|means)\b)`.

**Do differently.** A negative lookahead placed after an optional (or
quantified) token is only as strong as the shortest thing that token can
match. Put the optional part inside the lookahead, and give every exclusion a
control case that would match only if the exclusion failed.
