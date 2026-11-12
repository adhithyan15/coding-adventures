package host

import (
	"bytes"
	"errors"
	"os"
	"reflect"
	"strings"
	"testing"
)

func env(m map[string]string) Getenv { return func(k string) string { return m[k] } }

func TestTrustedTable(t *testing.T) {
	const on = "true"
	cases := []struct {
		name string
		env  map[string]string
		want bool
	}{
		{"push to main", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push"}, true},
		{"schedule on main", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "schedule"}, true},
		{"dispatch on main", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "workflow_dispatch"}, true},
		{"dispatch on feature", map[string]string{"GITHUB_REF": "refs/heads/feat", "GITHUB_EVENT_NAME": "workflow_dispatch"}, false},
		{"push to feature", map[string]string{"GITHUB_REF": "refs/heads/feat", "GITHUB_EVENT_NAME": "push"}, false},
		{"schedule off default", map[string]string{"GITHUB_REF": "refs/heads/feat", "GITHUB_EVENT_NAME": "schedule"}, false},
		{"pull_request", map[string]string{"GITHUB_REF": "refs/pull/7/merge", "GITHUB_EVENT_NAME": "pull_request"}, false},
		{"pull_request on main ref", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "pull_request"}, false},
		{"pull_request_target on main ref", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "pull_request_target"}, false},
		{"merge queue push", map[string]string{"GITHUB_REF": "refs/heads/gh-readonly-queue/main/pr-1-abc", "GITHUB_EVENT_NAME": "push"}, false},
		{"merge_group", map[string]string{"GITHUB_REF": "refs/heads/gh-readonly-queue/main/pr-1-abc", "GITHUB_EVENT_NAME": "merge_group"}, false},
		{"unknown event", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "issue_comment"}, false},
		{"missing event", map[string]string{"GITHUB_REF": "refs/heads/main"}, false},
		{"missing ref", map[string]string{"GITHUB_EVENT_NAME": "push"}, false},
		{"empty everything", map[string]string{"GITHUB_REF": "", "GITHUB_EVENT_NAME": ""}, false},
		{"tag push", map[string]string{"GITHUB_REF": "refs/tags/main", "GITHUB_EVENT_NAME": "push"}, false},
		{"bare branch name", map[string]string{"GITHUB_REF": "main", "GITHUB_EVENT_NAME": "push"}, false},
		{"case differs", map[string]string{"GITHUB_REF": "refs/heads/Main", "GITHUB_EVENT_NAME": "push"}, false},
		{"event case differs", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "Push"}, false},
		{"custom default branch", map[string]string{"GITHUB_REF": "refs/heads/trunk", "GITHUB_EVENT_NAME": "push", "KILN_DEFAULT_BRANCH": "trunk"}, true},
		{"custom default, main is not", map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push", "KILN_DEFAULT_BRANCH": "trunk"}, false},
		{"not actions", map[string]string{"GITHUB_ACTIONS": "false", "GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push"}, false},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			m := map[string]string{"GITHUB_ACTIONS": on}
			for k, v := range c.env {
				m[k] = v
			}
			if got := NewGitHub(env(m), nil).Trusted(); got != c.want {
				t.Fatalf("Trusted() = %v, want %v", got, c.want)
			}
		})
	}
	// Without GITHUB_ACTIONS at all: closed.
	if NewGitHub(env(map[string]string{"GITHUB_REF": "refs/heads/main", "GITHUB_EVENT_NAME": "push"}), nil).Trusted() {
		t.Fatal("trusted without GITHUB_ACTIONS")
	}
}

func TestDetect(t *testing.T) {
	if _, ok := Detect(env(map[string]string{"GITHUB_ACTIONS": "true"})).(*GitHub); !ok {
		t.Fatal("want GitHub")
	}
	for _, v := range []string{"", "false", "TRUE", "1"} {
		if _, ok := Detect(env(map[string]string{"GITHUB_ACTIONS": v})).(*Local); !ok {
			t.Fatalf("GITHUB_ACTIONS=%q: want Local", v)
		}
	}
}

