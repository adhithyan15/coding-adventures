package recmerge

// The fuzz target and its oracle.
//
// FuzzMerge builds a random but BINDING-VALID world from the fuzzer's bytes
// (so every B check passes and only the rules M1-M15 decide), runs Merge, and
// compares both verdicts with an independent oracle: the STRICT one (what
// becomes a manifest) and the RELAXED, report-only one.
//
// "Independent" is the point.  The oracle shares nothing with the merge:
//
//   - paths are compared as SEGMENT LISTS, not as strings with a "/" check;
//   - there is no index and no work budget: every rule is a plain nested
//     loop over every record, O(n squared) and proud of it;
//   - it is written from the rule table of KILN04 section 7.1.
//
// On top of the equality it states, in its own words, the invariants that make
// the strict verdict sound.  If ANY action is emitted then:
//
//	I1  no action in the run was truncated (status other than built, steps_run
//	    below steps_total, a daemon-left reason): its missing steps could have
//	    written anything (M-1);
//	I2  no action in the run is order-dependent: none observed a path, listing
//	    or tracked file that another action wrote or changed, so every
//	    recorded write set is the write set in any order (M-3);
//	I3  no shard reports tracked changes that no record explains (M-2) or
//	    writes no trace explains (M3), and no write set is otherwise unknown;
//	I4  the emitted action itself has no observation overlapping another
//	    action's write, the inventory or the tainted set.
//
// A short seed corpus runs in plain `go test`; `go test -fuzz FuzzMerge
// ./internal/recmerge` explores further.

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// vocabulary of paths; deliberately full of prefix traps (a/b vs a/bc vs ab).
var vocab = []string{".gitignore", "a", "a/b", "a/b/c", "a/bc", "ab", "b", "b/c", "c/d", "c/d/e",
	"code/kiln/x", "pkg/a/BUILD", "pkg/b"}

var reasonVocab = []string{"daemon-left", "daemon:x", "network:y", "trace-incomplete:size", "host-read:z"}
var bindVocab = []string{"stream/1.1.1.1:1", "stream/1.1.1.1:2", "dgram/1.1.1.1:1"}

type bits struct {
	data []byte
	pos  int
}

func (b *bits) next() bool {
	i := b.pos / 8
	b.pos++
	if i >= len(b.data) {
		return false
	}
	return b.data[i]>>(uint(b.pos-1)%8)&1 == 1
}

// rare is true about one time in eight.
func (b *bits) rare() bool { return b.next() && b.next() && b.next() }

func (b *bits) subset(from []string, sparse bool) []string {
	var out []string
	for _, v := range from {
		on := b.next()
		if sparse && on {
			on = b.next() && b.next() // thin the lists so most worlds stay legal
		}
		if on {
			out = append(out, v)
		}
	}
	return sl(out...)
}

