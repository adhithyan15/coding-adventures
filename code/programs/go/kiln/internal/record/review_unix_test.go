//go:build unix

package record

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
)

// ---------------------------------------------------------------------------
// H1: the recorder's PATH and the git it runs.

func TestSafePath(t *testing.T) {
	d := canonTemp(t)
	checkout := filepath.Join(d, "repo")
	if err := os.MkdirAll(filepath.Join(checkout, ".lua", "bin"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(filepath.Join(checkout, ".lua", "bin"), filepath.Join(d, "looks-harmless")); err != nil {
		t.Fatal(err)
	}
	regions := []Region{{"the checkout", checkout}, {"tmpdir", filepath.Join(d, "tmp")}}

	ok, err := SafePath("/usr/bin:/bin:/opt/not/there", regions)
	if err != nil || ok.String() != "/usr/bin:/bin:/opt/not/there" || len(ok.Entries) != 3 || len(ok.Regions()) != 3 {
		t.Fatalf("%+v %v", ok, err)
	}
	for name, p := range map[string]string{
		"empty":                     "",
		"an empty entry (cwd)":      "/usr/bin::/bin",
		"a leading empty entry":     ":/usr/bin",
		"a trailing empty entry":    "/usr/bin:",
		"a relative entry":          "/usr/bin:bin",
		"dot":                       ".:/usr/bin",
		"the checkout's .lua/bin":   checkout + "/.lua/bin:/usr/bin",
		"the checkout itself":       checkout,
		"an ancestor of it":         d,
		"tmpdir":                    filepath.Join(d, "tmp") + "/x",
		"a symlink into checkout":   filepath.Join(d, "looks-harmless") + ":/usr/bin",
		"an unclean spelling of it": checkout + "/x/../.lua/bin",
	} {
		if _, err := SafePath(p, regions); err == nil {
			t.Errorf("%s: PATH %q was accepted", name, p)
		}
	}
}

func TestPathHelpers(t *testing.T) {
	if got := pathOf([]string{"A=1", "PATH=/a", "PATH=/b"}); got != "/b" {
		t.Fatal(got)
	}
	if pathOf([]string{"A=1"}) != "" {
		t.Fatal("found a PATH that is not there")
	}
	got := childEnv([]string{"A=1", "PATH=/a", "PATHEXT=x", "PATH=/b", "LD_PRELOAD=/x", "LD_DEBUG=all", "GCONV_PATH=/g", "LOCPATH=/l", "LD_LIBRARY_PATH=/z"},
		"/safe", []string{"LD_LIBRARY_PATH=/ok"})
	if strings.Join(got, ",") != "A=1,PATHEXT=x,PATH=/safe,LD_LIBRARY_PATH=/ok" {
		t.Fatal(got)
	}
}

// The review's proof of concept: a git planted in a directory the checkout is
// bound into, and put in front of PATH by an earlier workflow step. Prepare
// must refuse it before running anything, and must never run it.
func TestPrepareNeverRunsAGitFromThePath(t *testing.T) {
	p := newPrep(t)
	p.files["code/.lua/bin/git"] = "#!/bin/sh\n"
	p.commit()
	marker := filepath.Join(p.base, "git-was-run")
	script := []byte("#!/bin/sh\ntouch " + marker + "\nexit 1\n")
	planted := filepath.Join(p.root, "code", ".lua", "bin")
	if err := os.WriteFile(filepath.Join(planted, "git"), script, 0o755); err != nil {
		t.Fatal(err)
	}
	p.env = []string{"HOME=" + p.home, "PATH=" + planted + ":/usr/bin:/bin"}
	if _, err := p.prepare(); err == nil || !strings.Contains(err.Error(), "PATH entry") || !strings.Contains(err.Error(), "the checkout") {
		t.Fatalf("%v", err)
	}
	if _, err := os.Stat(marker); err == nil {
		t.Fatal("the planted git ran")
	}
}

// Even with a safe recorder PATH, a git on the PROCESS path is not used: every
// host-side Git call runs the one --git, by absolute path.
func TestPrepareRunsOnlyThePinnedGit(t *testing.T) {
	p := newPrep(t)
	p.commit()
	planted := filepath.Join(p.base, "planted")
	marker := filepath.Join(p.base, "planted-git-ran")
	if err := os.MkdirAll(planted, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(planted, "git"), []byte("#!/bin/sh\ntouch "+marker+"\nexit 1\n"), 0o755); err != nil {
		t.Fatal(err)
	}
	t.Setenv("PATH", planted+":/usr/bin:/bin")
	res := p.mustPrepare()
	if _, err := os.Stat(marker); err == nil {
		t.Fatal("a git found through the process PATH ran")
	}
	// tools.json records the absolute git, strace and the PATH step and collect
	// must use.
	var tools ToolsFile
	if err := json.Unmarshal(readPre(t, res, "tools.json"), &tools); err != nil {
		t.Fatal(err)
	}
	if tools.Schema != ToolsSchema || tools.Git != p.gitBin || tools.Strace != p.strace || tools.Path != "/usr/bin:/bin" {
		t.Fatalf("%+v", tools)
	}
	// The build tool got exactly that PATH, and nothing from the process.
	if len(p.toolEnvs) != 2 {
		t.Fatal(p.toolEnvs)
	}
	for _, env := range p.toolEnvs {
		n := 0
		for _, e := range env {
			if strings.HasPrefix(e, "PATH=") {
				n++
				if e != "PATH=/usr/bin:/bin" {
					t.Fatalf("the build tool's %s", e)
				}
			}
		}
		if n != 1 {
			t.Fatalf("%d PATH entries: %v", n, env)
		}
	}
}

func TestPrepareRefusesAnUnsafePathAndAnUnsafeGit(t *testing.T) {
	cases := map[string]struct {
		before func(p *prep)
		opts   func(p *prep, o *Options)
		want   string
	}{
		"no PATH":                {before: func(p *prep) { p.env = []string{"HOME=" + p.home} }, want: "PATH is empty"},
		"an empty PATH entry":    {before: func(p *prep) { p.env = []string{"HOME=" + p.home, "PATH=/usr/bin::/bin"} }, want: "empty entry"},
		"a relative PATH entry":  {before: func(p *prep) { p.env = []string{"HOME=" + p.home, "PATH=bin:/usr/bin"} }, want: "relative"},
		"a PATH entry in tmpdir": {before: func(p *prep) { p.env = []string{"HOME=" + p.home, "PATH=" + p.tmpdir + "/bin"} }, want: "tmpdir"},
		"a PATH entry in --dir":  {before: func(p *prep) { p.env = []string{"HOME=" + p.home, "PATH=" + p.dir + "/pre"} }, want: "record directory"},
		"a PATH link into the checkout": {before: func(p *prep) {
			if err := os.MkdirAll(filepath.Join(p.root, "code", "bin"), 0o755); err != nil {
				p.t.Fatal(err)
			}
			link := filepath.Join(p.base, "innocent")
			if err := os.Symlink(filepath.Join(p.root, "code", "bin"), link); err != nil {
				p.t.Fatal(err)
			}
			p.env = []string{"HOME=" + p.home, "PATH=" + link}
		}, want: "overlaps the checkout"},
		"git inside the checkout": {before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.root, "git"), []byte("g"), 0o755)
			p.gitBin = filepath.Join(p.root, "git")
		}, want: "--git"},
		"git that is missing": {opts: func(p *prep, o *Options) { o.Git = filepath.Join(p.base, "bin", "nogit") }, want: "no such file"},
		"git that is a symlink": {before: func(p *prep) {
			link := filepath.Join(p.base, "bin", "git-link")
			if err := os.Symlink(p.gitBin, link); err != nil {
				p.t.Fatal(err)
			}
			p.gitBin = link
		}, want: "not canonical"},
		"a relative git": {opts: func(p *prep, o *Options) { o.Git = "git" }, want: "absolute"},
		"strace inside tmpdir": {before: func(p *prep) {
			writeBytes(p.t, filepath.Join(p.tmpdir, "strace"), []byte("s"), 0o755)
			p.strace = filepath.Join(p.tmpdir, "strace")
		}, want: "--strace"},
	}
	for name, c := range cases {
		t.Run(name, func(t *testing.T) {
			p := newPrep(t)
			p.commit()
			if c.before != nil {
				c.before(p)
			}
			o := p.options()
			if c.opts != nil {
				c.opts(p, &o)
			}
			t.Cleanup(p.writable)
			if _, err := Prepare(o); err == nil || !strings.Contains(err.Error(), c.want) {
				t.Fatalf("got %v, want %q", err, c.want)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// Ownership and mode of the directories the recorder keeps private.

func TestClaimedDirectories(t *testing.T) {
	d := canonTemp(t)
	for name, c := range map[string]struct {
		mode os.FileMode
		ok   bool
	}{"0755": {0o755, true}, "0700": {0o700, true}, "0750": {0o750, true}, "group-writable": {0o770, false}, "world-writable": {0o777, false}, "0702": {0o702, false}} {
		dir := filepath.Join(d, "rec-"+name)
		if err := os.Mkdir(dir, 0o700); err != nil {
			t.Fatal(err)
		}
		if err := os.Chmod(dir, c.mode); err != nil {
			t.Fatal(err)
		}
		_, err := checkDirArg(dir)
		if (err == nil) != c.ok {
			t.Errorf("%s: %v", name, err)
		}
		if err == nil {
			if st, _ := os.Stat(dir); st.Mode().Perm() != 0o700 {
				t.Errorf("%s: mode forced to %v, want 0700", name, st.Mode().Perm())
			}
		}
		// The same rule for the step's tmpdir.
		tmp := filepath.Join(d, "tmp-"+name)
		os.Mkdir(tmp, 0o700)
		os.Chmod(tmp, c.mode)
		if err := ensureTmpdir(tmp); (err == nil) != c.ok {
			t.Errorf("tmpdir %s: %v", name, err)
		}
	}
	if os.Geteuid() == 0 { // only root can hand a directory to someone else
		other := filepath.Join(d, "someone-elses")
		os.Mkdir(other, 0o700)
		os.Chown(other, 12345, 12345)
		if _, err := checkDirArg(other); err == nil || !strings.Contains(err.Error(), "owned by") {
			t.Errorf("a directory owned by another user was claimed: %v", err)
		}
	}
}

// ---------------------------------------------------------------------------
// Sockets and FIFOs in the checkout, and the host scan's failure modes.

func TestScanTreeRefusesASocketOrFifoInTheCheckout(t *testing.T) {
	root := cleanRepo(t)
	if err := syscall.Mkfifo(filepath.Join(root, "code", "pipe"), 0o600); err != nil {
		t.Fatal(err)
	}
	if err := ScanTree(root); err == nil || !strings.Contains(err.Error(), "code/pipe") {
		t.Fatalf("%v", err)
	}
	os.Remove(filepath.Join(root, "code", "pipe"))
	// A socket's path is limited to about 100 bytes, so use a short checkout.
	requireGit(t)
	short := shortTemp(t)
	runGit(t, short, "init", "-q")
	writeFile(t, short, "f", "x")
	runGit(t, short, "add", "-A")
	runGit(t, short, "-c", "commit.gpgsign=false", "commit", "-q", "-m", "x")
	listenUnix(t, filepath.Join(short, "s"))
	if err := ScanTree(short); err == nil || !strings.Contains(err.Error(), "socket or FIFO") {
		t.Fatalf("%v", err)
	}
}

// M2: a directory that can be searched but not listed hides its names, not its
// sockets.
func TestHostScanFailsOnASearchableButUnlistableDirectory(t *testing.T) {
	if os.Geteuid() == 0 {
		t.Skip("root lists everything")
	}
	root := shortTemp(t)
	hidden := filepath.Join(root, "hidden")
	if err := os.Mkdir(hidden, 0o755); err != nil {
		t.Fatal(err)
	}
	listenUnix(t, filepath.Join(hidden, "s"))
	if err := os.Chmod(hidden, 0o100); err != nil { // --x: traversable, not listable
		t.Fatal(err)
	}
	defer os.Chmod(hidden, 0o755)
	if err := HostScan([]string{root}); err == nil {
		t.Fatal("a directory with search permission and no read permission was skipped")
	}
	// With no search permission either, a step cannot reach anything in it.
	if err := os.Chmod(hidden, 0); err != nil {
		t.Fatal(err)
	}
	if err := HostScan([]string{root}); err != nil {
		t.Fatalf("a directory nobody can enter failed the scan: %v", err)
	}
}

func TestHostScanBelievesOnlyARealMountPoint(t *testing.T) {
	root := shortTemp(t)
	sub := filepath.Join(root, "sub")
	if err := os.Mkdir(sub, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := syscall.Mkfifo(filepath.Join(sub, "pipe"), 0o600); err != nil {
		t.Fatal(err)
	}
	oldDev, oldMounts := devOf, mountPoints
	defer func() { devOf, mountPoints = oldDev, oldMounts }()
	devOf = func(fi os.FileInfo) (uint64, bool) {
		if fi.Name() == "sub" {
			return 99, true
		}
		return oldDev(fi)
	}
	// A different st_dev that mountinfo does not list (or no mountinfo at all)
	// is scanned through: the scan can only find more, never less.
	mountPoints = func() map[string]bool { return nil }
	if err := HostScan([]string{root}); err == nil {
		t.Fatal("a directory with another st_dev but no mountinfo entry was skipped")
	}
	mountPoints = func() map[string]bool { return map[string]bool{"/elsewhere": true} }
	if err := HostScan([]string{root}); err == nil {
		t.Fatal("a directory that mountinfo does not list was skipped")
	}
	mountPoints = func() map[string]bool { return map[string]bool{sub: true} }
	if err := HostScan([]string{root}); err != nil {
		t.Fatalf("a listed mount point was entered: %v", err)
	}
}

func TestUnescapeMountAndMountinfo(t *testing.T) {
	for in, want := range map[string]string{
		`/plain`: "/plain", `/a\040b`: "/a b", `/t\011x`: "/t\tx", `/back\134slash`: `/back\slash`, `/short\04`: `/short\04`, `/bad\zzz`: `/bad\zzz`,
	} {
		if got := unescapeMount(in); got != want {
			t.Errorf("unescapeMount(%q) = %q, want %q", in, got, want)
		}
	}
	m := readMountPoints()
	if m != nil && !m["/"] {
		t.Fatalf("mountinfo parsed but has no root mount: %d entries", len(m))
	}
}

func TestHostScanStopsAtMaxReported(t *testing.T) {
	root := shortTemp(t)
	for i := 0; i < MaxReported+10; i++ {
		if err := syscall.Mkfifo(filepath.Join(root, "p"+string(rune('a'+i))), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	err := HostScan([]string{root})
	te, ok := err.(*TreeError)
	if !ok || len(te.Problems) != MaxReported || te.Total != MaxReported {
		t.Fatalf("%T %v", err, err)
	}
}
