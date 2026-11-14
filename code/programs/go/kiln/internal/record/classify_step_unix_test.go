//go:build unix

package record

// Tests of the classification epilogue and the serial lock of `kiln record
// step` (KILN04 section 4.8, decision P13).
//
// Like step_unix_test.go they need no strace and no sandbox: the "tracer" is a
// shell script that evals the step's line. Here the line also PLAYS BACK a
// canned trace (`cat "$aux/<name>.trace" >&$n`), which is real strace -y text in
// the shape of the sandbox helper's trace (a root that execs Kiln and makes one
// init child, which re-execs Kiln and then execs the step), so the real parser
// and the real classifier run on it, against a real temporary checkout. The line
// can also change the disk (`ln -s`, `rm`), which is what the motivating test
// needs: the disk at the end of an action is what the classifier must see.

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"syscall"
	"testing"
)

// --- canned traces ----------------------------------------------------------

func straceStr(s string) string {
	s = strings.ReplaceAll(s, `\`, `\\`)
	s = strings.ReplaceAll(s, `"`, `\"`)
	return `"` + s + `"`
}

func straceList(args ...string) string {
	q := make([]string, len(args))
	for i, a := range args {
		q[i] = straceStr(a)
	}
	return "[" + strings.Join(q, ", ") + "]"
}

// cannedOpts bends a canned trace away from the healthy shape.
type cannedOpts struct {
	rootSHA, initSHA string // default: the sealed configuration's digest
	noRootArgv       bool   // the root's argv is cut short (strace prints "...")
	noInitExec       bool   // the init child never re-execs Kiln
	noStepExec       bool   // the sandbox refused: nothing was exec'd
	noInit           bool   // the root died before it made the init child
	rootExit         int    // the root's exit line; default: status
	hasRootExit      bool
}

// canned renders the trace of one step of action running line, whose root and
// init exit with status, with body (lines of the step's calls, no pid) between
// the step's exec and its exit.
func (f *stepFix) canned(action, line string, status int, o cannedOpts, body ...string) string {
	kiln, cfg := f.pre.Config.KilnExe, f.pre.ConfigPath
	sha := f.pre.Binding.SandboxConfigSHA256
	if o.rootSHA == "" {
		o.rootSHA = sha
	}
	if o.initSHA == "" {
		o.initSHA = sha
	}
	pkg := f.pre.Config.PackageDir(action)
	rootArgv := straceList("kiln", "sandbox-exec", "--config", cfg, "--config-sha256", o.rootSHA, "--stdout-fd", "3", "--stderr-fd", "4", "--", "sh", "-c", line)
	if o.noRootArgv {
		rootArgv = `["kiln", "sandbox-exec", ...]`
	}
	rootExit := status
	if o.hasRootExit {
		rootExit = o.rootExit
	}
	var b strings.Builder
	w := func(format string, a ...any) { fmt.Fprintf(&b, format+"\n", a...) }
	w(`10  execve(%s, %s, 0x0 /* 1 vars */) = 0`, straceStr(kiln), rootArgv)
	if o.noInit {
		w(`10  +++ exited with %d +++`, rootExit)
		return b.String()
	}
	w(`10  unshare(CLONE_NEWUSER) = 0`)
	w(`10  clone(child_stack=NULL, flags=CLONE_NEWPID|SIGCHLD) = 11`)
	w(`11  unshare(CLONE_NEWNS) = 0`)
	w(`11  mount("none", "/", NULL, MS_REC|MS_PRIVATE, NULL) = 0`)
	w(`11  pivot_root(".", ".") = 0`)
	w(`11  chdir(%s) = 0`, straceStr(pkg))
	w(`11  prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) = 0`)
	w(`11  seccomp(SECCOMP_SET_MODE_FILTER, 0, 0x1) = 0`)
	if !o.noInitExec {
		w(`11  execve("/proc/self/exe", %s, 0x0 /* 1 vars */) = 0`,
			straceList("kiln", "sandbox-init", "--config", cfg, "--sha256", o.initSHA, "--action", action, "--uid", "1", "--gid", "1", "--", "sh", "-c", line))
	}
	if !o.noStepExec {
		w(`11  execve("/bin/sh", %s, 0x0 /* 1 vars */) = 0`, straceList("sh", "-c", line))
		for _, l := range body {
			w("11  %s", l)
		}
	}
	w(`11  +++ exited with %d +++`, status)
	w(`10  wait4(11, [{WIFEXITED(s) && WEXITSTATUS(s) == %d}], 0, NULL) = 11`, status)
	w(`10  +++ exited with %d +++`, rootExit)
	return b.String()
}

// playLine is the shell line of a step that plays back trace `name`, after
// running shell (which may change the disk) and then exits with status.
func playLine(shell, name string, status int) string {
	pre := ""
	if shell != "" {
		pre = shell + "; "
	}
	return fmt.Sprintf(`%scat "$aux/%s.trace" >&$n; exit %d`, pre, name, status)
}

// give writes a canned trace the line will play back.
func (f *stepFix) give(name, action, line string, status int, o cannedOpts, body ...string) {
	f.t.Helper()
	writeBytes(f.t, filepath.Join(f.aux, name+".trace"), []byte(f.canned(action, line, status, o, body...)), 0o644)
}

// planOf renders a steps export for the three fixture packages.
func planOf(a, b, z []string) string {
	q := func(l []string) string {
		out := make([]string, len(l))
		for i, s := range l {
			out[i] = jsonStr(s)
		}
		return "[" + strings.Join(out, ",") + "]"
	}
	return `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[` +
		`{"package":"` + actA + `","steps":` + q(a) + `},` +
		`{"package":"` + actB + `","steps":` + q(b) + `},` +
		`{"package":"` + actZ + `","steps":` + q(z) + `}]}`
}

func jsonStr(s string) string {
	var b strings.Builder
	b.WriteByte('"')
	for _, r := range s {
		switch r {
		case '"', '\\':
			b.WriteByte('\\')
			b.WriteRune(r)
		default:
			b.WriteRune(r)
		}
	}
	b.WriteByte('"')
	return b.String()
}

func (f *stepFix) readRecord(action string) ClassifiedRecord {
	f.t.Helper()
	data, err := os.ReadFile(RecordFile(f.p.dir, action))
	if err != nil {
		f.t.Fatal(err)
	}
	rec, err := DecodeClassified(data)
	if err != nil {
		f.t.Fatalf("%v\n%s", err, data)
	}
	return rec
}

// classifyNow classifies action again, against the disk as it is now.
func (f *stepFix) classifyNow(action string, step int) ClassifiedRecord {
	f.t.Helper()
	rec, err := classifyAction(ClassifyDeps{}, f.pre, action, step, f.sidecar(action, step))
	if err != nil {
		f.t.Fatal(err)
	}
	return rec
}

// --- the motivating case ----------------------------------------------------

// The package has a tracked link L -> d1. Action A writes through it. Action B
// then retargets L -> d2 on the disk. A is classified when A ends, so the walk
// through L agrees with what strace saw (the write is at d1/x) and the record is
// clean. Classified AFTER B (what collect used to do) the walk goes through d2,
// disagrees with strace's -y path, and A gets an unstable-path reason and an
// unknown write set: the action is charged for what B did.
//
// (This is the case where the disk CAN be checked: an `open` is compared with
// the kernel's -y path, so the move is caught late as unstable-path. A link the
// action creates itself is resolved from the disk as well, and there only an
// `open` has that cross-check: see the silent-misplacement test below, which is
// the stronger case for P13.)
func TestEpilogueClassifiesAtActionEndNotAfterALaterActionChangedTheDisk(t *testing.T) {
	lineA := playLine(`cd "$root/$KILN_ACTION" && echo hi > L/x`, "a", 0)
	lineB := playLine(`cd "$root/`+actA+`" && rm L && ln -s d2 L`, "b", 0)
	f := newStepFixLinks(t, planOf([]string{lineA}, []string{lineB}, nil), map[string]string{
		actA + "/d1/keep": "1\n", actA + "/d2/keep": "2\n",
	}, map[string]string{actA + "/L": "d1"}, true)
	pkg := f.pre.Config.PackageDir(actA)
	f.give("a", actA, lineA, 0, cannedOpts{},
		`openat(AT_FDCWD, "L/x", O_WRONLY|O_CREAT|O_TRUNC, 0666) = 3<`+pkg+`/d1/x>`)
	f.give("b", actB, lineB, 0, cannedOpts{})
	if target, err := os.Readlink(filepath.Join(pkg, "L")); err != nil || target != "d1" {
		t.Fatalf("the fixture's tracked link: %q %v", target, err)
	}

	resA := f.must(actA, 0, 1, lineA)
	if resA.ExitCode != 0 || resA.Record != RecordWritten {
		t.Fatalf("%d %q %q", resA.ExitCode, resA.Record, resA.RecordWhy)
	}
	atEnd := f.readRecord(actA)
	if !atEnd.ParseOK || atEnd.Record == nil || atEnd.StepsRun != 1 || !atEnd.StepNetNS || atEnd.BindingID != f.pre.BindingID {
		t.Fatalf("%+v", atEnd)
	}
	rel := actA + "/"
	if !containsStr(atEnd.Record.Writes, rel+"d1/x") || unstable(atEnd.Record.Reasons) {
		t.Fatalf("classified at A's end the write is at d1/x and the path is stable: %v %v", atEnd.Record.Writes, atEnd.Record.Reasons)
	}

	// B runs and retargets the link.
	if res := f.must(actB, 0, 1, lineB); res.ExitCode != 0 || res.Record != RecordWritten {
		t.Fatalf("%d %q %q", res.ExitCode, res.Record, res.RecordWhy)
	}
	if target, err := os.Readlink(filepath.Join(pkg, "L")); err != nil || target != "d2" {
		t.Fatalf("B did not retarget the link: %q %v", target, err)
	}
	// What was written at A's end is untouched ...
	if again := f.readRecord(actA); !reflect.DeepEqual(again, atEnd) {
		t.Fatal("A's record changed after B ran")
	}
	// ... and classifying A now, as `collect` used to, charges A for B's change.
	late := f.classifyNow(actA, 0)
	if !late.ParseOK || late.Record == nil {
		t.Fatalf("%+v", late)
	}
	if !unstable(late.Record.Reasons) || late.WritesComplete {
		t.Fatalf("classified late, A must be unstable and incomplete (the premise of P13): complete %v, %v", late.WritesComplete, late.Record.Reasons)
	}
	t.Logf("at A's end: complete %v, reasons %v; late: complete %v, reasons %v", atEnd.WritesComplete, atEnd.Record.Reasons, late.WritesComplete, late.Record.Reasons)
}

func unstable(reasons []string) bool {
	for _, r := range reasons {
		if strings.Contains(r, "unstable-path") {
			return true
		}
	}
	return false
}

// --- when the epilogue runs -------------------------------------------------

func TestEpilogueRunsExactlyAtActionEnd(t *testing.T) {
	t.Run("not after a step that is not the last and succeeded", func(t *testing.T) {
		f := newStepFix(t)
		res := f.must(actB, 0, 2, "exit 0")
		if res.Record != "" || exists(RecordFile(f.p.dir, actB)) || exists(RecordFailedFile(f.p.dir, actB)) {
			t.Fatalf("classified half an action: %q", res.Record)
		}
		// The last step ends it.
		res = f.must(actB, 1, 2, "exit 0")
		if res.Record != RecordWritten || !exists(RecordFile(f.p.dir, actB)) {
			t.Fatalf("%q %q", res.Record, res.RecordWhy)
		}
	})
	t.Run("after a failing step that ends the action", func(t *testing.T) {
		f := newStepFix(t)
		res := f.must(actB, 0, 2, "exit 3")
		if res.ExitCode != 3 || res.Record != RecordWritten {
			t.Fatalf("%d %q %q", res.ExitCode, res.Record, res.RecordWhy)
		}
		rec := f.readRecord(actB)
		// The fake tracer's trace does not parse, so the record says so and carries
		// no raw record: never an empty one.
		if rec.ParseOK || rec.Record != nil || rec.StepsRun != 1 || len(rec.ParseReasons) == 0 || rec.WritesComplete {
			t.Fatalf("%+v", rec)
		}
		// A step that is asked to run after its action ended (a build tool that
		// went on) is refused and runs nothing: its record would be for fewer
		// steps than ran. The record is untouched.
		before, _ := os.ReadFile(RecordFile(f.p.dir, actB))
		runs := f.runs()
		f.refused(f.opts(actB, 1, 2, "exit 0"), "has already been classified")
		after, _ := os.ReadFile(RecordFile(f.p.dir, actB))
		if string(before) != string(after) || exists(RecordFailedFile(f.p.dir, actB)) || f.runs() != runs {
			t.Fatal("record.json was overwritten, or a record.failed was left")
		}
	})
	t.Run("after a step that did not start", func(t *testing.T) {
		f := newStepFix(t)
		if err := os.Chmod(f.p.strace, 0o644); err != nil {
			t.Fatal(err)
		}
		res := f.must(actB, 0, 2, "exit 0")
		if res.ExitCode != ExitRefused || res.Record != RecordWritten {
			t.Fatalf("%d %q %q", res.ExitCode, res.Record, res.RecordWhy)
		}
		rec := f.readRecord(actB)
		if rec.ParseOK || rec.StepsRun != 0 || rec.StepNetNS || !strings.Contains(strings.Join(rec.ParseReasons, "|"), "did not start") {
			t.Fatalf("%+v", rec)
		}
	})
	t.Run("a refused step classifies nothing", func(t *testing.T) {
		f := newStepFix(t)
		o := f.opts(actB, 1, 2, "exit 0") // step 1 before step 0
		f.refused(o, "before step 0")
		if exists(RecordFile(f.p.dir, actB)) || exists(RecordFailedFile(f.p.dir, actB)) {
			t.Fatal("a refused step wrote a record")
		}
	})
}

// --- the record's content ---------------------------------------------------

func TestEpilogueRecordContentForACannedTrace(t *testing.T) {
	line := playLine(`cd "$root/$KILN_ACTION"`, "z", 0)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	pkg := f.pre.Config.PackageDir(actZ)
	policy := f.p.root + "/code/kiln/legacy/pinned.json" // tracked, in no package
	f.give("z", actZ, line, 0, cannedOpts{},
		`openat(AT_FDCWD, `+straceStr(policy)+`, O_RDONLY) = 3<`+policy+`>`,
		`openat(AT_FDCWD, "out.txt", O_WRONLY|O_CREAT|O_TRUNC, 0666) = 4<`+pkg+`/out.txt>`)
	res := f.must(actZ, 0, 1, line)
	if res.Record != RecordWritten {
		t.Fatalf("%q %q", res.Record, res.RecordWhy)
	}
	raw, _ := os.ReadFile(RecordFile(f.p.dir, actZ))
	rec := f.readRecord(actZ)
	if !rec.ParseOK || len(rec.ParseReasons) != 0 || rec.Record == nil || rec.Record.Package != actZ {
		t.Fatalf("%+v", rec)
	}
	if !containsStr(rec.Record.Entries, "path:code/kiln/legacy/pinned.json") || !containsStr(rec.Record.Writes, actZ+"/out.txt") {
		t.Fatalf("entries %v writes %v", rec.Record.Entries, rec.Record.Writes)
	}
	// The file is exactly the encoder's bytes (compact, no newline) and decodes
	// back to the same record.
	again, err := EncodeClassified(rec)
	if err != nil || string(again) != string(raw) || strings.HasSuffix(string(raw), "\n") {
		t.Fatalf("%v", err)
	}
	if st, err := os.Stat(RecordFile(f.p.dir, actZ)); err != nil || st.Mode().Perm() != 0o600 {
		t.Fatalf("%v %v", st, err)
	}
	// writes_complete is the classifier's own answer, copied.
	if want := f.classifyNow(actZ, 0).WritesComplete; rec.WritesComplete != want {
		t.Fatalf("%v vs %v", rec.WritesComplete, want)
	}
}

// --- the parse half of G7 ---------------------------------------------------

func TestEpilogueParseHalfOfG7(t *testing.T) {
	type tc struct {
		name   string
		status int
		opts   cannedOpts
		want   string // a substring of the reasons; "" means parse_ok
	}
	cases := []tc{
		{"a healthy step", 0, cannedOpts{}, ""},
		{"a healthy step that exits 3", 3, cannedOpts{}, ""},
		{"a step that exits 125 with its execve present", 125, cannedOpts{}, ""},
		{"the root's exit line is not root_status", 3, cannedOpts{rootExit: 0, hasRootExit: true}, "root's exit line"},
		{"the root ran with another --config-sha256", 0, cannedOpts{rootSHA: strings.Repeat("a", 64)}, "another --config-sha256"},
		{"the init read another configuration", 0, cannedOpts{initSHA: strings.Repeat("b", 64)}, "another configuration"},
		{"the root's argv is cut", 0, cannedOpts{noRootArgv: true}, "--config-sha256 cannot be read"},
		{"the init's re-exec is missing", 0, cannedOpts{noInitExec: true}, "--sha256 cannot be read"},
		{"a refusal: the init died before the step", 125, cannedOpts{noStepExec: true}, "sandbox-refused"},
		{"a refusal: the root died before the init", 125, cannedOpts{noInit: true}, "sandbox-refused"},
		{"no step exec and not 125 is a broken trace", 3, cannedOpts{noStepExec: true}, "trace-incomplete:helper-no-exec"},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			line := playLine("", "z", c.status)
			f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
			f.give("z", actZ, line, c.status, c.opts)
			res := f.must(actZ, 0, 1, line)
			// The exit status is the step's, whatever the epilogue concluded.
			wantExit := c.status
			if res.ExitCode != wantExit || res.Record != RecordWritten {
				t.Fatalf("exit %d (want %d), record %q %q", res.ExitCode, wantExit, res.Record, res.RecordWhy)
			}
			rec := f.readRecord(actZ)
			if c.want == "" {
				if !rec.ParseOK || rec.Record == nil || len(rec.ParseReasons) != 0 {
					t.Fatalf("%+v", rec)
				}
				return
			}
			if rec.ParseOK || rec.Record != nil || rec.WritesComplete || !strings.Contains(strings.Join(rec.ParseReasons, "|"), c.want) {
				t.Fatalf("want a reason with %q: %+v", c.want, rec)
			}
		})
	}
}

