package recmerge

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// ---------------------------------------------------------------------------
// The baseline and emission.

func TestCleanWorldEmitsEveryAction(t *testing.T) {
	w := std(t)
	r := w.merge()
	if !r.Emitted || r.RunVoidCount != 0 || len(r.RunVoid) != 0 {
		t.Fatalf("%+v", r.RunVoid)
	}
	wantRelaxed(t, r, "pkg/a", "pkg/b", "pkg/c", "pkg/d")
	if len(r.Manifests) != 4 || len(r.Passes) != 4 {
		t.Fatalf("%d manifests, %d passes", len(r.Manifests), len(r.Passes))
	}
	for i, m := range r.Manifests {
		p := r.Passes[i]
		if m.Target != p.Target || m.OS != "linux" || m.Arch != "x64" || m.Revision != rev || m.Run != "42" ||
			p.Kind != "pass" || p.Kiln != "kiln/0.1.0 traced/2 recorder/1" || m.At.Nanosecond() != 0 ||
			!m.At.Equal(p.At) || m.At.Location().String() != "UTC" {
			t.Errorf("manifest %+v pass %+v", m, p)
		}
		if len(m.Entries) != 1 || m.Entries[0] != "pkg:"+m.Target {
			t.Errorf("entries %v", m.Entries)
		}
	}
	if r.Manifests[0].Target != "pkg/a" || r.Manifests[3].Target != "pkg/d" {
		t.Error("manifests are not sorted by action")
	}
}

// G3: every copy is checked, only the assigned shard's copy is emitted, and
// the manifest and keys come from THAT copy's shard.
func TestOnlyTheAssignedCopyIsEmitted(t *testing.T) {
	w := std(t)
	w.entries(0, "pkg/a", "path:from/shard0")
	w.entries(1, "pkg/a", "path:from/shard1")
	r := w.merge()
	wantRelaxed(t, r, "pkg/a", "pkg/b", "pkg/c", "pkg/d")
	m := r.Manifests[0]
	if m.Target != "pkg/a" || strings.Join(m.Entries, ",") != "path:from/shard0,pkg:pkg/a" {
		t.Fatalf("%v", m.Entries)
	}
	if m.Weak != weakOf(w.shards[0].Facts, "pkg/a") {
		t.Error("the weak key is not the assigned shard's")
	}
	if r.Passes[0].Key != mustStrong(t, w, 0, "pkg/a", m.Entries) {
		t.Error("the strong key is not the assigned shard's")
	}
}

func mustStrong(t *testing.T, w *world, shard int, action string, names []string) [32]byte {
	t.Helper()
	k, err := w.keys.Strong(w.shards[shard].Facts, action, names)
	if err != nil {
		t.Fatal(err)
	}
	return k
}

// G3: a reason on a PREREQUISITE copy in another shard voids the action.
func TestReasonOnAPrerequisiteCopyVoidsTheAction(t *testing.T) {
	w := std(t)
	w.rec(1, "pkg/a").Record.Reasons = []string{"network:connect"}
	r := w.merge()
	if !r.relaxedOK() {
		t.Fatalf("a per-action reason must not void the run: %v", r.RunVoid)
	}
	wantRelaxed(t, r, "pkg/b", "pkg/c", "pkg/d")
	if !r.out(t, "pkg/a").has("network:connect") {
		t.Error("the reason did not reach the action")
	}
}

func TestPerCopyStatusGateAndRefusal(t *testing.T) {
	cases := map[string]func(w *world){
		"status": func(w *world) { w.rec(1, "pkg/a").Status = "failed" },
		"gate":   func(w *world) { w.rec(0, "pkg/a").Gate = "submodule" },
		"refusal": func(w *world) {
			w.keys.refuse = map[string]string{"pkg/a": "no-build-file:linux"}
			w.all("pkg/a", func(r *recfmt.ShardRecord) { r.Weak = ""; r.Refusal = "no-build-file:linux" })
		},
	}
	for name, mut := range cases {
		w := std(t)
		mut(w)
		r := w.merge()
		if !r.relaxedOK() {
			t.Fatalf("%s: %v", name, r.RunVoid)
		}
		wantRelaxed(t, r, "pkg/b", "pkg/c", "pkg/d")
		if r.out(t, "pkg/a").Recordable {
			t.Errorf("%s: recordable", name)
		}
	}
}

func TestEmissionRefusals(t *testing.T) {
	cases := map[string]struct {
		mut  func(w *world)
		want string
	}{
		"too many entries": {func(w *world) {
			e := []string{}
			for i := 0; i < 8193; i++ {
				e = append(e, "path:"+strings.Repeat("x", 3)+string(rune('a'+i%26))+strings.Repeat("y", i/26))
			}
			w.rec(1, "pkg/c").Record.Entries = sl(append(e, "pkg:pkg/c")...)
		}, "manifest-too-large"},
		"long name": {func(w *world) {
			w.entries(1, "pkg/c", "path:"+strings.Repeat("x", 600))
		}, "manifest-too-large"},
		"no own pkg": {func(w *world) {
			w.rec(1, "pkg/c").Record.Entries = []string{"path:x"}
		}, "manifest-invalid"},
		"strong key fails": {func(w *world) { w.keys.strongErr = errString("tree changed") }, "manifest-invalid"},
	}
	for name, c := range cases {
		w := std(t)
		c.mut(w)
		r := w.merge()
		if !r.relaxedOK() {
			t.Fatalf("%s: run void %v", name, r.RunVoid)
		}
		got := r.out(t, "pkg/c")
		if got.RelaxedEmittable || !got.hasKind(c.want) {
			t.Errorf("%s: %+v", name, got)
		}
		if !got.Recordable && name != "strong key fails" {
			t.Errorf("%s: shape problems are not rule hits", name)
		}
	}
	// Ledger-level validation also guards the output.
	w := std(t)
	in := w.input()
	in.Kiln = "kiln\nbad"
	if r := Merge(in); len(r.Manifests) != 0 || !r.out(t, "pkg/a").hasKind("manifest-invalid") {
		t.Errorf("an invalid pass record was emitted: %+v", r.Outcomes)
	}
}

