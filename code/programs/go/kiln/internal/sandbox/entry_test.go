package sandbox

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestParseExecArgs(t *testing.T) {
	a, err := ParseExecArgs([]string{"--config", "/etc/k.json", "--", "sh", "-c", "make"})
	if err != nil || a.Config != "/etc/k.json" || !reflect.DeepEqual(a.Argv, []string{"sh", "-c", "make"}) {
		t.Fatalf("%+v %v", a, err)
	}
	for name, args := range map[string][]string{
		"none":          {},
		"no argv":       {"--config", "/c", "--"},
		"no separator":  {"--config", "/c", "sh", "x"},
		"wrong flag":    {"--cfg", "/c", "--", "sh"},
		"relative":      {"--config", "c.json", "--", "sh"},
		"unclean":       {"--config", "/a/../c", "--", "sh"},
		"nul in argv":   {"--config", "/c", "--", "sh", "a\x00b"},
		"empty command": {"--config", "/c", "--", ""},
	} {
		_, err := ParseExecArgs(args)
		if name == "empty command" {
			if err != nil {
				t.Errorf("%s: an empty argv[0] is the step's own problem: %v", name, err)
			}
			continue
		}
		if err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
}

func initArgs(extra ...string) []string {
	sum := strings.Repeat("a", 64)
	return append([]string{"--config", "/c.json", "--sha256", sum, "--action", "pkg", "--uid", "501", "--gid", "20", "--"}, extra...)
}

func TestParseInitArgs(t *testing.T) {
	a, err := ParseInitArgs(initArgs("sh", "-c", "x"))
	if err != nil {
		t.Fatal(err)
	}
	if a.Config != "/c.json" || a.Action != "pkg" || a.UID != 501 || a.GID != 20 || len(a.SHA256) != 64 || !reflect.DeepEqual(a.Argv, []string{"sh", "-c", "x"}) {
		t.Fatalf("%+v", a)
	}
	bad := map[string][]string{
		"short":        initArgs()[:5],
		"no argv":      initArgs(),
		"order":        append([]string{"--sha256", "x", "--config", "/c"}, initArgs("sh")[4:]...),
		"uid":          strings.Fields("--config /c.json --sha256 " + strings.Repeat("a", 64) + " --action p --uid x --gid 1 -- sh"),
		"gid":          strings.Fields("--config /c.json --sha256 " + strings.Repeat("a", 64) + " --action p --uid 1 --gid -1 -- sh"),
		"big uid":      strings.Fields("--config /c.json --sha256 " + strings.Repeat("a", 64) + " --action p --uid 99999999999 --gid 1 -- sh"),
		"sha length":   strings.Fields("--config /c.json --sha256 abc --action p --uid 1 --gid 1 -- sh"),
		"separator":    strings.Fields("--config /c.json --sha256 " + strings.Repeat("a", 64) + " --action p --uid 1 --gid 1 x sh"),
		"relative cfg": strings.Fields("--config c.json --sha256 " + strings.Repeat("a", 64) + " --action p --uid 1 --gid 1 -- sh"),
		"nul":          initArgs("sh", "a\x00"),
	}
	for name, args := range bad {
		if _, err := ParseInitArgs(args); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
}

func TestReadConfig(t *testing.T) {
	dir := t.TempDir()
	p := filepath.Join(dir, "c.json")
	if err := os.WriteFile(p, []byte(`{"x":1}`), 0o600); err != nil {
		t.Fatal(err)
	}
	data, sum, err := ReadConfig(p)
	want := sha256.Sum256([]byte(`{"x":1}`))
	if err != nil || string(data) != `{"x":1}` || sum != hex.EncodeToString(want[:]) {
		t.Fatalf("%q %s %v", data, sum, err)
	}
	if _, _, err := ReadConfig(filepath.Join(dir, "missing")); err == nil {
		t.Error("missing file")
	}
	if _, _, err := ReadConfig(dir); err == nil {
		t.Error("a directory is not a configuration")
	}
	big := filepath.Join(dir, "big")
	f, _ := os.Create(big)
	f.Truncate(MaxConfigBytes + 1)
	f.Close()
	if _, _, err := ReadConfig(big); err == nil || !strings.Contains(err.Error(), "too large") {
		t.Errorf("oversize: %v", err)
	}
}

func TestPathLookup(t *testing.T) {
	env := []string{"A=b", "PATH=/opt/x/bin::rel/bin:/usr/bin:/usr/bin/", "Z=1"}
	got := PathLookup("sh", env)
	want := []string{"/opt/x/bin/sh", "/usr/bin/sh", "/usr/bin/sh"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("%v", got)
	}
	if got := PathLookup("sh", []string{"A=b"}); len(got) != 0 {
		t.Errorf("no PATH: %v", got)
	}
	if got := PathLookup("sh", []string{"PATH="}); len(got) != 0 {
		t.Errorf("empty PATH: %v", got)
	}
	if got := PathLookup("sh", []string{"PATHX=/bin", "XPATH=/bin"}); len(got) != 0 {
		t.Errorf("look-alike names: %v", got)
	}
}

func TestUnsupportedOrSupportedEntryPoints(t *testing.T) {
	// On unsupported platforms both commands refuse; on the supported one the
	// argument checks run first.  Neither may start anything here.
	var out bytes.Buffer
	if code := RunExec([]string{"--bogus"}, &out); code != ExitFailure || out.Len() == 0 {
		t.Errorf("RunExec: %d %q", code, out.String())
	}
	out.Reset()
	if code := RunInit([]string{"--bogus"}, &out); code != ExitFailure || out.Len() == 0 {
		t.Errorf("RunInit: %d %q", code, out.String())
	}
	if ExitFailure == 126 || ExitFailure == 127 {
		t.Error("the failure status must not collide with the shell's")
	}
}
