package lexer

// Deterministic matchers for the common shapes of token patterns
// ==============================================================
//
// Go's regexp package is the right general tool, but it is slow when it is
// called once per token. Each call sets up an NFA or backtracker, and, worse,
// the work it does grows with the *remaining input*: the backtracker clears a
// visited-bitmap sized by the input length, and for large inputs the engine
// falls back to a much slower NFA simulation altogether.
//
// Most token patterns are tiny and "obviously deterministic":
//
//	[A-Za-z_][A-Za-z0-9_]*          identifier
//	-?[0-9]+                        integer
//	0x[0-9a-fA-F]+                  hex number
//	[A-Za-z0-9_]+<[^>\n]*>          name<annotation>
//	"([^"\\]|\\.)*"(\.\.\.)?        quoted string with escapes
//
// For those, a few lines of straight-line code that look at one byte at a time
// give the same answer as the regexp engine, in tens of nanoseconds.
//
// A pattern qualifies only if we can *prove* the straight-line matcher agrees
// with leftmost-first regexp semantics. Anything we cannot prove falls back to
// the regexp engine, so the optimisation can only ever make a rule faster,
// never different.
//
// The supported shape
// -------------------
//
// A pattern is a sequence of steps. Each step is one of:
//
//	lit    a fixed byte string                          "abc"
//	class  exactly one rune from a set                  [a-z]
//	run    min..max repetitions of one class, greedy    [a-z]+  [0-9]{2,4}  -?
//	loop   min..max repetitions of a group, greedy      (?:a|bc)*   ("x")?
//
// Inside a loop, the group is an alternation of branches and each branch is a
// sequence of lits and classes (each exactly once). Nested loops are not
// supported.
//
// Why greedy-without-backtracking is exact
// ----------------------------------------
//
// A regexp engine implementing Perl semantics tries the greedy choice first and
// backtracks to "fewer repetitions" only if the rest of the pattern then fails.
// Backtracking can help only if the rest of the pattern could start with a byte
// that the repeated item also accepts. So we require, for every repeating step:
//
//	FIRST(repeated item)  intersect  FIRST(rest of pattern)  =  empty
//
// (where "rest" extends through any following steps that can match nothing).
// Then once the greedy run stops, no shorter run could have worked either, and
// the straight-line answer equals the backtracking answer.
//
// For alternation inside a loop, branches are tried in order and the first that
// matches wins. That is exact when no two branches can both match at the same
// position (checked by walking them side by side until some byte position has
// disjoint byte sets), because then trying the second branch after the first
// succeeded could never produce a different overall match.
//
// If a branch fails after committing to its first byte, the loop simply ends
// at the start of that iteration: backtracking could only offer even fewer
// iterations, and each of those would leave the next byte (which is one the
// loop accepts) for the rest of the pattern, which the FIRST/FOLLOW condition
// says it cannot take.
//
// Multi-byte text
// ---------------
//
// regexp works on runes: a class matches a whole UTF-8 sequence, and every
// byte of invalid UTF-8 counts as one U+FFFD. A class here either contains all
// non-ASCII runes (like [^"] or .) or none of them; anything in between is not
// supported. Matching a high byte then consumes utf8.DecodeRuneInString's width,
// exactly as regexp advances.

import (
	"regexp/syntax"
	"unicode"
	"unicode/utf8"
)

// ---------------------------------------------------------------------------
// Program representation
// ---------------------------------------------------------------------------

type stepKind uint8

const (
	stepLit   stepKind = iota // fixed bytes
	stepClass                 // exactly one rune from a class
	stepRun                   // greedy repetition of one class
	stepLoop                  // greedy repetition of a group
)

// unit is a character class: a set of ASCII bytes plus, optionally, "every
// non-ASCII rune".
type unit struct {
	set  byteSet // only entries below 0x80 are set here
	high bool
}