type errString string

func (e errString) Error() string { return string(e) }

// ---------------------------------------------------------------------------
// M5, M6: another action's write.

func TestM5UntrackedObservationOfAnotherActionsWrite(t *testing.T) {
	for _, p := range []struct{ reader, writer string }{{"pkg/a", "pkg/b"}, {"pkg/b", "pkg/a"}} {
		w := std(t)
		w.rec(0, p.writer).Record.Writes = sl("out/x")
		w.rec(0, p.reader).Record.UntrackedObservations = sl("out/x")
		r := w.merge()
		if !r.relaxedOK() {
			t.Fatalf("%v", r.RunVoid)
		}
		if got := r.out(t, p.reader); got.RelaxedEmittable || !got.has("reads-product:out/x") {
			t.Errorf("%s did not lose: %+v", p.reader, got)
		}
		// The writer is not a reader, so it is not voided by M5 (M10 would need
		// an overlapping write root).
		if !r.out(t, p.writer).RelaxedEmittable {
			t.Errorf("%s was voided", p.writer)
		}
	}
}

func TestM5OverlapGoesBothWays(t *testing.T) {
	for name, c := range map[string]struct{ write, observe string }{
		"write under observed dir":  {"out/x/y", "out/x"},
		"observed under write root": {"out", "out/x/y"},
		"equal":                     {"out/x", "out/x"},
	} {
		w := std(t)
		w.rec(0, "pkg/b").Record.Writes = sl(c.write)
		w.rec(0, "pkg/a").Record.UntrackedObservations = sl(c.observe)
		r := w.merge()
		if got := r.out(t, "pkg/a"); got.RelaxedEmittable || !got.has("reads-product:"+c.write) {
			t.Errorf("%s: %+v", name, got)
		}
	}
}

// A path that merely shares a spelling prefix is not inside the directory.
func TestPathPrefixConfusionIsNotAnOverlap(t *testing.T) {
	w := std(t)
	w.inv(0, "out/x") // explains the observation (and is itself an M9 hit)
	w.rec(0, "pkg/b").Record.Writes = sl("out/xy", "out/x-y", "out/x.z")
	w.rec(0, "pkg/a").Record.UntrackedObservations = sl("out/x")
	w.rec(0, "pkg/a").Record.AbsentObservations = sl("out/x")
	r := w.merge()
	for _, bad := range []string{"out/xy", "out/x-y", "out/x.z"} {
		if r.out(t, "pkg/a").has("reads-product:" + bad) {
			t.Errorf("a/b matched a/bc: %s", bad)
		}
	}
	if !r.out(t, "pkg/a").has("reads-product:out/x") {
		t.Error("the inventory hit is missing")
	}
	if !r.out(t, "pkg/b").RelaxedEmittable {
		t.Error("b was voided by a name that only shares a prefix")
	}
	// anc is exact.
	for _, c := range []struct {
		p, q string
		want bool
	}{{"a/b", "a/b", true}, {"a/b", "a/b/c", true}, {"a/b", "a/bc", false}, {"a/bc", "a/b", false}, {"a/b", "a", false},
		{".", "a", true}, {"a", ".", false}, {".", ".", true}, {"a", "ab", false}, {"", "a", false}} {
		if anc(c.p, c.q) != c.want {
			t.Errorf("anc(%q, %q) != %v", c.p, c.q, c.want)
		}
	}
	if !overlap("a/b/c", "a/b") || overlap("a/b", "a/bc") || !overlap(".", "x") {
		t.Error("overlap")
	}
}

func TestM5AcrossShards(t *testing.T) {
	// The reader is in shard 0 and the writer in shard 1.  The reader's
	// observation is explained in ITS shard by the inventory, so the run is not
	// void; the other shard's write is still a hit.
	w := std(t)
	w.inv(0, "out")
	w.rec(1, "pkg/c").Record.Writes = sl("out/x")
	w.rec(0, "pkg/b").Record.UntrackedObservations = sl("out/x")
	r := w.merge()
	if !r.relaxedOK() {
		t.Fatalf("%v", r.RunVoid)
	}
	if got := r.out(t, "pkg/b"); got.RelaxedEmittable || !got.has("reads-product:out/x") || !got.has("reads-product:out") {
		t.Errorf("%+v", got)
	}
	if !r.out(t, "pkg/c").RelaxedEmittable {
		t.Error("the writer was voided")
	}
}

