package traced

import (
	"math/rand"
	"os"
	"strconv"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// The BIASED differential for KILN04 D50 rule 2c (a successful read-only open
// with no -y annotation).
//
// The parser now emits such an open with an EMPTY ReturnFDPath.  The classifier
// must still cover everything the open really looked at, WITHOUT the kernel's
// own answer to compare with.  The random generator (model_test.go) almost never
// ends reason-free, so it would reach the read-set oracle with this shape about
// once in four thousand actions.  This generator writes actions whose only
// operations are ones the classifier takes without a reason, mixes ANNOTATED
// and BARE reads of the same names (a bare read is the annotated one with its
// ReturnFDPath cleared, which is exactly what the parser does for rule 2c), and
// hands the record to the same oracle: when no reason is raised, every path the
// model looked at is covered.
//
// The reads reach tracked files, files reached through a tracked symlink (the
// walk crosses it), a final symlink, untracked leftovers, and files the action
// itself made under the tmpfs, so both the strict stability rule and the tmpfs
// containment rule see bare opens.

// bareBias counts what the biased mode reached.
var bareBias struct{ cases, bareOpens, clean int }

func buildBareModel(data []byte) (*model, *memTree, []*trace.Trace) {
	i := 0
	next := func() int {
		b := int(data[i%len(data)])
		i++
		return b
	}
	home := homeChoices[next()%2]
	root := rootChoices[next()%2](home)
	tmp := tmpChoices[next()%2]
	m, tree := newModel(root, home, tmp)
	var steps []*trace.Trace
	for s, nsteps := 0, 1+next()%2; s < nsteps; s++ {
		steps = append(steps, bareStep(m, s, next))
	}
	return m, tree, steps
}

func bareStep(m *model, step int, next func() int) *trace.Trace {
	m.procs[1] = &mproc{m.root, m.root}
	dir := m.tmp + "/c" + strconv.Itoa(step)
	reads := []string{
		m.root + "/code/p/a/f1", m.root + "/code/p/b/g", m.root + "/code/p/b/f2", m.root + "/code/p/a/left.txt",
		m.root + "/code/p/a/lnk/g",   // through a tracked directory symlink
		m.root + "/code/p/a/lnk",     // the symlink itself, as a final component
		m.root + "/code/p/a/leftlnk", // an untracked final symlink
		m.root + "/code/p/b/g/",      // a file with a slash: fails, still a look
		m.root + "/code/p/a/nope",    // absent: a failed open is never bare
	}
	var evs []trace.Event
	evs = append(evs, m.doMkdirLike(1, "mkdir", dir, "")...)
	for k, n := 0, 3+next()%14; k < n; k++ {
		switch next() % 6 {
		case 0, 1, 2:
			evs = append(evs, m.doOpen(1, reads[next()%len(reads)], 0, -1)...)
		case 3:
			p := dir + "/t" + strconv.Itoa(next()%3)
			evs = append(evs, m.doOpen(1, p, 1, -1)...) // make it (a write: stays annotated)
		case 4:
			p := dir + "/t" + strconv.Itoa(next()%3)
			evs = append(evs, m.doOpen(1, p, 0, -1)...) // read the action's own file
		default:
			evs = append(evs, m.doStat(1, reads[next()%len(reads)], next()%2 == 0)...)
		}
	}
	// Clear the annotation of most successful read-only opens: the parser's rule 2c.
	for k := range evs {
		e := &evs[k]
		if e.Kind == trace.KindOpen && e.Flags == "O_RDONLY" && e.Result.OK && next()%8 != 0 {
			e.ReturnFDPath = ""
			bareBias.bareOpens++
		}
	}
	tr := &trace.Trace{RootPid: 1, Procs: map[int]*trace.Process{1: {Pid: 1, Exit: &trace.Exit{Pid: 1}}}}
	sh := execve("/usr/bin/sh", "sh", "-c", "step")
	sh.Pid, sh.Cwd = 1, m.root
	tr.Events = append([]trace.Event{sh}, evs...)
	for k := range tr.Events {
		tr.Events[k].Line, tr.Events[k].StartLine = k+1, k+1
	}
	tr.Exits = append(tr.Exits, trace.Exit{Pid: 1})
	m.stepExecs = append(m.stepExecs, m.execs)
	m.execs = nil
	return tr
}

func runBareDifferential(t *testing.T, data []byte) {
	t.Helper()
	m, tree, steps := buildBareModel(data)
	h := newHarness(t)
	h.cfg.Root, h.cfg.Home, h.cfg.Tmpdir = m.root, m.home, m.tmp
	h.cfg.ScratchDirs = modelScratch
	h.cfg.Packages = NewPackageIndex([]string{"code/p/a", "code/p/b"})
	h.cfg.Tree = tree
	h.cfg.FS = modelFS{m}
	recipe := make([]string, len(steps))
	for i := range recipe {
		recipe[i] = "step"
	}
	census := NewCensus()
	rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: steps, Recipe: recipe, StepNetNS: !m.noNetNS, Census: census})
	if err != nil {
		return
	}
	bareBias.cases++
	if len(rec.Reasons) == 0 {
		bareBias.clean++
	}
	judgeRecord(t, m, tree, rec, census)
}

// TestDifferentialBareOpenBiased demands the biased mode reaches the oracle with
// bare opens a few hundred times (the floors are for the default 800 seeds and
// scale with KILN_DIFF_BIASED_SEEDS).  Without the floor the test would pass
// vacuously if every action were refused for some unrelated reason.
func TestDifferentialBareOpenBiased(t *testing.T) {
	n := 800
	if testing.Short() {
		n = 100
	}
	if v, err := strconv.Atoi(os.Getenv("KILN_DIFF_BIASED_SEEDS")); err == nil && v > 0 {
		n = v
	}
	bareBias.cases, bareBias.bareOpens, bareBias.clean = 0, 0, 0
	for seed := 0; seed < n; seed++ {
		r := rand.New(rand.NewSource(int64(2_000_003 + seed)))
		data := make([]byte, 60+r.Intn(200))
		for i := range data {
			data[i] = byte(r.Intn(256))
		}
		t.Run("", func(t *testing.T) {
			defer func() {
				if t.Failed() {
					t.Logf("bare-open seed %d", seed)
				}
			}()
			runBareDifferential(t, data)
		})
	}
	t.Logf("bare-open differential over %d seeds: %d actions classified, %d bare opens, %d reason-free reached the read-set oracle", n, bareBias.cases, bareBias.bareOpens, bareBias.clean)
	if testing.Short() {
		return
	}
	if want := bareCleanFloor * n / 800; bareBias.clean < want {
		t.Errorf("only %d reason-free actions with bare opens reached the oracle (floor %d)", bareBias.clean, want)
	}
	if want := bareOpenFloor * n / 800; bareBias.bareOpens < want {
		t.Errorf("only %d bare opens were generated (floor %d)", bareBias.bareOpens, want)
	}
}

// Floors at 800 seeds; set after the first run (see the log line above).
const (
	bareCleanFloor = 250
	bareOpenFloor  = 3000
)
