package recmerge

// The fuzz target and its oracle.
//
// FuzzMerge builds a random but BINDING-VALID world from the fuzzer's bytes
// (so every B check passes and only the rules M1-M15 decide), runs Merge, and
// compares the emitted set with an independent oracle.
//
// "Independent" is the point.  The oracle shares nothing with the merge:
//
//   - paths are compared as SEGMENT LISTS, not as strings with a "/" check;
//   - there is no index: every rule is a plain nested loop over every
//     record, O(n squared) and proud of it;
//   - it is written from the rule table of KILN04 section 7.1, not from
//     merge.go.
//
// The stated invariant it checks is the plan's: if action a is emitted then no
// path in any of a's observation lists overlaps another action's write, the
// pre-BUILD inventory, or the tainted set, and the run has no unknown write
// set.  The oracle goes further and demands the SAME emitted set, so a rule
// that is too strict (a lost pass) fails the fuzz target as surely as one that
// is too lax (a false hit).
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

var reasonVocab = []string{"daemon:x", "network:y", "trace-incomplete:size", "host-read:z"}
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
	for si, sh := range w.shards {
		if b.next() {
			sh.Inventory.Paths = b.subset(vocab[1:10], true)
		}
		for _, tok := range []string{"tracked-changed", "state-preloaded"} {
			if b.next() && b.next() {
				sh.Header.Void = sl(append(sh.Header.Void, tok)...)
			}
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
			if b.next() && b.next() && b.next() {
				r.Status = "failed"
			}
			if b.next() && b.next() && b.next() {
				r.Unknown = "tracer:status"
			}
			if b.next() && b.next() {
				y := b.next()
				r.Complete = &y
			}
			_ = si
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

// oracle returns whether the run is void and the set of emitted actions.
func oracle(w *world) (bool, map[string]bool) {
	var recs []orec
	for i, s := range w.shards {
		for k := range s.Records {
			recs = append(recs, orec{i, &s.Records[k]})
		}
	}
	void := false
	shardVoid := map[int]bool{}
	for i, s := range w.shards {
		if s.Header.PostInventoryUnexplained > 0 {
			void = true
		}
		for _, tok := range s.Header.Void {
			switch tok {
			case "tracked-changed", "state-preloaded":
				shardVoid[i] = true
			default:
				void = true
			}
		}
	}
	hasTI := func(r *recfmt.ShardRecord) bool {
		for _, x := range r.Record.Reasons {
			if strings.HasPrefix(x, "trace-incomplete") {
				return true
			}
		}
		return false
	}
	for _, c := range recs {
		r := c.r
		switch {
		case r.Unknown != "", r.StepsRun < r.StepsTotal, r.Complete != nil && !*r.Complete:
			void = true
		case hasTI(r) && (!w.honour || r.Complete == nil):
			void = true
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
				void = true
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
			if strings.HasPrefix(x, "daemon:") || x == "daemon" {
				daemonAction[c.r.Action] = true
			}
		}
	}
	emitted := map[string]bool{}
	for a := range writes {
		hit := false
		for _, c := range recs {
			if c.r.Action != a {
				continue
			}
			r := c.r
			if len(r.Record.Reasons) > 0 || r.Status != recfmt.StatusBuilt {
				hit = true
			}
			inv := w.shards[c.shard].Inventory.Paths
			seen := append(append([]string{}, r.Record.UntrackedObservations...), r.Record.AbsentObservations...)
			for _, u := range seen {
				for b, ws := range writes {
					for _, wr := range ws {
						hit = hit || (b != a && isOv(u, wr))
					}
				}
				for _, i := range inv {
					hit = hit || isOv(u, i)
				}
			}
			for _, tr := range r.Record.Traversed {
				for b := range writes {
					if b == a {
						continue
					}
					for _, wr := range append(append([]string{}, writes[b]...), tracked[b]...) {
						hit = hit || isAnc(wr, tr)
					}
				}
				for _, i := range inv {
					hit = hit || isAnc(i, tr)
				}
			}
			for _, e := range r.Record.Entries {
				switch {
				case strings.HasPrefix(e, "list:"):
					d := e[5:]
					for b, ws := range writes {
						for _, wr := range ws {
							hit = hit || (b != a && wr != d && isAnc(d, wr))
						}
					}
					for _, tn := range tainted {
						hit = hit || isAnc(d, tn)
					}
				default:
					p := e[strings.IndexByte(e, ':')+1:]
					for _, tn := range tainted {
						hit = hit || isOv(p, tn)
					}
				}
			}
		}
		for _, w1 := range writes[a] {
			for b, ws := range writes {
				for _, w2 := range ws {
					hit = hit || (b != a && isOv(w1, w2))
				}
			}
		}
		for _, tn := range tainted {
			hit = hit || isOv(a, tn) || tn == ".gitignore" || strings.HasSuffix(tn, "/.gitignore") ||
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
					hit = hit || isOv(u, wr)
				}
			}
			hit = hit || len(binds[a]) > 0
		}
		for b, bs := range binds {
			for _, x := range bs {
				for _, y := range binds[a] {
					hit = hit || (b != a && x == y)
				}
			}
		}
		s, ok := assigned[a]
		emitted[a] = !void && ok && !shardVoid[s] && !hit
	}
	return void, emitted
}

// check runs the merge and the oracle on a world and compares them.
func check(t *testing.T, w *world) {
	t.Helper()
	r := w.merge()
	void, want := oracle(w)
	if r.Emitted == void {
		t.Fatalf("run void: merge says %v (%v), oracle says %v", !r.Emitted, r.RunVoid, void)
	}
	got := map[string]bool{}
	for _, o := range r.Outcomes {
		if o.Emitted {
			got[o.Action] = true
		}
	}
	for a, e := range want {
		if e != got[a] {
			t.Fatalf("action %s: merge emitted=%v, oracle says %v\nresult %+v", a, got[a], e, r.Outcomes)
		}
	}
	// The stated invariant, directly: an emitted action has no unknown write
	// set anywhere and no observation overlapping a write, the inventory or
	// the tainted set.  (The equality above implies it; this keeps the
	// sentence of the plan under test in its own words.)
	for a := range got {
		if void {
			t.Fatalf("%s emitted in a void run", a)
		}
		for _, c := range func() []orec {
			var o []orec
			for i, s := range w.shards {
				for k := range s.Records {
					if s.Records[k].Action == a {
						o = append(o, orec{i, &s.Records[k]})
					}
				}
			}
			return o
		}() {
			obs := append(append([]string{}, c.r.Record.UntrackedObservations...), c.r.Record.AbsentObservations...)
			for _, o := range w.shards {
				for k := range o.Records {
					if o.Records[k].Action == a {
						continue
					}
					for _, u := range obs {
						for _, wr := range o.Records[k].Record.Writes {
							if isOv(u, wr) {
								t.Fatalf("emitted %s observes %s, written by %s", a, u, o.Records[k].Action)
							}
						}
					}
				}
			}
			for _, u := range obs {
				for _, i := range w.shards[c.shard].Inventory.Paths {
					if isOv(u, i) {
						t.Fatalf("emitted %s observes %s, which pre-existed", a, u)
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
	x := newPathIndex()
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
	s := newPathSet([]string{"a", "a/b", "c"})
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
