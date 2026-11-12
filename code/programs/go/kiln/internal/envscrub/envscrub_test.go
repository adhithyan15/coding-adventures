package envscrub

import (
	"reflect"
	"testing"
)

func TestScrubEachRule(t *testing.T) {
	removed := []string{
		"GITHUB_TOKEN", "ACTIONS_RUNTIME_TOKEN", "ACTIONS_ID_TOKEN_REQUEST_TOKEN", "ACTIONS_ID_TOKEN_REQUEST_URL",
		"ACTIONS_CACHE_URL", "GITHUB_ENV", "GITHUB_PATH", "GITHUB_OUTPUT", "GITHUB_STATE", "GITHUB_STEP_SUMMARY",
		"SSH_AUTH_SOCK", "GOOGLE_APPLICATION_CREDENTIALS", "DATABASE_URL", "NPM_CONFIG__AUTH",
		"NPM_TOKEN", "my_secret", "DB_PASSWORD", "DB_PASS", "DB_PASSWD", "GH_PAT", "API_KEY", "AWS_CREDENTIALS",
		"AWS_SECRET_ACCESS_KEY", "INPUT_KEY", "input_lower", "github_token",
	}
	var in, want []string
	for _, n := range removed {
		in = append(in, n+"=v")
	}
	keep := []string{"PATH=/bin", "HOME=/h", "TOKEN_NOT=keep", "=C:=C:\\dir", "", "NOEQ", "SECRET=short", "GITHUB_SHA=abc", "KEYBOARD=us"}
	for _, k := range keep {
		in = append(in, k)
		want = append(want, k)
	}
	if got := Scrub(in); !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v\nwant %v", got, want)
	}
	if len(Scrub(nil)) != 0 {
		t.Fatal("nil")
	}
}