func TestM5WriterIsAPrerequisiteCopyInTheReadersShard(t *testing.T) {
	// pkg/a runs in shard 1 only as c's prerequisite; its copy writes gen/out
	// and c probes the same name absent.  The copy counts as a's write.
	w := std(t)
	w.rec(1, "pkg/a").Record.Writes = sl("gen/out")
	w.rec(1, "pkg/c").Record.AbsentObservations = sl("gen/out")
	r := w.merge()
	if got := r.out(t, "pkg/c"); got.RelaxedEmittable || !got.has("reads-product:gen/out") {
		t.Errorf("%+v", got)
	}
	if !r.out(t, "pkg/a").RelaxedEmittable {
		t.Error("a was voided by its own write")
	}
}

func TestM6AbsentName(t *testing.T) {
	// Positive, in both orders, across shards: nothing needs explaining.
	for _, p := range []struct {
		reader, writer string
		rshard, wshard int
	}{{"pkg/a", "pkg/c", 0, 1}, {"pkg/c", "pkg/a", 1, 0}, {"pkg/b", "pkg/d", 0, 2}, {"pkg/d", "pkg/b", 2, 0}} {
		w := std(t)
		w.rec(p.wshard, p.writer).Record.Writes = sl("tmp/made")
		w.rec(p.rshard, p.reader).Record.AbsentObservations = sl("tmp/made/x")
		r := w.merge()
		if got := r.out(t, p.reader); got.RelaxedEmittable || !got.has("reads-product:tmp/made") {
			t.Errorf("%+v: %+v", p, got)
		}
	}
	// Negative: a different name.
	w := std(t)
	w.rec(1, "pkg/c").Record.Writes = sl("tmp/other")
	w.rec(0, "pkg/b").Record.AbsentObservations = sl("tmp/made")
	if r := w.merge(); !r.out(t, "pkg/b").RelaxedEmittable || !r.out(t, "pkg/c").RelaxedEmittable {
		t.Error("a non-overlapping probe was voided")
	}
}

// ---------------------------------------------------------------------------
// M7: a changed listing.

func TestM7ChangedListing(t *testing.T) {
	cases := map[string]struct {
		d      string
		writes string
		hit    bool
	}{
		"sibling created":  {"dir", "dir/new", true},
		"deeper write":     {"dir", "dir/sub/deep/new", true},
		"root listing":     {".", "anywhere", true},
		"other directory":  {"dir", "dir2/new", false},
		"prefix confusion": {"dir", "dirx", false},
		"parent write":     {"dir/sub", "dir/other", false},
	}
	for name, c := range cases {
		for _, p := range [][2]string{{"pkg/a", "pkg/b"}, {"pkg/b", "pkg/a"}} {
			w := std(t)
			w.entries(0, p[0], "list:"+c.d)
			w.rec(0, p[1]).Record.Writes = sl(c.writes)
			r := w.merge()
			got := r.out(t, p[0])
			if got.hasKind("reads-product") != c.hit || got.RelaxedEmittable == c.hit {
				t.Errorf("%s %v: %+v", name, p, got)
			}
			if c.hit && !got.has("reads-product:"+c.writes) {
				t.Errorf("%s: %+v", name, got)
			}
		}
	}
	// Across shards.
	w := std(t)
	w.entries(0, "pkg/b", "list:dir")
	w.rec(2, "pkg/d").Record.Writes = sl("dir/new")
	if r := w.merge(); r.out(t, "pkg/b").RelaxedEmittable || !r.out(t, "pkg/b").has("reads-product:dir/new") {
		t.Error("a listing changed by another shard's write was kept")
	}
}

// ---------------------------------------------------------------------------
// M8: a traversed component another action replaced.

func TestM8TraversedComponentReplacedInAnotherShard(t *testing.T) {
	w := std(t)
	w.rec(0, "pkg/b").Record.Traversed = sl("link", "link/sub")
	w.rec(1, "pkg/c").Record.Writes = sl("link") // unlinked and re-created
	r := w.merge()
	if !r.relaxedOK() {
		t.Fatalf("an unstable path is the action's problem, not the run's: %v", r.RunVoid)
	}
	if got := r.out(t, "pkg/b"); got.RelaxedEmittable || !got.has("trace-incomplete:unstable-path") {
		t.Errorf("%+v", got)
	}
	if !r.out(t, "pkg/c").RelaxedEmittable {
		t.Error("the replacer was voided")
	}
}

func TestM8TrackedDirectoryRenamedByAnotherAction(t *testing.T) {
	w := std(t)
	w.rec(0, "pkg/a").Record.Traversed = sl("tdir", "tdir/inner")
	w.rec(0, "pkg/b").Record.TrackedWrites = sl("tdir")
	r := w.merge()
	if got := r.out(t, "pkg/a"); got.RelaxedEmittable || !got.has("trace-incomplete:unstable-path") {
		t.Errorf("%+v", got)
	}
}

func TestM8NegativesAndOwnWrites(t *testing.T) {
	w := std(t)
	w.rec(0, "pkg/b").Record.Traversed = sl("link")
	w.rec(0, "pkg/b").Record.Writes = sl("link") // its own write: its writes are known
	w.rec(0, "pkg/a").Record.Traversed = sl("link2", "d/e")
	w.rec(1, "pkg/c").Record.Writes = sl("d/e/f") // beneath the traversed component, not above it
	r := w.merge()
	if !r.out(t, "pkg/b").RelaxedEmittable {
		t.Errorf("an action was voided by its own write: %+v", r.out(t, "pkg/b"))
	}
	if r.out(t, "pkg/a").hasKind("trace-incomplete") {
		t.Errorf("%+v", r.out(t, "pkg/a"))
	}
}

