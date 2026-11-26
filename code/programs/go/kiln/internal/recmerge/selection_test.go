package recmerge

// The language selection in the merge (KILN04 7.3)
// ================================================
//
// A recording shard may plan only one language's prerequisite-closed packages
// (`kiln record prepare --language go`). The merge then has two new duties:
//
//  1. every shard must have sealed the SAME selection, which is the one the
//     merge acts on (B3), and the plan must be of that language;
//  2. an action may read, with no declared edge, where an UNSELECTED package's
//     build outputs would be. In the selected run those outputs are absent, and
//     the merge checks absences only against writes of actions in the run, so
//     nothing contradicts the probe. Such an action loses its record.

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// selectLang seals lang in every shard's binding (a new binding id, named by the
// header and every record line, exactly as prepare would have written it) and
// tells the merge what was left out.
func (w *world) selectLang(lang string, foreign ...string) {
	w.t.Helper()
	for _, sh := range w.shards {
		sh.Binding.Language = lang
		bb, err := recfmt.EncodeBinding(sh.Binding)
		if err != nil {
			w.t.Fatal(err)
		}
		sh.BindingID = sha(bb)
		sh.Header.BindingID = sh.BindingID
		for i := range sh.Records {
			sh.Records[i].BindingID = sh.BindingID
		}
	}
	w.lang, w.foreign = lang, foreign
}

func TestSelectionRunsAndRecordsTheSelectedPackages(t *testing.T) {
	w := std(t)
	w.selectLang("go", "other/x", "other/y")
	r := w.merge()
	if !r.Emitted || len(r.emittedActions()) != 4 {
		t.Fatalf("void %v, emitted %v", r.RunVoid, r.emittedActions())
	}
}

// The shards must agree with each other, and with what the merge acts on, which
// is read from the bindings. Shards sealed `go` and `all` cannot both be right.
func TestShardsThatDisagreeOnTheSelectionVoidTheRun(t *testing.T) {
	for _, odd := range []int{0, 1, 2} {
		w := std(t)
		w.selectLang("go", "other/x")
		// One shard alone says all languages (its binding id changes with it).
		sh := w.shards[odd]
		sh.Binding.Language = ""
		bb, _ := recfmt.EncodeBinding(sh.Binding)
		sh.BindingID, sh.Header.BindingID = sha(bb), sha(bb)
		for i := range sh.Records {
			sh.Records[i].BindingID = sh.BindingID
		}
		wantVoid(t, "shard "+string(rune('0'+odd))+" disagrees", w.merge(), "language selection")
	}
	// And the other way round: the merge is told `all` (a flag, a stale default)
	// while the shards sealed go. The bindings are what the shards recorded, so
	// it is the merge's expectation that is wrong, and the run is void.
	w := std(t)
	w.selectLang("go")
	w.lang = ""
	wantVoid(t, "merge expects all", w.merge(), "language selection")
}

// A plan holding a package of another language was not built with the selection
// the bindings seal.
func TestAPlanOutsideTheSelectionVoidsTheRun(t *testing.T) {
	w := std(t)
	w.selectLang("go")
	in := w.input()
	in.Expected.PlanOutsideSelection = 1
	wantVoid(t, "plan outside selection", Merge(in), "not of the sealed language selection")
}

// The core fail-closed rule: an ABSENT path inside an unselected package costs
// the action its record, and only that action. Everything else is still
// recorded, and the run is not void (the reason is not an order dependence).
func TestAbsentProbeOfAForeignPackageCostsTheActionItsRecord(t *testing.T) {
	w := std(t)
	w.selectLang("go", "other/x")
	w.rec(1, "pkg/c").Record.AbsentObservations = sl("other/x/target/release/libx.a")
	r := w.merge()
	if !r.Emitted {
		t.Fatalf("a foreign probe voided the whole run: %v", r.RunVoid)
	}
	o := r.out(t, "pkg/c")
	if o.Emitted || o.Recordable || !o.has(ForeignRead) {
		t.Fatalf("pkg/c: %+v", o)
	}
	if o.OrderDependent {
		t.Fatal("the foreign reason is the action's own, not an order dependence")
	}
	for _, a := range []string{"pkg/a", "pkg/b", "pkg/d"} {
		if !r.out(t, a).Emitted {
			t.Errorf("%s lost its record for another action's probe", a)
		}
	}
	// Manifests and passes are index-aligned with the emitted actions: three
	// actions, so three of each, none of them pkg/c's.
	if len(r.Manifests) != 3 || len(r.Passes) != 3 {
		t.Fatalf("%d manifests, %d passes; want 3 of each", len(r.Manifests), len(r.Passes))
	}
}

