package sandbox

import (
	"reflect"
	"strings"
	"testing"
)

func kinds(plan []Mount) []MountKind {
	out := make([]MountKind, len(plan))
	for i, m := range plan {
		out[i] = m.Kind
	}
	return out
}

func TestPlanOrder(t *testing.T) {
	c := goodConfig()
	plan, err := Plan(c)
	if err != nil {
		t.Fatal(err)
	}
	stage := "/home/runner/_tmp/.kiln-newroot"
	if StageRoot(c) != stage {
		t.Fatalf("stage %q", StageRoot(c))
	}
	want := []Mount{
		{Kind: KindMakePrivate, Target: "/", Flags: MsRec | MsPrivate},
		{Kind: KindHostMkdir, Target: "/home/runner/.cargo/registry"},
		{Kind: KindStageTmpfs, Target: "/home/runner/_tmp", FSType: "tmpfs", Flags: MsNosuid | MsNodev, Data: "mode=0755,uid=1000,gid=1000"},
		{Kind: KindStageDir, Target: stage},
		{Kind: KindRootBind, Source: "/", Target: stage, Flags: MsBind | MsRec},
		{Kind: KindReadOnly, Target: stage},
		{Kind: KindDev, Target: "/dev", FSType: "tmpfs", Flags: MsNosuid | MsNoexec, Data: "mode=0755,size=64k,nr_inodes=64,uid=1000,gid=1000"},
		{Kind: KindMask, Target: "/run", FSType: "tmpfs", Flags: MsRdonly | MsNosuid | MsNodev | MsNoexec, Data: "mode=0555,size=4k,nr_inodes=2", Optional: true},
		{Kind: KindMask, Target: "/var/run", FSType: "tmpfs", Flags: MsRdonly | MsNosuid | MsNodev | MsNoexec, Data: "mode=0555,size=4k,nr_inodes=2", Optional: true},
		// $HOME, then TMPDIR (it lies under HOME, so HOME is the parent).
		{Kind: KindEnsureDir, Target: "/home/runner"},
		{Kind: KindTmpfs, Target: "/home/runner", FSType: "tmpfs", Flags: MsNosuid | MsNodev, Data: "mode=0755,uid=1000,gid=1000"},
		{Kind: KindEnsureDir, Target: "/home/runner/_tmp"},
		{Kind: KindTmpfs, Target: "/home/runner/_tmp", FSType: "tmpfs", Flags: MsNosuid | MsNodev, Data: "mode=1777,uid=1000,gid=1000"},
		// The repository, writable, at its own path; then each state root.
		{Kind: KindEnsureDir, Target: c.RepoRoot},
		{Kind: KindBind, Source: c.RepoRoot, Target: c.RepoRoot, Flags: MsBind},
		{Kind: KindEnsureDir, Target: "/home/runner/.cargo/registry"},
		{Kind: KindBind, Source: "/home/runner/.cargo/registry", Target: "/home/runner/.cargo/registry", Flags: MsBind},
	}
	// Masks, shallowest first.
	maskFlags := uintptr(MsRdonly | MsNosuid | MsNodev | MsNoexec)
	for _, m := range []struct {
		p      string
		inRepo bool
	}{
		{"/home/runner/kiln-state", false},
		{"/home/runner/work/_temp", false},
		{"/home/runner/work/repo/.git", true},
	} {
		want = append(want, Mount{Kind: KindMask, Target: m.p, FSType: "tmpfs", Flags: maskFlags, Data: "mode=0555,size=4k,nr_inodes=2", InRepo: m.inRepo})
	}
	want = append(want,
		Mount{Kind: KindProc, Source: "proc", Target: "/proc", FSType: "proc", Flags: MsNosuid | MsNodev | MsNoexec, Data: "hidepid=2"},
		Mount{Kind: KindPivot, Target: stage},
		Mount{Kind: KindVerifyAbsent},
	)
	if !reflect.DeepEqual(plan, want) {
		for i := 0; i < len(plan) || i < len(want); i++ {
			var g, w any
			if i < len(plan) {
				g = plan[i]
			}
			if i < len(want) {
				w = want[i]
			}
			if !reflect.DeepEqual(g, w) {
				t.Errorf("step %d:\n got  %+v\n want %+v", i, g, w)
			}
		}
	}
}

