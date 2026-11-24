//go:build unix

package record

// The vacuity counters of the census exec-head block (KILN04 D48 step 6):
// go_actions, go_built_ok and tmpfs_execs, and the line that says a sealed shard
// exercised nothing.

import (
	"bytes"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// A believed exec raises no census reason, so the classifier's side file of an
// action whose every exec was believed is EMPTY; the count must still reach the
// shard's total.
func TestCensusExecHeadTmpfsExecsComesFromEmptyAndNonEmptyCensuses(t *testing.T) {
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

	believed := traced.NewCensus()
	believed.TmpfsExecs = 4 // nothing refused: no entries at all
	if st, _ := r.readCensus(write("a", believed), "code/p/a"); st != censusEmpty {
		t.Fatalf("a census with only believed execs is empty of reasons, got state %v", st)
	}
	mixed := traced.NewCensus()
	mixed.TmpfsExecs = 3
	mixed.Note(execHeadSubKindPrefix+"exechead-pair", "/tmp/x/t.test")
	if st, _ := r.readCensus(write("b", mixed), "code/p/b"); st != censusNonEmpty {
		t.Fatalf("a census with a refusal is not empty, got state %v", st)
	}
	if got := acc.execHeadSummary().TmpfsExecs; got != 7 {
		t.Fatalf("tmpfs_execs %d, want 4+3", got)
	}

	// Bounds: a negative or over-cap count makes the file invalid and is not summed.
	for name, n := range map[string]int64{"neg": -1, "big": traced.MaxTmpfsExecs + 1} {
		adir := filepath.Join(dir, name)
		if err := os.MkdirAll(adir, 0o700); err != nil {
			t.Fatal(err)
		}
		doc := fmt.Sprintf(`{"schema":%q,"action":"code/p/%s","total":0,"overflow_total":0,"overflow":{},"entries":[],"exec_head_tmpfs_execs":%d}`, CensusSchema, name, n)
		if err := os.WriteFile(filepath.Join(adir, CensusFileName), []byte(doc), 0o600); err != nil {
			t.Fatal(err)
		}
		if st, _ := r.readCensus(adir, "code/p/"+name); st != censusInvalid {
			t.Errorf("%s: state %v, want invalid", name, st)
		}
	}
	if got := acc.execHeadSummary().TmpfsExecs; got != 7 {
		t.Fatalf("an invalid file changed tmpfs_execs to %d", got)
	}

	// Saturation: the sum holds at its cap rather than wrapping.
	acc.execHead.TmpfsExecs = satAdd(1<<62, traced.MaxTmpfsExecs)
	if acc.execHead.TmpfsExecs != 1<<62 {
		t.Fatalf("the sum wrapped: %d", acc.execHead.TmpfsExecs)
	}
}

// A census whose tmpfs_execs is below the sum of its own exechead-* occurrences
// (entries plus overflow) contradicts itself and is refused; one at or above the
// sum, or with no exechead-* kind at all, is accepted.
func TestCensusExecHeadTmpfsExecsBelowItsSubCodesIsRefused(t *testing.T) {
	dir := t.TempDir()
	try := func(name string, tmpfs int64, entries, overflow string) censusState {
		adir := filepath.Join(dir, name)
		if err := os.MkdirAll(adir, 0o700); err != nil {
			t.Fatal(err)
		}
		doc := fmt.Sprintf(`{"schema":%q,"action":"code/p/%s","total":9,"overflow_total":0,"overflow":{%s},"entries":[%s],"exec_head_tmpfs_execs":%d}`,
			CensusSchema, name, overflow, entries, tmpfs)
		if err := os.WriteFile(filepath.Join(adir, CensusFileName), []byte(doc), 0o600); err != nil {
			t.Fatal(err)
		}
		r := &censusRun{o: CensusOptions{}, acc: newCensusAcc(newRedactor(nil))}
		st, _ := r.readCensus(adir, "code/p/"+name)
		return st
	}
	ent := `{"kind":"trace-incomplete:exec-unreadable:exechead-pair","path":"/tmp/x","count":3}`
	ovf := `"trace-incomplete:exec-unreadable:exechead-gone":2`
	other := `{"kind":"host-read","path":"/etc/x","count":50}`
	for name, c := range map[string]struct {
		tmpfs   int64
		entries string
		ovf     string
		want    censusState
	}{
		"entries only, below":  {2, ent, "", censusInvalid},
		"entries only, equal":  {3, ent, "", censusNonEmpty},
		"overflow counts":      {4, ent, ovf, censusInvalid},
		"overflow, enough":     {5, ent, ovf, censusNonEmpty},
		"forged zero":          {0, ent, "", censusInvalid},
		"other kinds ignored":  {0, other, "", censusNonEmpty},
		"saturated count held": {traced.MaxTmpfsExecs, ent, ovf, censusNonEmpty},
	} {
		if got := try(strings.ReplaceAll(name, " ", "_"), c.tmpfs, c.entries, c.ovf); got != c.want {
			t.Errorf("%s: state %v, want %v", name, got, c.want)
		}
	}
}

// The vacuity line is the FIRST line of the Exec heads section when a sealed
// shard has no Go action or no tmpfs exec, and absent otherwise; the three
// counters are always printed for a sealed shard.
func TestCensusExecHeadVacuityLine(t *testing.T) {
	section := func(goActions, goBuilt int, tmpfs int64, sealed bool) string {
		acc := newCensusAcc(newRedactor(nil))
		acc.execHead.Sealed = sealed
		acc.execHead.GoActions, acc.execHead.GoBuiltOK, acc.execHead.TmpfsExecs = goActions, goBuilt, tmpfs
		md, _, _ := RenderCensusMarkdown(acc.summary(10))
		s := md[strings.Index(md, "## Exec heads"):]
		return s[:strings.Index(s, "## Reasons by kind")]
	}
	const want = "## Exec heads\n\n" + ExecHeadVacuousLine + "\n"
	// No Go action but tmpfs execs (rustdoc test binaries): the mechanism WAS
	// exercised, so only the separate no-Go line is printed, never the vacuity one.
	ng := section(0, 0, 5, true)
	if !strings.HasPrefix(ng, "## Exec heads\n\n"+ExecHeadNoGoLine+"\n") || strings.Contains(ng, ExecHeadVacuousLine) {
		t.Errorf("no go, tmpfs execs > 0:\n%s", ng)
	}
	for name, c := range map[string][3]int64{"no tmpfs execs": {3, 2, 0}, "neither": {0, 0, 0}} {
		s := section(int(c[0]), int(c[1]), c[2], true)
		if !strings.HasPrefix(s, want) {
			t.Errorf("%s: the section does not start with the vacuity line:\n%s", name, s)
		}
		if !strings.Contains(s, fmt.Sprintf("go actions %d; go actions built ok %d; execs under the step tmpfs judged by the exec head %d", c[0], c[1], c[2])) {
			t.Errorf("%s: the counters are not printed:\n%s", name, s)
		}
	}
	if s := section(3, 2, 5, true); strings.Contains(s, "nothing exercised them") || !strings.Contains(s, "execs under the step tmpfs judged by the exec head 5") {
		t.Errorf("an exercised shard:\n%s", s)
	}
	if s := section(0, 0, 0, false); strings.Contains(s, "nothing exercised them") || !strings.Contains(s, "WITHOUT exec heads") {
		t.Errorf("an unsealed shard is not judged vacuous:\n%s", s)
	}
}

// A Go action that built, whose census carries tmpfs execs: the counters add up.
func TestCensusExecHeadGoActionAccumulation(t *testing.T) {
	acc := newCensusAcc(newRedactor(nil))
	acc.execHead.Sealed = true
	acc.noteGoAction("go", true)
	acc.noteGoAction("x", true) // another language: counts nothing
	dir := t.TempDir()
	c := traced.NewCensus()
	c.TmpfsExecs = 9
	data, err := EncodeCensus("code/packages/go/p", c)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, CensusFileName), data, 0o600); err != nil {
		t.Fatal(err)
	}
	r := &censusRun{o: CensusOptions{}, acc: acc}
	r.readCensus(dir, "code/packages/go/p")
	e := acc.execHeadSummary()
	if e.GoActions != 1 || e.GoBuiltOK != 1 || e.TmpfsExecs != 9 {
		t.Fatalf("go %d built %d tmpfs %d", e.GoActions, e.GoBuiltOK, e.TmpfsExecs)
	}
	acc.noteGoAction("go", false)
	if e := acc.execHeadSummary(); e.GoActions != 2 || e.GoBuiltOK != 1 {
		t.Fatalf("a failed Go build: %+v", e)
	}
}

