// Package satsolver provides an incremental CDCL SAT solver and CNF builder.
package satsolver

import (
	"errors"
	"fmt"
	"math"
	"sort"
)

type Var uint32
type Lit uint32
type LBool int8

const (
	Undef     LBool  = 0
	False     LBool  = -1
	True      LBool  = 1
	maxPacked uint32 = 1 << 31
	maxVar    uint32 = (1 << 30) - 1
)

// Literals are packed as (variable<<1)|sign: even values are positive and
// odd values are negative. Thus l^1 is the complement. Watch buckets below
// are keyed by a watched literal's complement, so the newly true literal
// selects clauses whose watched literal just became false.

type SolveResult string

const (
	SolveSat     SolveResult = "sat"
	SolveUnsat   SolveResult = "unsat"
	SolveUnknown SolveResult = "unknown"
)

type SolverConfig struct {
	MaxVars                                          uint32
	MaxClauses, MaxClauseLiterals, MaxStoredLiterals uint64
}
type SolverOptions struct{ MaxConflicts, MaxSteps *uint64 }
type satClause struct {
	lits     []Lit
	w0, w1   int
	learnt   bool
	deleted  bool
	activity float64
}
type Solver struct {
	cfg                                          SolverConfig
	vars                                         uint32
	cs                                           []*satClause
	inputClauseCount                             uint64
	inputLiteralCount                            uint64
	watches                                      [][]int
	val                                          []int8
	level                                        []int
	reason                                       []int
	trail                                        []Lit
	limits                                       []int
	qhead                                        int
	rootUnsat                                    bool
	model                                        []bool
	modelOK                                      bool
	core                                         []Lit
	Conflicts, Decisions, Propagations, Restarts uint64
	activity                                     []float64
	inc                                          float64
	phase                                        []int8
	restartBase, restartIndex, sinceRestart      uint64
	learnedLimit                                 int
}