// --- failures of the work itself --------------------------------------------

func TestEpilogueLeavesRecordFailedAndNeverChangesTheStatus(t *testing.T) {
	line := playLine("", "z", 7)
	setup := func(t *testing.T) *stepFix {
		f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
		f.give("z", actZ, line, 7, cannedOpts{})
		return f
	}
	check := func(t *testing.T, f *stepFix, o StepOptions, want string) {
		t.Helper()
		res, err := RunStep(o)
		if err != nil {
			t.Fatal(err)
		}
		if res.ExitCode != 7 || res.Record != RecordFailed || !strings.Contains(res.RecordWhy, want) {
			t.Fatalf("exit %d record %q why %q (want %q)", res.ExitCode, res.Record, res.RecordWhy, want)
		}
		if exists(RecordFile(f.p.dir, actZ)) {
			t.Fatal("a record.json beside the record.failed")
		}
		why, err := os.ReadFile(RecordFailedFile(f.p.dir, actZ))
		if err != nil || !strings.Contains(string(why), want) {
			t.Fatalf("%q %v", why, err)
		}
		// The sidecar is the step's own and unchanged by any of this.
		if sc := f.sidecar(actZ, 0); sc.RootStatus != 7 || !sc.Clean {
			t.Fatalf("%+v", sc)
		}
	}
	t.Run("traces over the cap", func(t *testing.T) {
		f := setup(t)
		o := f.opts(actZ, 0, 1, line)
		o.MaxTraceTotal = 50
		o.ChildCmd = func(args, env []string) *exec.Cmd {
			t.Error("started the classifier for an oversize action")
			return exec.Command("/usr/bin/true")
		}
		check(t, f, o, "traces-too-large")
	})
	t.Run("a sealed file that changed", func(t *testing.T) {
		// The step's own line (which runs after VerifyPre passed) appends to
		// graph.json: the epilogue reads it again and must find the digest wrong.
		swap := playLine(`g="$aux/../record/pre/graph.json"; chmod 600 "$g"; echo x >> "$g"`, "z", 7)
		f := newStepFixCustom(t, planOf(nil, nil, []string{swap}), nil, true)
		f.give("z", actZ, swap, 7, cannedOpts{})
		check(t, f, f.opts(actZ, 0, 1, swap), "was modified")
	})
}

