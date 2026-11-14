//go:build unix

package record

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// M3: the dynamic loader. A library planted in the checkout must not be loaded
// by anything the recorder starts on the host side.

func TestLoaderEnv(t *testing.T) {
	d := canonTemp(t)
	checkout := filepath.Join(d, "repo")
	os.MkdirAll(filepath.Join(checkout, "lib"), 0o755)
	os.Symlink(filepath.Join(checkout, "lib"), filepath.Join(d, "innocent"))
	regions := []Region{{"the checkout", checkout}, {"tmpdir", filepath.Join(d, "tmp")}}

	entries, dirs, err := LoaderEnv([]string{
		"PATH=/usr/bin", "LD_LIBRARY_PATH=/usr/lib:/opt/lib", "LD_PRELOAD=", "LD_BIND_NOW=1", "GCONV_PATH=/usr/lib/gconv", "LOCPATH=relative",
	}, regions)
	if err != nil {
		t.Fatal(err)
	}
	if strings.Join(entries, ",") != "LD_LIBRARY_PATH=/usr/lib:/opt/lib" || len(dirs) == 0 {
		t.Fatalf("%v %v", entries, dirs)
	}
	if e, _, err := LoaderEnv([]string{"PATH=/usr/bin", "LD_LIBRARY_PATH="}, regions); err != nil || len(e) != 0 {
		t.Fatalf("an empty variable is an unset one: %v %v", e, err)
	}
	for name, env := range map[string][]string{
		"LD_PRELOAD":               {"LD_PRELOAD=" + checkout + "/evil.so"},
		"LD_PRELOAD anywhere":      {"LD_PRELOAD=/usr/lib/libfoo.so"},
		"LD_AUDIT":                 {"LD_AUDIT=/usr/lib/libaudit.so"},
		"an empty entry (cwd)":     {"LD_LIBRARY_PATH=/usr/lib:"},
		"a leading empty entry":    {"LD_LIBRARY_PATH=:/usr/lib"},
		"a relative entry":         {"LD_LIBRARY_PATH=lib"},
		"dot":                      {"LD_LIBRARY_PATH=."},
		"an entry in the checkout": {"LD_LIBRARY_PATH=" + checkout + "/lib"},
		"an entry in tmpdir":       {"LD_LIBRARY_PATH=" + filepath.Join(d, "tmp")},
		"a link into the checkout": {"LD_LIBRARY_PATH=" + filepath.Join(d, "innocent")},
		"the last LD_PRELOAD wins": {"LD_PRELOAD=", "LD_PRELOAD=/x.so"},
	} {
		if _, _, err := LoaderEnv(env, regions); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	// GCONV_PATH and LOCPATH are colon LISTS; they are never passed on at all
	// (and never judged), whatever they hold.
	for _, v := range []string{"/usr/lib/gconv:" + checkout + "/g", "/usr/lib/gconv:", checkout + "/g", filepath.Join(d, "innocent")} {
		e, dirs, err := LoaderEnv([]string{"GCONV_PATH=" + v, "LOCPATH=" + v}, regions)
		if err != nil || len(e) != 0 || len(dirs) != 0 {
			t.Fatalf("%q: %v %v %v", v, e, dirs, err)
		}
	}
}

// An entry that does not exist YET is resolved through its deepest existing
// ancestor: /x/bin where /x is a link into the checkout and bin is not there.
func TestSafePathResolvesAMissingEntryThroughItsAncestor(t *testing.T) {
	d := canonTemp(t)
	checkout := filepath.Join(d, "repo")
	os.MkdirAll(checkout, 0o755)
	os.Symlink(checkout, filepath.Join(d, "x"))
	regions := []Region{{"the checkout", checkout}}
	if _, err := SafePath(filepath.Join(d, "x", "bin")+":/usr/bin", regions); err == nil {
		t.Fatal("a missing entry beneath a link into the checkout was accepted")
	}
	if _, err := SafePath(filepath.Join(d, "x", "a", "b", "c"), regions); err == nil {
		t.Fatal("a deeper missing entry was accepted")
	}
	if got, err := resolveDeepest(filepath.Join(d, "x", "bin")); err != nil || got != filepath.Join(checkout, "bin") {
		t.Fatal(got, err)
	}
	if got, err := resolveDeepest("/no/such/dir/at/all"); err != nil || got != "/no/such/dir/at/all" {
		t.Fatal(got, err)
	}
	if _, err := SafePath(filepath.Join(d, "elsewhere", "bin"), regions); err != nil {
		t.Fatal(err)
	}
}

func TestPrepareRefusesLoaderVariables(t *testing.T) {
	cases := map[string]func(p *prep) []string{
		"LD_PRELOAD":                     func(p *prep) []string { return []string{"LD_PRELOAD=/usr/lib/x.so"} },
		"LD_AUDIT":                       func(p *prep) []string { return []string{"LD_AUDIT=/usr/lib/x.so"} },
		"an empty LD_LIBRARY_PATH entry": func(p *prep) []string { return []string{"LD_LIBRARY_PATH=/usr/lib:"} },
		"a checkout LD_LIBRARY_PATH":     func(p *prep) []string { return []string{"LD_LIBRARY_PATH=" + p.root + "/lib"} },
		"a record-directory entry":       func(p *prep) []string { return []string{"LD_LIBRARY_PATH=" + p.dir + "/pre"} },
	}
	for name, extra := range cases {
		t.Run(name, func(t *testing.T) {
			p := newPrep(t)
			p.commit()
			p.env = append(p.env, extra(p)...)
			t.Cleanup(p.writable)
			if _, err := Prepare(p.options()); err == nil {
				t.Fatal("accepted")
			}
			if len(p.toolCalls) != 0 {
				t.Fatal("the build tool was started")
			}
		})
	}
}

// M4: the build tool's environment is an allowlist. With every dangerous
// variable set in the process, the build tool sees exactly these and nothing
// else.
func TestBuildToolGetsExactlyTheAllowlist(t *testing.T) {
	p := newPrep(t)
	p.commit()
	for k, v := range map[string]string{
		"GIT_EXEC_PATH": p.root + "/x", "GIT_CONFIG_GLOBAL": p.root + "/c", "GIT_CONFIG_COUNT": "9", "GIT_DIR": p.root + "/.git",
		"GIT_CONFIG_KEY_0": "core.fsmonitor", "GIT_CONFIG_VALUE_0": p.root + "/evil", "GIT_SSH_COMMAND": "evil",
		"GLIBC_TUNABLES": "glibc.malloc.check=3", "GCONV_PATH": "/usr/lib/gconv:" + p.root + "/g", "LOCPATH": p.root + "/l",
		"LD_BIND_NOW": "1", "TZDIR": p.root + "/tz", "GITHUB_OUTPUT": p.root + "/out", "GITHUB_TOKEN": "t", "SECRET_THING": "s", "TMPDIR": p.root + "/t",
	} {
		p.env = append(p.env, k+"="+v)
	}
	p.env = append(p.env, "LD_LIBRARY_PATH=/usr/lib")
	res := p.mustPrepare()
	want := []string{
		"HOME=" + p.home, "PATH=/usr/bin:/bin", "LD_LIBRARY_PATH=/usr/lib",
		"GIT_CONFIG_NOSYSTEM=1", "GIT_CONFIG_SYSTEM=" + os.DevNull, "GIT_CONFIG_GLOBAL=" + os.DevNull, "GIT_OPTIONAL_LOCKS=0",
		"GIT_TERMINAL_PROMPT=0", "LC_ALL=C", "GIT_NO_REPLACE_OBJECTS=1", "GIT_NO_LAZY_FETCH=1",
		"GIT_CONFIG_COUNT=3", "GIT_CONFIG_KEY_0=core.fsmonitor", "GIT_CONFIG_VALUE_0=false",
		"GIT_CONFIG_KEY_1=core.hooksPath", "GIT_CONFIG_VALUE_1=" + os.DevNull,
		"GIT_CONFIG_KEY_2=protocol.file.allow", "GIT_CONFIG_VALUE_2=never",
	}
	if len(p.toolEnvs) != 2 {
		t.Fatal(p.toolEnvs)
	}
	for _, env := range p.toolEnvs {
		if strings.Join(env, "\n") != strings.Join(want, "\n") {
			t.Fatalf("the build tool's environment is not exactly the allowlist:\n got %q\nwant %q", env, want)
		}
	}
	// pre/tools.json records exactly that environment for `record step`.
	var tools ToolsFile
	if err := json.Unmarshal(readPre(t, res, "tools.json"), &tools); err != nil {
		t.Fatal(err)
	}
	if strings.Join(tools.BuildToolEnv, "\n") != strings.Join(want, "\n") {
		t.Fatalf("%q", tools.BuildToolEnv)
	}
}

func TestBuildToolEnvWithoutLoaderVariables(t *testing.T) {
	got := buildToolEnv("/h", "/usr/bin", nil)
	if got[0] != "HOME=/h" || got[1] != "PATH=/usr/bin" || len(got) != 2+8+7 {
		t.Fatalf("%q", got)
	}
	for _, e := range got {
		if strings.HasPrefix(e, "LD_") || strings.HasPrefix(e, "GCONV") || strings.HasPrefix(e, "LOCPATH") {
			t.Fatalf("%s", e)
		}
	}
}

// A state root over a loader directory would let a step plant a library there.
func TestStateRootsStayOutOfLoaderDirectories(t *testing.T) {
	p := newPrep(t)
	p.roots = []string{`{"path": "$HOME/lib", "kind": "state", "ecosystem": "x"}`}
	p.commit()
	p.env = append(p.env, "LD_LIBRARY_PATH="+filepath.Join(p.home, "lib"))
	t.Cleanup(p.writable)
	if _, err := Prepare(p.options()); err == nil || !strings.Contains(err.Error(), "state root") {
		t.Fatalf("%v", err)
	}
}

// Every host-side subprocess starts in a directory a step cannot write: the
// build tool in the record directory, git and strace in "/".
func TestHostSideSubprocessesStartOutsideTheCheckout(t *testing.T) {
	p := newPrep(t)
	p.commit()
	opts := p.options()
	var dirs []string
	inner := opts.RunTool
	opts.RunTool = func(r ToolRun) error { dirs = append(dirs, r.Dir); return inner(r) }
	t.Cleanup(p.writable)
	if _, err := Prepare(opts); err != nil {
		t.Fatal(err)
	}
	if len(dirs) != 2 {
		t.Fatal(dirs)
	}
	for _, d := range dirs {
		if d != p.dir || d == "" || within(d, p.root) {
			t.Fatalf("the build tool started in %q", d)
		}
	}
	cwd := filepath.Join(canonTemp(t), "pwd")
	script := script2(t, "pwd -P > "+cwd+"; echo 'strace -- version 6'")
	if _, err := straceVersion(script); err != nil {
		t.Fatal(err)
	}
	if got, _ := os.ReadFile(cwd); strings.TrimSpace(string(got)) != "/" {
		t.Fatalf("strace started in %q", got)
	}
}

func script2(t *testing.T, body string) string {
	t.Helper()
	p := filepath.Join(canonTemp(t), "tool")
	writeBytes(t, p, []byte("#!/bin/sh\n"+body+"\n"), 0o755)
	return p
}

func TestSearchByMode(t *testing.T) {
	const me, mygid = 1000, 1000
	for name, c := range map[string]struct {
		perm       os.FileMode
		uid, gid   uint32
		euid, egid uint32
		groups     []int
		want       bool
	}{
		"owner with x":                          {0o100, me, 5, me, mygid, nil, true},
		"owner without x":                       {0o077, me, 5, me, mygid, nil, false},
		"owner x denied even if others have it": {0o001, me, 5, me, mygid, nil, false},
		"group with x":                          {0o010, 7, 8, me, mygid, []int{8}, true},
		"group without x":                       {0o707, 7, 8, me, mygid, []int{8}, false},
		"primary group":                         {0o010, 7, mygid, me, mygid, nil, true},
		"other with x":                          {0o001, 7, 8, me, mygid, []int{9}, true},
		"other without x":                       {0o770, 7, 8, me, mygid, []int{9}, false},
		"root":                                  {0o000, 7, 8, 0, 0, nil, true},
	} {
		if got := searchByMode(c.perm, c.uid, c.gid, c.euid, c.egid, c.groups); got != c.want {
			t.Errorf("%s: %v", name, got)
		}
	}
}

func TestSearchableFailsClosed(t *testing.T) {
	if !searchable(filepath.Join(canonTemp(t), "does-not-exist")) {
		t.Fatal("a path that cannot be examined counted as unsearchable")
	}
	if !searchable(canonTemp(t)) {
		t.Fatal("a directory we own was unsearchable")
	}
}

func TestSealFailureReportsAndRestores(t *testing.T) {
	d := canonTemp(t)
	os.Mkdir(filepath.Join(d, "actions"), 0o755)
	w := &sealWriter{pre: d, sums: map[string]string{}}
	if err := w.write("a.json", []byte("x")); err != nil {
		t.Fatal(err)
	}
	// Make the pass fail by dropping a symlink in AFTER the content check: drive
	// the failure path through readOnly + writable directly.
	os.Symlink(d, filepath.Join(d, "actions", "sneaky"))
	if err := w.readOnly(); err == nil {
		t.Fatal("readOnly accepted a symlink")
	}
	w.writable()
	st, err := os.Stat(filepath.Join(d, "actions"))
	if err != nil || st.Mode().Perm() != 0o700 {
		t.Fatalf("%v %v", st, err)
	}
}

// A DANGLING link is followed by hand: a step can create its target.
func TestSafePathFollowsDanglingLinks(t *testing.T) {
	d := canonTemp(t)
	checkout := filepath.Join(d, "repo")
	os.MkdirAll(checkout, 0o755)
	regions := []Region{{"the checkout", checkout}}
	// <d>/dangling -> <checkout>/x/bin, and <checkout>/x does not exist.
	os.Symlink(filepath.Join(checkout, "x", "bin"), filepath.Join(d, "dangling"))
	if _, err := SafePath(filepath.Join(d, "dangling")+":/usr/bin", regions); err == nil {
		t.Fatal("a dangling link to <checkout>/x/bin was accepted")
	}
	// A relative dangling link, with a remainder beneath it.
	os.Symlink("repo/new", filepath.Join(d, "rel"))
	if _, err := SafePath(filepath.Join(d, "rel", "deeper", "bin"), regions); err == nil {
		t.Fatal("a relative dangling link into the checkout was accepted")
	}
	// A chain of dangling links.
	os.Symlink(filepath.Join(d, "dangling"), filepath.Join(d, "chain"))
	if _, err := SafePath(filepath.Join(d, "chain"), regions); err == nil {
		t.Fatal("a chain of dangling links was accepted")
	}
	// A dangling link that points somewhere harmless is fine.
	os.Symlink(filepath.Join(d, "elsewhere", "bin"), filepath.Join(d, "harmless"))
	if _, err := SafePath(filepath.Join(d, "harmless"), regions); err != nil {
		t.Fatal(err)
	}
	// A loop is refused, not followed forever.
	os.Symlink(filepath.Join(d, "b"), filepath.Join(d, "a"))
	os.Symlink(filepath.Join(d, "a"), filepath.Join(d, "b"))
	if _, err := SafePath(filepath.Join(d, "a"), regions); err == nil || !strings.Contains(err.Error(), "symbolic links") {
		t.Fatalf("%v", err)
	}
}
