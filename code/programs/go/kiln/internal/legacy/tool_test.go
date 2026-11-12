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
	f.write("code/packages/go/other/go.mod", "module other\nreplace lib => ../lib\n")
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

func TestGoWorkDirs(t *testing.T) {
	got, err := goWorkDirs("go 1.26\n\nuse ./a\nuse \"./q\" // c\nuse (\n\t./b\n\t// ./no\n)\nreplace x => ./r\n")
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(got, []string{"./a", "./q", "./b", "./r"}) {
		t.Fatalf("got %v", got)
	}
	if _, err := goWorkDirs("use .\\a\n"); err == nil {
		t.Fatal("backslash use accepted")
	}
	if _, err := goWorkDirs("replace a => ..\\x\n"); err == nil {
		t.Fatal("backslash replace accepted")
	}
}

func TestBuildToolFollowsGoWork(t *testing.T) {
	f := newFixture(t)
	f.write("go.work", "go 1.26\nuse (\n\t./code/packages/go/w1\n)\nuse ./code/packages/go/w2\n")
	f.write("code/packages/go/w1/w.go", "package w\n")
	f.write("code/packages/go/w1/go.mod", "module w1\nreplace z => ../w3\n")
	f.write("code/packages/go/w2/w.go", "package w\n")
	f.write("code/packages/go/w2/go.mod", "module w2\nreplace z => ../w1\n") // cycle with w1
	f.write("code/packages/go/w3/w.go", "package w3\n")
	pl, err := f.planner()
	if err != nil {
		t.Fatal(err)
	}
	files, err := pl.buildToolFiles()
	if err != nil {
		t.Fatal(err)
	}
	for _, w := range []string{"code/packages/go/w1/w.go", "code/packages/go/w2/w.go", "code/packages/go/w3/w.go"} {
		if !contains(files, w) {
			t.Fatalf("%s missing: %v", w, files)
		}
	}
	// A change in a go.work module changes every key.
	a := f.plan(baseGraph()).rec(t, "x/z").Key
	f.write("code/packages/go/w3/w.go", "package w3 // changed\n")
	if f.plan(baseGraph()).rec(t, "x/z").Key == a {
		t.Fatal("go.work module not in the tool digest")
	}
}

func TestBuildToolRefusesTargetsOutsideTheRepo(t *testing.T) {
	cases := map[string]map[string]string{
		"go.mod leaves the repo":  {BuildToolPath + "/go.mod": "module bt\nreplace a => ../../../../../elsewhere\n"},
		"go.mod names the root":   {BuildToolPath + "/go.mod": "module bt\nreplace a => ../../../..\n"},
		"go.mod names parent":     {BuildToolPath + "/go.mod": "module bt\nreplace a => ../../../../..\n"},
		"go.work leaves the repo": {"go.work": "use ../outside\n"},
		"go.work names the root":  {"go.work": "use .\n"},
		"go.work bad backslash":   {"go.work": "use .\\a\n"},
		"replaced module leaves":  {"code/packages/go/lib/go.mod": "module lib\nreplace a => ../../../../../x\n"},
	}
	for name, files := range cases {
		t.Run(name, func(t *testing.T) {
			f := newFixture(t)
			for p, c := range files {
				f.write(p, c)
			}
			if _, err := BuildPlan(baseGraph(), f.opts("linux")); err == nil {
				t.Fatal("accepted")
			}
		})
	}
	f := newFixture(t)
	f.write("go.work", "use ./code/packages/go/lib\n")
	f.remove("go.work")
	f.write("go.work", "use ./code/packages/go/lib\n")
	f.write("code/packages/go/lib/go.mod", "module lib\n")
	// And an unreadable go.work is an error too.
	o := f.opts("linux")
	o.Source = &faulty{Source: o.Source, failOpen: map[string]bool{"go.work": true}}
	if _, err := BuildPlan(baseGraph(), o); err == nil {
		t.Fatal("unreadable go.work accepted")
	}
}