// first returns every byte this unit can start a match with.
func (u *unit) first() byteSet {
	s := u.set
	if u.high {
		s.addHighBytes()
	}
	return s
}

// at returns the number of bytes the unit consumes at s[pos], or 0 for no match.
func (u *unit) at(s string, pos int) int {
	if pos >= len(s) {
		return 0
	}
	b := s[pos]
	if b < utf8.RuneSelf {
		if u.set[b] {
			return 1
		}
		return 0
	}
	if u.high {
		_, w := utf8.DecodeRuneInString(s[pos:])
		return w
	}
	return 0
}

// step is one element of a matcher program.
type step struct {
	kind stepKind
	lit  string
	u    unit
	min  int
	max  int // -1 = unbounded

	// loop only: the branches and, for each first byte, which branches to try.
	branches [][]step
	dispatch *[256][]uint8
}

// matcher is a straight-line program equivalent to a regexp anchored at 0.
type matcher struct {
	steps []step
}

// match returns the length of the match at the start of s, or -1.
func (m *matcher) match(s string) int {
	pos := 0
	for i := range m.steps {
		st := &m.steps[i]
		switch st.kind {
		case stepLit:
			if len(s)-pos < len(st.lit) || s[pos:pos+len(st.lit)] != st.lit {
				return -1
			}
			pos += len(st.lit)
		case stepClass:
			w := st.u.at(s, pos)
			if w == 0 {
				return -1
			}
			pos += w
		case stepRun:
			if !st.u.high {
				// ASCII-only classes can inspect their byte set directly. Compute
				// the bounded scan length from the remaining input so adding max
				// to pos cannot overflow for a large repetition bound.
				start := pos
				limit := len(s) - pos
				if st.max >= 0 && st.max < limit {
					limit = st.max
				}
				end := pos + limit // limit <= len(s)-pos, so end <= len(s)
				for pos < end && st.u.set[s[pos]] {
					pos++
				}
				if pos-start < st.min {
					return -1
				}
				continue
			}

			count := 0
			for st.max < 0 || count < st.max {
				w := st.u.at(s, pos)
				if w == 0 {
					break
				}
				pos += w
				count++
			}
			if count < st.min {
				return -1
			}
		case stepLoop:
			count := 0
			for st.max < 0 || count < st.max {
				if pos >= len(s) {
					break
				}
				np := -1
				for _, bi := range st.dispatch[s[pos]] {
					if p, ok := runBranch(st.branches[bi], s, pos); ok {
						np = p
						break
					}
				}
				if np < 0 {
					break
				}
				pos = np
				count++
			}
			if count < st.min {
				return -1
			}
		}
	}
	return pos
}

// runBranch matches a loop branch (a sequence of lits and single classes) at pos.
func runBranch(branch []step, s string, pos int) (int, bool) {
	for i := range branch {
		st := &branch[i]
		if st.kind == stepLit {
			if len(s)-pos < len(st.lit) || s[pos:pos+len(st.lit)] != st.lit {
				return 0, false
			}
			pos += len(st.lit)
			continue
		}
		w := st.u.at(s, pos)
		if w == 0 {
			return 0, false
		}
		pos += w
	}
	return pos, true
}

// ---------------------------------------------------------------------------
// Building a matcher from a parsed pattern
// ---------------------------------------------------------------------------

// buildMatcher returns a matcher for pattern, or nil if the pattern is not of
// the supported shape. pattern is the user's pattern (not the ^(?:...) wrapper);
// the match is implicitly anchored at the start of the input.
func buildMatcher(pattern string) *matcher {
	re, err := syntax.Parse(pattern, syntax.Perl)
	if err != nil {
		return nil
	}
	var items []*syntax.Regexp
	if re.Op == syntax.OpConcat {
		items = re.Sub
	} else {
		items = []*syntax.Regexp{re}
	}

	steps := make([]step, 0, len(items))
	for _, it := range items {
		st, ok := toStep(it)
		if !ok {
			return nil
		}
		steps = append(steps, st...)
	}
	if len(steps) == 0 {
		return nil
	}
	if !followConditionHolds(steps) {
		return nil
	}
	return &matcher{steps: steps}
}