// ---------------------------------------------------------------------------
// M9: pre-existing leftovers.

func TestM9InventoryAgainstEachObservationList(t *testing.T) {
	for _, list := range []string{"untracked", "absent"} {
		for name, c := range map[string]struct {
			inv, obs string
			hit      bool
		}{
			"equal":      {"nm", "nm", true},
			"ancestor":   {"nm", "nm/pkg/x", true},
			"descendant": {"nm/pkg", "nm", true},
			"sibling":    {"nmx", "nm", false},
			"unrelated":  {"other", "nm", false},
		} {
			w := std(t)
			w.inv(0, c.inv)
			r0 := w.rec(0, "pkg/a")
			if list == "untracked" {
				// Explained by the inventory only when it overlaps; otherwise M2 fires.
				r0.Record.UntrackedObservations = sl(c.obs)
			} else {
				r0.Record.AbsentObservations = sl(c.obs)
			}
			r := w.merge()
			got := r.out(t, "pkg/a")
			if got.has("reads-product:"+c.inv) != c.hit {
				t.Errorf("%s/%s: %+v", list, name, got)
			}
			if list == "untracked" && !c.hit && !r.cause("M2") {
				t.Errorf("%s/%s: an unexplained observation must void the run", list, name)
			}
			if list == "absent" && !c.hit && !got.RelaxedEmittable {
				t.Errorf("%s/%s: %+v", list, name, got)
			}
		}
	}
	// Other shards' inventories are not this shard's leftovers.
	w := std(t)
	w.inv(1, "nm")
	w.rec(0, "pkg/b").Record.AbsentObservations = sl("nm")
	if r := w.merge(); !r.out(t, "pkg/b").RelaxedEmittable {
		t.Error("another shard's inventory was applied")
	}
}

func TestM9TraversedUnderLeftover(t *testing.T) {
	for name, c := range map[string]struct {
		inv, trav string
		hit       bool
	}{
		"equal":      {"nm", "nm", true},
		"ancestor":   {"nm", "nm/sub", true},
		"descendant": {"nm/sub", "nm", false},
		"sibling":    {"nmx", "nm/sub", false},
	} {
		w := std(t)
		w.inv(0, c.inv)
		w.rec(0, "pkg/a").Record.Traversed = sl(c.trav)
		got := w.merge().out(t, "pkg/a")
		if got.has("trace-incomplete:unstable-path") != c.hit {
			t.Errorf("%s: %+v", name, got)
		}
	}
}

// ---------------------------------------------------------------------------
// M10: overlapping write roots.

func TestM10OverlappingWriteRoots(t *testing.T) {
	for name, c := range map[string]struct {
		a, b string
		hit  bool
	}{
		"equal":   {"out", "out", true},
		"nested":  {"out", "out/x", true},
		"nested2": {"out/x/y", "out/x", true},
		"sibling": {"out/a", "out/b", false},
		"prefix":  {"out/a", "out/ab", false},
	} {
		w := std(t)
		w.rec(0, "pkg/a").Record.Writes = sl(c.a)
		w.rec(0, "pkg/b").Record.Writes = sl(c.b)
		r := w.merge()
		for _, who := range []string{"pkg/a", "pkg/b"} {
			if got := r.out(t, who); got.hasKind("reads-product") != c.hit || got.RelaxedEmittable == c.hit {
				t.Errorf("%s %s: %+v", name, who, got)
			}
		}
	}
}

func TestM10SharedRustTargetVoidsAllThree(t *testing.T) {
	w := std(t)
	const target = "code/packages/rust/target"
	w.rec(0, "pkg/a").Record.Writes = sl(target)
	w.rec(0, "pkg/b").Record.Writes = sl(target)
	w.rec(1, "pkg/c").Record.Writes = sl(target)
	r := w.merge()
	wantRelaxed(t, r, "pkg/d")
	for _, a := range []string{"pkg/a", "pkg/b", "pkg/c"} {
		if !r.out(t, a).has("reads-product:" + target) {
			t.Errorf("%s: %+v", a, r.out(t, a))
		}
	}
}

func TestAnActionNeverVoidsItselfThroughItsOwnWrites(t *testing.T) {
	w := std(t)
	// pkg/a runs in shards 0 and 1 (a prerequisite copy) and writes the same
	// root in both, plus a nested write, a probe of it and a listing of it.
	w.all("pkg/a", func(r *recfmt.ShardRecord) { r.Record.Writes = sl("gen", "gen/x") })
	w.all("pkg/a", func(r *recfmt.ShardRecord) { r.Record.AbsentObservations = sl("gen/y") })
	w.all("pkg/a", func(r *recfmt.ShardRecord) { r.Record.Entries = sl("pkg:pkg/a", "list:gen") })
	w.all("pkg/a", func(r *recfmt.ShardRecord) { r.Record.Traversed = sl("gen") })
	r := w.merge()
	if !r.out(t, "pkg/a").RelaxedEmittable {
		t.Fatalf("%+v", r.out(t, "pkg/a"))
	}
}

// ---------------------------------------------------------------------------
// M11, M12: tainted tracked paths.

