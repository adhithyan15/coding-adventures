package legacy

import (
	"reflect"
	"testing"
)

func TestGoModReplaces(t *testing.T) {
	got, err := goModReplaces(`module m

go 1.26

replace a.example/x => ../x
replace a.example/remote v1.0.0 => b.example/y v1.1.0
replace (
	a.example/y v1.2.3 => ./y // comment
	a.example/z => "../z z"
	// a.example/w => ../commented
)
replace	a.example/t => ../../t
require c.example/q v1.0.0
`)
	if err != nil {
		t.Fatal(err)
	}
	want := []string{"../x", "./y", "../z", "../../t"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	if _, err := goModReplaces("replace a => ..\\x\n"); err == nil {
		t.Fatal("backslash target accepted")
	}
	if got, _ := goModReplaces("replace a\nreplace (\n b\n)\nreplace c =>\n"); len(got) != 0 {
		t.Fatalf("got %v", got)
	}
}

func TestBuildToolFilesVariants(t *testing.T) {
	tool := func(f *fixture) []string {
		pl, err := f.planner()
		if err != nil {
			t.Fatal(err)
		}
		files, err := pl.buildToolFiles()
		if err != nil {
			t.Fatal(err)
		}
		return files
	}
	f := newFixture(t)
	got := tool(f)
	want := []string{"code/packages/go/lib/lib.go", BuildToolPath + "/go.mod", BuildToolPath + "/main.go", "go.work"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	// Transitive replace: lib's own go.mod points at another in-repo module.
	f.write("code/packages/go/lib/go.mod", "module lib\nreplace other => ../other\n")
	f.write("code/packages/go/other/o.go", "package o\n")
	f.write("go.work.sum", "x")
	f.write("vendor/v/v.go", "package v\n")
	got = tool(f)
	for _, w := range []string{"code/packages/go/other/o.go", "go.work.sum", "vendor/v/v.go"} {
		if !contains(got, w) {
			t.Fatalf("%s missing from %v", w, got)
		}
	}
	// A replace cycle terminates.
	f.write("code/packages/go/other/go.mod", "module other\nreplace lib => ../lib\nreplace root => ../../../..\n")
	_ = tool(f)
}

func TestBuildToolErrors(t *testing.T) {
	f := newFixture(t)
	f.remove(BuildToolPath + "/go.mod")
	if _, err := BuildPlan(baseGraph(), f.opts("linux")); err == nil {
		t.Fatal("missing build tool go.mod accepted")
	}
	f = newFixture(t)
	f.write(BuildToolPath+"/go.mod", "replace a => ..\\x\n")
	if _, err := BuildPlan(baseGraph(), f.opts("linux")); err == nil {
		t.Fatal("backslash replace accepted")
	}
	f = newFixture(t)
	f.write("code/packages/go/lib/go.mod", "replace a => ..\\x\n")
	if _, err := BuildPlan(baseGraph(), f.opts("linux")); err == nil {
		t.Fatal("backslash replace in a replaced module accepted")
	}
}