// toStep converts one parsed item into steps (a literal may become several).
func toStep(re *syntax.Regexp) ([]step, bool) {
	switch re.Op {
	case syntax.OpCapture:
		return toStep(re.Sub[0])

	case syntax.OpConcat:
		var out []step
		for _, sub := range re.Sub {
			st, ok := toStep(sub)
			if !ok {
				return nil, false
			}
			out = append(out, st...)
		}
		return out, true

	case syntax.OpLiteral, syntax.OpCharClass, syntax.OpAnyChar, syntax.OpAnyCharNotNL:
		units, lit, ok := unitsOf(re)
		if !ok {
			return nil, false
		}
		if lit != "" {
			return []step{{kind: stepLit, lit: lit}}, true
		}
		out := make([]step, len(units))
		for i := range units {
			out[i] = step{kind: stepClass, u: units[i], min: 1, max: 1}
		}
		return out, true

	case syntax.OpStar, syntax.OpPlus, syntax.OpQuest, syntax.OpRepeat:
		if re.Flags&syntax.NonGreedy != 0 {
			return nil, false
		}
		min, max := 0, -1
		switch re.Op {
		case syntax.OpPlus:
			min = 1
		case syntax.OpQuest:
			max = 1
		case syntax.OpRepeat:
			min, max = re.Min, re.Max
			if min > 1000 || max > 1000 {
				return nil, false // keep programs small
			}
		}
		return repeatStep(re.Sub[0], min, max)
	}
	return nil, false
}

// unitsOf converts a literal / class / any-char node into either a literal
// string (lit != "") or a list of single-byte-position units (a case-folded
// literal becomes one unit per letter).
func unitsOf(re *syntax.Regexp) (units []unit, lit string, ok bool) {
	switch re.Op {
	case syntax.OpLiteral:
		for _, r := range re.Rune {
			if r == utf8.RuneError || !utf8.ValidRune(r) {
				return nil, "", false
			}
		}
		if re.Flags&syntax.FoldCase == 0 {
			return nil, string(re.Rune), len(re.Rune) > 0
		}
		for _, r := range re.Rune {
			if r >= utf8.RuneSelf {
				return nil, "", false
			}
			var u unit
			u.set[byte(r)] = true
			for f := unicode.SimpleFold(r); f != r; f = unicode.SimpleFold(f) {
				if f >= utf8.RuneSelf {
					return nil, "", false
				}
				u.set[byte(f)] = true
			}
			units = append(units, u)
		}
		return units, "", len(units) > 0

	case syntax.OpCharClass:
		var u unit
		for i := 0; i+1 < len(re.Rune); i += 2 {
			lo, hi := re.Rune[i], re.Rune[i+1]
			for r := lo; r <= hi && r < utf8.RuneSelf; r++ {
				u.set[byte(r)] = true
			}
			if hi >= utf8.RuneSelf {
				// Only "every non-ASCII rune" is representable.
				if lo > utf8.RuneSelf || hi < unicode.MaxRune {
					return nil, "", false
				}
				u.high = true
			}
		}
		return []unit{u}, "", true

	case syntax.OpAnyChar:
		var u unit
		u.set.addAll()
		u.high = true
		return []unit{u}, "", true

	case syntax.OpAnyCharNotNL:
		var u unit
		u.set.addAll()
		u.set['\n'] = false
		u.high = true
		return []unit{u}, "", true
	}
	return nil, "", false
}