func TestM11TaintedSet(t *testing.T) {
	for name, c := range map[string]struct {
		entry   string
		tainted string
		hit     bool
	}{
		"path equal":         {"path:cfg/x.json", "cfg/x.json", true},
		"path under tainted": {"path:cfg/sub/y", "cfg/sub", true},
		"path above tainted": {"path:cfg", "cfg/x.json", true},
		"path sibling":       {"path:cfg/x", "cfg/xy", false},
		"pkg overlap":        {"pkg:lib/q", "lib/q/src/main.go", true},
		"pkg nested package": {"pkg:lib/q", "lib/q/inner/f", true},
		"pkg sibling":        {"pkg:lib/q", "lib/qq/f", false},
		"list above":         {"list:cfg", "cfg/x.json", true},
		"list root":          {"list:.", "cfg/x.json", true},
		"list exact":         {"list:cfg/x.json", "cfg/x.json", true},
		// L-6: a directory listed BENEATH a tainted path overlaps it (the
		// tainted path may be a directory that was moved away).
		"list below the tainted": {"list:cfg/x.json/inside", "cfg/x.json", true},
		"list sibling":           {"list:cfg/sub", "cfg/x.json", false},
		"list prefix":            {"list:cf", "cfg/x.json", false},
	} {
		// B writes a tracked file; A reads it.  Both orders of who is who.
		for _, p := range [][2]string{{"pkg/a", "pkg/b"}, {"pkg/b", "pkg/a"}} {
			w := std(t)
			w.entries(0, p[0], c.entry)
			w.rec(0, p[1]).Record.TrackedWrites = sl(c.tainted)
			got := w.merge().out(t, p[0])
			if got.hasKind("reads-product") != c.hit || got.RelaxedEmittable == c.hit {
				t.Errorf("%s %v: %+v", name, p, got)
			}
			if c.hit && !got.has("reads-product:"+c.tainted) {
				t.Errorf("%s: %+v", name, got)
			}
		}
	}
	// A different shard's taint counts, and an action's own taint counts too.
	w := std(t)
	w.entries(0, "pkg/b", "path:cfg/x")
	w.rec(2, "pkg/d").Record.TrackedWrites = sl("cfg/x")
	if r := w.merge(); r.out(t, "pkg/b").RelaxedEmittable {
		t.Error("another shard's tracked write was ignored")
	}
	w = std(t)
	w.entries(0, "pkg/b", "path:cfg/x")
	w.rec(0, "pkg/b").Record.TrackedWrites = sl("cfg/x")
	if r := w.merge(); r.out(t, "pkg/b").RelaxedEmittable {
		t.Error("an action that rewrote a tracked file it read was kept")
	}
}

func TestM12TaintedRecipeAndMetadata(t *testing.T) {
	for name, c := range map[string]struct {
		tainted string
		hit     []string // actions that lose
	}{
		"its BUILD":             {"pkg/a/BUILD", []string{"pkg/a"}},
		"its BUILD variant":     {"pkg/a/BUILD_windows", []string{"pkg/a"}},
		"its directory":         {"pkg/a", []string{"pkg/a"}},
		"an ancestor directory": {"pkg", []string{"pkg/a", "pkg/b", "pkg/c", "pkg/d"}},
		"a sibling package":     {"pkg/ab/f", nil},
		"gitignore":             {".gitignore", []string{"pkg/a", "pkg/b", "pkg/c", "pkg/d"}},
		"nested gitignore":      {"pkg/a/.gitignore", []string{"pkg/a", "pkg/b", "pkg/c", "pkg/d"}},
		"not a gitignore":       {"pkg/ab/x.gitignore", nil},
		"build tool":            {"code/programs/go/build-tool/main.go", []string{"pkg/a", "pkg/b", "pkg/c", "pkg/d"}},
		"build tool ancestor":   {"code/programs/go", []string{"pkg/a", "pkg/b", "pkg/c", "pkg/d"}},
		"kiln":                  {"code/kiln/legacy/env.json", []string{"pkg/a", "pkg/b", "pkg/c", "pkg/d"}},
		"build tool lookalike":  {"code/programs/go/build-tool2/x", nil},
	} {
		// "B rewrites A's BUILD" is always another action's tracked write; it
		// is a hit whether or not B restores it (the record cannot tell).
		w := std(t)
		w.rec(2, "pkg/d").Record.TrackedWrites = sl(c.tainted)
		r := w.merge()
		lose := map[string]bool{}
		for _, a := range c.hit {
			lose[a] = true
		}
		for _, a := range []string{"pkg/a", "pkg/b", "pkg/c", "pkg/d"} {
			if got := r.out(t, a); got.RelaxedEmittable == lose[a] {
				t.Errorf("%s: %s: %+v", name, a, got)
			}
		}
	}
}

// ---------------------------------------------------------------------------
// M13, M14.

