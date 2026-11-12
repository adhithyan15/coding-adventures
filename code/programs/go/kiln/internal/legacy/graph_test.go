package legacy

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

const okGraph = `{"schema":"kiln-legacy-graph/1","packages":[
 {"name":"a","rel_path":"code/a","language":"go",
  "build_files":{"linux":"BUILD","darwin":"BUILD","windows":null},
  "deps":{"linux":["b"],"darwin":[],"windows":null},
  "dev_deps":["b"],"repo_inputs":["code/x"],"repo_input_globs":["code/**/?*.mll"],"extra_toolchains":["dotnet"]},
 {"name":"b","rel_path":"code/b","language":"go",
  "build_files":{"linux":"BUILD","darwin":"BUILD","windows":"BUILD_windows"},
  "deps":{"linux":[],"darwin":[],"windows":[]},
  "dev_deps":null,"repo_inputs":null,"repo_input_globs":null,"extra_toolchains":null,"repo_inputs_unknown":true}]}`

func TestParseGraphOK(t *testing.T) {
	g, err := ParseGraph([]byte(okGraph))
	if err != nil {
		t.Fatal(err)
	}
	if len(g.Packages) != 2 || g.Packages[0].BuildFiles["windows"] != nil || !g.Packages[1].RepoInputsUnknown {
		t.Fatalf("%+v", g.Packages)
	}
}

func TestParseGraphRejects(t *testing.T) {
	mut := func(from, to string) string {
		if !strings.Contains(okGraph, from) {
			t.Fatalf("test bug: %q not in graph", from)
		}
		return strings.Replace(okGraph, from, to, 1)
	}
	bad := map[string]string{
		"not json":            "nope",
		"trailing data":       okGraph + " {}",
		"wrong schema":        mut("kiln-legacy-graph/1", "kiln-legacy-graph/2"),
		"unknown top field":   mut(`"packages"`, `"x":1,"packages"`),
		"unknown pkg field":   mut(`"language":"go"`, `"language":"go","bogus":1`),
		"unsorted packages":   mut(`"name":"b"`, `"name":"0"`),
		"duplicate name":      mut(`"name":"b"`, `"name":"a"`),
		"empty name":          mut(`"name":"b"`, `"name":""`),
		"control in name":     mut(`"name":"b"`, `"name":"b\u0001"`),
		"long name":           mut(`"name":"b"`, `"name":"`+strings.Repeat("b", MaxNameBytes+1)+`"`),
		"bad rel_path":        mut(`"code/b"`, `"code/../b"`),
		"empty rel_path":      mut(`"code/b"`, `""`),
		"shared rel_path":     mut(`"code/b"`, `"code/a"`),
		"empty language":      mut(`"name":"a","rel_path":"code/a","language":"go"`, `"name":"a","rel_path":"code/a","language":""`),
		"missing os in build": mut(`"build_files":{"linux":"BUILD","darwin":"BUILD","windows":null}`, `"build_files":{"linux":"BUILD","darwin":"BUILD"}`),
		"extra os in deps":    mut(`"deps":{"linux":[],"darwin":[],"windows":[]}`, `"deps":{"linux":[],"darwin":[],"windows":[],"plan9":[]}`),
		"wrong os in deps":    mut(`"deps":{"linux":[],"darwin":[],"windows":[]}`, `"deps":{"linux":[],"darwin":[],"plan9":[]}`),
		"wrong os in build":   mut(`"build_files":{"linux":"BUILD","darwin":"BUILD","windows":null}`, `"build_files":{"linux":"BUILD","darwin":"BUILD","plan9":null}`),
		"build file path":     mut(`"windows":"BUILD_windows"`, `"windows":"sub/BUILD"`),
		"build file dotdot":   mut(`"windows":"BUILD_windows"`, `"windows":".."`),
		"unknown dep":         mut(`"linux":["b"]`, `"linux":["nope"]`),
		"unsorted deps":       strings.Replace(mut(`"linux":["b"]`, `"linux":["b","a"]`), `"dev_deps":["b"]`, `"dev_deps":["b"]`, 1),
		"duplicate dev_dep":   mut(`"dev_deps":["b"]`, `"dev_deps":["b","b"]`),
		"unknown dev_dep":     mut(`"dev_deps":["b"]`, `"dev_deps":["zzz"]`),
		"bad repo_input":      mut(`"code/x"`, `"/abs"`),
		"unsorted inputs":     mut(`"repo_inputs":["code/x"]`, `"repo_inputs":["code/y","code/x"]`),
		"bad glob":            mut(`"code/**/?*.mll"`, `"/abs/*"`),
		"unsorted globs":      mut(`"repo_input_globs":["code/**/?*.mll"]`, `"repo_input_globs":["b","a"]`),
		"empty toolchain":     mut(`"extra_toolchains":["dotnet"]`, `"extra_toolchains":[""]`),
		"control toolchain":   mut(`"extra_toolchains":["dotnet"]`, `"extra_toolchains":["dot\u0007net"]`),
		"unsorted toolchains": mut(`"extra_toolchains":["dotnet"]`, `"extra_toolchains":["z","a"]`),
	}
	for name, in := range bad {
		t.Run(name, func(t *testing.T) {
			if _, err := ParseGraph([]byte(in)); err == nil {
				t.Fatal("accepted")
			}
		})
	}
	if _, err := ParseGraph(make([]byte, MaxGraphBytes+1)); err == nil {
		t.Fatal("oversized graph accepted")
	}
}

func TestReadGraphFile(t *testing.T) {
	dir := t.TempDir()
	good := filepath.Join(dir, "g.json")
	if err := os.WriteFile(good, []byte(okGraph), 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := ReadGraphFile(good); err != nil {
		t.Fatal(err)
	}
	if _, err := ReadGraphFile(filepath.Join(dir, "missing.json")); err == nil {
		t.Fatal("missing file accepted")
	}
	if _, err := ReadGraphFile(dir); err == nil {
		t.Fatal("directory accepted")
	}
	huge := filepath.Join(dir, "huge.json")
	fh, _ := os.Create(huge)
	_ = fh.Truncate(MaxGraphBytes + 10)
	fh.Close()
	if _, err := ReadGraphFile(huge); err == nil {
		t.Fatal("oversized file accepted")
	}
}

func TestShortSanitises(t *testing.T) {
	if got := short("a\x1b[31mb"); strings.ContainsRune(got, 0x1b) {
		t.Fatal("control character survived")
	}
	if got := short(strings.Repeat("x", 100)); len(got) > 50 {
		t.Fatal("not truncated")
	}
}

func TestGraphLimits(t *testing.T) {
	defer func(p, e int) { packageLimit, edgeLimit = p, e }(packageLimit, edgeLimit)
	packageLimit = 1
	if _, err := ParseGraph([]byte(okGraph)); err == nil || !strings.Contains(err.Error(), "packages exceed") {
		t.Fatalf("package cap: %v", err)
	}
	packageLimit = MaxPackages
	// okGraph has 1 dev_dep and 1 dep: two edges.
	edgeLimit = 1
	if _, err := ParseGraph([]byte(okGraph)); err == nil || !strings.Contains(err.Error(), "edges") {
		t.Fatalf("edge cap: %v", err)
	}
	edgeLimit = 2
	if _, err := ParseGraph([]byte(okGraph)); err != nil {
		t.Fatalf("at the cap: %v", err)
	}
	if MaxPackages != 100_000 || MaxEdges != 5_000_000 {
		t.Fatal("limits changed")
	}
}