// --- the serial lock and seq ------------------------------------------------

func TestSerialLockRefusesAnOverlappingStep(t *testing.T) {
	f := newStepFix(t)
	held, err := LockSerial(f.p.dir)
	if err != nil {
		t.Fatal(err)
	}
	before := f.runs()
	_, err = f.run(actB, 0, 2, "exit 0")
	if err == nil || !strings.Contains(err.Error(), "traces/.serial") {
		t.Fatalf("%v", err)
	}
	if f.runs() != before {
		t.Fatal("a step that could not take the lock was run")
	}
	if !exists(StepFile(f.p.dir, actB, 0, "overlap")) {
		t.Fatal("no overlap marker")
	}
	if exists(StepFile(f.p.dir, actB, 0, "trace")) || exists(StepFile(f.p.dir, actB, 0, "json")) {
		t.Fatal("a refused step left a trace or a sidecar")
	}
	held.Close()
	// Released: the step runs (the marker stays, for collect to void the shard).
	if res := f.must(actB, 0, 2, "exit 0"); res.Sidecar.Seq != 0 {
		t.Fatalf("seq %d: a refused step must not take a number", res.Sidecar.Seq)
	}
	if !exists(StepFile(f.p.dir, actB, 0, "overlap")) {
		t.Fatal("the marker went away")
	}
}