func TestM13DaemonPeers(t *testing.T) {
	setup := func() *world {
		w := std(t)
		w.inv(1, "srv")
		w.rec(1, "pkg/c").Record.Reasons = sl("daemon:/work/r/srv/sock")
		w.rec(1, "pkg/c").Record.UntrackedObservations = sl("srv/sock")
		return w
	}
	// Another action whose write root holds the socket path.
	w := setup()
	w.rec(2, "pkg/d").Record.Writes = sl("srv")
	r := w.merge()
	if got := r.out(t, "pkg/d"); got.RelaxedEmittable || !got.has("reads-product:daemon-peer") {
		t.Errorf("%+v", got)
	}
	if got := r.out(t, "pkg/b"); !got.RelaxedEmittable {
		t.Errorf("an unrelated action was voided: %+v", got)
	}
	// Another action that only bound a port.
	w = setup()
	w.rec(0, "pkg/b").Record.Binds = sl("stream/127.0.0.1:8080")
	if got := w.merge().out(t, "pkg/b"); got.RelaxedEmittable || !got.has("reads-product:daemon-peer") {
		t.Errorf("%+v", got)
	}
	// The daemon action alone is not its own peer.
	w = setup()
	w.rec(1, "pkg/c").Record.Binds = sl("stream/127.0.0.1:8080")
	w.rec(1, "pkg/c").Record.Writes = sl("srv/own")
	if got := w.merge().out(t, "pkg/c"); got.has("reads-product:daemon-peer") {
		t.Errorf("a daemon action voided itself as a peer: %+v", got)
	}
	// Without a daemon reason nothing happens.
	w = setup()
	w.rec(1, "pkg/c").Record.Reasons = []string{}
	w.rec(0, "pkg/b").Record.Binds = sl("stream/127.0.0.1:8080")
	w.rec(2, "pkg/d").Record.Writes = sl("srv")
	r = w.merge()
	if r.out(t, "pkg/b").has("reads-product:daemon-peer") || r.out(t, "pkg/d").has("reads-product:daemon-peer") {
		t.Error("daemon-peer without a daemon")
	}
}

// The classifier caps reasons at 16 per kind; the kind must survive the cap so
// the daemon rule still sees it, and the merge's own output obeys the same cap.
func TestDaemonKindSurvivesTheReasonCap(t *testing.T) {
	m := map[string]bool{"daemon:zzz": true}
	for i := 0; i < 40; i++ {
		m["aaa:"+strings.Repeat("a", i+1)] = true
		m["daemon-left"] = true
	}
	capped := traced.CapReasons(m)
	if !hasKind(capped, "daemon") {
		t.Fatalf("the daemon reason was capped away: %v", capped)
	}
	w := std(t)
	var rs []string
	for i := 0; i < recfmt.MaxReasonsPerKind; i++ {
		rs = append(rs, "network:"+strings.Repeat("n", i+1))
	}
	rs = append(rs, "daemon:last")
	w.rec(1, "pkg/c").Record.Reasons = sl(rs...)
	w.rec(0, "pkg/b").Record.Binds = sl("stream/1.2.3.4:5")
	if got := w.merge().out(t, "pkg/b"); !got.has("reads-product:daemon-peer") {
		t.Errorf("%+v", got)
	}
}

func TestM14SharedPorts(t *testing.T) {
	w := std(t)
	w.rec(0, "pkg/a").Record.Binds = sl("stream/127.0.0.1:8080")
	w.rec(2, "pkg/d").Record.Binds = sl("stream/127.0.0.1:8080")
	w.rec(0, "pkg/b").Record.Binds = sl("dgram/127.0.0.1:8080") // same port, another socket type
	r := w.merge()
	for _, a := range []string{"pkg/a", "pkg/d"} {
		if got := r.out(t, a); got.RelaxedEmittable || !got.has("reads-product:shared-port") {
			t.Errorf("%s: %+v", a, got)
		}
	}
	if got := r.out(t, "pkg/b"); !got.RelaxedEmittable {
		t.Errorf("another socket type is not a shared port: %+v", got)
	}
	// Two copies of one action binding the same port are not two actions.
	w = std(t)
	w.all("pkg/a", func(r *recfmt.ShardRecord) { r.Record.Binds = sl("stream/127.0.0.1:9") })
	if got := w.merge().out(t, "pkg/a"); !got.RelaxedEmittable {
		t.Errorf("an action shared a port with itself: %+v", got)
	}
}

// ---------------------------------------------------------------------------
// M1 to M4.

func TestM1UnknownWriteSetVoidsTheWholeRun(t *testing.T) {
	cases := map[string]func(r *recfmt.ShardRecord){
		"unknown_writes":     func(r *recfmt.ShardRecord) { r.Unknown = "tracer:signal" },
		"writes_complete=no": func(r *recfmt.ShardRecord) { f := false; r.Complete = &f },
		"trace-incomplete":   func(r *recfmt.ShardRecord) { r.Record.Reasons = sl("trace-incomplete:exec-unreadable") },
		"partly run":         func(r *recfmt.ShardRecord) { r.StepsTotal = 2; r.Status = "failed" },
		// The reviewer case: the lists are empty and only the flag says so.
		"empty lists": func(r *recfmt.ShardRecord) {
			r.Unknown = "steps-missing"
			r.Record = recfmt.Normalize(recfmt.RawRecord{Package: r.Action})
		},
	}
	for name, mut := range cases {
		w := std(t)
		mut2 := mut
		w.rec(2, "pkg/d").StepsRun, w.rec(2, "pkg/d").Steps = 0, nil
		w.rec(2, "pkg/d").StepsTotal = 0
		mut2(w.rec(2, "pkg/d"))
		r := w.merge()
		if r.Emitted || !r.cause("M1") {
			t.Errorf("%s: run not void: %v", name, r.RunVoid)
		}
		if len(r.Manifests) != 0 || len(r.Passes) != 0 || len(r.emittedActions()) != 0 {
			t.Errorf("%s: emitted although the run is void", name)
		}
		// The report still names every action and the recordable ones.
		if len(r.Outcomes) != 4 || !r.out(t, "pkg/b").Recordable {
			t.Errorf("%s: the report is incomplete: %+v", name, r.Outcomes)
		}
	}
}

