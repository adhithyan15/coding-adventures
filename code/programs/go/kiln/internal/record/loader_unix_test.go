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
	if strings.Join(entries, ",") != "LD_LIBRARY_PATH=/usr/lib:/opt/lib,GCONV_PATH=/usr/lib/gconv" || len(dirs) == 0 {
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
	// GCONV_PATH and LOCPATH are dropped, not refused, when unsafe.
	e, _, err := LoaderEnv([]string{"GCONV_PATH=" + checkout + "/g", "LOCPATH=" + filepath.Join(d, "innocent")}, regions)
	if err != nil || len(e) != 0 {
		t.Fatalf("%v %v", e, err)
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
	if got := resolveDeepest(filepath.Join(d, "x", "bin")); got != filepath.Join(checkout, "bin") {
		t.Fatal(got)
	}
	if got := resolveDeepest("/no/such/dir/at/all"); got != "/no/such/dir/at/all" {
		t.Fatal(got)
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

func TestPrepareValidatesLoaderVariablesAndRecordsThem(t *testing.T) {
	p := newPrep(t)
	p.commit()
	p.env = append(p.env, "LD_LIBRARY_PATH=/usr/lib", "LD_BIND_NOW=1", "GCONV_PATH=gconv", "LOCPATH=/usr/lib/locale")
	res := p.mustPrepare()
	var tools ToolsFile
	if err := json.Unmarshal(readPre(t, res, "tools.json"), &tools); err != nil {
		t.Fatal(err)
	}
	if strings.Join(tools.LoaderEnv, ",") != "LD_LIBRARY_PATH=/usr/lib,LOCPATH=/usr/lib/locale" {
		t.Fatalf("%v", tools.LoaderEnv)
	}
	for _, env := range p.toolEnvs {
		got := map[string]string{}
		for _, e := range env {
			k, v, _ := strings.Cut(e, "=")
			got[k] = v
		}
		if got["LD_LIBRARY_PATH"] != "/usr/lib" || got["LOCPATH"] != "/usr/lib/locale" || got["LD_BIND_NOW"] != "" || got["GCONV_PATH"] != "" {
			t.Fatalf("the build tool's loader environment: %v", env)
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