func NewSolver(config ...SolverConfig) (*Solver, error) {
	// Zero fields retain these conservative defaults; callers may lower, never
	// raise, them. The literal encoding's larger numeric range is not an
	// allocation budget.
	c := SolverConfig{MaxVars: 1 << 20, MaxClauses: 1 << 20, MaxClauseLiterals: 1 << 24, MaxStoredLiterals: 1 << 24}
	if len(config) > 1 {
		return nil, errors.New("at most one solver config")
	}
	if len(config) == 1 {
		x := config[0]
		if x.MaxVars > 0 {
			c.MaxVars = x.MaxVars
		}
		if x.MaxClauses > 0 {
			c.MaxClauses = x.MaxClauses
		}
		if x.MaxClauseLiterals > 0 {
			c.MaxClauseLiterals = x.MaxClauseLiterals
		}
		if x.MaxStoredLiterals > 0 {
			c.MaxStoredLiterals = x.MaxStoredLiterals
		}
	}
	if c.MaxVars > maxVar+1 {
		return nil, errors.New("MaxVars exceeds literal representation")
	}
	if c.MaxVars > 1<<20 || c.MaxClauses > 1<<20 || c.MaxClauseLiterals > 1<<24 || c.MaxStoredLiterals > 1<<24 {
		return nil, errors.New("configured resource ceilings may lower, not raise, the hard defaults")
	}
	if c.MaxClauses == 0 || c.MaxClauseLiterals == 0 || c.MaxStoredLiterals == 0 {
		return nil, errors.New("resource ceilings must be positive")
	}
	return &Solver{cfg: c, inc: 1, restartBase: 32, restartIndex: 1, learnedLimit: 2000}, nil
}
func MakeLit(v Var, neg bool) (Lit, error) {
	if uint32(v) > maxVar {
		return 0, errors.New("variable is not representable")
	}
	x := uint32(v) << 1
	if neg {
		x |= 1
	}
	return Lit(x), nil
}
func validLit(l Lit) bool { return uint32(l) < maxPacked }
func NegateLit(l Lit) (Lit, error) {
	if !validLit(l) {
		return 0, errors.New("invalid literal")
	}
	return l ^ 1, nil
}
func VarOf(l Lit) (Var, error) {
	if !validLit(l) {
		return 0, errors.New("invalid literal")
	}
	return Var(uint32(l) >> 1), nil
}
func SignOf(l Lit) (bool, error) {
	if !validLit(l) {
		return false, errors.New("invalid literal")
	}
	return uint32(l)&1 != 0, nil
}
func LitToString(l Lit) (string, error) {
	if !validLit(l) {
		return "", errors.New("invalid literal")
	}
	n := uint64(uint32(l)>>1) + 1
	if uint32(l)&1 != 0 {
		return fmt.Sprintf("-%d", n), nil
	}
	return fmt.Sprint(n), nil
}
func (s *Solver) NewVar() (Var, error) {
	if s.vars >= s.cfg.MaxVars || s.vars > maxVar {
		return 0, errors.New("variable ceiling exceeded")
	}
	v := s.vars
	s.vars++
	s.val = append(s.val, 0)
	s.level = append(s.level, 0)
	s.reason = append(s.reason, -1)
	s.activity = append(s.activity, 0)
	s.phase = append(s.phase, -1)
	s.watches = append(s.watches, nil, nil)
	s.clearResult()
	return Var(v), nil
}
func (s *Solver) NVars() uint32 { return s.vars }
func (s *Solver) NClauses() uint64 {
	var n uint64
	for _, c := range s.cs {
		if !c.deleted && len(c.lits) > 1 {
			n++
		}
	}
	return n
}
func (s *Solver) storedLiteralCount() uint64 {
	// Original literals include root units (which have no clause object).
	// Learned literals count while live; the physical arena has a separate cap
	// because tombstones and watcher IDs still consume memory after reduction.
	n := s.inputLiteralCount
	for _, c := range s.cs {
		if c.learnt && !c.deleted {
			n += uint64(len(c.lits))
		}
	}
	return n
}
func (s *Solver) Okay() bool       { return !s.rootUnsat }
func (s *Solver) clearResult()     { s.model = nil; s.modelOK = false; s.core = nil }
func (s *Solver) litOK(l Lit) bool { return validLit(l) && uint32(l)>>1 < s.vars }
func (s *Solver) litVal(l Lit) int8 {
	x := s.val[uint32(l)>>1]
	if x == 0 {
		return -1
	}
	if uint32(l)&1 != 0 {
		x = -x
	}
	if x > 0 {
		return 1
	}
	return 0
}
func (s *Solver) enqueue(l Lit, r int) bool {
	v := uint32(l) >> 1
	x := int8(1)
	if uint32(l)&1 != 0 {
		x = -1
	}
	if s.val[v] != 0 {
		return s.val[v] == x
	}
	s.val[v] = x
	s.level[v] = len(s.limits)
	s.reason[v] = r
	s.trail = append(s.trail, l)
	return true
}
func (s *Solver) newLevel() { s.limits = append(s.limits, len(s.trail)) }
func (s *Solver) cancel(level int) {
	if len(s.limits) <= level {
		return
	}
	to := s.limits[level]
	for i := len(s.trail) - 1; i >= to; i-- {
		v := uint32(s.trail[i]) >> 1
		// Save the last decision phase before making the variable undefined.
		s.phase[v] = s.val[v]
		s.val[v] = 0
		s.reason[v] = -1
		s.level[v] = 0
	}
	s.trail = s.trail[:to]
	s.limits = s.limits[:level]
	if s.qhead > len(s.trail) {
		s.qhead = len(s.trail)
	}
}
func (s *Solver) AddClause(in []Lit) (bool, error) {
	// Validate the entire request and its growth quotas before allocation; a
	// malformed tail must not leave a partially grown solver.
	maxRequired := s.vars
	for _, l := range in {
		if !validLit(l) {
			return false, fmt.Errorf("invalid literal %d", l)
		}
		if v := uint32(l) >> 1; v >= maxRequired {
			maxRequired = v + 1
		}
	}
	if maxRequired > s.cfg.MaxVars {
		return false, errors.New("variable resource ceiling exceeded")
	}
	if s.inputLiteralCount+uint64(len(in)) > s.cfg.MaxClauseLiterals || s.inputClauseCount >= s.cfg.MaxClauses {
		return false, errors.New("clause resource ceiling exceeded")
	}
	if s.storedLiteralCount()+uint64(len(in)) > s.cfg.MaxStoredLiterals {
		return false, errors.New("stored literal ceiling exceeded")
	}
	// Match the TypeScript API: a well-formed literal grows the variable
	// universe, but preflight the entire request first to keep refusal atomic.
	for s.vars < maxRequired {
		if _, err := s.NewVar(); err != nil {
			return false, err
		}
	}
	s.clearResult()
	s.cancel(0)
	if s.rootUnsat {
		return false, nil
	}
	// Simplify against the established permanent root assignment. Watches on
	// a newly added clause cannot observe root literals whose qhead was already
	// consumed, so detect satisfied/unit/conflicting additions here.
	reduced := make([]Lit, 0, len(in))
	for _, l := range in {
		switch s.litVal(l) {
		case 1:
			return true, nil
		case 0:
			// Permanently false at root; omit it from the stored clause.
		default:
			reduced = append(reduced, l)
		}
	}
	if len(reduced) == 0 && len(in) > 0 { // all-false clauses are retained as the permanent empty clause below
		s.inputLiteralCount += uint64(len(in))
		s.inputClauseCount++
		s.rootUnsat = true
		return false, nil
	}
	ls := append([]Lit(nil), reduced...)
	sort.Slice(ls, func(i, j int) bool { return ls[i] < ls[j] })
	out := ls[:0]
	for _, l := range ls {
		if len(out) > 0 && l == out[len(out)-1] {
			continue
		}
		if len(out) > 0 && uint32(l) == uint32(out[len(out)-1])^1 {
			return true, nil
		}
		out = append(out, l)
	}
	ls = out
	s.inputClauseCount++
	s.inputLiteralCount += uint64(len(in))
	if len(ls) == 0 {
		s.rootUnsat = true
		return false, nil
	}
	if len(ls) == 1 {
		if !s.enqueue(ls[0], -1) {
			s.rootUnsat = true
			return false, nil
		}
		if conflict := s.propagate(math.MaxUint64); conflict >= 0 {
			s.rootUnsat = true
			return false, nil
		}
		return true, nil
	}
	c := &satClause{lits: append([]Lit(nil), ls...)}
	id := len(s.cs)
	s.cs = append(s.cs, c)
	c.w0 = 0
	c.w1 = 1
	s.watches[uint32(ls[c.w0])^1] = append(s.watches[uint32(ls[c.w0])^1], id)
	s.watches[uint32(ls[c.w1])^1] = append(s.watches[uint32(ls[c.w1])^1], id)
	if conflict := s.propagate(math.MaxUint64); conflict >= 0 {
		s.rootUnsat = true
		return false, nil
	}
	return true, nil
}