// The Go counters count Go actions only.  The fixture's actions are language "x"
// (like census run A's Rust ones): they built, but none of them is a Go action,
// so go_actions and go_built_ok stay 0 beside a nonzero built_ok.
func TestCensusExecHeadGoCountersIgnoreOtherLanguages(t *testing.T) {
	f := censusDirFix(t)
	_, s, _, _ := runCensus(t, f)
	if s.Actions.BuiltOK != 2 || s.ExecHead.GoActions != 0 || s.ExecHead.GoBuiltOK != 0 {
		t.Fatalf("built ok %d, go actions %d, go built ok %d", s.Actions.BuiltOK, s.ExecHead.GoActions, s.ExecHead.GoBuiltOK)
	}
}

// The stdout census line shows tmpfs_execs for a sealed shard only.
func TestCensusStdoutShowsTmpfsExecsWhenSealed(t *testing.T) {
	for _, sealed := range []bool{false, true} {
		p := newPrep(t)
		p.execHead = sealed
		f := newStepFixOn(t, p, nil, planSteps, nil, nil, false, nil)
		out := t.TempDir()
		var stdout bytes.Buffer
		if _, err := RunCensus(CensusRunOptions{CensusOptions: CensusOptions{Dir: f.p.dir}, Out: filepath.Join(out, "c.json"), MD: filepath.Join(out, "c.md")}, &stdout); err != nil {
			t.Fatalf("sealed=%v: %v", sealed, err)
		}
		if got := strings.Contains(stdout.String(), " tmpfs_execs=0"); got != sealed {
			t.Errorf("sealed=%v: stdout %q", sealed, stdout.String())
		}
	}
}