// The lock is also refused for an environment that names no action: the step
// still runs nothing, and there is simply no marker to leave.
func TestSerialLockRefusalWithoutAnActionLeavesNoMarker(t *testing.T) {
	f := newStepFix(t)
	held, err := LockSerial(f.p.dir)
	if err != nil {
		t.Fatal(err)
	}
	defer held.Close()
	o := f.opts(actB, 0, 2, "exit 0")
	o.Environ = []string{"junk"}
	if _, err := RunStep(o); err == nil {
		t.Fatal("ran")
	}
	for _, env := range [][]string{
		{EnvAction + "=../../x", EnvStep + "=0"},
		{EnvAction + "=" + actB, EnvStep + "=zero"},
		{EnvAction + "=" + actB},
		{EnvAction + "=" + actB, EnvAction + "=" + actB, EnvStep + "=0"},
	} {
		o.Environ = env
		if _, err := RunStep(o); err == nil {
			t.Fatal("ran")
		}
	}
	if exists(StepFile(f.p.dir, actB, 0, "overlap")) {
		t.Fatal("a marker for a step that could not be named")
	}
}

func TestSeqIsContiguousAcrossStepsAndActions(t *testing.T) {
	f := newStepFix(t)
	var got []int
	for _, s := range []struct {
		a     string
		n, of int
	}{{actA, 0, 1}, {actB, 0, 2}, {actB, 1, 2}, {actZ, 0, 1}} {
		line := "exit 0"
		if s.a == actA {
			line = planSteps[strings.Index(planSteps, "echo hi"):strings.Index(planSteps, `"]},{"package":"code/packages/x/b"`)]
		}
		got = append(got, f.must(s.a, s.n, s.of, line).Sidecar.Seq)
	}
	if !reflect.DeepEqual(got, []int{0, 1, 2, 3}) {
		t.Fatalf("%v", got)
	}
	if data, _ := os.ReadFile(filepath.Join(f.p.dir, "traces", ".seq")); string(data) != "4\n" {
		t.Fatalf("%q", data)
	}
	// The sidecars on disk say the same.
	if f.sidecar(actB, 1).Seq != 2 {
		t.Fatal("the file's seq differs")
	}
	// A refused step takes no number: step 0 of B again is refused (it ran).
	f.refused(f.opts(actB, 0, 2, "exit 0"), "already run")
	if data, _ := os.ReadFile(filepath.Join(f.p.dir, "traces", ".seq")); string(data) != "4\n" {
		t.Fatalf("a refused step advanced the counter: %q", data)
	}
}