// genWorld fills std's records from the bytes.
func genWorld(t *testing.T, data []byte) *world {
	w := std(t)
	b := &bits{data: data}
	allowUntracked := b.next()
	w.honour = b.next()
	// A refusal is per action (the merge's keys must refuse it too).
	for _, a := range []string{"pkg/a", "pkg/b", "pkg/c", "pkg/d"} {
		if b.rare() {
			if w.keys.refuse == nil {
				w.keys.refuse = map[string]string{}
			}
			w.keys.refuse[a] = "host-unknown"
			w.all(a, func(r *recfmt.ShardRecord) { r.Weak, r.Refusal = "", "host-unknown" })
		}
	}
	for _, sh := range w.shards {
		if b.next() {
			sh.Inventory.Paths = b.subset(vocab[1:10], true)
		}
		for _, tok := range []string{"tracked-changed", "state-preloaded"} {
			if b.next() && b.next() {
				sh.Header.Void = sl(append(sh.Header.Void, tok)...)
			}
		}
		if b.rare() {
			sh.Header.TrackedChangedUnexplained = 1
		}
		for i := range sh.Records {
			r := &sh.Records[i]
			rr := &r.Record
			rr.Writes = b.subset(vocab[1:10], true)
			rr.TrackedWrites = b.subset(vocab, true)
			rr.AbsentObservations = b.subset(vocab[1:10], true)
			if allowUntracked {
				rr.UntrackedObservations = b.subset(vocab[1:10], true)
			}
			rr.Traversed = b.subset(vocab[1:10], true)
			rr.Binds = b.subset(bindVocab, true)
			rr.Reasons = b.subset(reasonVocab, true)
			ents := []string{"pkg:" + r.Action}
			if b.next() {
				ents = append(ents, "list:.")
			}
			for _, v := range b.subset(vocab, true) {
				switch {
				case b.next():
					ents = append(ents, "list:"+v)
				case b.next():
					ents = append(ents, "path:"+v)
				default:
					ents = append(ents, "pkg:"+v)
				}
			}
			rr.Entries = sl(ents...)
			if b.rare() {
				r.Status = "failed"
			}
			if b.rare() {
				r.Gate = "submodule"
			}
			if b.rare() {
				r.Unknown = "tracer:status"
			}
			// Steps: how many ran, how many exist, which were clean.
			r.StepsRun, r.StepsTotal = 1, 1
			if b.rare() {
				r.StepsRun = 0
			}
			if b.rare() {
				r.StepsTotal = 2
			}
			r.Steps = make([]recfmt.Step, r.StepsRun)
			for k := range r.Steps {
				r.Steps[k].Clean = !b.rare()
			}
			if b.rare() && r.Status == recfmt.StatusBuilt {
				r.StepsSHA = dGraph // not the regenerated recipe: B6
			}
			if b.next() && b.next() {
				y := b.next()
				r.Complete = &y
			}
		}
	}
	return w
}

// ---------------------------------------------------------------------------
// The oracle.

func segs(p string) []string {
	if p == "." {
		return nil
	}
	return strings.Split(p, "/")
}

// isAnc: the segments of p are a prefix of the segments of q.
func isAnc(p, q string) bool {
	sp, sq := segs(p), segs(q)
	if len(sp) > len(sq) {
		return false
	}
	for i := range sp {
		if sp[i] != sq[i] {
			return false
		}
	}
	return true
}

func isOv(p, q string) bool { return isAnc(p, q) || isAnc(q, p) }

type orec struct {
	shard int
	r     *recfmt.ShardRecord
}

type verdict struct {
	strictVoid, relaxedVoid bool
	strict, relaxed         map[string]bool
	truncated, order        map[string]bool
}