// A shard in another shard with an unknown write set voids actions that look clean.
func TestM1AUnknownWriterInAnotherShardWouldHideAnAbsentProbe(t *testing.T) {
	w := std(t)
	w.rec(0, "pkg/b").Record.AbsentObservations = sl("made/later") // nobody wrote it that we know of
	w.rec(2, "pkg/d").Unknown = "tracer:status"                    // ...but d's writes are unknown
	r := w.merge()
	if r.Emitted || len(r.emittedActions()) != 0 {
		t.Fatal("b would have been a false hit")
	}
}

func TestWritesCompleteSwitch(t *testing.T) {
	yes := true
	mk := func(honour bool, c *bool) Result {
		w := std(t)
		w.honour = honour
		w.rec(2, "pkg/d").Record.Reasons = sl("trace-incomplete:exec-unreadable")
		w.rec(2, "pkg/d").Complete = c
		return w.merge()
	}
	// Until PR 4 (switch off): every trace-incomplete is unknown, whatever the field says.
	for _, c := range []*bool{nil, &yes} {
		if r := mk(false, c); r.Emitted {
			t.Error("the switch is off, the run must be void")
		}
	}
	// Switch on and the record says its writes are complete: only d is lost.
	r := mk(true, &yes)
	if !r.relaxedOK() {
		t.Fatalf("%v", r.RunVoid)
	}
	wantRelaxed(t, r, "pkg/a", "pkg/b", "pkg/c")
	if !r.out(t, "pkg/d").hasKind("trace-incomplete") {
		t.Error("the reason is missing")
	}
	// Switch on, field absent: unknown, as before.
	if r := mk(true, nil); r.Emitted {
		t.Error("an absent writes_complete must stay unknown")
	}
	// writes_complete=false is unknown whatever the reasons say.
	w := std(t)
	w.honour = true
	no := false
	w.rec(2, "pkg/d").Complete = &no
	if r := w.merge(); r.Emitted {
		t.Error("writes_complete=false must void the run")
	}
	// A record claiming complete writes with no reason is fine.
	w = std(t)
	w.honour = true
	w.rec(2, "pkg/d").Complete = &yes
	wantEmitted(t, w.merge(), "pkg/a", "pkg/b", "pkg/c", "pkg/d")
}

func TestM2UnexplainedObservation(t *testing.T) {
	// Nobody wrote it and no inventory holds it.
	w := std(t)
	w.rec(0, "pkg/b").Record.UntrackedObservations = sl("ghost")
	r := w.merge()
	if r.Emitted || !r.cause("M2") {
		t.Fatalf("%v", r.RunVoid)
	}
	// Explained only by a writer in ANOTHER shard: still unexplained.
	w = std(t)
	w.rec(1, "pkg/c").Record.Writes = sl("ghost")
	w.rec(0, "pkg/b").Record.UntrackedObservations = sl("ghost")
	if r := w.merge(); r.Emitted || !r.cause("M2") {
		t.Fatalf("a writer in another shard explained it: %v", r.RunVoid)
	}
	// The action's own write does not explain what it observed.
	w = std(t)
	w.rec(0, "pkg/b").Record.Writes = sl("ghost")
	w.rec(0, "pkg/b").Record.UntrackedObservations = sl("ghost")
	if r := w.merge(); r.Emitted || !r.cause("M2") {
		t.Fatalf("an action's own write explained its observation: %v", r.RunVoid)
	}
	// Explained by the shard's inventory, or by a writer in the same shard.
	for name, setup := range map[string]func(w *world){
		"inventory":   func(w *world) { w.inv(0, "ghost") },
		"same shard":  func(w *world) { w.rec(0, "pkg/a").Record.Writes = sl("ghost") },
		"root writer": func(w *world) { w.rec(0, "pkg/a").Record.Writes = sl("ghost") },
		"ancestor":    func(w *world) { w.rec(0, "pkg/a").Record.Writes = sl("gh") },
	} {
		w := std(t)
		setup(w)
		w.rec(0, "pkg/b").Record.UntrackedObservations = sl("ghost/x")
		if name == "ancestor" {
			w.rec(0, "pkg/b").Record.UntrackedObservations = sl("gh/x")
		}
		if r := w.merge(); !r.relaxedOK() || r.out(t, "pkg/b").RelaxedEmittable {
			t.Errorf("%s: %v %+v", name, r.RunVoid, r.out(t, "pkg/b"))
		}
	}
}

