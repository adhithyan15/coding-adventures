package record

import (
	"errors"
	"io/fs"
	"os"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// KILN04 D43, the sealed root.  The classifier treats a path as absent without
// asking the disk when the sandbox's new root cannot contain it.  That is sound
// only if everything internal/sandbox/plan.go MOUNTS is inside the classifier's
// exists-set, and the classifier's scratch directories are the plan's.  Both
// sides are built here from the same configuration the recorder uses, so a
// change to the plan (a new mount, a new scratch tmpfs) that the classifier
// does not know about fails this test instead of silently turning a present
// directory into an "absent" constant.

// stubTree and stubFS are the smallest traced.Tree and traced.FS that let
// traced prepare a Config: every directory exists, nothing is tracked.
type stubTree struct{}

func (stubTree) Stat(string) traced.Node          { return traced.Node{} }
func (stubTree) Head(string, int) ([]byte, error) { return nil, errors.New("none") }

type stubFS struct{}

func (stubFS) Lstat(string) (traced.FileInfo, error) {
	return traced.FileInfo{Kind: traced.KindDir}, nil
}
func (stubFS) Readlink(string) (string, error)      { return "", fs.ErrInvalid }
func (stubFS) ReadHead(string, int) ([]byte, error) { return nil, fs.ErrNotExist }

// tracedConfigFor is the traced.Config classifyInputs builds for a sandbox
// configuration: same roots, same derived binds, same scratch list.
func tracedConfigFor(in ConfigInput, cfg *sandbox.Config) traced.Config {
	return traced.Config{
		Root: cfg.RepoRoot, Packages: traced.NewPackageIndex(nil), Tree: stubTree{}, FS: stubFS{},
		HostRoots: in.Roots, HostBinds: DerivedStateRoots(in.Home, in.Roots), ScratchDirs: SandboxScratchDirs(),
		Home: cfg.Home, Tmpdir: cfg.Tmpdir,
	}
}

func TestSealedRootExistsSetCoversEverythingThePlanMounts(t *testing.T) {
	seen := 0
	for _, v := range []struct{ root, tmp string }{
		{"/home/runner/work/repo/repo", "/tmp/kiln-tmp"},
		{"/srv/w/repo", "/var/tmp/kt"},
		{"/tmp/w/repo", "/tmp/kiln-tmp"},
		{"/home/runner/work/repo/repo", "/home/runner/.kt"},
	} {
		in := baseInput(runnerFS())
		in.Root, in.Tmpdir = v.root, v.tmp
		cfg, err := BuildSandboxConfig(in)
		if err != nil {
			t.Fatalf("%v: %v", v, err)
		}
		plan, err := sandbox.Plan(cfg)
		if err != nil {
			t.Fatalf("%v: %v", v, err)
		}
		tcfg := tracedConfigFor(in, cfg)
		has := func(p string) bool {
			ok, err := traced.SandboxHas(tcfg, p)
			if err != nil {
				t.Fatalf("%v: %v", v, err)
			}
			return ok
		}
		for _, m := range plan {
			switch m.Kind {
			case sandbox.KindRORoot, sandbox.KindTmpfs, sandbox.KindBind, sandbox.KindEnsureDir, sandbox.KindMask, sandbox.KindDev, sandbox.KindProc, sandbox.KindSealDir:
			default:
				continue // host-side steps (staging, make-private, seal, pivot) name no path of the new root
			}
			seen++
			// The mount point and every directory the plan creates on the way to it.
			for p := m.Target; p != "" && p != "/"; p = p[:strings.LastIndexByte(p, '/')] {
				if !has(p) {
					t.Errorf("%v: the plan mounts or creates %s (%s), but the classifier's exists-set lacks it: "+
						"it would be taken for a path the sealed root cannot contain", v, p, m.Kind)
				}
			}
		}
		// Every scratch tmpfs the plan mounts is one of the classifier's, or Home,
		// Tmpdir, or an anchor beneath Home.
		for _, m := range plan {
			if m.Kind != sandbox.KindTmpfs {
				continue
			}
			ok := m.Target == cfg.Home || m.Target == cfg.Tmpdir || strings.HasPrefix(m.Target, cfg.Home+"/")
			for _, d := range tcfg.ScratchDirs {
				ok = ok || m.Target == d
			}
			if !ok {
				t.Errorf("%v: the plan mounts a tmpfs at %s that is neither Home, Tmpdir, an anchor beneath Home, nor a ScratchDirs entry", v, m.Target)
			}
		}
		// Each classifier scratch directory is really mounted.
		for _, d := range tcfg.ScratchDirs {
			mounted := false
			for _, m := range plan {
				mounted = mounted || (m.Kind == sandbox.KindTmpfs && m.Target == d)
			}
			if !mounted {
				t.Errorf("%v: ScratchDirs lists %s but the plan mounts no tmpfs there", v, d)
			}
		}
		// What must be absent, is: the host IPC paths (they would be refused by
		// the helper if present) and the places the census named.
		for _, p := range []string{"/var/run", "/var/run/docker.sock", "/package.json", "/go.work", "/home/package.json", "/opt/rh", "/include", "/pyvenv.cfg"} {
			if strings.HasPrefix(p, v.root) || strings.HasPrefix(p, "/home/runner/") {
				continue
			}
			if has(p) {
				t.Errorf("%v: %s is in the exists-set but no plan step mounts it", v, p)
			}
		}
	}
	if seen == 0 {
		t.Fatal("no plan step was checked")
	}
}

func TestSandboxScratchDirsIsTheSandboxList(t *testing.T) {
	got := SandboxScratchDirs()
	if !reflect.DeepEqual(got, sandbox.ScratchDirs) {
		t.Fatalf("SandboxScratchDirs = %v, sandbox.ScratchDirs = %v", got, sandbox.ScratchDirs)
	}
	got[0] = "/changed"
	if sandbox.ScratchDirs[0] == "/changed" {
		t.Error("SandboxScratchDirs aliases the sandbox's list")
	}
}

// classifyInputs must hand the scratch list to the classifier: a Config built
// without it silently turns the whole rule off.
func TestClassifyInputsPassesTheScratchDirs(t *testing.T) {
	src, err := os.ReadFile("classify_step.go")
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(src), "ScratchDirs: SandboxScratchDirs()") {
		t.Error("classifyInputs does not set traced.Config.ScratchDirs from SandboxScratchDirs()")
	}
}