// repeatStep builds a run (single-class body) or a loop (group body).
func repeatStep(body *syntax.Regexp, min, max int) ([]step, bool) {
	for body.Op == syntax.OpCapture {
		body = body.Sub[0]
	}

	// A single class (or one-byte literal): a run.
	if body.Op == syntax.OpCharClass || body.Op == syntax.OpAnyChar || body.Op == syntax.OpAnyCharNotNL ||
		(body.Op == syntax.OpLiteral && len(body.Rune) == 1) {
		units, lit, ok := unitsOf(body)
		if !ok {
			return nil, false
		}
		if lit != "" {
			var u unit
			if lit[0] >= utf8.RuneSelf {
				// A repeated multi-byte rune is not a one-byte class; leave
				// such a pattern to the regexp engine.
				return nil, false
			}
			u.set[lit[0]] = true
			units = []unit{u}
		}
		if len(units) != 1 {
			return nil, false
		}
		return []step{{kind: stepRun, u: units[0], min: min, max: max}}, true
	}

	// Otherwise a group: an alternation of branches, each a sequence of
	// lits/classes. A nested alternation such as \(?:x|uHHHH) (which the regexp
	// parser produces by factoring a common prefix) is expanded back into
	// whole branches, preserving priority order.
	branches, ok := expandBranches(body)
	if !ok || len(branches) == 0 {
		return nil, false
	}
	if !branchesExclusive(branches) {
		return nil, false
	}
	st := step{kind: stepLoop, min: min, max: max, branches: branches}
	st.dispatch = new([256][]uint8)
	for bi, br := range branches {
		f := firstOfSteps(br[:1])
		for b := 0; b < 256; b++ {
			if f[b] {
				st.dispatch[b] = append(st.dispatch[b], uint8(bi))
			}
		}
	}
	return []step{st}, true
}

// maxBranches bounds the expansion of nested alternations.
const maxBranches = 32

// expandBranches turns a group body into a list of branches (each a sequence of
// lits and single classes), distributing any alternation over its neighbours:
//
//	\(?:a|b)c   ->   \ac | \bc
//
// Order is preserved, so leftmost-first priority is unchanged.
func expandBranches(re *syntax.Regexp) ([][]step, bool) {
	switch re.Op {
	case syntax.OpCapture:
		return expandBranches(re.Sub[0])
	case syntax.OpAlternate:
		var out [][]step
		for _, sub := range re.Sub {
			e, ok := expandBranches(sub)
			if !ok {
				return nil, false
			}
			out = append(out, e...)
			if len(out) > maxBranches {
				return nil, false
			}
		}
		return out, true
	case syntax.OpConcat:
		out := [][]step{nil}
		for _, sub := range re.Sub {
			e, ok := expandBranches(sub)
			if !ok {
				return nil, false
			}
			next := make([][]step, 0, len(out)*len(e))
			for _, prefix := range out {
				for _, tail := range e {
					joined := append(append([]step{}, prefix...), tail...)
					next = append(next, joined)
				}
			}
			if len(next) > maxBranches {
				return nil, false
			}
			out = next
		}
		return out, true
	}
	bs, ok := branchSteps(re)
	if !ok || len(bs) == 0 {
		return nil, false
	}
	return [][]step{bs}, true
}

// branchSteps converts a loop branch: only lits and single classes allowed.
func branchSteps(re *syntax.Regexp) ([]step, bool) {
	switch re.Op {
	case syntax.OpCapture:
		return branchSteps(re.Sub[0])
	case syntax.OpConcat:
		var out []step
		for _, sub := range re.Sub {
			st, ok := branchSteps(sub)
			if !ok {
				return nil, false
			}
			out = append(out, st...)
		}
		return out, true
	case syntax.OpLiteral, syntax.OpCharClass, syntax.OpAnyChar, syntax.OpAnyCharNotNL:
		return toStep(re)
	case syntax.OpRepeat:
		// A fixed count, such as [0-9a-fA-F]{4}, is just that many copies.
		if re.Min != re.Max || re.Max > 64 {
			return nil, false
		}
		one, ok := branchSteps(re.Sub[0])
		if !ok {
			return nil, false
		}
		var out []step
		for i := 0; i < re.Min; i++ {
			out = append(out, one...)
		}
		return out, true
	}
	return nil, false
}

