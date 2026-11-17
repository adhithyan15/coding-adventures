package parser

// Left-recursion analysis.
//
// THE GROWTH LOOP AND WHY IT IS EXPENSIVE
//
// parseRuleBody supports left-recursive rules (`e = e PLUS t | t`) with the
// seed-growing technique of Warth et al. ("Packrat Parsers Can Support Left
// Recursion", 2008):
//
//  1. Before parsing the body, memoise a FAILURE for (rule, position). If the
//     body reaches the same rule at the same position, that inner call hits
//     the failure and the recursion bottoms out. The body then matches via
//     its non-recursive alternative: that is the "seed".
//  2. Store the seed as the rule's result and run the body AGAIN from the same
//     position. Now the inner call hits the seed and the body can match
//     something longer (`e PLUS t` using the seed as `e`).
//  3. Repeat while the match keeps getting longer; keep the longest.
//
// Step 2 and 3 run after EVERY successful rule, because the engine does not
// know in advance whether a rule is left-recursive. For a rule that is not,
// the re-run can never grow the match (nothing in the body ever consults the
// seed), so it ends after one wasted iteration. The waste is not small: that
// iteration re-boxes every token, builds a second children slice and a
// memo-hit node per sub-rule, and then discards all of it. Skipping it is a
// measured 2x on every grammar we have.
//
// WHEN IS SKIPPING IT SOUND?
//
// The re-run reads the seed only through a rule reference (kRule) that parses
// the SAME rule at the SAME position, that is, one reached without consuming
// a token. If a rule cannot reach itself that way, no call in the re-run can
// observe the seed, so the re-run repeats the first run exactly: it replays
// the same memo entries (nothing is added: the memo is keyed by rule and
// position, and the first run created every entry it needs), records the same
// failures (already in the set), and ends the loop because the match did not
// get longer. So running it changes no tree, position, error text, limit
// refusal, frame count, depth or memo-slot count, and skipping it changes
// nothing either. The tests prove this by running every grammar both ways.
//
// THE ANALYSIS (an over-approximation)
//
// "Reaches itself without consuming a token" is a property of the grammar
// alone, so markLeftRecursive computes it once at compile time:
//
//  1. NULLABLE rules (and elements): those that can match without consuming a
//     token. Computed as a fixpoint, because a rule is nullable if its body
//     is, and bodies mention rules.
//  2. LEFT CALLS of an element: the rules that can be entered before any token
//     is consumed, i.e. those in first position, or after a nullable prefix.
//  3. A rule is left-recursive if it can reach itself through left calls.
//
// The analysis must NEVER say "not left-recursive" for a rule that is, so
// whenever the engine's exact behaviour is subtle it errs towards true:
//
//   - Optional, repetition and both lookaheads count as nullable. (A
//     lookahead consumes nothing even when its body does.)
//   - A token reference named EOF, or of the EOF token type, counts as
//     nullable: matching the synthetic EOF past the end of the tokens does not
//     advance the position. A literal with empty text is nullable too (it can
//     match the zero-width EOF).
//   - The body of a lookahead is a left call (it runs at the current position).
//   - A separated repetition contributes BOTH its element and its separator
//     as left calls, even when the element is not nullable (strictly, the
//     separator only follows the element).
//   - A rule reference to an undefined rule can never match, so it neither is
//     nullable nor calls anything.
//
// Over-approximating only costs speed (the loop runs when it need not); it
// never changes a result, since the loop is a no-op for a non-left-recursive
// rule anyway. Compiled's forceLeftRec knob exists so tests can check this.

import "github.com/adhithyan15/coding-adventures/code/packages/go/lexer"

// markLeftRecursive sets compiledRule.leftRec for every rule. With force set,
// every rule is marked (the pre-analysis behaviour, used by tests).
func (c *Compiled) markLeftRecursive(force bool) {
	n := len(c.rules)
	if force {
		for i := range c.rules {
			c.rules[i].leftRec = true
		}
		return
	}

	// Step 1: the nullable fixpoint. A rule only ever turns nullable, so the
	// loop ends after at most n changes.
	nullable := make([]bool, n)
	for changed := true; changed; {
		changed = false
		for i := range c.rules {
			if !nullable[i] && elementNullable(c.rules[i].body, nullable) {
				nullable[i] = true
				changed = true
			}
		}
	}

	// Step 2: for each rule, the rules its body can enter at its first token
	// position.
	left := make([][]int32, n)
	for i := range c.rules {
		collectLeftCalls(c.rules[i].body, nullable, &left[i])
	}

	// Step 3: a rule is left-recursive if a walk along left-call edges from it
	// gets back to it. One depth-first search per rule: O(rules * edges), at
	// compile time, once per grammar.
	seen := make([]bool, n)
	var stack []int32
	for i := range c.rules {
		clear(seen)
		stack = append(stack[:0], left[i]...)
		for len(stack) > 0 {
			r := stack[len(stack)-1]
			stack = stack[:len(stack)-1]
			if int(r) == i {
				c.rules[i].leftRec = true
				break
			}
			if !seen[r] {
				seen[r] = true
				stack = append(stack, left[r]...)
			}
		}
	}
}

// elementNullable reports whether n can match without consuming a token
// (over-approximately: see the file comment), given the nullable-so-far set
// of rules.
func elementNullable(n *node, nullable []bool) bool {
	switch n.kind {
	case kSeq:
		for _, s := range n.subs {
			if !elementNullable(s, nullable) {
				return false
			}
		}
		return true
	case kAlt:
		for _, s := range n.subs {
			if elementNullable(s, nullable) {
				return true
			}
		}
		return false
	case kRep, kOpt, kPosLook, kNegLook:
		return true
	case kGroup, kOneOrMore:
		return elementNullable(n.sub, nullable)
	case kSepRep:
		return !n.atLeastOne || elementNullable(n.sub, nullable)
	case kRule:
		return n.rule >= 0 && nullable[n.rule]
	case kTok:
		return n.name == "EOF" || n.tokType == lexer.TokenEOF
	case kLit:
		return n.value == ""
	}
	return false // kFail
}

// collectLeftCalls appends the ids of the rules n can enter before consuming
// any token. A sequence contributes its first element, and the next one as
// long as everything before it is nullable.
func collectLeftCalls(n *node, nullable []bool, out *[]int32) {
	switch n.kind {
	case kSeq:
		for _, s := range n.subs {
			collectLeftCalls(s, nullable, out)
			if !elementNullable(s, nullable) {
				return
			}
		}
	case kAlt:
		for _, s := range n.subs {
			collectLeftCalls(s, nullable, out)
		}
	case kRep, kOpt, kPosLook, kNegLook, kGroup, kOneOrMore:
		collectLeftCalls(n.sub, nullable, out)
	case kSepRep:
		collectLeftCalls(n.sub, nullable, out)
		collectLeftCalls(n.sep, nullable, out)
	case kRule:
		if n.rule >= 0 {
			*out = append(*out, n.rule)
		}
	}
}