func TestGitHubIdentityImageShardWork(t *testing.T) {
	g := NewGitHub(env(map[string]string{
		"GITHUB_SHA": "abc", "GITHUB_REF_NAME": "main", "GITHUB_EVENT_NAME": "push", "GITHUB_RUN_ID": "9",
		"ImageOS": "ubuntu24", "ImageVersion": "2025.1", "KILN_SHARD_INDEX": "2", "KILN_SHARD_COUNT": "4",
		"RUNNER_TEMP": "/rt",
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
	if !strings.HasPrefix(g.WorkDir(), "/rt") && !strings.HasPrefix(g.WorkDir(), `\rt`) {
		t.Fatalf("workdir %q", g.WorkDir())
	}

	pr := NewGitHub(env(map[string]string{"GITHUB_HEAD_REF": "feat", "GITHUB_REF_NAME": "7/merge"}), nil)
	if pr.Identity().Branch != "feat" {
		t.Fatal("PR branch should be head ref")
	}
	if !strings.HasSuffix(pr.ImageID(), "/unknown/unknown") {
		t.Fatalf("self-hosted image %q", pr.ImageID())
	}
	if !strings.HasPrefix(pr.WorkDir(), os.TempDir()) {
		t.Fatalf("fallback workdir %q", pr.WorkDir())
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

func TestReportEscaping(t *testing.T) {
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
	}
	for _, c := range cases {
		var b bytes.Buffer
		NewGitHub(nil, &b).Report(c.e)
		if b.String() != c.want {
			t.Errorf("got %q want %q", b.String(), c.want)
		}
		// Property: every emitted line is either a command we meant (starts
		// with ::) or contains no injected command at line start.
	}
	// An injected newline can never create a second command line.
	var b bytes.Buffer
	NewGitHub(nil, &b).Report(Event{Level: Error, Title: "t\n::x::", Message: "m\n::y::"})
	if strings.Count(b.String(), "\n") != 1 {
		t.Fatalf("injection produced extra lines: %q", b.String())
	}
}

func TestLocal(t *testing.T) {
	var b bytes.Buffer
	l := NewLocal(&b)
	if !l.Trusted() {
		t.Fatal("local is trusted")
	}
	if i, n := l.Shard(); i != 0 || n != 1 {
		t.Fatal("shard")
	}
	if id := l.Identity(); id.OS == "" || id.Arch == "" {
		t.Fatal("identity")
	}
	if !strings.HasPrefix(l.ImageID(), "local/") {
		t.Fatal("image")
	}
	l.Report(Event{Level: GroupStart, Message: "g"})
	l.Report(Event{Level: Error, Title: "T", Message: "boom"})
	l.Report(Event{Level: Info, Message: "hi"})
	l.Report(Event{Level: GroupEnd})
	if b.String() != "== g ==\nerror: T: boom\nhi\n" {
		t.Fatalf("got %q", b.String())
	}
	d := l.WorkDir()
	if d == "" || d != l.WorkDir() {
		t.Fatal("workdir not stable")
	}
	defer os.Remove(d)
	if st, err := os.Stat(d); err != nil || !st.IsDir() {
		t.Fatalf("workdir missing: %v", err)
	}

	f := NewLocal(&b)
	f.mkdirTemp = func(string, string) (string, error) { return "", errors.New("no") }
	if !strings.HasSuffix(f.WorkDir(), "kiln") {
		t.Fatalf("fallback %q", f.WorkDir())
	}
}

func TestDetectLocalAndGitHubHaveOutput(t *testing.T) {
	// Detect wires os.Stdout; just make sure Report does not panic.
	Detect(env(nil)).Report(Event{Level: Info, Message: ""})
}

func TestScrubSecrets(t *testing.T) {
	in := []string{
		"PATH=/bin", "GITHUB_TOKEN=x", "ACTIONS_RUNTIME_TOKEN=x", "ACTIONS_ID_TOKEN_REQUEST_TOKEN=x",
		"ACTIONS_ID_TOKEN_REQUEST_URL=u", "NPM_TOKEN=x", "my_secret=x", "DB_PASSWORD=x", "INPUT_KEY=x",
		"input_lower=x", "github_token=x", "HOME=/h", "TOKEN_NOT=keep", "=C:=C:\\dir", "", "NOEQ", "SECRET=short",
	}
	want := []string{"PATH=/bin", "HOME=/h", "TOKEN_NOT=keep", "=C:=C:\\dir", "", "NOEQ", "SECRET=short"}
	got := ScrubSecrets(in)
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v", got)
	}
	if len(ScrubSecrets(nil)) != 0 {
		t.Fatal("nil")
	}
}
