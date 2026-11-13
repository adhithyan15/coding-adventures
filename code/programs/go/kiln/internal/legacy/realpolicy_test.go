package legacy

import (
	"encoding/json"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// repoRoot finds the checkout this test runs in, or skips (a sparse checkout
// may not hold the committed policy files).
func repoRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	for ; ; dir = filepath.Dir(dir) {
		if _, err := os.Stat(filepath.Join(dir, ToolchainsPath)); err == nil {
			if _, err := os.Stat(filepath.Join(dir, "mise.toml")); err == nil {
				return dir
			}
		}
		if filepath.Dir(dir) == dir {
			t.Skip("repository policy files not in this checkout")
		}
	}
}

// pinSource says where a pinned tool's version is committed: the key of
// mise.toml [tools], or of code/specs/data/toolchain-pins.json.
var pinSource = map[string]string{
	"go": "mise:go", "python3": "mise:python", "uv": "mise:uv", "node": "mise:node", "ruby": "mise:ruby",
	"rustc": "mise:rust", "cargo": "mise:rust", "ghc": "mise:ghc", "cabal": "mise:cabal", "dart": "mise:dart",
	"gradle": "mise:gradle", "kotlinc": "mise:kotlin", "lua": "mise:lua",
	"dotnet": "pins:ci_only.dotnet", "elixir": "pins:ci_only.elixir", "erl": "pins:ci_only.erlang",
	"bundle": "pins:bundler",
	// ci.yml asserts these versions in shell steps (OCaml job), not via a
	// setup-action input, so toolchain_pins.py does not manage them.
	"ocamlc": "ci:5.2.1", "opam": "ci:2.5.2",
}

// TestCommittedPolicyFiles loads the REAL toolchains.json, env.json and
// host-roots.json, and checks every exact pin against the repository's source
// of truth (mise.toml and code/specs/data/toolchain-pins.json), so a toolchain
// bump that forgets the Kiln pin fails here.
func TestCommittedPolicyFiles(t *testing.T) {
	root := repoRoot(t)
	read := func(rel string) []byte {
		b, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(rel)))
		if err != nil {
			t.Fatal(err)
		}
		return b
	}
	tc, err := ParseToolchains(read(ToolchainsPath))
	if err != nil {
		t.Fatal(err)
	}
	if _, err := ParseEnvPolicy(read(EnvPath)); err != nil {
		t.Fatal(err)
	}
	if _, err := ParseHostRoots(read(HostRootsPath), tc); err != nil {
		t.Fatal(err)
	}
	// The always-included set of KILN03 4 and KILN04 3.1.
	tools, _ := tc.ToolsFor(nil, "linux", "img")
	var names []string
	for _, tl := range tools {
		names = append(names, tl.Name)
	}
	if got := strings.Join(names, " "); got != "bash cc ld sh strace" {
		t.Fatalf("always-included tools on linux = %q", got)
	}
	if mac, _ := tc.ToolsFor(nil, "darwin", "img"); len(mac) != 6 || mac[len(mac)-1].Name != "xcode-select" {
		t.Fatalf("darwin: %+v", mac)
	}

	mise := map[string]string{}
	line := regexp.MustCompile(`^([a-z_]+)\s*=\s*"([^"]+)"\s*$`)
	for _, l := range strings.Split(string(read("mise.toml")), "\n") {
		if m := line.FindStringSubmatch(l); m != nil {
			mise[m[1]] = m[2]
		}
	}
	var pins struct {
		CIOnly  map[string]string `json:"ci_only"`
		Bundler string            `json:"bundler"`
	}
	if err := json.Unmarshal(read("code/specs/data/toolchain-pins.json"), &pins); err != nil {
		t.Fatal(err)
	}
	want := func(src string) string {
		kind, name, _ := strings.Cut(src, ":")
		switch kind {
		case "mise":
			return mise[name]
		case "ci":
			return name
		}
		if name == "bundler" {
			return pins.Bundler
		}
		return pins.CIOnly[strings.TrimPrefix(name, "ci_only.")]
	}
	for _, n := range tc.Names() {
		e, _ := tc.Entry(n)
		for tool, pin := range e.Pins {
			if pin == ImagePin {
				continue
			}
			src, ok := pinSource[tool]
			if !ok {
				t.Errorf("%s: tool %s has an exact pin %s with no committed source in this test", n, tool, pin)
				continue
			}
			if w := want(src); w != pin {
				t.Errorf("%s: %s is pinned to %s but %s says %q", n, tool, pin, src, w)
			}
		}
		// A toolchain-cache directory carries the pinned version.
		for tool, dirs := range e.Roots {
			for _, d := range dirs {
				if strings.HasPrefix(d, "/opt/hostedtoolcache/") && !strings.Contains(d, "/"+e.Pins[tool]+"/") {
					t.Errorf("%s: root %s does not carry the pin %s", n, d, e.Pins[tool])
				}
			}
		}
	}
}
