package trace

import (
	"fmt"
	"math/rand"
	"os"
	"reflect"
	"strconv"
	"strings"
	"testing"
)

// The BIASED differential for KILN04 D50, at the level of strace text.
//
// A random trace almost never contains `= ? ERESTARTNOINTR` or an open with no
// annotation, so the generic fuzzers never reach the two rules.  This generator
// writes a trace from a small vocabulary (ordinary spawns with children, chdir,
// relative and absolute opens) in TWO spellings of the same run:
//
//	the plain twin    no restarted spawn, every open annotated;
//	the shaped trace  restarted spawns inserted at random places (inline or
//	                  split, fork, vfork or clone, a signal line between the
//	                  restart and the next call), and most successful read-only
//	                  opens stripped of their annotation.
//
// The oracle is equality: both must parse, and to the same events (apart from
// line numbers and the annotation), processes and exits.  A rule that dropped a
// real spawn, paired a restart with a later clone, stole a split call's start
// cwd, or let an open change meaning when its annotation went, shows up as a
// difference.  The counters must be non-zero in the hundreds: a generator that
// never inserted a restart would pass vacuously.

type d50Gen struct {
	r      *rand.Rand
	plain  []string
	shaped []string
	next   int // the next child pid
	cwd    string
	// counts of what the shaped trace holds
	restarts, bare int
}

func (g *d50Gen) both(line string) {
	g.plain = append(g.plain, line)
	g.shaped = append(g.shaped, line)
}

// restart inserts a restarted spawn into the shaped trace only.
func (g *d50Gen) restart(pid int) {
	if g.r.Intn(3) == 0 {
		return
	}
	g.restarts++
	p := strconv.Itoa(pid)
	res := " = ? ERESTARTNOINTR (To be restarted)"
	var lines []string
	switch g.r.Intn(5) {
	case 0:
		lines = []string{p + "  fork()" + res}
	case 1:
		lines = []string{p + "  vfork()" + res}
	case 2:
		lines = []string{p + "  clone(" + d50Flags + ")" + res}
	case 3:
		lines = []string{p + "  clone(" + d50Flags + " <unfinished ...>", p + "  <... clone resumed>)" + res}
	default:
		lines = []string{p + "  fork( <unfinished ...>", p + "  <... fork resumed>)" + res}
	}
	if g.r.Intn(2) == 0 {
		lines = append(lines, p+"  --- SIGUSR1 {si_signo=SIGUSR1, si_code=SI_USER, si_pid=9, si_uid=0} ---")
	}
	g.shaped = append(g.shaped, lines...)
}

func (g *d50Gen) open(pid int, path, resolved string) {
	p := strconv.Itoa(pid)
	flags := []string{"O_RDONLY", "O_RDONLY|O_CLOEXEC", "O_RDONLY|O_DIRECTORY|O_CLOEXEC", "O_PATH|O_CLOEXEC"}[g.r.Intn(4)]
	g.plain = append(g.plain, fmt.Sprintf("%s  openat(AT_FDCWD, %q, %s) = 3<%s>", p, path, flags, resolved))
	if g.r.Intn(8) != 0 {
		g.bare++
		g.shaped = append(g.shaped, fmt.Sprintf("%s  openat(AT_FDCWD, %q, %s) = 3", p, path, flags))
	} else {
		g.shaped = append(g.shaped, fmt.Sprintf("%s  openat(AT_FDCWD, %q, %s) = 3<%s>", p, path, flags, resolved))
	}
}

