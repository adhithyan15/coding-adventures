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
	want := "CARGO_HOME=/h CARGO_TARGET_DIR=/t=x CI=true TZ="
	if strings.Join(got, " ") != want {
		t.Fatalf("got %q want %q", strings.Join(got, " "), want)
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
