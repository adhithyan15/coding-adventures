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

func targets(plan []Mount, kind MountKind) []string {
	var out []string
	for _, m := range plan {
		if m.Kind == kind {
			out = append(out, m.Target)
		}
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
	scratch := func(p, mode string) []Mount {
		return []Mount{
			{Kind: KindEnsureDir, Target: p},
			{Kind: KindTmpfs, Target: p, FSType: "tmpfs", Flags: MsNosuid | MsNodev, Data: "mode=" + mode + ",uid=1000,gid=1000"},
		}
	}
	want := []Mount{
		{Kind: KindMakePrivate, Target: "/", Flags: MsRec | MsPrivate},
		{Kind: KindHostMkdir, Target: "/home/runner/.cargo/registry"},
		{Kind: KindStageTmpfs, Target: "/home/runner/_tmp", FSType: "tmpfs", Flags: MsNosuid | MsNodev, Data: "mode=0755,uid=1000,gid=1000"},
		{Kind: KindStageDir, Target: stage},
		// The new root is an EMPTY tmpfs: an allowlist, not a copy of "/".
		{Kind: KindRootTmpfs, Target: stage, FSType: "tmpfs", Flags: MsNosuid | MsNodev, Data: "mode=0755,uid=1000,gid=1000"},
	}
	// The read-only roots, shallowest first, then by name.
	for _, r := range []string{"/bin", "/etc", "/lib", "/lib64", "/usr", "/opt/hostedtoolcache/go/1.26.1"} {
		want = append(want, Mount{Kind: KindRORoot, Source: r, Target: r, Flags: MsBind})
	}
	// Scratch tmpfs, shallowest first: /run and /tmp (depth 1), then $HOME and
	// /var/tmp (depth 2, by name), then TMPDIR under $HOME.
	for _, s := range []struct{ p, mode string }{
		{"/run", "0755"}, {"/tmp", "1777"}, {"/home/runner", "0755"}, {"/var/tmp", "1777"}, {"/home/runner/_tmp", "1777"},
	} {
		want = append(want, scratch(s.p, s.mode)...)
	}
	want = append(want,
		Mount{Kind: KindEnsureDir, Target: "/dev"},
		Mount{Kind: KindDev, Target: "/dev", FSType: "tmpfs", Flags: MsNosuid | MsNoexec, Data: "mode=0755,size=64k,nr_inodes=64,uid=1000,gid=1000"},
		Mount{Kind: KindEnsureDir, Target: c.RepoRoot},
		Mount{Kind: KindBind, Source: c.RepoRoot, Target: c.RepoRoot, Flags: MsBind},
		Mount{Kind: KindEnsureDir, Target: "/home/runner/.cargo/registry"},
		Mount{Kind: KindBind, Source: "/home/runner/.cargo/registry", Target: "/home/runner/.cargo/registry", Flags: MsBind},
		Mount{Kind: KindMask, Target: "/home/runner/work/repo/.git", FSType: "tmpfs", Flags: MsRdonly | MsNosuid | MsNodev | MsNoexec, Data: "mode=0555,size=4k,nr_inodes=2"},
		Mount{Kind: KindEnsureDir, Target: "/proc"},
		Mount{Kind: KindProc, Source: "proc", Target: "/proc", FSType: "proc", Flags: MsNosuid | MsNodev | MsNoexec, Data: "hidepid=2"},
		Mount{Kind: KindSeal, Target: stage, Flags: MsRemount | MsRdonly | MsNosuid | MsNodev},
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

func TestPlanIsAnAllowlist(t *testing.T) {
	// The new root is never a bind of "/": every host path that appears is
	// either named in the configuration or a fixed device node.
	c := goodConfig()
	plan, _ := Plan(c)
	allowed := map[string]bool{c.RepoRoot: true}
	for _, r := range c.RoRoots {
		allowed[r] = true
	}
	for _, s := range c.StateRoots {
		allowed[s] = true
	}
	for _, m := range plan {
		if m.Source == "/" || (m.Kind == KindMakePrivate && m.Target != "/") {
			t.Errorf("a step binds the whole filesystem: %+v", m)
		}
		if (m.Kind == KindRORoot || m.Kind == KindBind) && !allowed[m.Source] {
			t.Errorf("%s of %q, which the configuration does not name", m.Kind, m.Source)
		}
		if m.Flags&MsRec != 0 && m.Kind != KindMakePrivate {
			t.Errorf("a recursive bind: %+v", m)
		}
	}
	for _, k := range kinds(plan) {
		if k == "root-bind" || k == "read-only" {
			t.Errorf("the old rbind-of-/ kinds are back: %s", k)
		}
	}
	// Every read-only root is read-only by the kernel, not by convention: the
	// executor sets MOUNT_ATTR_RDONLY for each KindRORoot, and the root tmpfs is
	// sealed read-only before the pivot.
	if got := targets(plan, KindSeal); len(got) != 1 || got[0] != StageRoot(c) {
		t.Errorf("seal: %v", got)
	}
}

func TestPlanDependencies(t *testing.T) {
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
	before(KindHostMkdir, KindStageTmpfs)   // the host paths are still plain
	before(KindStageDir, KindRootTmpfs)     // the new root needs its directory
	before(KindRootTmpfs, KindRORoot)       // the roots go onto the new root
	before(KindRORoot, KindTmpfs)           // the scratch tmpfs hide nothing they should not
	before(KindTmpfs, KindBind)             // a checkout under $HOME survives the tmpfs
	before(KindDev, KindBind)               // /dev is built before the checkout
	before(KindBind, KindMask)              // masks go over the checkout
	before(KindMask, KindProc)              // proc last of the mounts ...
	before(KindProc, KindSeal)              // ... then the seal ...
	before(KindSeal, KindPivot)             // ... the pivot ...
	before(KindPivot, KindVerifyAbsent)     // ... and the check of what the step sees
	if first[KindVerifyAbsent] != len(plan)-1 {
		t.Error("the verification is not last")
	}
}

func TestPlanScratchTmpfs(t *testing.T) {
	// Every scratch place is a fresh tmpfs, once, parents before children.
	for name, edit := range map[string]func(*Config){
		"default":               func(c *Config) {},
		"tmpdir is /tmp":        func(c *Config) { c.Tmpdir, c.Masks = "/tmp", []string{c.RepoRoot + "/.git"} },
		"home under tmpdir":     func(c *Config) { c.Tmpdir, c.Home = "/scratch", "/scratch/home"; c.StateRoots = nil },
		"home is /root":         func(c *Config) { c.Home = "/root"; c.Tmpdir = "/var/tmp/t" },
		"tmpdir under /var/tmp": func(c *Config) { c.Tmpdir = "/var/tmp/k" },
	} {
		t.Run(name, func(t *testing.T) {
			c := goodConfig()
			c.RepoRoot, c.StateRoots = "/work/repo", []string{"/state"}
			c.Masks = []string{"/work/repo/.git"}
			edit(c)
			if err := c.Validate(); err != nil {
				t.Fatal(err)
			}
			plan, err := Plan(c)
			if err != nil {
				t.Fatal(err)
			}
			got := targets(plan, KindTmpfs)
			seen := map[string]bool{}
			for i, p := range got {
				if seen[p] {
					t.Errorf("%s mounted twice: %v", p, got)
				}
				seen[p] = true
				for _, q := range got[i+1:] {
					if within(p, q) && p != q {
						t.Errorf("%s is mounted before its parent %s: %v", p, q, got)
					}
				}
			}
			for _, need := range []string{c.Home, c.Tmpdir, "/tmp", "/var/tmp", "/run"} {
				if !seen[need] {
					t.Errorf("%s is not a fresh tmpfs: %v", need, got)
				}
			}
		})
	}
}

func TestPlanRefusesInvalidConfig(t *testing.T) {
	c := goodConfig()
	c.Home = "relative"
	if _, err := Plan(c); err == nil {
		t.Fatal("planned an invalid configuration")
	}
}

func TestPlanMaskOrder(t *testing.T) {
	c := goodConfig()
	c.Masks = []string{c.RepoRoot + "/z", c.RepoRoot + "/a", c.RepoRoot + "/.git"}
	plan, err := Plan(c)
	if err != nil {
		t.Fatal(err)
	}
	got := targets(plan, KindMask)
	want := []string{c.RepoRoot + "/.git", c.RepoRoot + "/a", c.RepoRoot + "/z"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("mask order %v", got)
	}
}

func TestPlanNeverBindsFromTheStagingArea(t *testing.T) {
	// Every source is a path of the OLD root, and none lies beneath the
	// staging directory, where the host's TMPDIR was shadowed.
	c := goodConfig()
	plan, _ := Plan(c)
	for _, m := range plan {
		if (m.Kind == KindBind || m.Kind == KindRORoot) && strings.HasPrefix(m.Source, c.Tmpdir) {
			t.Errorf("source %q is inside the staging area", m.Source)
		}
	}
}

func TestHostIPCPathsAreAllUnderScratchOrReplacedDirs(t *testing.T) {
	// What verifyAbsent checks must be places the sandbox gives no host content
	// to: /run and /var/run.
	for _, p := range HostIPCPaths {
		if !within(p, "/run") && !within(p, "/var/run") {
			t.Errorf("%s is not under /run or /var/run", p)
		}
	}
	if !reflect.DeepEqual(ScratchDirs, []string{"/tmp", "/var/tmp", "/run"}) {
		t.Errorf("scratch dirs %v", ScratchDirs)
	}
}

func TestPrivSteps(t *testing.T) {
	got := PrivSteps()
	want := []PrivStep{{PrivNoNewPrivs, 1}, {PrivBoundingDrop, 64}, {PrivAmbientClear, 0}, {PrivCapsetEmpty, 0}}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("%v", got)
	}
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