// propagate checks the budget before consuming a trail item. UNKNOWN therefore
// leaves qhead at the earliest unprocessed propagation, including at level zero.
func (s *Solver) propagate(budget uint64) int {

	var used uint64
	for s.qhead < len(s.trail) {
		if used >= budget {
			return -2
		}
		p := s.trail[s.qhead]
		s.qhead++
		used++
		s.Propagations++
		// Watch lists are keyed by the complement of the watched literal,
		// which is the literal just made true. Check the limit before consuming
		// qhead so UNKNOWN preserves an exact retry point.
		wi := uint32(p)
		list := s.watches[wi]
		write := 0
		for pos, id := range list {
			c := s.cs[id]
			if c.deleted {
				continue
			}
			fw := c.w0
			if uint32(c.lits[fw])^1 != wi {
				fw = c.w1
			}
			ow := c.w0
			if fw == c.w0 {
				ow = c.w1
			}
			other := c.lits[ow]
			if s.litVal(other) > 0 {
				s.watches[wi][write] = id
				write++
				continue
			}
			rep := -1
			for k, l := range c.lits {
				if k != fw && k != ow && s.litVal(l) != 0 {
					rep = k
					break
				}
			}
			if rep >= 0 {
				if fw == c.w0 {
					c.w0 = rep
				} else {
					c.w1 = rep
				}
				s.watches[uint32(c.lits[rep])^1] = append(s.watches[uint32(c.lits[rep])^1], id)
				continue
			}
			s.watches[wi][write] = id
			write++
			if s.litVal(other) == 0 {
				s.watches[wi] = append(s.watches[wi][:write], list[pos+1:]...)
				s.Conflicts++
				return id
			}
			if !s.enqueue(other, id) {
				s.watches[wi] = append(s.watches[wi][:write], list[pos+1:]...)
				s.Conflicts++
				return id
			}
		}
		s.watches[wi] = s.watches[wi][:write]
	}
	return -1
}

