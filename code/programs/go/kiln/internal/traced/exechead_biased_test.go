package traced

import (
	"debug/elf"
	"encoding/binary"
	"math/rand"
	"os"
	"strconv"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// The BIASED mode of the exec-head differential (KILN04 D48 step 6 prerequisite).
//
// The random generator (model_test.go) writes every kind of operation, so almost
// every action has some other reason to be refused (a path outside the
// checkout, a stale cwd, a write to a read-only host file, ...) and only about
// 1 seed in 4000 ends as an honest, reason-free exec case that reaches the
// read-set oracle.  The exec-head case is only worth something when the
// classifier BELIEVES a head, because only then does the oracle compare the read
// set the classifier built from that head with what the model really looked at.
//
// This generator therefore writes actions whose ONLY would-be refusal is the
// exec of a binary under the step's fresh tmpfs, which no plain record can read:
//
//   - the operations are limited to ones the classifier takes without a reason
//     (mkdir and create under the tmpfs, a read of a tracked file, a stat);
//   - the execs are of a binary built under the tmpfs (an ELF with no loader, or
//     with a loader inside the checkout, or with the host's loader), of a host
//     binary, of a checkout binary, and of a name that does not exist, so a pid
//     usually has SEVERAL execs, failed ones among them;
//   - one to four pids, the init (pid 1) anchoring the step's own `sh` exec and
//     usually exec'ing more;
//   - host-binary and tmpfs-binary execs MIX in one pid, which is the
//     same-thread chain the ordinals must keep straight;
//   - a tmpfs binary is sometimes REWRITTEN after the exec that used it, so the
//     head the supervisor pinned (the bytes at that instant) differs from the
//     bytes the model's disk ends with.  The loader of an ELF inside the checkout
//     is a look the model records (execChain), so a classifier that reads the
//     wrong head, or forgets to walk the believed head's loader, leaves a path
//     uncovered and the read-set oracle fails.
//
// A script (`#!`) under the tmpfs is left out on purpose: /proc/<tgid>/exe is its
// interpreter, so an honest sidecar says `mismatch` and the classifier refuses
// it; the random generator covers that.  Every case here is thus believed.
//
// Selection is by its own test (TestDifferentialExecHeadBiased); every one of the
// eight sidecar perturbations is applied to every case (checkExecHeads).

// biasedELF is the head of the tmpfs binary the model writes: the variant is
// chosen by the data.
func biasedELF(m *model, v int) []byte {
	switch v % 4 {
	case 0:
		return buildELFStatic()
	case 1:
		return buildELF(m.root+"/code/p/b/g", elf.ELFCLASS64, binary.LittleEndian) // a loader inside the checkout (tracked file)
	case 2:
		return buildELF(m.root+"/code/p/a/f1", elf.ELFCLASS64, binary.LittleEndian)
	}
	return buildELF(loader, elf.ELFCLASS64, binary.LittleEndian) // the host's loader
}

// buildBiasedModel runs a whole biased action from data.
func buildBiasedModel(data []byte) (*model, *memTree, []*trace.Trace) {
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
		steps = append(steps, biasedStep(m, s, next))
	}
	return m, tree, steps
}

