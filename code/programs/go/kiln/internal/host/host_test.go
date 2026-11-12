package host

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"strings"
	"testing"
)

func env(m map[string]string) Getenv { return func(k string) string { return m[k] } }

// eventFile writes a GitHub-style event payload and returns its path.
func eventFile(t *testing.T, defaultBranch string) string {
	t.Helper()
	p := filepath.Join(t.TempDir(), "event.json")
	body := `{"repository":{"default_branch":"` + defaultBranch + `"},"other":1}`
	if err := os.WriteFile(p, []byte(body), 0o600); err != nil {
		t.Fatal(err)
	}
	return p
}

func TestTrustedTable(t *testing.T) {
	cases := []struct {
		name string
		env  map[string]string
		def  string // default branch in the event file ("" = no file)
		want bool
	}{
		{"push to main", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push"}, "main", true},
		{"schedule on main", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "schedule"}, "main", true},
		{"dispatch on main", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "workflow_dispatch"}, "main", true},
		{"dispatch on feature", map[string]string{"GITHUB_REF": "refs/heads/feat", "GITHUB_EVENT_NAME": "workflow_dispatch"}, "main", false},
		{"push to feature", map[string]string{"GITHUB_REF": "refs/heads/feat", "GITHUB_EVENT_NAME": "push"}, "main", false},
		{"schedule off default", map[string]string{"GITHUB_REF": "refs/heads/feat", "GITHUB_EVENT_NAME": "schedule"}, "main", false},
		{"pull_request", map[string]string{"GITHUB_REF": "refs/pull/7/merge", "GITHUB_EVENT_NAME": "pull_request"}, "main", false},
		{"pull_request on main ref", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "pull_request"}, "main", false},
		{"pull_request_target on main ref", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "pull_request_target"}, "main", false},
		{"merge queue push", map[string]string{"GITHUB_REF": "refs/heads/gh-readonly-queue/main/pr-1-abc", "GITHUB_EVENT_NAME": "push"}, "main", false},
		{"merge_group", map[string]string{"GITHUB_REF": "refs/heads/gh-readonly-queue/main/pr-1-abc", "GITHUB_EVENT_NAME": "merge_group"}, "main", false},
		{"unknown event", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "issue_comment"}, "main", false},
		{"missing event", map[string]string{"GITHUB_REF": "refs/heads/main"}, "main", false},
		{"missing ref", map[string]string{"GITHUB_EVENT_NAME": "push"}, "main", false},
		{"empty everything", map[string]string{"GITHUB_REF": "", "GITHUB_EVENT_NAME": ""}, "main", false},
		{"tag push named main", map[string]string{"GITHUB_REF": "refs/tags/main", "GITHUB_EVENT_NAME": "push"}, "main", false},
		{"bare branch name", map[string]string{"GITHUB_REF": "main", "GITHUB_EVENT_NAME": "push"}, "main", false},
		{"case differs", map[string]string{"GITHUB_REF": "refs/heads/Main", "GITHUB_EVENT_NAME": "push"}, "main", false},
		{"event case differs", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "Push"}, "main", false},
		{"event default is trunk", map[string]string{"GITHUB_REF": "refs/heads/trunk", "GITHUB_EVENT_NAME": "push"}, "trunk", true},
		{"event default trunk, main is not", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push"}, "trunk", false},
		{"forged KILN_DEFAULT_BRANCH on feature push", map[string]string{"GITHUB_REF": "refs/heads/feat", "GITHUB_EVENT_NAME": "push", "KILN_DEFAULT_BRANCH": "feat"}, "main", false},
		{"KILN_DEFAULT_BRANCH narrows: mismatch", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push", "KILN_DEFAULT_BRANCH": "other"}, "main", false},
		{"KILN_DEFAULT_BRANCH agrees", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push", "KILN_DEFAULT_BRANCH": "main"}, "main", true},
		{"KILN_DEFAULT_BRANCH alone is not enough", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push", "KILN_DEFAULT_BRANCH": "main"}, "", false},
		{"no event file", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push"}, "", false},
		{"not actions", map[string]string{"GITHUB_ACTIONS": "false", "GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push"}, "main", false},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			m := map[string]string{"GITHUB_ACTIONS": "true"}
			if c.def != "" {
				m["GITHUB_EVENT_PATH"] = eventFile(t, c.def)
			}
			for k, v := range c.env {
				m[k] = v
			}
			if got := NewGitHub(env(m), nil).Trusted(); got != c.want {
				t.Fatalf("Trusted() = %v, want %v", got, c.want)
			}
		})
	}
	// Without GITHUB_ACTIONS at all: closed.
	if NewGitHub(env(map[string]string{"GITHUB_EVENT_PATH": eventFile(t, "main"), "GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push"}), nil).Trusted() {
		t.Fatal("trusted without GITHUB_ACTIONS")
	}
}

func TestTrustedBadEventFiles(t *testing.T) {
	dir := t.TempDir()
	write := func(name, body string) string {
		p := filepath.Join(dir, name)
		if err := os.WriteFile(p, []byte(body), 0o600); err != nil {
			t.Fatal(err)
		}
		return p
	}
	big := make([]byte, maxEventBytes+10)
	for i := range big {
		big[i] = ' '
	}
	paths := map[string]string{
		"garbage":   write("g.json", "not json"),
		"empty":     write("e.json", ""),
		"nofield":   write("n.json", `{"repository":{}}`),
		"norepo":    write("r.json", `{}`),
		"wrongtype": write("w.json", `{"repository":{"default_branch":5}}`),
		"oversized": write("o.json", string(big)),
		"directory": dir,
		"missing":   filepath.Join(dir, "nope.json"),
	}
	for name, p := range paths {
		g := NewGitHub(env(map[string]string{"GITHUB_ACTIONS": "true", "GITHUB_EVENT_PATH": p, "GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push"}), nil)
		if g.Trusted() {
			t.Errorf("%s: trusted", name)
		}
	}
}

func TestDetect(t *testing.T) {
	type K = map[string]string
	cases := []struct {
		name string
		env  K
		want string
	}{
		{"github", K{"GITHUB_ACTIONS": "true", "CI": "true", "GITHUB_SHA": "x"}, "github"},
		{"clean", K{}, "local"},
		{"unrelated vars", K{"HOME": "/h"}, "local"},
		{"CI only", K{"CI": "true"}, "untrusted"},
		{"CI any value", K{"CI": "1"}, "untrusted"},
		{"gitlab", K{"GITLAB_CI": "true"}, "untrusted"},
		{"actions false", K{"GITHUB_ACTIONS": "false"}, "untrusted"},
		{"actions TRUE case", K{"GITHUB_ACTIONS": "TRUE"}, "untrusted"},
		{"actions 1", K{"GITHUB_ACTIONS": "1", "GITHUB_SHA": "x"}, "untrusted"},
		{"GITHUB_ star only", K{"GITHUB_SHA": "x"}, "untrusted"},
		{"runner star only", K{"RUNNER_OS": "Linux"}, "untrusted"},
		{"lowercase github_", K{"github_run_id": "1"}, "untrusted"},
	}
	for _, c := range cases {
		var environ []string
		for k, v := range c.env {
			environ = append(environ, k+"="+v)
		}
		h := Detect(env(c.env), environ)
		got := ""
		switch h.(type) {
		case *GitHub:
			got = "github"
		case *Local:
			got = "local"
		case *Untrusted:
			got = "untrusted"
		}
		if got != c.want {
			t.Errorf("%s: got %s want %s", c.name, got, c.want)
		}
	}
}

func TestHostTrustAndLedgerFlags(t *testing.T) {
	l, u, g := NewLocal(nil), NewUntrusted(nil), NewGitHub(env(nil), nil)
	if l.Trusted() || u.Trusted() || g.Trusted() {
		t.Fatal("none of these may be trusted")
	}
	if !l.WritesLocalLedger() || u.WritesLocalLedger() || g.WritesLocalLedger() {
		t.Fatal("only Local writes the local ledger")
	}
	for _, h := range []Host{l, u, g} {
		if i, n := h.Shard(); i != 0 || n != 1 {
			t.Fatal("shard")
		}
	}
	if id := u.Identity(); id.OS == "" || id.Arch == "" {
		t.Fatal("identity")
	}
	if !strings.HasPrefix(u.ImageID(), "untrusted/") {
		t.Fatal("image")
	}
	d, err := u.WorkDir()
	if err != nil {
		t.Fatal(err)
	}
	os.Remove(d)
}

func TestGitHubIdentityImageShardWork(t *testing.T) {
	rt := t.TempDir()
	g := NewGitHub(env(map[string]string{
		"GITHUB_SHA": "abc", "GITHUB_REF_NAME": "main", "GITHUB_EVENT_NAME": "push", "GITHUB_RUN_ID": "9",
		"ImageOS": "ubuntu24", "ImageVersion": "2025.1", "KILN_SHARD_INDEX": "2", "KILN_SHARD_COUNT": "4",
		"RUNNER_TEMP": rt,
	}), nil)
	id := g.Identity()
	if id.Revision != "abc" || id.Branch != "main" || id.Event != "push" || id.RunID != "9" || id.OS == "" || id.Arch == "" {
		t.Fatalf("identity %+v", id)
	}
	if !strings.HasSuffix(g.ImageID(), "/ubuntu24/2025.1") || !strings.HasPrefix(g.ImageID(), "github/") {
		t.Fatalf("image %q", g.ImageID())
	}
	if i, n := g.Shard(); i != 2 || n != 4 {
		t.Fatalf("shard %d/%d", i, n)
	}
	d, err := g.WorkDir()
	if err != nil || !strings.HasPrefix(d, rt) {
		t.Fatalf("workdir %q %v", d, err)
	}

	pr := NewGitHub(env(map[string]string{"GITHUB_HEAD_REF": "feat", "GITHUB_REF_NAME": "7/merge"}), nil)
	if pr.Identity().Branch != "feat" {
		t.Fatal("PR branch should be head ref")
	}
	if !strings.HasSuffix(pr.ImageID(), "/unknown/unknown") {
		t.Fatalf("self-hosted image %q", pr.ImageID())
	}
	d2, err := pr.WorkDir()
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(d2)
	if !strings.HasPrefix(d2, os.TempDir()) {
		t.Fatalf("fallback workdir %q", d2)
	}

	evil := NewGitHub(env(map[string]string{"ImageOS": "ubuntu\n::error::x/../", "ImageVersion": "é 1"}), nil)
	if strings.ContainsAny(evil.ImageID()[len("github/"):], "\n:é ") || strings.Count(evil.ImageID(), "/") != 4 {
		t.Fatalf("unsanitised image %q", evil.ImageID())
	}
}

func TestWorkDirNoPredictableFallback(t *testing.T) {
	boom := errors.New("no")
	g := NewGitHub(env(nil), nil)
	g.mkdirTemp = func(string, string) (string, error) { return "", boom }
	if d, err := g.WorkDir(); err == nil || d != "" {
		t.Fatalf("want error, got %q %v", d, err)
	}
	l := NewLocal(nil)
	l.mkdirTemp = func(string, string) (string, error) { return "", boom }
	if _, err := l.WorkDir(); err == nil {
		t.Fatal("want error")
	}
}

func TestShardFallbacks(t *testing.T) {
	for _, c := range [][2]string{{"", ""}, {"x", "2"}, {"0", "0"}, {"-1", "2"}, {"2", "2"}, {"0", "y"}} {
		g := NewGitHub(env(map[string]string{"KILN_SHARD_INDEX": c[0], "KILN_SHARD_COUNT": c[1]}), nil)
		if i, n := g.Shard(); i != 0 || n != 1 {
			t.Fatalf("%v -> %d/%d", c, i, n)
		}
	}
}

func TestReportGitHubEscaping(t *testing.T) {
	cases := []struct {
		e    Event
		want string
	}{
		{Event{Level: Error, Message: "bad 100%\r\n::error::pwned"}, "::error::bad 100%25%0D%0A::error::pwned\n"},
		{Event{Level: Warning, Title: "a:b,c%", Message: "m"}, "::warning title=a%3Ab%2Cc%25::m\n"},
		{Event{Level: Notice, Message: "n"}, "::notice::n\n"},
		{Event{Level: GroupStart, Message: "g\n::stop-commands::"}, "::group::g%0A::stop-commands::\n"},
		{Event{Level: GroupEnd}, "::endgroup::\n"},
		{Event{Level: Info, Message: "ok\r\n::error::x\r  ##[error]y\nfine"}, "ok\nkiln: ::error::x\nkiln:   ##[error]y\nfine\n"},
		// Control characters are stripped everywhere.
		{Event{Level: Error, Message: "a\vb\fc\x1b[31md\u0085e\u009bf\x07g\x00h i\tj"}, "::error::abc[31mdefghi\tj\n"},
		{Event{Level: Error, Title: "t\x1b\v", Message: "m"}, "::error title=t::m\n"},
		// Defanging is unconditional: "::" anywhere in an info line.
		{Event{Level: Info, Message: "x ::error::y"}, "kiln: x ::error::y\n"},
		{Event{Level: Info, Message: "\v\f\u0085::error::z"}, "kiln: ::error::z\n"},
		{Event{Level: Info, Message: "  ::error::z"}, "kiln:   ::error::z\n"},
		{Event{Level: Info, Message: "a ##[b"}, "kiln: a ##[b\n"},
		{Event{Level: Info, Message: "plain"}, "plain\n"},
	}
	for _, c := range cases {
		var b bytes.Buffer
		NewGitHub(nil, &b).Report(c.e)
		if b.String() != c.want {
			t.Errorf("got %q want %q", b.String(), c.want)
		}
	}
	var b bytes.Buffer
	NewGitHub(nil, &b).Report(Event{Level: Error, Title: "t\n::x::", Message: "m\n::y::"})
	if strings.Count(b.String(), "\n") != 1 {
		t.Fatalf("injection produced extra lines: %q", b.String())
	}
}

func TestReportPlainSanitises(t *testing.T) {
	var b bytes.Buffer
	l := NewLocal(&b)
	l.Report(Event{Level: GroupStart, Message: "g"})
	l.Report(Event{Level: Error, Title: "T\x1b", Message: "boom\x1b[2J\u0085\vx"})
	l.Report(Event{Level: Info, Message: "hi\r\n::error::bad"})
	l.Report(Event{Level: GroupStart, Message: "::g\n2"})
	l.Report(Event{Level: GroupEnd})
	got := b.String()
	if strings.ContainsAny(got, "\x1b\v\u0085") {
		t.Fatalf("control characters survived: %q", got)
	}
	for _, line := range strings.Split(strings.TrimRight(got, "\n"), "\n") {
		if strings.Contains(line, "::") && !strings.HasPrefix(line, "kiln: ") {
			t.Fatalf("undefanged line %q", line)
		}
	}
	if !strings.Contains(got, "== g ==\nerror: T: boom[2Jx\nhi\n") {
		t.Fatalf("got %q", got)
	}
}

func TestLocal(t *testing.T) {
	l := NewLocal(&bytes.Buffer{})
	if i, n := l.Shard(); i != 0 || n != 1 {
		t.Fatal("shard")
	}
	if id := l.Identity(); id.OS == "" || id.Arch == "" {
		t.Fatal("identity")
	}
	if !strings.HasPrefix(l.ImageID(), "local/") {
		t.Fatal("image")
	}
	d, err := l.WorkDir()
	if err != nil {
		t.Fatal(err)
	}
	d2, _ := l.WorkDir()
	if d == "" || d != d2 {
		t.Fatal("workdir not stable")
	}
	defer os.Remove(d)
	if st, err := os.Stat(d); err != nil || !st.IsDir() {
		t.Fatalf("workdir missing: %v", err)
	}
	if runtime.GOOS != "windows" {
		if st, _ := os.Stat(d); st.Mode().Perm() != 0o700 {
			t.Fatalf("mode %v", st.Mode().Perm())
		}
	}
}

func TestScrubSecretsDelegates(t *testing.T) {
	got := ScrubSecrets([]string{"A=1", "GITHUB_TOKEN=x"})
	if !reflect.DeepEqual(got, []string{"A=1"}) {
		t.Fatal(got)
	}
}

func TestUntrustedReport(t *testing.T) {
	var b bytes.Buffer
	NewUntrusted(&b).Report(Event{Level: Warning, Message: "w\x1b"})
	if b.String() != "warning: w\n" {
		t.Fatalf("got %q", b.String())
	}
}
