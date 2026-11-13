package sandbox

import (
	"encoding/json"
	"reflect"
	"strings"
	"testing"
)

func goodConfig() *Config {
	return &Config{
		RepoRoot:   "/home/runner/work/repo",
		Home:       "/home/runner",
		Tmpdir:     "/home/runner/_tmp",
		StateRoots: []string{"/home/runner/.cargo/registry"},
		Masks:      []string{"/home/runner/work/repo/.git", "/home/runner/work/_temp", "/home/runner/kiln-state"},
		EnvByAction: map[string][]string{
			"code/packages/go/a": {"HOME=/home/runner", "LANG=C.UTF-8", "PATH=/usr/bin:/bin", "TZ=UTC"},
			".":                  {"PATH=/usr/bin"},
		},
		KilnExe: "/opt/kiln/bin/kiln",
	}
}

func mustJSON(t *testing.T, c *Config) []byte {
	t.Helper()
	b, err := json.Marshal(c)
	if err != nil {
		t.Fatal(err)
	}
	return b
}

func TestParseConfigGood(t *testing.T) {
	c, err := ParseConfig(mustJSON(t, goodConfig()))
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(c, goodConfig()) {
		t.Fatalf("round trip: %+v", c)
	}
	if got := c.Actions(); !reflect.DeepEqual(got, []string{".", "code/packages/go/a"}) {
		t.Errorf("actions %v", got)
	}
	if c.PackageDir("code/packages/go/a") != "/home/runner/work/repo/code/packages/go/a" || c.PackageDir(".") != c.RepoRoot {
		t.Error("package dir")
	}
}