// compactWatches discards every historical bucket backing array and watcher
// entry after a learned-clause deletion pass. The remaining IDs are rebuilt
// from the two live watched positions of each non-unit clause.
func (s *Solver) compactWatches() {
	fresh := make([][]int, len(s.watches))
	for id, c := range s.cs {
		if c.deleted || len(c.lits) < 2 {
			continue
		}
		w0 := uint32(c.lits[c.w0]) ^ 1
		w1 := uint32(c.lits[c.w1]) ^ 1
		fresh[w0] = append(fresh[w0], id)
		fresh[w1] = append(fresh[w1], id)
	}
	for i, bucket := range fresh {
		if len(bucket) == 0 {
			continue
		}
		exact := make([]int, len(bucket))
		copy(exact, bucket)
		fresh[i] = exact
	}
	s.watches = fresh
}

func (s *Solver) analyze(conf int) ([]Lit, int, bool) {
	// First-UIP walks backward over the implication graph. The pivot variable
	// is skipped in its own reason (which contains its opposite literal), so
	// each resolution step strictly reduces the current-level path count.
	seen := make([]bool, s.vars)
	learn := []Lit{0}
	path := 0
	idx := len(s.trail) - 1
	p := Lit(0)
	havePivot := false
	ci := conf
	for {
		c := s.cs[ci]
		c.activity += 1
		for _, q := range c.lits {
			v := uint32(q) >> 1
			if (havePivot && v == uint32(p)>>1) || seen[v] || s.level[v] == 0 {
				continue
			}
			seen[v] = true
			s.activity[v] += s.inc
			if s.level[v] == len(s.limits) {
				path++
			} else {
				learn = append(learn, q)
			}
		}
		for idx >= 0 && !seen[uint32(s.trail[idx])>>1] {
			idx--
		}
		if idx < 0 {
			return nil, 0, false
		}
		p = s.trail[idx]
		havePivot = true
		idx--
		v := uint32(p) >> 1
		seen[v] = false
		path--
		if path == 0 {
			learn[0] = p ^ 1
			break
		}
		ci = s.reason[v]
		if ci < 0 {
			return nil, 0, false
		}
	}
	back := 0
	if len(learn) > 1 {
		best := 1
		for i := 2; i < len(learn); i++ {
			if s.level[uint32(learn[i])>>1] > s.level[uint32(learn[best])>>1] {
				best = i
			}
		}
		learn[1], learn[best] = learn[best], learn[1]
		back = s.level[uint32(learn[1])>>1]
	}
	s.inc *= 1.05
	if len(learn) == 0 || !havePivot || s.level[uint32(learn[0])>>1] != len(s.limits) {
		return nil, 0, false
	}
	return learn, back, true
}
func (s *Solver) addLearned(ls []Lit) (int, bool) {
	if s.storedLiteralCount()+uint64(len(ls)) > s.cfg.MaxStoredLiterals || uint64(len(s.cs)) >= s.cfg.MaxClauses+s.cfg.MaxStoredLiterals {
		return -1, false
	}
	c := &satClause{lits: append([]Lit(nil), ls...), learnt: true}
	id := len(s.cs)
	s.cs = append(s.cs, c)
	c.w0 = 0
	c.w1 = 0
	if len(ls) > 1 {
		c.w1 = 1
		s.watches[uint32(ls[1])^1] = append(s.watches[uint32(ls[1])^1], id)
	}
	s.watches[uint32(ls[0])^1] = append(s.watches[uint32(ls[0])^1], id)
	return id, true
}
func luby(i uint64) uint64 {
	k := uint64(1)
	for (uint64(1)<<k)-1 < i {
		k++
	}
	if i == (uint64(1)<<k)-1 {
		return uint64(1) << (k - 1)
	}
	return luby(i - (uint64(1) << (k - 1)) + 1)
}
func (s *Solver) reduceLearned() {
	locked := make(map[int]bool)
	for _, r := range s.reason {
		if r >= 0 {
			locked[r] = true
		}
	}
	var candidates []int
	for i, c := range s.cs {
		if c.learnt && !c.deleted && len(c.lits) > 2 && !locked[i] {
			candidates = append(candidates, i)
		}
	}
	sort.Slice(candidates, func(i, j int) bool { return s.cs[candidates[i]].activity < s.cs[candidates[j]].activity })
	for i := 0; i < len(candidates)/2; i++ {
		c := s.cs[candidates[i]]
		c.deleted = true
		c.lits = nil
	}
	// Deleted watch IDs otherwise leave oversized historical buckets behind.
	// Between reductions, each clause's two watch IDs can occupy only buckets
	// for literals present in that clause; total retained capacity is bounded
	// by the slice growth factor times stored literal occurrences.
	s.compactWatches()
}