func TestPlanDependencies(t *testing.T) {
	// The orderings the design rests on, checked by position.
	plan, err := Plan(goodConfig())
	if err != nil {
		t.Fatal(err)
	}
	first := map[MountKind]int{}
	last := map[MountKind]int{}
	for i, m := range plan {
		if _, ok := first[m.Kind]; !ok {
			first[m.Kind] = i
		}
		last[m.Kind] = i
	}
	before := func(a, b MountKind) {
		t.Helper()
		if last[a] >= first[b] {
			t.Errorf("%s must all come before %s", a, b)
		}
	}
	before(KindMakePrivate, KindStageTmpfs) // nothing propagates to the host
	before(KindHostMkdir, KindStageTmpfs)   // the host is still writable
	before(KindStageDir, KindRootBind)      // the new root needs its directory
	before(KindRootBind, KindReadOnly)      // read-only is applied to the copy ...
	before(KindReadOnly, KindTmpfs)         // ... and the writable parts come after it
	before(KindTmpfs, KindBind)             // a checkout under $HOME survives the tmpfs
	// Configured masks go over the binds (the /run masks are not configured).
	firstMask := -1
	for i, m := range plan {
		if m.Kind == KindMask && !m.Optional {
			firstMask = i
			break
		}
	}
	if firstMask < last[KindBind] {
		t.Error("masks must all come after the binds")
	}
	before(KindReadOnly, KindDev) // /dev is replaced after the copy is read-only
	before(KindMask, KindProc)    // proc last of the mounts ...
	before(KindProc, KindPivot)   // ... then the pivot
	if first[KindPivot] != len(plan)-2 || first[KindVerifyAbsent] != len(plan)-1 {
		t.Error("the pivot, then the verification, are not last")
	}
}

func TestPlanScratchOrderWhenHomeIsUnderTmpdir(t *testing.T) {
	c := goodConfig()
	c.Tmpdir, c.Home = "/scratch", "/scratch/home"
	c.StateRoots, c.Masks = nil, nil
	plan, err := Plan(c)
	if err != nil {
		t.Fatal(err)
	}
	var tmpfs []string
	for _, m := range plan {
		if m.Kind == KindTmpfs {
			tmpfs = append(tmpfs, m.Target)
		}
	}
	if !reflect.DeepEqual(tmpfs, []string{"/scratch", "/scratch/home"}) {
		t.Errorf("tmpfs order %v: the parent must be mounted first", tmpfs)
	}
	// Disjoint home and tmpdir: home first.
	c.Tmpdir, c.Home = "/tmpdir", "/homedir"
	plan, _ = Plan(c)
	tmpfs = nil
	for _, m := range plan {
		if m.Kind == KindTmpfs {
			tmpfs = append(tmpfs, m.Target)
		}
	}
	if !reflect.DeepEqual(tmpfs, []string{"/homedir", "/tmpdir"}) {
		t.Errorf("tmpfs order %v", tmpfs)
	}
}

func TestPlanRefusesInvalidConfig(t *testing.T) {
	c := goodConfig()
	c.Home = "relative"
	if _, err := Plan(c); err == nil {
		t.Fatal("planned an invalid configuration")
	}
}

func TestPlanMaskDepthOrder(t *testing.T) {
	c := goodConfig()
	c.Masks = []string{"/z/b/c", "/a/b", "/a", "/m", "/a/c"}
	plan, err := Plan(c)
	if err != nil {
		t.Fatal(err)
	}
	var got []string
	for _, m := range plan {
		if m.Kind == KindMask && !m.Optional {
			got = append(got, m.Target)
		}
	}
	if !reflect.DeepEqual(got, []string{"/a", "/m", "/a/b", "/a/c", "/z/b/c"}) {
		t.Errorf("mask order %v", got)
	}
}

func TestPlanNeverMountsFromTheNewRoot(t *testing.T) {
	// Every bind's source is a path of the OLD root, and none lies beneath the
	// staging directory, which is where the host's TMPDIR was shadowed.
	plan, _ := Plan(goodConfig())
	for _, m := range plan {
		if m.Kind == KindBind && strings.HasPrefix(m.Source, StageRoot(goodConfig())) {
			t.Errorf("bind source %q is inside the staging area", m.Source)
		}
	}
}

func TestPrivSteps(t *testing.T) {
	got := PrivSteps()
	want := []PrivStep{{PrivNoNewPrivs, 1}, {PrivBoundingDrop, 64}, {PrivAmbientClear, 0}, {PrivCapsetEmpty, 0}}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("%v", got)
	}
	// capabilities are needed until the last step removes them.
	idx := map[PrivOp]int{}
	for i, s := range got {
		idx[s.Op] = i
	}
	if idx[PrivBoundingDrop] >= idx[PrivCapsetEmpty] {
		t.Error("the bounding set is dropped with CAP_SETPCAP, so before capset removes it")
	}
	if idx[PrivNoNewPrivs] != 0 {
		t.Error("no_new_privs first")
	}
	if !reflect.DeepEqual(StartCaps(), []uintptr{8, 12, 21}) {
		t.Errorf("start caps %v", StartCaps())
	}
}