func TestParseConfigStrict(t *testing.T) {
	good := string(mustJSON(t, goodConfig()))
	cases := map[string]string{
		"trailing data":   good + " {}",
		"two values":      good + good,
		"not an object":   `[]`,
		"empty":           ``,
		"unknown key":     strings.Replace(good, `"masks"`, `"extra":1,"masks"`, 1),
		"wrong case":      strings.Replace(good, `"repo_root"`, `"Repo_Root"`, 1),
		"duplicate key":   strings.Replace(good, `"masks"`, `"home":"/x","masks"`, 1),
		"dup action":      strings.Replace(good, `"PATH=/usr/bin"]`, `"PATH=/usr/bin"],".":["A=b"]`, 1),
		"missing key":     strings.Replace(good, `"kiln_exe":"/opt/kiln/bin/kiln"`, `"kiln_exes":"/opt/kiln/bin/kiln"`, 1),
		"wrong type":      strings.Replace(good, `"masks":[`, `"masks":[1,`, 1),
		"null":            `null`,
		"bad json":        `{"repo_root":`,
		"missing masks":   strings.Replace(good, `"masks"`, `"maskz"`, 1),
		"nested dup":      strings.Replace(good, `"HOME=/home/runner"`, `"HOME=/home/runner"],"x":[`, 1),
		"duplicate empty": `{"a":1,"a":2}`,
	}
	for name, text := range cases {
		if _, err := ParseConfig([]byte(text)); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	if _, err := ParseConfig(make([]byte, MaxConfigBytes+1)); err == nil {
		t.Error("oversize config accepted")
	}
}

func TestDuplicateKeyScanner(t *testing.T) {
	for _, ok := range []string{`{}`, `{"a":1,"b":{"a":2}}`, `{"a":[{"a":1},{"a":2}],"b":null}`, `[1,2]`, `"x"`} {
		if err := checkNoDuplicateKeys([]byte(ok)); err != nil {
			t.Errorf("%s: %v", ok, err)
		}
	}
	for _, bad := range []string{`{"a":1,"a":2}`, `{"a":{"b":1,"b":2}}`, `[{"a":1,"a":1}]`, `{"a":[],"a":[]}`} {
		if err := checkNoDuplicateKeys([]byte(bad)); err == nil {
			t.Errorf("%s: accepted", bad)
		}
	}
}

func TestValidateRefusals(t *testing.T) {
	cases := []struct {
		name string
		edit func(*Config)
		want string
	}{
		{"relative repo", func(c *Config) { c.RepoRoot = "work/repo" }, "not absolute"},
		{"unclean repo", func(c *Config) { c.RepoRoot = "/home/runner/work/../work/repo" }, "not clean"},
		{"trailing slash", func(c *Config) { c.RepoRoot += "/" }, "not clean"},
		{"root repo", func(c *Config) { c.RepoRoot = "/" }, "filesystem root"},
		{"empty home", func(c *Config) { c.Home = "" }, "empty"},
		{"colon", func(c *Config) { c.Home = "/home/run:ner" }, "separator"},
		{"comma", func(c *Config) { c.Tmpdir = "/tmp/a,b" }, "separator"},
		{"newline", func(c *Config) { c.Masks = []string{"/a\nb"} }, "control"},
		{"nul", func(c *Config) { c.Masks = []string{"/a\x00b"} }, "control"},
		{"not utf8", func(c *Config) { c.Masks = []string{"/a\xffb"} }, "UTF-8"},
		{"long", func(c *Config) { c.Masks = []string{"/" + strings.Repeat("a", maxPath)} }, "too long"},
		{"home in repo", func(c *Config) { c.Home = c.RepoRoot + "/home" }, "inside repo_root"},
		{"tmpdir is repo", func(c *Config) { c.Tmpdir = c.RepoRoot }, "inside repo_root"},
		{"tmpdir in repo", func(c *Config) { c.Tmpdir = c.RepoRoot + "/tmp" }, "inside repo_root"},
		{"home is tmpdir", func(c *Config) { c.Tmpdir = c.Home }, "differ"},
		{"repo in tmpdir", func(c *Config) { c.Tmpdir = "/home/runner/work"; c.Masks = nil }, "inside tmpdir"},
		{"kiln inside repo", func(c *Config) { c.KilnExe = c.RepoRoot + "/target/kiln" }, "kiln_exe"},
		{"relative kiln", func(c *Config) { c.KilnExe = "kiln" }, "not absolute"},
		{"state in repo", func(c *Config) { c.StateRoots = []string{c.RepoRoot + "/s"} }, "overlaps repo_root"},
		{"state is repo", func(c *Config) { c.StateRoots = []string{c.RepoRoot} }, "overlaps repo_root"},
		{"state holds repo", func(c *Config) { c.StateRoots = []string{"/home/runner/work"} }, "overlaps repo_root"},
		{"state in tmpdir", func(c *Config) { c.StateRoots = []string{c.Tmpdir + "/s"} }, "overlaps tmpdir"},
		{"state holds tmpdir", func(c *Config) { c.StateRoots = []string{"/home/runner/_tmp"} }, "overlaps tmpdir"},
		{"state covers home", func(c *Config) {
			c.RepoRoot, c.Tmpdir, c.Masks = "/work/repo", "/scratch", nil
			c.StateRoots = []string{"/home"}
		}, "covers home"},
		{"state is home", func(c *Config) {
			c.RepoRoot, c.Tmpdir, c.Masks = "/work/repo", "/scratch", nil
			c.StateRoots = []string{c.Home}
		}, "covers home"},
		{"dup state", func(c *Config) { c.StateRoots = append(c.StateRoots, c.StateRoots[0]) }, "duplicate state"},
		{"mask covers repo", func(c *Config) { c.Masks = []string{"/home/runner/work"} }, "covers"},
		{"mask is repo", func(c *Config) { c.Masks = []string{c.RepoRoot} }, "covers"},
		{"mask covers home", func(c *Config) { c.Masks = []string{"/home"} }, "covers"},
		{"mask is tmpdir", func(c *Config) { c.Masks = []string{c.Tmpdir} }, "covers"},
		{"mask covers state", func(c *Config) { c.Masks = []string{"/home/runner/.cargo"} }, "covers"},
		{"mask deep in repo", func(c *Config) { c.Masks = []string{c.RepoRoot + "/a/.git"} }, "direct child"},
		{"mask in state root", func(c *Config) { c.Masks = []string{c.StateRoots[0] + "/x"} }, "state root"},
		{"kiln in state root", func(c *Config) { c.KilnExe = c.StateRoots[0] + "/kiln" }, "kiln_exe"},
		{"kiln in tmpdir", func(c *Config) { c.KilnExe = c.Tmpdir + "/kiln" }, "kiln_exe"},
		{"repo in /run", func(c *Config) { c.RepoRoot, c.Masks = "/run/work", nil }, "replaces"},
		{"home in /dev", func(c *Config) { c.Home, c.Masks = "/dev/home", nil }, "replaces"},
		{"tmpdir in /var/run", func(c *Config) { c.Tmpdir = "/var/run/t" }, "replaces"},
		{"state in /run", func(c *Config) { c.StateRoots = []string{"/run/state"} }, "replaces"},
		{"dup mask", func(c *Config) { c.Masks = []string{"/a", "/a"} }, "duplicate mask"},
		{"too many masks", func(c *Config) {
			c.Masks = nil
			for i := 0; i <= maxMasks; i++ {
				c.Masks = append(c.Masks, "/m"+strings.Repeat("x", i%7)+string(rune('a'+i%26))+strings.Repeat("y", i/26))
			}
		}, "too many"},
		{"no actions", func(c *Config) { c.EnvByAction = map[string][]string{} }, "empty"},
		{"nil actions", func(c *Config) { c.EnvByAction = nil }, "empty"},
		{"absolute action", func(c *Config) { c.EnvByAction["/abs"] = nil }, "relative"},
		{"dotdot action", func(c *Config) { c.EnvByAction["../x"] = nil }, "relative"},
		{"bare dotdot", func(c *Config) { c.EnvByAction[".."] = nil }, "relative"},
		{"unclean action", func(c *Config) { c.EnvByAction["a//b"] = nil }, "relative"},
		{"empty action", func(c *Config) { c.EnvByAction[""] = nil }, "empty"},
		{"control action", func(c *Config) { c.EnvByAction["a\nb"] = nil }, "control"},
		{"env without equals", func(c *Config) { c.EnvByAction["x"] = []string{"NOPE"} }, "NAME="},
		{"env empty name", func(c *Config) { c.EnvByAction["x"] = []string{"=v"} }, "NAME="},
		{"env nul", func(c *Config) { c.EnvByAction["x"] = []string{"A=b\x00c"} }, "UTF-8"},
		{"env not utf8", func(c *Config) { c.EnvByAction["x"] = []string{"A=\xff"} }, "UTF-8"},
		{"env kiln var", func(c *Config) { c.EnvByAction["x"] = []string{"KILN_ACTION=x"} }, "Kiln variable"},
		{"env repeat", func(c *Config) { c.EnvByAction["x"] = []string{"A=1", "A=2"} }, "repeats"},
		{"env many", func(c *Config) {
			var e []string
			for i := 0; i <= maxEnvEntries; i++ {
				e = append(e, "V"+strings.Repeat("a", i/26)+string(rune('A'+i%26))+"=1")
			}
			c.EnvByAction["x"] = e
		}, "too many"},
	}
	for _, tc := range cases {
		c := goodConfig()
		tc.edit(c)
		err := c.Validate()
		if err == nil || !strings.Contains(err.Error(), tc.want) {
			t.Errorf("%s: %v, want an error containing %q", tc.name, err, tc.want)
		}
	}
	if err := goodConfig().Validate(); err != nil {
		t.Fatal(err)
	}
}

func TestValidateAllowsHomeAboveRepoAndNestedState(t *testing.T) {
	c := goodConfig() // the repo is under $HOME, as on hosted runners
	if !within(c.RepoRoot, c.Home) {
		t.Fatal("test config should put the repo under home")
	}
	// A state root under home, a mask inside the repo, a mask under home.
	if err := c.Validate(); err != nil {
		t.Fatal(err)
	}
	// Home under tmpdir is fine too (the tmpfs are mounted parent first).
	c.Tmpdir, c.Home = "/scratch", "/scratch/home"
	c.StateRoots = []string{"/var/state"}
	c.Masks = []string{c.RepoRoot + "/.git"}
	if err := c.Validate(); err != nil {
		t.Fatal(err)
	}
}

func TestEnvFor(t *testing.T) {
	c := goodConfig()
	got, err := c.EnvFor("code/packages/go/a")
	if err != nil || !reflect.DeepEqual(got, c.EnvByAction["code/packages/go/a"]) {
		t.Fatalf("%v %v", got, err)
	}
	got[0] = "MUTATED=1" // a copy: the caller cannot change the configuration
	if c.EnvByAction["code/packages/go/a"][0] == "MUTATED=1" {
		t.Error("EnvFor aliases the configuration")
	}
	for _, bad := range []string{"", "code/packages/go/b", "code/packages/go/a/", "/home/runner/work/repo/code/packages/go/a"} {
		if _, err := c.EnvFor(bad); err == nil || !strings.Contains(err.Error(), "KILN_ACTION") {
			t.Errorf("%q: %v", bad, err)
		}
	}
	// An action with an empty environment is valid and yields an empty one.
	c.EnvByAction["empty"] = []string{}
	if e, err := c.EnvFor("empty"); err != nil || len(e) != 0 {
		t.Errorf("%v %v", e, err)
	}
}

func TestClip(t *testing.T) {
	if clip("short") != "short" || len(clip(strings.Repeat("x", 100))) != 67 {
		t.Error("clip")
	}
}