// ---------------------------------------------------------------------------
// Proof obligations
// ---------------------------------------------------------------------------

// firstOfSteps returns the bytes a match of the step sequence can start with,
// and is only meaningful for the first non-nullable prefix; see nullable.
func firstOfSteps(steps []step) byteSet {
	var out byteSet
	for i := range steps {
		st := &steps[i]
		f, nullable := stepFirst(st)
		out.union(&f)
		if !nullable {
			break
		}
	}
	return out
}

// stepFirst returns FIRST(step) and whether the step can match nothing.
func stepFirst(st *step) (byteSet, bool) {
	var out byteSet
	switch st.kind {
	case stepLit:
		out[st.lit[0]] = true
		return out, false
	case stepClass:
		return st.u.first(), false
	case stepRun:
		return st.u.first(), st.min == 0
	default: // loop
		for _, br := range st.branches {
			f := firstOfSteps(br[:1])
			out.union(&f)
		}
		return out, st.min == 0
	}
}

// allNullable reports whether every step can match the empty string.
func allNullable(steps []step) bool {
	for i := range steps {
		if _, nullable := stepFirst(&steps[i]); !nullable {
			return false
		}
	}
	return true
}

// followConditionHolds checks FIRST(repeated item) ∩ FIRST(rest) = ∅ for every
// step whose repetition count can vary.
func followConditionHolds(steps []step) bool {
	for i := range steps {
		st := &steps[i]
		if (st.kind != stepRun && st.kind != stepLoop) || st.min == st.max {
			continue
		}
		// If everything after this step can match nothing, the rest of the
		// pattern always succeeds, so the greedy choice is never revisited.
		if allNullable(steps[i+1:]) {
			continue
		}
		body, _ := stepFirst(st)
		rest := firstOfSteps(steps[i+1:])
		for b := 0; b < 256; b++ {
			if body[b] && rest[b] {
				return false
			}
		}
	}
	return true
}

// branchesExclusive checks that no two branches can match at the same position.
func branchesExclusive(branches [][]step) bool {
	for i := 0; i < len(branches); i++ {
		for j := i + 1; j < len(branches); j++ {
			if !twoBranchesExclusive(branches[i], branches[j]) {
				return false
			}
		}
	}
	return true
}

// branchUnits flattens a branch into one unit per consumed byte position.
// A class that contains non-ASCII runes consumes a variable number of bytes, so
// it is only usable as the last place the comparison looks at.
func branchUnits(br []step) []unit {
	var out []unit
	for i := range br {
		st := &br[i]
		if st.kind == stepLit {
			for k := 0; k < len(st.lit); k++ {
				var u unit
				u.set[st.lit[k]] = true
				if st.lit[k] >= utf8.RuneSelf {
					u.set[st.lit[k]] = false
					u.high = true
				}
				out = append(out, u)
			}
			continue
		}
		out = append(out, st.u)
	}
	return out
}

// twoBranchesExclusive walks the branches position by position until some
// position has disjoint byte sets (then they can never both match), giving up
// if both could match and a variable-width class is involved.
func twoBranchesExclusive(a, b []step) bool {
	ua, ub := branchUnits(a), branchUnits(b)
	for k := 0; k < len(ua) && k < len(ub); k++ {
		if unitsDisjoint(&ua[k], &ub[k]) {
			return true
		}
		if ua[k].high || ub[k].high {
			return false
		}
	}
	return false
}

func unitsDisjoint(a, b *unit) bool {
	if a.high && b.high {
		return false
	}
	for i := 0; i < utf8.RuneSelf; i++ {
		if a.set[i] && b.set[i] {
			return false
		}
	}
	return true
}