func biasedStep(m *model, step int, next func() int) *trace.Trace {
	npid := 1 + next()%4
	for p := 1; p <= npid; p++ {
		m.procs[p] = &mproc{m.root, m.root}
	}
	dir := m.tmp + "/b" + strconv.Itoa(step)
	names := []string{dir + "/t0", dir + "/t1", dir + "/t2"}
	type pending struct {
		pid int
		run func() []trace.Event
	}
	var ops []pending
	add := func(pid int, run func() []trace.Event) {
		ops = append(ops, pending{pid, run})
	}
	write := func(pid int, p string, variant int) {
		add(pid, func() []trace.Event {
			ev := m.doOpen(pid, p, 1, -1)
			if n := m.byPath(p); n != nil && n.kind == KindFile && ev[0].Result.OK {
				n.data = biasedELF(m, variant)
			}
			return ev
		})
	}
	exec := func(pid int, p string) { add(pid, func() []trace.Event { return m.doExec(pid, p) }) }
	hostExec := func(pid int) {
		switch next() % 3 {
		case 0:
			exec(pid, "/usr/bin/sh")
		case 1:
			exec(pid, m.root+"/code/p/a/f1")
		default:
			exec(pid, "/usr/bin/nope") // a failed exec of a host name
		}
	}
	add(1, func() []trace.Event { return m.doMkdirLike(1, "mkdir", dir, "") })
	write(1, names[0], next())
	write(1, names[1], next())
	// Every pid starts with an exec (so every pid has ordinals to keep straight),
	// pid 1's of a binary built under the tmpfs and the others' of a host binary
	// or a missing tmpfs name, so pid 1's adjacent execs differ from its `sh`.
	//
	// Half the cases are in ROTATION mode: each pid's execs walk a pool of six
	// distinct paths in order, so no two neighbouring lines of a pid (the init's
	// `sh` line included, which is why pid 1 never reaches the pool's `sh`) share
	// a path.  Only then does the ordinal-slip perturbation (7) apply, since a
	// slip onto a line with the very same path cannot be told apart.
	rot := next()%2 == 0
	pool := []string{names[0], names[1], names[2], "/usr/bin/sh", m.root + "/code/p/a/f1", "/usr/bin/nope"}
	count := map[int]int{}
	rotExec := func(pid int) {
		start, limit := (pid*2)%6, 5
		if pid == 1 {
			start, limit = 4, 4 // f1, nope, t0, t1: never `sh`
		}
		if count[pid] >= limit {
			p := m.root + "/code/p/a/f1"
			add(pid, func() []trace.Event { return m.doStat(pid, p, true) })
			return
		}
		exec(pid, pool[(start+count[pid])%6])
		count[pid]++
	}
	if rot {
		rotExec(1)
		for p := 2; p <= npid; p++ {
			rotExec(p)
		}
	} else {
		exec(1, names[next()%2])
		for p := 2; p <= npid; p++ {
			if next()%3 == 0 {
				exec(p, names[2]) // not built: ENOENT under the tmpfs
			} else {
				hostExec(p)
			}
		}
	}
	for k, n := 0, 4+next()%14; k < n; k++ {
		pid := 1 + next()%npid
		kind := next() % 9
		if rot && (kind <= 4 || kind == 8) {
			rotExec(pid)
			continue
		}
		switch kind {
		case 0, 1, 2:
			exec(pid, names[next()%3]) // maybe built, maybe not yet, maybe rewritten
		case 3, 4:
			hostExec(pid)
		case 5:
			write(pid, names[next()%3], next())
		case 6:
			p := m.root + "/code/p/b/g"
			add(pid, func() []trace.Event { return m.doOpen(pid, p, 0, -1) })
		case 7:
			p := m.root + "/code/p/a/f1"
			add(pid, func() []trace.Event { return m.doStat(pid, p, true) })
		default:
			exec(pid, names[next()%3])
		}
	}

	tr := &trace.Trace{RootPid: 1, Procs: map[int]*trace.Process{}}
	sh := execve("/usr/bin/sh", "sh", "-c", "step")
	sh.Pid, sh.Line, sh.StartLine, sh.Cwd = 1, 1, 1, m.root
	tr.Events = append(tr.Events, sh)
	for k, o := range ops {
		for _, e := range o.run() {
			e.Line = k + 2
			e.StartLine = e.Line
			tr.Events = append(tr.Events, e)
		}
	}
	// Fix the lines to be strictly increasing across events.
	for k := range tr.Events {
		tr.Events[k].Line, tr.Events[k].StartLine = k+1, k+1
	}
	for p := npid; p >= 1; p-- {
		pr := &trace.Process{Pid: p, Exit: &trace.Exit{Pid: p}}
		if p > 1 {
			pr.Parent = 1
			tr.Exits = append(tr.Exits, trace.Exit{Pid: p})
		}
		tr.Procs[p] = pr
	}
	tr.Exits = append(tr.Exits, trace.Exit{Pid: 1})
	m.stepExecs = append(m.stepExecs, m.execs)
	m.execs = nil
	return tr
}

// TestDifferentialExecHeadBiased runs the biased generator over fixed seeds and
// demands it reaches what the random one cannot: hundreds of honest, reason-free
// cases that the read-set oracle judged, and every perturbation applied to cases
// that would otherwise be believed.  KILN_DIFF_BIASED_SEEDS sets the seed count.
func TestDifferentialExecHeadBiased(t *testing.T) {
	n := 800
	if testing.Short() {
		n = 60
	}
	if v, err := strconv.Atoi(os.Getenv("KILN_DIFF_BIASED_SEEDS")); err == nil && v > 0 {
		n = v
	}
	ehBias = ehCounters{}
	for seed := 0; seed < n; seed++ {
		r := rand.New(rand.NewSource(int64(1_000_003 + seed)))
		data := make([]byte, 60+r.Intn(200))
		for i := range data {
			data[i] = byte(r.Intn(256))
		}
		t.Run("", func(t *testing.T) {
			defer func() {
				if t.Failed() {
					t.Logf("biased seed %d", seed)
				}
			}()
			m, tree, steps := buildBiasedModel(data)
			runDifferentialOn(t, data, m, tree, steps, false, true)
		})
	}
	t.Logf("biased exec-head differential over %d seeds: honest-clean %d of %d with an exec; slipped applied %v, applied and refused %v",
		n, ehBias.honestClean, ehBias.withExec, ehBias.slipped[1:], ehBias.perRefused[1:])
	if testing.Short() {
		return
	}
	// Floors, for the default seed count (800), scaled for another count.  The
	// seeds are fixed, so the measured values (logged above) only move when the
	// generator or the classifier does; the floors sit a little below them.
	if want := biasedHonestFloor * n / 800; ehBias.honestClean < want {
		t.Errorf("the biased mode reached only %d honest, reason-free cases (floor %d)", ehBias.honestClean, want)
	}
	for k := 1; k < 13; k++ {
		if want := biasedSlipFloor[k] * n / 800; ehBias.perRefused[k] < want {
			t.Errorf("perturbation %d was applied and refused only %d times (floor %d)", k, ehBias.perRefused[k], want)
		}
	}
}

// biasedHonestFloor is the honest believed-oracle cases at 800 seeds (measured
// 800); biasedSlipFloor[k] the applied-and-refused runs of perturbation k
// (measured 798 each, except 7, the ordinal slip, which needs every pid's
// neighbouring paths to differ: 292; 8, which needs a failed exec: 766; 12, the
// result-less exec, which needs every pid to have a failed one: 325; 9, 10, 11 are
// the wrong syscall, an unknown init pid and a confirmed `#!` head).
const biasedHonestFloor = 700

var biasedSlipFloor = [13]int{0, 700, 700, 700, 700, 700, 700, 250, 700, 700, 700, 700, 280}