func TestSeqCounterFileIsNeverSilentlyReset(t *testing.T) {
	for name, content := range map[string]string{
		"no newline":    "3",
		"negative":      "-1\n",
		"leading zero":  "01\n",
		"text":          "abc\n",
		"two lines":     "1\n2\n",
		"a huge number": "99999999999\n",
	} {
		t.Run(name, func(t *testing.T) {
			f := newStepFix(t)
			f.p.writable()
			os.Chmod(filepath.Join(f.p.dir, "traces"), 0o700)
			writeBytes(t, filepath.Join(f.p.dir, "traces", ".seq"), []byte(content), 0o600)
			f.refused(f.opts(actZ, 0, 1, "exit 0"), "traces/.seq")
			if exists(StepFile(f.p.dir, actZ, 0, "json")) {
				t.Fatal("ran")
			}
		})
	}
	// An EMPTY file is what a kill between a truncate and a write used to leave,
	// and used to read as 0 (a counter that restarts). It is refused, like a
	// missing one: `prepare` makes the file as "0\n".
	for name, mk := range map[string]func(p string){
		"an empty file":  func(p string) { writeBytes(t, p, nil, 0o600) },
		"a missing file": func(p string) { os.Remove(p) },
	} {
		t.Run(name, func(t *testing.T) {
			f := newStepFix(t)
			mk(filepath.Join(f.p.dir, "traces", ".seq"))
			f.refused(f.opts(actZ, 0, 1, "exit 0"), "traces/.seq")
			if exists(StepFile(f.p.dir, actZ, 0, "json")) {
				t.Fatal("ran")
			}
		})
	}
}