func (s *Solver) Solve(opts ...SolverOptions) (SolveResult, error) {
	return s.SolveWithAssumptions(nil, opts...)
}
func (s *Solver) SolveWithAssumptions(as []Lit, opts ...SolverOptions) (SolveResult, error) {
	if len(opts) > 1 {
		return SolveUnknown, errors.New("at most one options value")
	}
	maxRequired := s.vars
	for _, l := range as {
		if !validLit(l) {
			return SolveUnknown, fmt.Errorf("invalid assumption %d", l)
		}
		if v := uint32(l) >> 1; v >= maxRequired {
			maxRequired = v + 1
		}
	}
	if maxRequired > s.cfg.MaxVars {
		return SolveUnknown, errors.New("variable resource ceiling exceeded")
	}
	for s.vars < maxRequired {
		if _, err := s.NewVar(); err != nil {
			return SolveUnknown, err
		}
	}
	o := SolverOptions{}
	if len(opts) == 1 {
		o = opts[0]
	}
	maxConflicts, maxSteps := uint64(math.MaxUint64), uint64(math.MaxUint64)
	if o.MaxConflicts != nil {
		maxConflicts = *o.MaxConflicts
	}
	if o.MaxSteps != nil {
		maxSteps = *o.MaxSteps
	}
	remaining := func(limit, used uint64) uint64 {
		if limit == math.MaxUint64 {
			return limit
		}
		if used >= limit {
			return 0
		}
		return limit - used
	}
	s.clearResult()
	s.cancel(0)
	if s.rootUnsat {
		return SolveUnsat, nil
	}
	startConflicts, startPropagations := s.Conflicts, s.Propagations
	// Root work is checked before assumptions. A contradiction already
	// observed at level zero is permanent and takes precedence over budgets.
	rootConflict := s.propagate(remaining(maxSteps, s.Propagations-startPropagations))
	if rootConflict == -2 {
		return SolveUnknown, nil
	}
	if rootConflict >= 0 {
		s.rootUnsat = true
		return SolveUnsat, nil
	}
	unique := make([]Lit, 0, len(as))
	seen := make(map[Lit]bool, len(as))
	for _, a := range as {
		if seen[a] {
			continue
		}
		seen[a] = true
		unique = append(unique, a)
		switch s.litVal(a) {
		case 1:
			s.newLevel()
		case 0:
			s.core = append([]Lit(nil), unique...)
			s.cancel(0)
			return SolveUnsat, nil
		default:
			s.newLevel()
			s.Decisions++
			s.enqueue(a, -1)
			c := s.propagate(remaining(maxSteps, s.Propagations-startPropagations))
			if c == -2 {
				s.cancel(0)
				return SolveUnknown, nil
			}
			if c >= 0 {
				if maxConflicts != math.MaxUint64 && s.Conflicts-startConflicts > maxConflicts {
					s.cancel(0)
					return SolveUnknown, nil
				}
				s.core = append([]Lit(nil), unique...)
				s.cancel(0)
				return SolveUnsat, nil
			}
		}
	}
	assumptionLevels := len(s.limits)
	pendingConflict := -1
	for {
		conflict := pendingConflict
		pendingConflict = -1
		if conflict < 0 {
			conflict = s.propagate(remaining(maxSteps, s.Propagations-startPropagations))
		}
		if conflict == -2 {
			s.cancel(0)
			return SolveUnknown, nil
		}
		if conflict >= 0 {
			// A true root conflict is sticky and outranks the conflict budget.
			if len(s.limits) == 0 {
				s.rootUnsat = true
				s.cancel(0)
				return SolveUnsat, nil
			}
			if len(s.limits) <= assumptionLevels {
				if maxConflicts != math.MaxUint64 && s.Conflicts-startConflicts > maxConflicts {
					s.cancel(0)
					return SolveUnknown, nil
				}
				s.core = append([]Lit(nil), unique...)
				s.cancel(0)
				return SolveUnsat, nil
			}
			if maxConflicts != math.MaxUint64 && s.Conflicts-startConflicts > maxConflicts {
				s.cancel(0)
				return SolveUnknown, nil
			}
			s.sinceRestart++
			learned, back, valid := s.analyze(conflict)
			if !valid {
				s.cancel(0)
				return SolveUnknown, errors.New("CDCL conflict-analysis invariant failed")
			}
			// Assumptions are decisions, not resolution reasons. Learned clauses
			// therefore remain permanent consequences; jump to the natural level,
			// assert the clause there, then replay only assumptions that were cut.
			s.cancel(back)
			id, stored := s.addLearned(learned)
			if !stored {
				s.cancel(0)
				return SolveUnknown, nil
			}
			if !s.enqueue(learned[0], id) {
				if back == 0 {
					s.rootUnsat = true
				} else if back <= len(unique) {
					s.core = append([]Lit(nil), unique[:back]...)
				} else {
					return SolveUnknown, errors.New("learned clause was not asserting after backjump")
				}
				s.cancel(0)
				return SolveUnsat, nil
			}
			postLearn := s.propagate(remaining(maxSteps, s.Propagations-startPropagations))
			if postLearn == -2 {
				s.cancel(0)
				return SolveUnknown, nil
			}
			if postLearn >= 0 {
				if back == 0 {
					s.rootUnsat = true
					s.cancel(0)
					return SolveUnsat, nil
				}
				if back <= assumptionLevels {
					if maxConflicts != math.MaxUint64 && s.Conflicts-startConflicts > maxConflicts {
						s.cancel(0)
						return SolveUnknown, nil
					}
					s.core = append([]Lit(nil), unique[:back]...)
					s.cancel(0)
					return SolveUnsat, nil
				}
				pendingConflict = postLearn
			}
			if back < assumptionLevels {
				for i := back; i < len(unique); i++ {
					a := unique[i]
					switch s.litVal(a) {
					case 1:
						s.newLevel()
					case 0:
						s.core = append([]Lit(nil), unique[:i+1]...)
						s.cancel(0)
						return SolveUnsat, nil
					default:
						s.newLevel()
						s.Decisions++
						s.enqueue(a, -1)
						c := s.propagate(remaining(maxSteps, s.Propagations-startPropagations))
						if c == -2 {
							s.cancel(0)
							return SolveUnknown, nil
						}
						if c >= 0 {
							if maxConflicts != math.MaxUint64 && s.Conflicts-startConflicts > maxConflicts {
								s.cancel(0)
								return SolveUnknown, nil
							}
							s.core = append([]Lit(nil), unique[:i+1]...)
							s.cancel(0)
							return SolveUnsat, nil
						}
					}
				}
				assumptionLevels = len(s.limits)
			}
			learnedCount := 0
			for _, c := range s.cs {
				if c.learnt && !c.deleted {
					learnedCount++
				}
			}
			// The conflict clause is a live analysis input even when it is not a
			// reason clause. Defer database deletion until it has been analyzed.
			if pendingConflict < 0 && learnedCount > s.learnedLimit {
				s.reduceLearned()
			}
			if pendingConflict < 0 && s.sinceRestart >= s.restartBase*luby(s.restartIndex) {
				s.cancel(assumptionLevels)
				s.qhead = 0
				s.Restarts++
				s.restartIndex++
				s.sinceRestart = 0
			}
			continue
		}
		v := -1
		score := -1.0
		for i, x := range s.val {
			if x == 0 && s.activity[i] > score {
				score = s.activity[i]
				v = i
			}
		}
		if v < 0 {
			s.model = make([]bool, s.vars)
			for i, x := range s.val {
				s.model[i] = x > 0
			}
			s.modelOK = true
			return SolveSat, nil
		}
		s.newLevel()
		s.Decisions++
		lit := Lit(v << 1)
		if s.phase[v] <= 0 {
			lit |= 1
		}
		s.enqueue(lit, -1)
	}
}
func (s *Solver) Model() ([]bool, bool) {
	if !s.modelOK {
		return nil, false
	}
	return append([]bool(nil), s.model...), true
}
func (s *Solver) FailedCore() []Lit { return append([]Lit(nil), s.core...) }
func (s *Solver) Value(l Lit) (LBool, error) {
	if !validLit(l) {
		return Undef, errors.New("invalid literal")
	}
	if uint32(l)>>1 >= s.vars {
		return Undef, nil
	}
	x := s.litVal(l)
	if x < 0 {
		return Undef, nil
	}
	if x == 0 {
		return False, nil
	}
	return True, nil
}
func (s *Solver) VarValue(v Var) LBool {
	if uint32(v) >= s.vars || s.val[v] == 0 {
		return Undef
	}
	if s.val[v] < 0 {
		return False
	}
	return True
}
func (s *Solver) Propagate(maxSteps uint64) int {
	c := s.propagate(maxSteps)
	if c >= 0 && len(s.limits) == 0 {
		// Public root propagation can discover the permanent contradiction
		// before Solve sees it; preserve that knowledge across later calls.
		s.rootUnsat = true
	}
	return c
}
func (s *Solver) Reset() { s.clearResult(); s.cancel(0) }