// oracle decides both verdicts.
func oracle(w *world) verdict {
	var recs []orec
	for i, s := range w.shards {
		for k := range s.Records {
			recs = append(recs, orec{i, &s.Records[k]})
		}
	}
	v := verdict{strict: map[string]bool{}, relaxed: map[string]bool{}, truncated: map[string]bool{}, order: map[string]bool{}}
	hard := false
	shardVoid := map[int]bool{}
	for i, s := range w.shards {
		if s.Header.PostInventoryUnexplained > 0 || s.Header.TrackedChangedUnexplained > 0 {
			hard = true
		}
		for _, tok := range s.Header.Void {
			switch tok {
			case "tracked-changed", "state-preloaded":
				shardVoid[i] = true
			default:
				hard = true
			}
		}
	}
	has := func(r *recfmt.ShardRecord, prefix string) bool {
		for _, x := range r.Record.Reasons {
			if strings.HasPrefix(x, prefix) {
				return true
			}
		}
		return false
	}
	for _, c := range recs {
		r := c.r
		unclean := false
		for _, st := range r.Steps {
			unclean = unclean || !st.Clean
		}
		switch {
		case r.Unknown != "", r.Complete != nil && !*r.Complete, unclean,
			r.Status == recfmt.StatusBuilt && r.StepsSHA != w.steps[r.Action], has(r, "trace-incomplete:recipe-mismatch"):
			hard = true
		case has(r, "trace-incomplete") && (!w.honour || r.Complete == nil):
			hard = true
		}
		if r.Status != recfmt.StatusBuilt || r.StepsRun < r.StepsTotal || has(r, "daemon-left") {
			v.truncated[r.Action] = true
		}
	}
	// M2.
	for _, c := range recs {
		for _, u := range c.r.Record.UntrackedObservations {
			explained := false
			for _, p := range w.shards[c.shard].Inventory.Paths {
				explained = explained || isOv(u, p)
			}
			for _, o := range recs {
				if o.shard != c.shard || o.r.Action == c.r.Action {
					continue
				}
				for _, wr := range o.r.Record.Writes {
					explained = explained || isOv(u, wr)
				}
			}
			if !explained {
				hard = true
			}
		}
	}
	// Per-action unions and the tainted set.
	writes := map[string][]string{}
	tracked := map[string][]string{}
	binds := map[string][]string{}
	var tainted []string
	for _, c := range recs {
		a := c.r.Action
		writes[a] = append(writes[a], c.r.Record.Writes...)
		tracked[a] = append(tracked[a], c.r.Record.TrackedWrites...)
		binds[a] = append(binds[a], c.r.Record.Binds...)
		tainted = append(tainted, c.r.Record.TrackedWrites...)
	}
	assigned := map[string]int{}
	for _, p := range w.plan {
		for _, a := range p.Assigned {
			assigned[a] = p.Index
		}
	}
	daemonAction := map[string]bool{}
	for _, c := range recs {
		for _, x := range c.r.Record.Reasons {
			if strings.HasPrefix(x, "daemon:") || x == "daemon" || x == "daemon-left" {
				daemonAction[c.r.Action] = true
			}
		}
	}
	own := map[string]bool{}
	for a := range writes {
		dep := false // order dependence: M5-M14
		for _, c := range recs {
			if c.r.Action != a {
				continue
			}
			r := c.r
			if len(r.Record.Reasons) > 0 || r.Status != recfmt.StatusBuilt || r.Gate != "" || r.Refusal != "" {
				own[a] = true
			}
			inv := w.shards[c.shard].Inventory.Paths
			seen := append(append([]string{}, r.Record.UntrackedObservations...), r.Record.AbsentObservations...)
			for _, u := range seen {
				for b, ws := range writes {
					for _, wr := range ws {
						dep = dep || (b != a && isOv(u, wr))
					}
				}
				for _, i := range inv {
					dep = dep || isOv(u, i)
				}
			}
			for _, tr := range r.Record.Traversed {
				for b := range writes {
					if b == a {
						continue
					}
					for _, wr := range append(append([]string{}, writes[b]...), tracked[b]...) {
						dep = dep || isAnc(wr, tr)
					}
				}
				for _, i := range inv {
					dep = dep || isAnc(i, tr)
				}
			}
			for _, e := range r.Record.Entries {
				switch {
				case strings.HasPrefix(e, "list:"):
					d := e[5:]
					for b, ws := range writes {
						for _, wr := range ws {
							dep = dep || (b != a && isOv(d, wr))
						}
					}
					for _, tn := range tainted {
						dep = dep || isOv(d, tn)
					}
				default:
					p := e[strings.IndexByte(e, ':')+1:]
					for _, tn := range tainted {
						dep = dep || isOv(p, tn)
					}
				}
			}
		}
		for _, w1 := range writes[a] {
			for b, ws := range writes {
				for _, w2 := range ws {
					dep = dep || (b != a && isOv(w1, w2))
				}
			}
		}
		for _, tn := range tainted {
			dep = dep || isOv(a, tn) || tn == ".gitignore" || strings.HasSuffix(tn, "/.gitignore") ||
				isOv("code/programs/go/build-tool", tn) || isOv("code/kiln", tn)
		}
		// M13 is per ACTION: a daemon reported by any copy makes the action a
		// daemon action, and every copy's untracked observations count.
		for _, c := range recs {
			if !daemonAction[c.r.Action] || c.r.Action == a {
				continue
			}
			for _, u := range c.r.Record.UntrackedObservations {
				for _, wr := range writes[a] {
					dep = dep || isOv(u, wr)
				}
			}
			dep = dep || len(binds[a]) > 0
		}
		for b, bs := range binds {
			for _, x := range bs {
				for _, y := range binds[a] {
					dep = dep || (b != a && x == y)
				}
			}
		}
		if dep {
			v.order[a] = true
		}
	}
	for a := range v.truncated {
		_ = a
	}
	v.relaxedVoid = hard
	v.strictVoid = hard || len(v.order) > 0 || len(v.truncated) > 0
	for a := range writes {
		s, ok := assigned[a]
		base := ok && !shardVoid[s] && !own[a] && !v.order[a]
		v.strict[a] = !v.strictVoid && base
		v.relaxed[a] = !v.relaxedVoid && base && !v.truncated[a]
	}
	return v
}