// prepare creates the counter as "0\n", private.
func TestPrepareCreatesTheSeqCounter(t *testing.T) {
	f := newStepFix(t)
	p := filepath.Join(f.p.dir, "traces", ".seq")
	if data, err := os.ReadFile(p); err != nil || string(data) != "0\n" {
		t.Fatalf("%q %v", data, err)
	}
	if st, _ := os.Stat(p); st.Mode().Perm() != 0o600 {
		t.Fatalf("%v", st.Mode())
	}
}

// The update is one write at offset 0, so every state the file can be in between
// two system calls is a whole number: counting up through the digit boundary
// never shortens it.
func TestSeqUpdateIsASingleOverwriteThatNeverShortens(t *testing.T) {
	d := canonTemp(t)
	os.Mkdir(filepath.Join(d, "traces"), 0o700)
	writeBytes(t, filepath.Join(d, "traces", ".seq"), []byte("8\n"), 0o600)
	l, err := LockSerial(d)
	if err != nil {
		t.Fatal(err)
	}
	defer l.Close()
	if err := l.OpenSeq(d); err != nil {
		t.Fatal(err)
	}
	var got []int
	for i := 0; i < 5; i++ {
		n, err := l.NextSeq()
		if err != nil {
			t.Fatal(err)
		}
		got = append(got, n)
		data, _ := os.ReadFile(filepath.Join(d, "traces", ".seq"))
		if want := fmt.Sprintf("%d\n", n+1); string(data) != want {
			t.Fatalf("after %d the file holds %q, want %q", n, data, want)
		}
	}
	if !reflect.DeepEqual(got, []int{8, 9, 10, 11, 12}) {
		t.Fatalf("%v", got)
	}
}

func TestSerialFilesMustBePrivateRegularFiles(t *testing.T) {
	for name, mk := range map[string]func(t *testing.T, dir string){
		"a symlink": func(t *testing.T, dir string) {
			other := filepath.Join(dir, "other")
			writeBytes(t, other, nil, 0o600)
			if err := os.Symlink(other, filepath.Join(dir, "traces", ".serial")); err != nil {
				t.Skip(err)
			}
		},
		"open to others": func(t *testing.T, dir string) {
			writeBytes(t, filepath.Join(dir, "traces", ".serial"), nil, 0o644)
			os.Chmod(filepath.Join(dir, "traces", ".serial"), 0o644)
		},
		"a directory": func(t *testing.T, dir string) {
			os.Mkdir(filepath.Join(dir, "traces", ".serial"), 0o700)
		},
	} {
		t.Run(name, func(t *testing.T) {
			f := newStepFix(t)
			mk(t, f.p.dir)
			if _, err := LockSerial(f.p.dir); err == nil {
				t.Fatal("locked")
			}
			f.refused(f.opts(actZ, 0, 1, "exit 0"), "traces")
		})
	}
	t.Run("traces is a symlink", func(t *testing.T) {
		d := canonTemp(t)
		other := filepath.Join(d, "other")
		os.Mkdir(other, 0o700)
		os.Symlink(other, filepath.Join(d, "traces"))
		if _, err := LockSerial(d); err == nil || !strings.Contains(err.Error(), "not a real directory") {
			t.Fatalf("%v", err)
		}
	})
	t.Run("traces cannot be made", func(t *testing.T) {
		if _, err := LockSerial(filepath.Join(canonTemp(t), "absent", "record")); err == nil {
			t.Fatal("locked")
		}
	})
	t.Run("NextSeq before OpenSeq", func(t *testing.T) {
		d := canonTemp(t)
		l, err := LockSerial(d)
		if err != nil {
			t.Fatal(err)
		}
		defer l.Close()
		if _, err := l.NextSeq(); err == nil {
			t.Fatal("a counter that was never opened")
		}
	})
}

// The step's lock order: directory lock (shared), then serial. A collect that
// holds the directory exclusively refuses the step before the serial lock is
// touched (no .serial file is made for it to hold).
func TestStepTakesTheDirectoryLockBeforeTheSerialLock(t *testing.T) {
	f := newStepFix(t)
	unlock, err := LockDir(f.p.dir, true)
	if err != nil {
		t.Fatal(err)
	}
	defer unlock()
	f.refused(f.opts(actZ, 0, 1, "exit 0"), "cannot be locked")
	if exists(filepath.Join(f.p.dir, "traces", ".serial")) {
		t.Fatal("the serial lock was taken before the directory lock")
	}
}

// An earlier step's sidecar that no longer decodes is a parse failure of the
// action, never a silent skip of that step.
func TestEpilogueAnUnreadableEarlierSidecarIsAParseFailure(t *testing.T) {
	l0 := playLine("", "b0", 0)
	l1 := playLine("", "b1", 0)
	f := newStepFixCustom(t, planOf(nil, []string{l0, l1}, nil), nil, true)
	f.give("b0", actB, l0, 0, cannedOpts{})
	f.give("b1", actB, l1, 0, cannedOpts{})
	f.must(actB, 0, 2, l0)
	if err := os.WriteFile(StepFile(f.p.dir, actB, 0, "json"), []byte("{"), 0o600); err != nil {
		t.Fatal(err)
	}
	res := f.must(actB, 1, 2, l1)
	if res.Record != RecordWritten {
		t.Fatalf("%q %q", res.Record, res.RecordWhy)
	}
	rec := f.readRecord(actB)
	if rec.ParseOK || rec.Record != nil || !strings.Contains(strings.Join(rec.ParseReasons, "|"), "step 0: its sidecar is unreadable") || rec.StepNetNS {
		t.Fatalf("%+v", rec)
	}
}

