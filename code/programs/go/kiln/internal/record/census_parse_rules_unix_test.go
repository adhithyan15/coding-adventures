//go:build unix

package record

// The census counters of the KILN04 D50 parse rules: restarted spawns (rule 2b)
// and bare read-only opens (rule 2c), so the next census shows whether they
// matter.

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

func TestCensusParseRuleCountersSumAcrossActions(t *testing.T) {
	dir := t.TempDir()
	write := func(action string, c *traced.Census) string {
		adir := filepath.Join(dir, action)
		if err := os.MkdirAll(adir, 0o700); err != nil {
			t.Fatal(err)
		}
		data, err := EncodeCensus("code/p/"+action, c)
		if err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(adir, CensusFileName), data, 0o600); err != nil {
			t.Fatal(err)
		}
		return adir
	}
	acc := newCensusAcc(newRedactor(nil))
	r := &censusRun{o: CensusOptions{}, acc: acc}
	a := traced.NewCensus()
	a.RestartedSpawns, a.BareOpens = 2, 10 // nothing refused: an empty census
	if st, _ := r.readCensus(write("a", a), "code/p/a"); st != censusEmpty {
		t.Fatalf("state %v, want empty", st)
	}
	b := traced.NewCensus()
	b.RestartedSpawns, b.BareOpens = 1, 5
	b.Note("host-read", "/etc/x")
	if st, _ := r.readCensus(write("b", b), "code/p/b"); st != censusNonEmpty {
		t.Fatalf("state %v, want non-empty", st)
	}
	if got := acc.parseRules; got.RestartedSpawns != 3 || got.BareOpens != 15 {
		t.Fatalf("parse rules %+v, want 3 and 15", got)
	}
	// An out-of-range count makes the file invalid and is not summed.
	bad := traced.NewCensus()
	bad.RestartedSpawns = traced.MaxParseRuleCount + 1
	if st, _ := r.readCensus(write("c", bad), "code/p/c"); st != censusInvalid {
		t.Fatalf("state %v, want invalid", st)
	}
	if got := acc.parseRules; got.RestartedSpawns != 3 {
		t.Fatalf("an invalid file was summed: %+v", got)
	}
	md, _, _ := RenderCensusMarkdown(acc.summary(CensusTopN))
	for _, want := range []string{"## Parse rules", "(rule 2b): 3", "(rule 2c): 15"} {
		if !strings.Contains(md, want) {
			t.Errorf("markdown lacks %q", want)
		}
	}
}