// With every language recorded, the same observation is an ordinary absent
// probe: nothing changes.
func TestTheSameProbeIsOrdinaryWhenEveryLanguageIsRecorded(t *testing.T) {
	w := std(t)
	w.rec(1, "pkg/c").Record.AbsentObservations = sl("other/x/target/release/libx.a")
	r := w.merge()
	if !r.Emitted || !r.out(t, "pkg/c").Emitted {
		t.Fatalf("%v %+v", r.RunVoid, r.out(t, "pkg/c"))
	}
}

// An UNTRACKED path inside an unselected package: the action is dropped. (Such
// an observation must also be explained by the inventory or another action's
// write, M2, and being explained makes the action order dependent, M9; so the
// run is void here by the existing rules. What this test pins is that the
// foreign reason is on the action regardless.)
func TestUntrackedObservationOfAForeignPackageIsDropped(t *testing.T) {
	w := std(t)
	w.selectLang("go", "other/x")
	w.inv(1, "other/x/node_modules")
	w.rec(1, "pkg/c").Record.UntrackedObservations = sl("other/x/node_modules/left-pad")
	r := w.merge()
	o := r.out(t, "pkg/c")
	if o.Emitted || !o.has(ForeignRead) {
		t.Fatalf("pkg/c: %+v", o)
	}
}

// Reading a foreign package's TRACKED sources is an ordinary manifest entry:
// no absent or untracked observation, nothing to drop.
func TestTrackedReadOfAForeignPackageIsStillRecorded(t *testing.T) {
	w := std(t)
	w.selectLang("go", "other/x")
	w.entries(1, "pkg/c", "path:other/x/src/lib.rs", "list:other/x/src")
	r := w.merge()
	o := r.out(t, "pkg/c")
	if !r.Emitted || !o.Emitted || o.has(ForeignRead) {
		t.Fatalf("void %v, pkg/c: %+v", r.RunVoid, o)
	}
}

// Packages nest. A path is judged by its NEAREST enclosing package: inside a
// selected package that lives under a foreign package's directory, it is the
// selected package's; a path that merely shares a spelling prefix with a
// foreign directory is outside it.
func TestForeignRuleUsesTheNearestEnclosingPackage(t *testing.T) {
	cases := []struct {
		path string
		drop bool
	}{
		{"pkg/zz/out", true},     // in the foreign package "pkg"
		{"pkg/a/out", false},     // in the selected package pkg/a, nested under "pkg"
		{"pkg/a", false},         // the selected package itself
		{"pkg", true},            // the foreign package's own directory
		{"pkgx/out", false},      // not inside "pkg": a spelling prefix only
		{"other/x/y/z", true},    // under the second foreign package
		{"other", false},         // an ancestor of a foreign package is not inside it
		{"unrelated/out", false}, // belongs to no package at all
	}
	for _, c := range cases {
		w := std(t)
		w.selectLang("go", "pkg", "other/x")
		w.rec(1, "pkg/c").Record.AbsentObservations = sl(c.path)
		o := w.merge().out(t, "pkg/c")
		if o.has(ForeignRead) != c.drop {
			t.Errorf("%s: dropped=%v, want %v", c.path, o.has(ForeignRead), c.drop)
		}
	}
}

// MUTANT KILLER. A merge that took the foreign set, or the set of actions, from
// what a shard claims instead of from the regenerated plan would accept a shard
// that records an unselected package as though it had run it. The plan is the
// authority: that record is a package the shard does not run, and the run is
// void.
func TestAShardClaimingAnUnselectedPackageIsVoid(t *testing.T) {
	w := std(t)
	w.selectLang("go", "other/x")
	r := *w.rec(2, "pkg/d")
	r.Action, r.Record.Package, r.Record.Entries = "other/x", "other/x", []string{"pkg:other/x"}
	w.shards[2].Records = append(w.shards[2].Records, r)
	wantVoid(t, "extra package", w.merge(), "does not run")

	// The same claim cannot widen the foreign rule either: a package the plan
	// runs is never foreign, whatever a shard's record says it is.
	w = std(t)
	w.selectLang("go", "other/x")
	w.rec(1, "pkg/c").Record.AbsentObservations = sl("pkg/a/out")
	if o := w.merge().out(t, "pkg/c"); o.has(ForeignRead) {
		t.Fatal("a selected package's path was treated as foreign")
	}
}

// The drop counts as reason text without any path.
func TestForeignReasonIsAFixedToken(t *testing.T) {
	if strings.ContainsAny(ForeignRead, "/ :") {
		t.Fatalf("%q carries path-like text", ForeignRead)
	}
}