// Two steps that ran, each under its own canned trace, are classified together.
func TestEpilogueClassifiesAllStepsOfAnActionTogether(t *testing.T) {
	l0 := playLine("", "b0", 0)
	l1 := playLine("", "b1", 0)
	f := newStepFixCustom(t, planOf(nil, []string{l0, l1}, nil), nil, true)
	pkg := f.pre.Config.PackageDir(actB)
	f.give("b0", actB, l0, 0, cannedOpts{}, `openat(AT_FDCWD, "one.txt", O_WRONLY|O_CREAT, 0666) = 3<`+pkg+`/one.txt>`)
	f.give("b1", actB, l1, 0, cannedOpts{}, `openat(AT_FDCWD, "two.txt", O_WRONLY|O_CREAT, 0666) = 3<`+pkg+`/two.txt>`)
	if res := f.must(actB, 0, 2, l0); res.Record != "" {
		t.Fatalf("%q", res.Record)
	}
	if res := f.must(actB, 1, 2, l1); res.Record != RecordWritten {
		t.Fatalf("%q %q", res.Record, res.RecordWhy)
	}
	rec := f.readRecord(actB)
	if !rec.ParseOK || rec.StepsRun != 2 || !containsStr(rec.Record.Writes, actB+"/one.txt") || !containsStr(rec.Record.Writes, actB+"/two.txt") {
		t.Fatalf("%+v %+v", rec, rec.Record)
	}
	if reasons := strings.Join(rec.Record.Reasons, "|"); strings.Contains(reasons, "recipe-mismatch") {
		t.Fatalf("both steps ran, the recipe matches: %s", reasons)
	}
}

// A sealed policy file that no longer parses is a record.failed, found when the
// classifier reads it, not a crash and not an empty record.
func TestEpilogueSealedInputsThatDoNotParse(t *testing.T) {
	for name, rel := range map[string]string{"the graph": "graph.json", "the host facts": "host-facts.json"} {
		t.Run(name, func(t *testing.T) {
			line := playLine("", "z", 0)
			f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
			f.give("z", actZ, line, 0, cannedOpts{})
			f.reseal(map[string][]byte{rel: []byte("{")}, true)
			res, err := RunStep(f.opts(actZ, 0, 1, line))
			if err != nil {
				t.Fatal(err)
			}
			if res.ExitCode != 0 || res.Record != RecordFailed || !strings.Contains(res.RecordWhy, rel) {
				t.Fatalf("%d %q %q", res.ExitCode, res.Record, res.RecordWhy)
			}
		})
	}
}

// The SILENT case (review L-1). A link an action creates itself is resolved from
// the DISK too (only under the fresh $HOME and TMPDIR is the trace's link text
// used), and only an `open` is checked against the kernel's -y path. So an
// action that creates L -> d1 and then runs `mkdir L/sub` has its write placed
// by whatever L points at when the classifier looks. Classified at the action's
// end that is d1/sub. Classified after a later action retargeted L it is
// d2/sub, with the write set still declared complete: nothing says it moved.
// This is the stronger case for P13, and the test records it.
func TestEpilogueSilentMisplacementOfAWriteThroughALinkTheActionMade(t *testing.T) {
	lineA := playLine(`cd "$root/$KILN_ACTION" && ln -s d1 L && mkdir L/sub`, "a", 0)
	lineB := playLine(`cd "$root/`+actA+`" && rm L && ln -s d2 L`, "b", 0)
	f := newStepFixCustom(t, planOf([]string{lineA}, []string{lineB}, nil), map[string]string{
		actA + "/d1/keep": "1\n", actA + "/d2/keep": "2\n",
	}, true)
	f.give("a", actA, lineA, 0, cannedOpts{},
		`symlink("d1", "L") = 0`,
		`mkdir("L/sub", 0777) = 0`)
	f.give("b", actB, lineB, 0, cannedOpts{})

	if res := f.must(actA, 0, 1, lineA); res.ExitCode != 0 || res.Record != RecordWritten {
		t.Fatalf("%d %q %q", res.ExitCode, res.Record, res.RecordWhy)
	}
	atEnd := f.readRecord(actA)
	rel := actA + "/"
	if !atEnd.ParseOK || !containsStr(atEnd.Record.Writes, rel+"d1/sub") || containsStr(atEnd.Record.Writes, rel+"d2/sub") {
		t.Fatalf("at A's end the mkdir through L is at d1/sub: %v %v", atEnd.Record.Writes, atEnd.Record.Reasons)
	}
	if res := f.must(actB, 0, 1, lineB); res.ExitCode != 0 {
		t.Fatalf("%d", res.ExitCode)
	}
	late := f.classifyNow(actA, 0)
	if !late.ParseOK || !containsStr(late.Record.Writes, rel+"d2/sub") || containsStr(late.Record.Writes, rel+"d1/sub") {
		t.Fatalf("classified late the write moved to d2/sub: %v %v", late.Record.Writes, late.Record.Reasons)
	}
	// ... and nothing in the late record says so: no unstable-path, and the
	// classifier still claims the write set complete when it did at the end.
	if unstable(late.Record.Reasons) || late.WritesComplete != atEnd.WritesComplete {
		t.Fatalf("the late record was expected to be silently wrong: complete %v (end: %v), %v", late.WritesComplete, atEnd.WritesComplete, late.Record.Reasons)
	}
}

