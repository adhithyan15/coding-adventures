package record

// Tests of the results reader (`-emit-results`, KILN04 section 4.6 and 4.8): the
// build tool's file is decoded as strictly as every other file `collect` reads.

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
)

func TestDecodeResults(t *testing.T) {
	ok := `{"schema":"kiln-legacy-results/1","packages":[` +
		`{"package":"code/a","status":"built","steps_run":2,"steps_total":2},` +
		`{"package":"code/b","status":"dep-skipped","steps_run":0,"steps_total":3}]}`
	r, err := DecodeResults([]byte(ok))
	if err != nil {
		t.Fatal(err)
	}
	if got := r.ByPackage(); len(got) != 2 || got["code/b"].Status != "dep-skipped" || got["code/a"].StepsRun != 2 {
		t.Fatalf("%+v", got)
	}
	if _, err := DecodeResults([]byte(`{"schema":"kiln-legacy-results/1","packages":[]}`)); err != nil {
		t.Fatalf("an empty results document is legal: %v", err)
	}

	pkg := func(name, status string, run, total int) string {
		return `{"package":"` + name + `","status":"` + status + `","steps_run":` + strconv.Itoa(run) + `,"steps_total":` + strconv.Itoa(total) + `}`
	}
	doc := func(pkgs ...string) string {
		return `{"schema":"kiln-legacy-results/1","packages":[` + strings.Join(pkgs, ",") + `]}`
	}
	for name, c := range map[string]struct{ in, want string }{
		"empty":             {``, "results"},
		"not json":          {`nope`, "results"},
		"trailing data":     {ok + ` {}`, "results"},
		"unknown key":       {`{"schema":"kiln-legacy-results/1","packages":[],"extra":1}`, "results"},
		"a duplicate key":   {`{"schema":"kiln-legacy-results/1","schema":"kiln-legacy-results/1","packages":[]}`, "results"},
		"a case variant":    {`{"Schema":"kiln-legacy-results/1","packages":[]}`, "results"},
		"another schema":    {`{"schema":"kiln-legacy-results/2","packages":[]}`, "schema"},
		"null packages":     {`{"schema":"kiln-legacy-results/1","packages":null}`, "packages is null"},
		"an invalid path":   {doc(pkg("a/../b", "built", 1, 1)), "package"},
		"an empty path":     {doc(pkg("", "built", 1, 1)), "package"},
		"unsorted":          {doc(pkg("code/b", "built", 1, 1), pkg("code/a", "built", 1, 1)), "not strictly sorted"},
		"a repeated":        {doc(pkg("code/a", "built", 1, 1), pkg("code/a", "built", 1, 1)), "not strictly sorted"},
		"an uppercase":      {doc(pkg("code/a", "Built", 1, 1)), "not a lowercase word"},
		"an empty status":   {doc(pkg("code/a", "", 1, 1)), "not a lowercase word"},
		"a long status":     {doc(pkg("code/a", strings.Repeat("a", 33), 1, 1)), "not a lowercase word"},
		"a status with ::":  {doc(pkg("code/a", "built::x", 1, 1)), "not a lowercase word"},
		"negative run":      {doc(pkg("code/a", "built", -1, 1)), "ran -1 of 1"},
		"run above total":   {doc(pkg("code/a", "built", 2, 1)), "ran 2 of 1"},
		"negative total":    {doc(pkg("code/a", "built", 0, -1)), "ran 0 of -1"},
		"a missing field":   {`{"schema":"kiln-legacy-results/1","packages":[{"package":"code/a","status":"built","steps_run":1}]}`, "results"},
		"a null in a field": {`{"schema":"kiln-legacy-results/1","packages":[{"package":null,"status":"built","steps_run":1,"steps_total":1}]}`, "results"},
	} {
		if _, err := DecodeResults([]byte(c.in)); err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("%s: %v (want %q)", name, err, c.want)
		}
	}
}

func TestReadResults(t *testing.T) {
	d := t.TempDir()
	good := filepath.Join(d, "results.json")
	if err := os.WriteFile(good, []byte(`{"schema":"kiln-legacy-results/1","packages":[]}`), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := ReadResults(good); err != nil {
		t.Fatal(err)
	}
	if _, err := ReadResults(filepath.Join(d, "absent.json")); err == nil || !strings.Contains(err.Error(), "--results") {
		t.Fatalf("%v", err)
	}
	if _, err := ReadResults(d); err == nil {
		t.Fatal("a directory was read as results")
	}
	// Over the bound: refused on the size, before reading (a sparse file costs
	// nothing).
	big := filepath.Join(d, "big.json")
	f, err := os.Create(big)
	if err != nil {
		t.Fatal(err)
	}
	f.Truncate(MaxResultsBytes + 1)
	f.Close()
	if _, err := ReadResults(big); err == nil || !strings.Contains(err.Error(), "over") {
		t.Fatalf("%v", err)
	}
	// A symlink is never followed.
	link := filepath.Join(d, "link.json")
	if err := os.Symlink(good, link); err == nil {
		if _, err := ReadResults(link); err == nil && !isWindows() {
			t.Fatal("a symlinked results file was read")
		}
	}
}

func isWindows() bool { return filepath.Separator == '\\' }