// check runs the merge and the oracle on a world and compares them.
func check(t *testing.T, w *world) {
	t.Helper()
	r := w.merge()
	v := oracle(w)
	if r.Emitted == v.strictVoid {
		t.Fatalf("strict run void: merge says %v (%v), oracle says %v", !r.Emitted, r.RunVoid, v.strictVoid)
	}
	if r.relaxedOK() == v.relaxedVoid {
		t.Fatalf("relaxed run void: merge says %v (%v), oracle says %v", !r.relaxedOK(), r.RelaxedRunVoid, v.relaxedVoid)
	}
	n := 0
	for _, o := range r.Outcomes {
		if o.Emitted != v.strict[o.Action] {
			t.Fatalf("action %s: merge emitted=%v, oracle says %v\nresult %+v", o.Action, o.Emitted, v.strict[o.Action], r.Outcomes)
		}
		if o.RelaxedEmittable != v.relaxed[o.Action] {
			t.Fatalf("action %s: merge relaxed=%v, oracle says %v\nresult %+v", o.Action, o.RelaxedEmittable, v.relaxed[o.Action], r.Outcomes)
		}
		if o.Emitted {
			n++
		}
	}
	// The relaxed verdict never produces output: manifests and passes are the
	// strict emissions, exactly.
	if len(r.Manifests) != n || len(r.Passes) != n {
		t.Fatalf("%d manifests for %d strict emissions", len(r.Manifests), n)
	}
	// The invariants I1-I4, directly.
	if n > 0 {
		if r.RunVoidCount != 0 || len(v.truncated) != 0 || len(v.order) != 0 {
			t.Fatalf("something was emitted in a run with truncated or order-dependent actions: %v %v", v.truncated, v.order)
		}
		for _, s := range w.shards {
			if s.Header.TrackedChangedUnexplained != 0 || s.Header.PostInventoryUnexplained != 0 {
				t.Fatal("something was emitted although a shard has unexplained changes")
			}
		}
		for _, o := range r.Outcomes {
			if !o.Emitted {
				continue
			}
			for i, s := range w.shards {
				for k := range s.Records {
					if s.Records[k].Action != o.Action {
						continue
					}
					obs := append(append([]string{}, s.Records[k].Record.UntrackedObservations...), s.Records[k].Record.AbsentObservations...)
					for _, u := range obs {
						for _, other := range w.shards {
							for j := range other.Records {
								if other.Records[j].Action == o.Action {
									continue
								}
								for _, wr := range other.Records[j].Record.Writes {
									if isOv(u, wr) {
										t.Fatalf("emitted %s observes %s, written by %s", o.Action, u, other.Records[j].Action)
									}
								}
							}
						}
						for _, p := range w.shards[i].Inventory.Paths {
							if isOv(u, p) {
								t.Fatalf("emitted %s observes %s, which pre-existed", o.Action, u)
							}
						}
					}
				}
			}
		}
	}
	// Determinism: the reverse shard order gives the same verdict.
	w2 := *w
	w2.shards = nil
	for i := len(w.shards) - 1; i >= 0; i-- {
		w2.shards = append(w2.shards, w.shards[i])
	}
	if !equalResult(r, w2.merge()) {
		t.Fatal("the result depends on the order of the shards")
	}
}