// An unclean tracer is never classified into a record that claims a write set:
// every unclean reason becomes a parse reason (review L-4).
func TestEpilogueAnUncleanStepMakesTheWriteSetUnknown(t *testing.T) {
	line := playLine(`echo noise >&2`, "z", 0) // the fake tracer's own stderr: not the step's
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f.give("z", actZ, line, 0, cannedOpts{})
	res := f.must(actZ, 0, 1, line)
	if res.Sidecar.Clean || res.Record != RecordWritten {
		t.Fatalf("%+v %q", res.Sidecar, res.Record)
	}
	rec := f.readRecord(actZ)
	if rec.ParseOK || rec.Record != nil || rec.WritesComplete || !strings.Contains(strings.Join(rec.ParseReasons, "|"), "step 0: unclean:"+UncleanStderr) {
		t.Fatalf("%+v", rec)
	}
}

// markOverlap only for a lock that is held (security review): a lock that cannot
// be taken for any other reason leaves no marker.
func TestOverlapMarkerOnlyWhenTheLockIsHeld(t *testing.T) {
	f := newStepFix(t)
	// traces/.serial is a directory: LockSerial fails, but not because it is held.
	os.Remove(filepath.Join(f.p.dir, "traces", ".serial"))
	if err := os.Mkdir(filepath.Join(f.p.dir, "traces", ".serial"), 0o700); err != nil {
		t.Fatal(err)
	}
	var busy *SerialBusyError
	_, err := LockSerial(f.p.dir)
	if err == nil || errors.As(err, &busy) {
		t.Fatalf("%v", err)
	}
	f.refused(f.opts(actB, 0, 2, "exit 0"), "traces")
	if exists(StepFile(f.p.dir, actB, 0, "overlap")) {
		t.Fatal("an overlap marker for a lock that was not held")
	}
	// A held lock is the typed error, and does earn the marker.
	os.Remove(filepath.Join(f.p.dir, "traces", ".serial"))
	held, err := LockSerial(f.p.dir)
	if err != nil {
		t.Fatal(err)
	}
	defer held.Close()
	_, err = LockSerial(f.p.dir)
	if !errors.As(err, &busy) || !errors.Is(err, syscall.EWOULDBLOCK) {
		t.Fatalf("%v", err)
	}
	f.refused(f.opts(actB, 0, 2, "exit 0"), "traces/.serial")
	if !exists(StepFile(f.p.dir, actB, 0, "overlap")) {
		t.Fatal("no marker for a held lock")
	}
}

// A sidecar must be the sidecar of the step and action it is found as.
func TestReadSidecarChecksItsOwnNames(t *testing.T) {
	f := newStepFix(t)
	f.must(actB, 0, 2, "exit 0")
	sc, err := readSidecar(f.p.dir, actB, 0)
	if err != nil || sc.Action != actB || sc.Step != 0 {
		t.Fatalf("%v", err)
	}
	// B's step 0 sidecar copied to the name of B's step 1, and of Z's step 0.
	data, _ := os.ReadFile(StepFile(f.p.dir, actB, 0, "json"))
	writeBytes(t, StepFile(f.p.dir, actB, 1, "json"), data, 0o600)
	if _, err := readSidecar(f.p.dir, actB, 1); err == nil || !strings.Contains(err.Error(), "is of step 0") {
		t.Fatalf("%v", err)
	}
	os.MkdirAll(StepDir(f.p.dir, actZ), 0o700)
	writeBytes(t, StepFile(f.p.dir, actZ, 0, "json"), data, 0o600)
	if _, err := readSidecar(f.p.dir, actZ, 0); err == nil || !strings.Contains(err.Error(), "is of step 0") {
		t.Fatalf("%v", err)
	}
	if _, err := readSidecar(f.p.dir, actZ, 5); err == nil {
		t.Fatal("a missing sidecar was read")
	}
}

// record.json carries the digest of the sealed directory it was made under.
func TestRecordCarriesTheSealedDigest(t *testing.T) {
	f := newStepFix(t)
	f.must(actZ, 0, 1, "exit 0")
	rec := f.readRecord(actZ)
	sealed, _ := os.ReadFile(filepath.Join(f.p.dir, "pre", "SEALED"))
	sum := sha256.Sum256(sealed)
	if rec.SealedSHA256 != hex.EncodeToString(sum[:]) || rec.SealedSHA256 != f.pre.SealedSHA256 {
		t.Fatalf("%s vs %x", rec.SealedSHA256, sum)
	}
}

func TestCheckTreeSize(t *testing.T) {
	if err := checkTreeSize(maxKeptFile); err != nil {
		t.Fatal(err)
	}
	if err := checkTreeSize(maxKeptFile + 1); err == nil || !strings.Contains(err.Error(), "tracked tree file") {
		t.Fatalf("%v", err)
	}
}