func d50Generate(seed int64) *d50Gen {
	g := &d50Gen{r: rand.New(rand.NewSource(seed)), next: 2, cwd: "/work/repo"}
	files := []string{"a.txt", "b/c.txt", "d.go"}
	for k, n := 0, 3+g.r.Intn(12); k < n; k++ {
		switch g.r.Intn(6) {
		case 0, 1: // a restarted spawn, then (usually) the real one
			g.restart(1)
			child := g.next
			g.next++
			flags := []string{"flags=SIGCHLD", "flags=CLONE_VM|CLONE_VFORK|SIGCHLD", "flags=CLONE_FS|SIGCHLD"}[g.r.Intn(3)]
			g.both(fmt.Sprintf("1  clone(child_stack=NULL, %s) = %d", flags, child))
			f := files[g.r.Intn(len(files))]
			g.open(child, g.cwd+"/"+f, g.cwd+"/"+f)
			g.both(fmt.Sprintf("%d  +++ exited with 0 +++", child))
		case 2: // an absolute read
			f := g.cwd + "/" + files[g.r.Intn(len(files))]
			g.restart(1)
			g.open(1, f, f)
		case 3: // chdir, then a relative read in the new cwd
			g.both(`1  chdir("/work/repo/b") = 0`)
			g.cwd = "/work/repo/b"
			g.restart(1)
			g.open(1, "c.txt", "/work/repo/b/c.txt")
		case 4: // a split spawn (restarted) around an open of another pid
			g.restart(1)
			g.open(1, "/work/repo/d.go", "/work/repo/d.go")
		default:
			g.both(`1  stat("/work/repo/a.txt", {st_mode=S_IFREG|0644, st_size=3, ...}) = 0`)
		}
	}
	g.both("1  +++ exited with 0 +++")
	return g
}

// d50Shape is what two traces must agree on.
type d50Shape struct {
	Events []Event
	Procs  map[int]Process
	Exits  []Exit
}

func d50ShapeOf(t *testing.T, tr *Trace) d50Shape {
	s := d50Shape{Procs: map[int]Process{}}
	for _, e := range tr.Events {
		e.Line, e.StartLine, e.ReturnFDPath = 0, 0, ""
		s.Events = append(s.Events, e)
	}
	for pid, p := range tr.Procs {
		s.Procs[pid] = *p
	}
	s.Exits = tr.Exits
	return s
}

var d50Bias struct{ cases, restarts, bare, withRestart, withBare int }

func TestD50BiasedDifferential(t *testing.T) {
	n := 800
	if testing.Short() {
		n = 100
	}
	if v, err := strconv.Atoi(os.Getenv("KILN_DIFF_BIASED_SEEDS")); err == nil && v > 0 {
		n = v
	}
	d50Bias = struct{ cases, restarts, bare, withRestart, withBare int }{}
	for seed := 0; seed < n; seed++ {
		g := d50Generate(int64(3_000_017 + seed))
		plain, err := parseText(t, strings.Join(g.plain, "\n")+"\n")
		if err != nil {
			t.Fatalf("seed %d: the plain twin is refused: %v\n%s", seed, err, strings.Join(g.plain, "\n"))
		}
		shaped, err := parseText(t, strings.Join(g.shaped, "\n")+"\n")
		if err != nil {
			t.Fatalf("seed %d: the shaped trace is refused: %v\n%s", seed, err, strings.Join(g.shaped, "\n"))
		}
		a, b := d50ShapeOf(t, plain), d50ShapeOf(t, shaped)
		if !reflect.DeepEqual(a, b) {
			t.Fatalf("seed %d: the twins parse differently\nplain  %+v\nshaped %+v\n%s", seed, a, b, strings.Join(g.shaped, "\n"))
		}
		if shaped.RestartedSpawns != g.restarts || shaped.BareOpens != g.bare || plain.RestartedSpawns != 0 || plain.BareOpens != 0 {
			t.Fatalf("seed %d: counters restarted %d/%d bare %d/%d", seed, shaped.RestartedSpawns, g.restarts, shaped.BareOpens, g.bare)
		}
		d50Bias.cases++
		d50Bias.restarts += g.restarts
		d50Bias.bare += g.bare
		if g.restarts > 0 {
			d50Bias.withRestart++
		}
		if g.bare > 0 {
			d50Bias.withBare++
		}
	}
	t.Logf("D50 biased differential over %d seeds: %d restarted spawns in %d traces, %d bare opens in %d traces", n, d50Bias.restarts, d50Bias.withRestart, d50Bias.bare, d50Bias.withBare)
	if testing.Short() {
		return
	}
	if want := 400 * n / 800; d50Bias.withRestart < want {
		t.Errorf("only %d traces held a restarted spawn (floor %d)", d50Bias.withRestart, want)
	}
	if want := 400 * n / 800; d50Bias.withBare < want {
		t.Errorf("only %d traces held a bare open (floor %d)", d50Bias.withBare, want)
	}
}