func FuzzMerge(f *testing.F) {
	for _, seed := range [][]byte{
		{},
		{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
		{0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00},
		{0x03, 0x10, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd},
		[]byte("a seed with some letters in it to make odd shapes"),
		[]byte("\x01\x00\x80\x40\x20\x10\x08\x04\x02\x01\x80\x40\x20\x10\x08\x04\x02\x01"),
		{0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55},
		{0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff},
		{0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00},
	} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) > 4096 {
			t.Skip()
		}
		check(t, genWorld(t, data))
	})
}

// The path index against a brute-force scan, over every pair from the vocabulary.
func TestIndexAgainstBruteForce(t *testing.T) {
	paths := append(append([]string{}, vocab...), "a/b/c/d", "a/b/d", "c", "c/d/e/f")
	x := newPathIndex(nil)
	for i, p := range paths {
		x.add(p, i)
		x.add(p, i) // a repeated add of the same id is a no-op
	}
	x.freeze()
	queries := append(append([]string{}, paths...), ".", "a/b", "zz")
	for _, q := range queries {
		var anc, desc, desc2, ov []string
		x.ancestors(q, func(p string, _ []int) bool { anc = append(anc, p); return true })
		x.descendants(q, true, func(p string, _ []int) bool { desc = append(desc, p); return true })
		x.descendants(q, false, func(p string, _ []int) bool { desc2 = append(desc2, p); return true })
		x.overlapping(q, func(p string, _ []int) bool { ov = append(ov, p); return true })
		var wa, wd, wd2, wo []string
		for _, p := range sl(paths...) {
			if isAnc(p, q) {
				wa = append(wa, p)
			}
			if isAnc(q, p) && p != q {
				wd = append(wd, p)
			}
			if isAnc(q, p) {
				wd2 = append(wd2, p)
			}
			if isOv(p, q) {
				wo = append(wo, p)
			}
		}
		if q == "." {
			wa = nil // "." is never indexed, and has no ancestors
		}
		same := func(a, b []string) bool { return strings.Join(sl(a...), "|") == strings.Join(sl(b...), "|") }
		if !same(anc, wa) || !same(desc, wd) || !same(desc2, wd2) || !same(ov, wo) {
			t.Errorf("%q: anc %v/%v desc %v/%v desc2 %v/%v ov %v/%v", q, anc, wa, desc, wd, desc2, wd2, ov, wo)
		}
		// Early stop works for every walk.
		n := 0
		x.overlapping(q, func(string, []int) bool { n++; return false })
		if n > 1 {
			t.Errorf("%q: the walk did not stop", q)
		}
		n = 0
		x.descendants(q, false, func(string, []int) bool { n++; return false })
		x.ancestors(q, func(string, []int) bool { n++; return false })
		if n > 2 {
			t.Errorf("%q: walks did not stop", q)
		}
	}
	// A pathSet answers the same questions without owners.
	s := newPathSet([]string{"a", "a/b", "c"}, nil)
	var got []string
	s.overlapping("a/b/x", func(p string) bool { got = append(got, p); return true })
	s.under("a", func(p string) bool { got = append(got, "u:"+p); return true })
	s.ancestors("a/b", func(p string) bool { got = append(got, "n:"+p); return true })
	s.all(func(p string) bool { got = append(got, "all:"+p); return false })
	if strings.Join(got, ",") != "a,a/b,u:a,u:a/b,n:a,n:a/b,all:a" {
		t.Errorf("%v", got)
	}
	if !anyOther([]int{1, 2}, 1) || anyOther([]int{1}, 1) || anyOther(nil, 1) {
		t.Error("anyOther")
	}
}

func TestReasonSetTrimsDeterministically(t *testing.T) {
	var r reasons
	for i := 0; i < 2000; i++ {
		r.add("reads-product:p" + strings.Repeat("x", i%700) + string(rune('a'+i%26)))
		r.add("trace-incomplete:unstable-path")
	}
	if !r.any || len(r.m) > 600 {
		t.Fatalf("%d reasons kept", len(r.m))
	}
	if !r.m["trace-incomplete:unstable-path"] {
		t.Error("a kind was lost by trimming")
	}
}
