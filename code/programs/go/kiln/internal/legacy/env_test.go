package legacy

import (
	"strings"
	"testing"
)

func mustEnv(t *testing.T, src string) *EnvPolicy {
	t.Helper()
	p, err := ParseEnvPolicy([]byte(src))
	if err != nil {
		t.Fatalf("ParseEnvPolicy: %v", err)
	}
	return p
}

func TestParseEnvPolicyRejects(t *testing.T) {
	cases := map[string]string{
		"unknown field":   `{"names": [], "x": 1}`,
		"trailing":        `{"names": []} {}`,
		"unsorted":        `{"names": ["B", "A"]}`,
		"duplicate":       `{"names": ["A", "A"]}`,
		"colon":           `{"names": ["kiln:root"]}`,
		"colon prefix":    `{"names": ["KILN:*"]}`,
		"bare star":       `{"names": ["*"]}`,
		"empty":           `{"names": [""]}`,
		"lowercase dash":  `{"names": ["a-b"]}`,
		"digit first":     `{"names": ["1A"]}`,
		"space":           `{"names": ["A B"]}`,
		"mid star":        `{"names": ["A*B"]}`,
		"secret token":    `{"names": ["NPM_TOKEN"]}`,
		"secret prefix":   `{"names": ["AUTH*"]}`,
		"secret key":      `{"names": ["API_KEY"]}`,
		"secret pat":      `{"names": ["GH_PAT"]}`,
		"secret lower":    `{"names": ["Secret_Thing"]}`,
		"secret password": `{"names": ["DB_PASSWORD"]}`,
		"scrubber list":   `{"names": ["DATABASE_URL"]}`,
		"too long":        `{"names": ["` + strings.Repeat("A", 129) + `"]}`,
	}
	for name, src := range cases {
		if _, err := ParseEnvPolicy([]byte(src)); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
}

func TestEnvFor(t *testing.T) {
	p := mustEnv(t, `{"names": ["CARGO_*", "CI", "LANG", "TZ"]}`)
	vars, err := p.EnvFor([]string{
		"PATH=/bin", "TZ=", "CARGO_HOME=/h", "CI=true", "CARGO_TARGET_DIR=/t=x",
		"CI=ignored-second", "HOME=/h", "=C:=C:\\dir", "", "NOEQ",
	})
	if err != nil {
		t.Fatal(err)
	}
	var got []string
	for _, v := range vars {
		got = append(got, v.Name+"="+v.Value)
		if v.Origin != 0 {
			t.Fatalf("origin of %s = %d", v.Name, v.Origin)
		}
	}
	want := "CARGO_HOME=/h CARGO_TARGET_DIR=/t=x CI=ignored-second TZ="
	if strings.Join(got, " ") != want {
		t.Fatalf("got %q want %q", strings.Join(got, " "), want)
	}
	// Duplicate names: the LAST entry wins, as in os/exec.
	for _, v := range vars {
		if v.Name == "CI" && v.Value != "ignored-second" {
			t.Fatalf("CI = %q, want the last value", v.Value)
		}
	}
	// An empty value stays distinct from unset: LANG is not there at all.
	for _, v := range vars {
		if v.Name == "LANG" {
			t.Fatal("an unset variable was included")
		}
	}
	// No match at all is an empty (not nil-hostile) section.
	if vars, err := p.EnvFor(nil); err != nil || len(vars) != 0 {
		t.Fatalf("%v %v", vars, err)
	}
	// The secret-like rule applies AFTER wildcard expansion: an error, not a skip.
	_, err = p.EnvFor([]string{"CARGO_REGISTRY_TOKEN=hunter2"})
	if err == nil || !strings.Contains(err.Error(), "CARGO_REGISTRY_TOKEN") {
		t.Fatalf("secret-like expansion: %v", err)
	}
	if err != nil && strings.Contains(err.Error(), "hunter2") {
		t.Fatal("the error leaks the value")
	}
	for _, e := range []string{"CARGO_AUTH_X=1", "CARGO_SIGNING_KEY=1", "CARGO_X_PAT=1", "CARGO_private=1"} {
		if _, err := p.EnvFor([]string{e}); err == nil {
			t.Errorf("%s accepted", e)
		}
	}
	// A value that is not valid UTF-8 cannot be keyed.
	if _, err := p.EnvFor([]string{"CI=\xff"}); err == nil {
		t.Fatal("invalid UTF-8 accepted")
	}
}

func TestEnvNamesAndSecretLike(t *testing.T) {
	p := mustEnv(t, `{"names": ["A*", "B", "C"]}`)
	if got := strings.Join(p.Names(), ","); got != "A*,B,C" {
		t.Fatalf("Names = %s", got)
	}
	if p.Digest == ([32]byte{}) {
		t.Fatal("digest not set")
	}
	for n, want := range map[string]bool{
		"GITHUB_TOKEN": true, "my_secret": true, "GIT_AUTHOR_NAME": true, "X_PRIVATE_Y": true,
		"SIGNING_KEY": true, "KEYBOARD": false, "RUSTFLAGS": false, "CARGO_HOME": false,
		"PASSWD": true, "AWS_CREDENTIALS": true, "ACTIONS_X": true,
	} {
		if secretLike(n) != want {
			t.Errorf("secretLike(%q) = %v", n, !want)
		}
	}
}

func TestEnvSecretMarkersAndValues(t *testing.T) {
	for _, n := range []string{"GPG_PASSPHRASE", "MY_COOKIE", "SESSION_ID", "STRIPEAPIKEY", "CODE_SIGNING_X", "AWS_ACCESS_KEY_ID"} {
		if !secretLike(n) {
			t.Errorf("%s not secret-like", n)
		}
		if _, err := ParseEnvPolicy([]byte(`{"names": ["` + n + `"]}`)); err == nil {
			t.Errorf("%s accepted in env.json", n)
		}
	}
	p := mustEnv(t, `{"names": ["CI", "FLAGS"]}`)
	for _, v := range []string{
		"https://user:pw@example.com/x", "ghp_abcdef", "gho_abc", "github_pat_11AA", "AKIAIOSFODNN7EXAMPLE",
		"xoxb-1-2", "xoxp-1", "-Ddb.password=hunter2 -Dx=1", "-Dpassword=x",
	} {
		_, err := p.EnvFor([]string{"FLAGS=" + v})
		if err == nil || !strings.Contains(err.Error(), "FLAGS") || strings.Contains(err.Error(), v) {
			t.Errorf("value %q: %v", v, err)
		}
	}
	for _, v := range []string{"https://example.com/x", "a@b", "-Dfoo=bar", "xoxo", "true"} {
		if _, err := p.EnvFor([]string{"FLAGS=" + v}); err != nil {
			t.Errorf("harmless value %q refused: %v", v, err)
		}
	}
}

func TestEnvCaseFoldOnWindows(t *testing.T) {
	old := envCaseFold
	defer func() { envCaseFold = old }()
	envCaseFold = true
	p := mustEnv(t, `{"names": ["CARGO_*", "CI"]}`)
	vars, err := p.EnvFor([]string{"Ci=1", "cargo_home=/h", "CI=2"})
	if err != nil || len(vars) != 2 || vars[1].Name != "cargo_home" && vars[0].Name != "cargo_home" {
		t.Fatalf("%v %v", vars, err)
	}
	for _, v := range vars {
		if v.Name == "CI" && v.Value != "2" {
			t.Fatalf("last-wins across case: %v", vars)
		}
	}
	envCaseFold = false
	if vars, _ := p.EnvFor([]string{"Ci=1"}); len(vars) != 0 {
		t.Fatal("case-folded match off Windows")
	}
}
