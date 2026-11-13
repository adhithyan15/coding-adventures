package legacy

import (
	"strings"
	"testing"
)

const hrToolchains = `{
  "go":   {"pins": {"go": "1.26.1"}, "roots": {"go": ["/opt/hostedtoolcache/go/1.26.1/x64/bin"]}},
  "rust": {"pins": {"cargo": "1.99.1"}}
}`

func TestParseHostRootsAccepts(t *testing.T) {
	tc := mustToolchains(t, hrToolchains)
	h, err := ParseHostRoots([]byte(`{
	  "roots": [
	    {"path": "$HOME/.cargo/registry", "kind": "state", "ecosystem": "rust", "fetchers": ["$HOME/.cargo/bin/cargo", "/x/cargo"]},
	    {"path": "/etc", "kind": "image"},
	    {"path": "/opt/hostedtoolcache/go/1.26.1", "kind": "tool", "tool": "go"},
	    {"path": "/proc/x", "kind": "ephemeral"},
	    {"path": "/usr", "kind": "image"}
	  ],
	  "ephemeral": ["/dev/fd/*", "/dev/null", "/sys/fs/cgroup/**"],
	  "absent-ok": ["$HOME/.config", "$TMPDIR/x"]
	}`), tc)
	if err != nil {
		t.Fatal(err)
	}
	if len(h.Roots) != 5 || h.Digest == ([32]byte{}) {
		t.Fatalf("%+v", h)
	}
}

func TestParseHostRootsRejects(t *testing.T) {
	tc := mustToolchains(t, hrToolchains)
	goRoot := `{"path": "/opt/hostedtoolcache/go/1.26.1", "kind": "tool", "tool": "go"}`
	wrap := func(roots string) string { return `{"roots": [` + roots + `], "ephemeral": [], "absent-ok": []}` }
	cases := map[string]string{
		"not json":              `x`,
		"unknown field":         `{"roots": [], "ephemeral": [], "absent-ok": [], "x": 1}`,
		"unknown root field":    wrap(`{"path": "/a", "kind": "image", "x": 1}`),
		"unsorted roots":        wrap(`{"path": "/b", "kind": "image"}, {"path": "/a", "kind": "image"}`),
		"duplicate root":        wrap(`{"path": "/a", "kind": "image"}, {"path": "/a", "kind": "image"}`),
		"relative":              wrap(`{"path": "a", "kind": "image"}`),
		"dotdot":                wrap(`{"path": "/a/../b", "kind": "image"}`),
		"too broad slash":       wrap(`{"path": "/", "kind": "image"}`),
		"too broad home":        wrap(`{"path": "$HOME", "kind": "image"}`),
		"too broad tmp":         wrap(`{"path": "$TMPDIR", "kind": "image"}`),
		"glob in root":          wrap(`{"path": "/a/*", "kind": "image"}`),
		"unknown kind":          wrap(`{"path": "/a", "kind": "other"}`),
		"image with tool":       wrap(`{"path": "/a", "kind": "image", "tool": "go"}`),
		"image with ecosystem":  wrap(`{"path": "/a", "kind": "image", "ecosystem": "rust"}`),
		"ephemeral fetchers":    wrap(`{"path": "/a", "kind": "ephemeral", "fetchers": ["/x"]}`),
		"tool without tool":     wrap(`{"path": "/a", "kind": "tool"}`),
		"tool unknown tool":     wrap(`{"path": "/a", "kind": "tool", "tool": "nope"}`),
		"tool with ecosystem":   wrap(`{"path": "/a", "kind": "tool", "tool": "go", "ecosystem": "x"}`),
		"tool with fetchers":    wrap(`{"path": "/a", "kind": "tool", "tool": "go", "fetchers": ["/x"]}`),
		"state without eco":     wrap(`{"path": "/a", "kind": "state"}`),
		"state with tool":       wrap(`{"path": "/a", "kind": "state", "ecosystem": "r", "tool": "go"}`),
		"state bad ecosystem":   wrap(`{"path": "/a", "kind": "state", "ecosystem": "a:b"}`),
		"fetchers unsorted":     wrap(`{"path": "/a", "kind": "state", "ecosystem": "r", "fetchers": ["/z", "/b"]}`),
		"fetcher relative":      wrap(`{"path": "/a", "kind": "state", "ecosystem": "r", "fetchers": ["z"]}`),
		"ephemeral unsorted":    `{"roots": [], "ephemeral": ["/b", "/a"], "absent-ok": []}`,
		"ephemeral relative":    `{"roots": [], "ephemeral": ["a"], "absent-ok": []}`,
		"ephemeral dotdot":      `{"roots": [], "ephemeral": ["/a/../b"], "absent-ok": []}`,
		"absent-ok unsorted":    `{"roots": [], "ephemeral": [], "absent-ok": ["/b", "/a"]}`,
		"absent-ok glob":        `{"roots": [], "ephemeral": [], "absent-ok": ["/a/*"]}`,
		"toolcache uncovered":   wrap(`{"path": "/etc", "kind": "image"}`),
		"toolcache wrong tool":  wrap(`{"path": "/opt/hostedtoolcache/go/1.26.1", "kind": "tool", "tool": "cargo"}`),
		"toolcache wrong depth": wrap(`{"path": "/opt/hostedtoolcache/go/1.26.1/x64/bin/sub", "kind": "tool", "tool": "go"}`),
	}
	for name, src := range cases {
		if _, err := ParseHostRoots([]byte(src), tc); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	// The covering root makes the same file valid, and a prefix match is
	// path-boundary aware: /opt/hostedtoolcache/go/1.26.10 does not cover 1.26.1.
	if _, err := ParseHostRoots([]byte(wrap(goRoot)), tc); err != nil {
		t.Fatal(err)
	}
	near := wrap(`{"path": "/opt/hostedtoolcache/go/1.26.10", "kind": "tool", "tool": "go"}`)
	if _, err := ParseHostRoots([]byte(near), tc); err == nil || !strings.Contains(err.Error(), "no tool root") {
		t.Fatalf("a sibling prefix covered the root: %v", err)
	}
}