// The builder emits Tseitin equivalences; vectors are little-endian and BvAdd
// returns the low input-width bits (the final carry is intentionally dropped).
type CnfBuilder struct{ solver *Solver }
type FullAdderResult struct{ Sum, Carry Lit }

func NewCnfBuilder(s *Solver) (*CnfBuilder, error) {
	if s == nil {
		var e error
		s, e = NewSolver()
		if e != nil {
			return nil, e
		}
	}
	return &CnfBuilder{s}, nil
}
func (b *CnfBuilder) GetSolver() *Solver   { return b.solver }
func (b *CnfBuilder) NewVar() (Var, error) { return b.solver.NewVar() }
func (b *CnfBuilder) NewLit() (Lit, error) {
	v, e := b.solver.NewVar()
	if e != nil {
		return 0, e
	}
	return MakeLit(v, false)
}
func (b *CnfBuilder) check(ls ...Lit) error {
	for _, l := range ls {
		if !b.solver.litOK(l) {
			return fmt.Errorf("invalid builder literal %d", l)
		}
	}
	return nil
}
func (b *CnfBuilder) preflight(vars uint64, clauses, literals uint64) error {
	// All gate operands are checked before this reservation. Checking cumulative
	// original and stored literals here prevents an error on the last emitted
	// clause from leaving a partial Tseitin definition.
	s := b.solver
	if uint64(s.vars)+vars > uint64(s.cfg.MaxVars) {
		return errors.New("builder variable resource ceiling exceeded")
	}
	if s.inputClauseCount+clauses > s.cfg.MaxClauses {
		return errors.New("builder clause resource ceiling exceeded")
	}
	if s.inputLiteralCount+literals > s.cfg.MaxClauseLiterals {
		return errors.New("builder original-literal resource ceiling exceeded")
	}
	if s.storedLiteralCount()+literals > s.cfg.MaxStoredLiterals {
		return errors.New("builder stored-literal resource ceiling exceeded")
	}
	return nil
}
func (b *CnfBuilder) And(a, c Lit) (Lit, error) {
	if e := b.check(a, c); e != nil {
		return 0, e
	}
	if e := b.preflight(1, 3, 7); e != nil {
		return 0, e
	}
	o, e := b.NewLit()
	if e != nil {
		return 0, e
	}
	for _, cl := range [][]Lit{{o ^ 1, a}, {o ^ 1, c}, {o, a ^ 1, c ^ 1}} {
		if _, e = b.solver.AddClause(cl); e != nil {
			return 0, e
		}
	}
	return o, nil
}
func (b *CnfBuilder) Or(a, c Lit) (Lit, error) {
	if e := b.check(a, c); e != nil {
		return 0, e
	}
	if e := b.preflight(1, 3, 7); e != nil {
		return 0, e
	}
	o, e := b.NewLit()
	if e != nil {
		return 0, e
	}
	for _, cl := range [][]Lit{{o, a ^ 1}, {o, c ^ 1}, {o ^ 1, a, c}} {
		if _, e = b.solver.AddClause(cl); e != nil {
			return 0, e
		}
	}
	return o, nil
}
func (b *CnfBuilder) Xor(a, c Lit) (Lit, error) {
	if e := b.check(a, c); e != nil {
		return 0, e
	}
	if e := b.preflight(1, 4, 12); e != nil {
		return 0, e
	}
	o, e := b.NewLit()
	if e != nil {
		return 0, e
	}
	for _, cl := range [][]Lit{{a, c, o ^ 1}, {a ^ 1, c ^ 1, o ^ 1}, {a, c ^ 1, o}, {a ^ 1, c, o}} {
		if _, e = b.solver.AddClause(cl); e != nil {
			return 0, e
		}
	}
	return o, nil
}
func (b *CnfBuilder) Equiv(a, c Lit) (Lit, error) {
	if e := b.check(a, c); e != nil {
		return 0, e
	}
	if e := b.preflight(1, 4, 12); e != nil {
		return 0, e
	}
	o, e := b.NewLit()
	if e != nil {
		return 0, e
	}
	for _, cl := range [][]Lit{{a, c ^ 1, o ^ 1}, {a ^ 1, c, o ^ 1}, {a, c, o}, {a ^ 1, c ^ 1, o}} {
		if _, e = b.solver.AddClause(cl); e != nil {
			return 0, e
		}
	}
	return o, nil
}
func (b *CnfBuilder) Not(a Lit) (Lit, error) {
	if e := b.check(a); e != nil {
		return 0, e
	}
	return a ^ 1, nil
}
func (b *CnfBuilder) Ite(sel, a, c Lit) (Lit, error) {
	if e := b.check(sel, a, c); e != nil {
		return 0, e
	}
	if e := b.preflight(1, 4, 12); e != nil {
		return 0, e
	}
	o, e := b.NewLit()
	if e != nil {
		return 0, e
	}
	for _, cl := range [][]Lit{{sel ^ 1, a ^ 1, o}, {sel ^ 1, a, o ^ 1}, {sel, c ^ 1, o}, {sel, c, o ^ 1}} {
		if _, e = b.solver.AddClause(cl); e != nil {
			return 0, e
		}
	}
	return o, nil
}
func (b *CnfBuilder) FullAdder(a, c, carry Lit) (FullAdderResult, error) {
	if e := b.check(a, c, carry); e != nil {
		return FullAdderResult{}, e
	}
	if e := b.preflight(5, 17, 45); e != nil {
		return FullAdderResult{}, e
	}
	x, e := b.Xor(a, c)
	if e != nil {
		return FullAdderResult{}, e
	}
	sum, e := b.Xor(x, carry)
	if e != nil {
		return FullAdderResult{}, e
	}
	ab, e := b.And(a, c)
	if e != nil {
		return FullAdderResult{}, e
	}
	xc, e := b.And(x, carry)
	if e != nil {
		return FullAdderResult{}, e
	}
	co, e := b.Or(ab, xc)
	return FullAdderResult{sum, co}, e
}
func (b *CnfBuilder) BvAdd(a, c []Lit) ([]Lit, error) {
	if len(a) != len(c) {
		return nil, errors.New("bit-vector width mismatch")
	}
	if e := b.check(append(append([]Lit{}, a...), c...)...); e != nil {
		return nil, e
	}
	if len(a) > 0 {
		vars := uint64(2)
		clauses, lits := uint64(7), uint64(19)
		if len(a) > 1 {
			vars += 5 * uint64(len(a)-1)
			clauses += 17 * uint64(len(a)-1)
			lits += 45 * uint64(len(a)-1)
		}
		if e := b.preflight(vars, clauses, lits); e != nil {
			return nil, e
		}
	}
	out := make([]Lit, len(a))
	if len(a) == 0 {
		return out, nil
	}
	var carry Lit
	for i := range a {
		if i == 0 {
			var e error
			out[i], e = b.Xor(a[i], c[i])
			if e != nil {
				return nil, e
			}
			carry, e = b.And(a[i], c[i])
			if e != nil {
				return nil, e
			}
		} else {
			fa, e := b.FullAdder(a[i], c[i], carry)
			if e != nil {
				return nil, e
			}
			out[i], carry = fa.Sum, fa.Carry
		}
	}
	return out, nil
}
func (b *CnfBuilder) BvEq(a, c []Lit) (Lit, error) {
	if len(a) != len(c) {
		return 0, errors.New("bit-vector width mismatch")
	}
	if e := b.check(append(append([]Lit{}, a...), c...)...); e != nil {
		return 0, e
	}
	if len(a) > 0 {
		n := uint64(len(a))
		if e := b.preflight(2*n-1, 7*n-3, 19*n-7); e != nil {
			return 0, e
		}
	}
	if len(a) == 0 {
		return b.TrueLit()
	}
	var acc Lit
	for i := range a {
		x, e := b.Equiv(a[i], c[i])
		if e != nil {
			return 0, e
		}
		if i == 0 {
			acc = x
		} else {
			acc, e = b.And(acc, x)
			if e != nil {
				return 0, e
			}
		}
	}
	return acc, nil
}
func (b *CnfBuilder) TrueLit() (Lit, error) {
	if e := b.preflight(1, 1, 1); e != nil {
		return 0, e
	}
	l, e := b.NewLit()
	if e != nil {
		return 0, e
	}
	_, e = b.solver.AddClause([]Lit{l})
	return l, e
}
func (b *CnfBuilder) FalseLit() (Lit, error) {
	if e := b.preflight(1, 1, 1); e != nil {
		return 0, e
	}
	l, e := b.NewLit()
	if e != nil {
		return 0, e
	}
	_, e = b.solver.AddClause([]Lit{l ^ 1})
	return l, e
}
func (b *CnfBuilder) AddClause(c []Lit) (bool, error) { return b.solver.AddClause(c) }
