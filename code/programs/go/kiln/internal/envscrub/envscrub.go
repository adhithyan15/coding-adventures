// Package envscrub removes credentials from an environment before Kiln
// starts a child process (KILN00 section 9.6 item 3).  It lives in its own
// tiny package so both the Host and Source adapters can use it without
// depending on each other.
package envscrub

import "strings"

// What is removed (names compare case-insensitively, because Windows
// environment names do):
//
//	exact:    GITHUB_TOKEN, GITHUB_ENV, GITHUB_PATH, GITHUB_OUTPUT,
//	          GITHUB_STATE, GITHUB_STEP_SUMMARY, SSH_AUTH_SOCK,
//	          GOOGLE_APPLICATION_CREDENTIALS, DATABASE_URL, NPM_CONFIG__AUTH
//	prefix:   INPUT_ (action inputs may carry secrets), ACTIONS_ (runtime and
//	          OIDC tokens and URLs)
//	suffix:   _TOKEN, _SECRET, _PASSWORD, _PASS, _PASSWD, _PAT, _KEY,
//	          _CREDENTIALS, _SECRET_ACCESS_KEY
//
// GITHUB_ENV / GITHUB_PATH / GITHUB_OUTPUT / GITHUB_STATE /
// GITHUB_STEP_SUMMARY name files a child could append to in order to poison
// LATER workflow steps, so they are removed even though they are not secrets.
//
// Over-removal only costs a child a variable it should not have had; keeping
// one costs a leaked credential.
var (
	names = map[string]bool{
		"GITHUB_TOKEN": true, "GITHUB_ENV": true, "GITHUB_PATH": true, "GITHUB_OUTPUT": true,
		"GITHUB_STATE": true, "GITHUB_STEP_SUMMARY": true, "SSH_AUTH_SOCK": true,
		"GOOGLE_APPLICATION_CREDENTIALS": true, "DATABASE_URL": true, "NPM_CONFIG__AUTH": true,
	}
	prefixes = []string{"INPUT_", "ACTIONS_"}
	suffixes = []string{"_TOKEN", "_SECRET", "_PASSWORD", "_PASS", "_PASSWD", "_PAT", "_KEY", "_CREDENTIALS", "_SECRET_ACCESS_KEY"}
)

// Scrub returns env ("NAME=value" entries) without credential-like entries,
// preserving order.
func Scrub(env []string) []string {
	out := make([]string, 0, len(env))
	for _, e := range env {
		if !IsSecretName(EntryName(e)) {
			out = append(out, e)
		}
	}
	return out
}

// EntryName returns the variable name of a "NAME=value" entry.  The search
// for "=" starts at index 1 so Windows' "=C:=C:\dir" pseudo-entries keep
// their name "=C:".
func EntryName(e string) string {
	if len(e) == 0 {
		return e
	}
	if i := strings.Index(e[1:], "="); i >= 0 {
		return e[:i+1]
	}
	return e
}

// IsSecretName reports whether a variable name is removed by Scrub.
func IsSecretName(name string) bool {
	n := strings.ToUpper(name)
	if names[n] {
		return true
	}
	for _, p := range prefixes {
		if strings.HasPrefix(n, p) {
			return true
		}
	}
	for _, s := range suffixes {
		if strings.HasSuffix(n, s) {
			return true
		}
	}
	return false
}