func TestM3AndM4HeaderVoids(t *testing.T) {
	for name, mut := range map[string]func(h *recfmt.ShardHeader){
		"unexplained": func(h *recfmt.ShardHeader) { h.PostInventoryUnexplained = 3 },
		"survivor":    func(h *recfmt.ShardHeader) { h.Void = sl("survivor") },
		"too large":   func(h *recfmt.ShardHeader) { h.Void = sl("records-too-large") },
		"unknown":     func(h *recfmt.ShardHeader) { h.Void = sl("mystery") },
	} {
		w := std(t)
		mut(&w.shards[1].Header)
		if r := w.merge(); r.Emitted || len(r.emittedActions()) != 0 {
			t.Errorf("%s: emitted %v", name, r.emittedActions())
		}
	}
	for _, tok := range []string{"tracked-changed", "state-preloaded"} {
		w := std(t)
		w.shards[0].Header.Void = sl(tok)
		r := w.merge()
		if !r.relaxedOK() {
			t.Fatalf("%s: %v", tok, r.RunVoid)
		}
		// Shard 0 is void: nothing ASSIGNED to it is emitted.  pkg/a's copy in
		// shard 1 is not an emission; pkg/c and pkg/d are unaffected.
		wantRelaxed(t, r, "pkg/c", "pkg/d")
		if !r.out(t, "pkg/a").ShardVoid || r.out(t, "pkg/c").ShardVoid {
			t.Error("ShardVoid is wrong")
		}
		if got := r.ShardVoid[0]; len(got) != 1 || got[0] != tok || len(r.ShardVoid) != 1 {
			t.Errorf("%v", r.ShardVoid)
		}
		// A shard-void shard's records still take part in the rules.
		w.rec(0, "pkg/b").Record.Writes = sl("out")
		w.rec(1, "pkg/c").Record.AbsentObservations = sl("out")
		if r := w.merge(); r.out(t, "pkg/c").RelaxedEmittable {
			t.Error("a void shard's writes were ignored")
		}
	}
}

// ---------------------------------------------------------------------------
// The exec-then-rewrite cases of section 7.1, each closed by existing rules.

// A execs its own product u, and B later rewrites or removes u.  B's write
// coarsens to the same write root or one nested in it, so M10 voids both.
func TestExecThenRewriteOwnProductByAnotherAction(t *testing.T) {
	w := std(t)
	w.rec(0, "pkg/a").Record.Writes = sl("build/bin")
	w.rec(1, "pkg/c").Record.Writes = sl("build/bin") // rewrites it
	r := w.merge()
	for _, a := range []string{"pkg/a", "pkg/c"} {
		if got := r.out(t, a); got.RelaxedEmittable || !got.has("reads-product:build/bin") {
			t.Errorf("%s: %+v", a, got)
		}
	}
	w = std(t)
	w.rec(0, "pkg/a").Record.Writes = sl("build")
	w.rec(1, "pkg/c").Record.Writes = sl("build") // removes it (a root holds no tracked file)
	if r := w.merge(); r.out(t, "pkg/a").RelaxedEmittable || r.out(t, "pkg/c").RelaxedEmittable {
		t.Error("nested rewrite not voided")
	}
}

// A execs a tracked file B rewrote: the file is in the tainted set and A's
// pkg:/path: entry covers it (M11).
func TestExecThenRewriteTrackedFile(t *testing.T) {
	w := std(t)
	w.entries(0, "pkg/a", "path:tools/run.sh")
	w.rec(0, "pkg/b").Record.TrackedWrites = sl("tools/run.sh")
	if got := w.merge().out(t, "pkg/a"); got.RelaxedEmittable || !got.has("reads-product:tools/run.sh") {
		t.Errorf("%+v", got)
	}
	// ...also when B restored it (the record cannot tell, and need not).
	w = std(t)
	w.entries(0, "pkg/a", "pkg:tools")
	w.rec(2, "pkg/d").Record.TrackedWrites = sl("tools/run.sh")
	if got := w.merge().out(t, "pkg/a"); got.RelaxedEmittable {
		t.Errorf("%+v", got)
	}
}

// A execs an untracked file A did not make: it is an UntrackedObservation, so
// M5, M9 or M2 applies.
func TestExecOfAnUntrackedFileAdidNotMake(t *testing.T) {
	for name, setup := range map[string]func(w *world){
		"another action wrote it (M5)": func(w *world) { w.rec(0, "pkg/b").Record.Writes = sl("bin/tool") },
		"pre-existing (M9)":            func(w *world) { w.inv(0, "bin") },
		"nobody made it (M2)":          func(w *world) {},
	} {
		w := std(t)
		setup(w)
		w.rec(0, "pkg/a").Record.UntrackedObservations = sl("bin/tool")
		r := w.merge()
		if name == "nobody made it (M2)" {
			if r.Emitted {
				t.Errorf("%s: run not void", name)
			}
			continue
		}
		if r.out(t, "pkg/a").RelaxedEmittable {
			t.Errorf("%s: kept", name)
		}
	}
}

// B's write set is unknown: M1.
func TestExecThenRewriteUnknownWriterVoidsTheRun(t *testing.T) {
	w := std(t)
	w.rec(0, "pkg/a").Record.Writes = sl("build/bin")
	w.rec(1, "pkg/c").Unknown = "tracer:signal"
	if r := w.merge(); r.Emitted {
		t.Error("an unknown writer must void the run")
	}
}

// Writes outside the repository cannot be seen by another action: $HOME,
// TMPDIR, /tmp and /run are per-step tmpfs and the ro_roots are read-only, so
// the merge has nothing to check.  Absolute paths are not valid record paths.
func TestOutOfRepositoryPathsAreNotRecordPaths(t *testing.T) {
	for _, p := range []string{"/tmp/x", "/run/x", "/home/runner/.cache"} {
		rec := recfmt.RawRecord{Package: "a", Writes: []string{p}}
		sr := recfmt.ShardRecord{BindingID: dGraph, Action: "a", Status: "built", StepsSHA: dSteps, Weak: dSteps, Record: recfmt.Normalize(rec)}
		if sr.Validate() == nil {
			t.Errorf("%s accepted", p)
		}
	}
}
