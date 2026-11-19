//go:build unix

package record

// The census triage fields (KILN04, census section): refused:<syscall>, the
// stricter-only row names, redacted line shapes, sub-codes and op classes read
// back from census.json, the invoker's own labels, and built_ok / emittable.
// Every fixture is host-independent (the step fixture's canned traces, stubs
// for the cross-check): nothing here execs a real-looking host path.

import (
	"bytes"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// refused:<syscall> comes out of a record's parse reasons by name when the name
// is in the closed table, and as refused:other otherwise.
func TestCensusReportsRefusedSyscallNames(t *testing.T) {
	f := censusDirFix(t)
	putRecord(t, f, actA, func(r *ClassifiedRecord) {
		r.ParseOK, r.Record, r.WritesComplete = false, nil, false
		r.ParseReasons = []string{
			"step 0: trace-incomplete:refused:ptrace",
			"step 1: trace-incomplete:refused:/home/secret/token",
			"step 2: trace-incomplete:refused:clone3",
		}
	})
	_, s, _, mdPath := runCensus(t, f)
	got := s.Explain.RecordParseReasons
	if got["refused:ptrace"] != 1 || got["refused:clone3"] != 1 || got["refused:other"] != 1 || len(got) != 3 {
		t.Errorf("parse reasons: %v", got)
	}
	md, _ := os.ReadFile(mdPath)
	if !strings.Contains(string(md), "`refused:ptrace` 1;") || strings.Contains(string(md), "secret") {
		t.Errorf("the Markdown: %q", md)
	}
}

// A stub cross-check that names allowed rows: the histogram, the details and the
// outlines of the stopped lines all carry the row's NAME; the exit stays strict.
func TestCensusNamesTheAllowedRowAndShapesTheLine(t *testing.T) {
	f := censusDirFix(t)
	res, s, _, mdPath := runCensus(t, f, func(o *CensusRunOptions) {
		o.CrossCheck = func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, trace.CrossCheckDetail) {
			return nil, nil, nil, trace.CrossCheckDetail{
				Allowed: 2, Rows: map[string]int{"signed pid": 1, "trailing space": 1},
				Hits: []trace.AllowedHit{{Row: "signed pid", Line: 1}, {Row: "trailing space", Line: 2}},
			}
		}
	})
	if res.Exit != CensusExitMismatch {
		t.Errorf("exit %d: an allowed divergence is still strict", res.Exit)
	}
	cc := s.CrossCheck
	if cc.Allowed != 4 || cc.AllowedRows["signed pid"] != 2 || cc.AllowedRows["trailing space"] != 2 || len(cc.AllowedRows) != 2 {
		t.Errorf("allowed %d rows %v", cc.Allowed, cc.AllowedRows)
	}
	joined := strings.Join(cc.Details, "\n")
	if !strings.Contains(joined, "row: signed pid") || !strings.Contains(joined, "row: trailing space") || strings.Contains(joined, "divergence(s)") {
		t.Errorf("details: %v", cc.Details)
	}
	if len(s.Shapes) != 2 {
		t.Fatalf("shapes: %+v", s.Shapes)
	}
	for _, sh := range s.Shapes {
		if !strings.HasPrefix(sh.Origin, "allowed:") || sh.Count != 2 || len(sh.Lines) == 0 {
			t.Errorf("shape %+v", sh)
		}
		last := sh.Lines[len(sh.Lines)-1]
		if last.Role != "stop" || last.Line < 1 || !strings.HasPrefix(last.Tokens, "PID") {
			t.Errorf("stop line %+v", last)
		}
	}
	md, _ := os.ReadFile(mdPath)
	for _, want := range []string{"`signed pid` 2;", "## Line shapes of refusals and allowed divergences (redacted)", "`allowed:signed pid`"} {
		if !strings.Contains(string(md), want) {
			t.Errorf("the Markdown lacks %q", want)
		}
	}
}

// A refusal's line is shaped too, with the fixed code of the refusal as origin;
// at most maxCensusShapes distinct outlines are kept and the rest are counted.
func TestCensusShapesAreBounded(t *testing.T) {
	f := censusDirFix(t)
	_, s, _, _ := runCensus(t, f, func(o *CensusRunOptions) {
		o.CrossCheck = func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, trace.CrossCheckDetail) {
			return nil, &trace.Error{Reason: "unparsable-line", Line: 1}, nil, trace.CrossCheckDetail{}
		}
	})
	if len(s.Shapes) != 1 || s.Shapes[0].Origin != "refusal:unparsable-line" || s.Shapes[0].Count != 2 {
		t.Fatalf("shapes: %+v", s.Shapes)
	}

	// The budgets, on a trace of 20 distinct lines.
	dir := t.TempDir()
	tr := filepath.Join(dir, "0.trace")
	var b bytes.Buffer
	for i := 0; i < 20; i++ {
		b.WriteString("1  " + strings.Repeat("f(", i+1) + strings.Repeat(")", i+1) + " = 0\n")
	}
	if err := os.WriteFile(tr, b.Bytes(), 0o600); err != nil {
		t.Fatal(err)
	}
	size := int64(b.Len())
	adir := filepath.Join(dir, "x")
	newRun := func() (*censusRun, *censusAcc) {
		a := newCensusAcc(nil)
		return &censusRun{o: CensusOptions{}, acc: a}, a
	}

	// A refusal code gets maxCensusShapeCallsCode computations; the rest are counted.
	r, a := newRun()
	for i := 1; i <= 20; i++ {
		r.noteShape(adir, tr, 0, size, "refusal:size", i)
	}
	if len(a.shapes) != maxCensusShapeCallsCode || a.shapesDropped != 20-maxCensusShapeCallsCode {
		t.Errorf("kept %d, dropped %d", len(a.shapes), a.shapesDropped)
	}

	// A flood of one refusal cannot starve allowed:* lines: they have their own
	// budget (8 slots, 32 computations).
	r, a = newRun()
	for i := 1; i <= 10; i++ {
		r.noteShape(adir, tr, 0, size, "refusal:clone3", i)
	}
	for i := 1; i <= 20; i++ {
		r.noteShape(adir, tr, 0, size, "allowed:row "+strings.Repeat("x", i), i)
	}
	allowed := 0
	for _, sh := range a.shapes {
		if strings.HasPrefix(sh.Origin, "allowed:") {
			allowed++
		}
	}
	// Lines 1 and 2 were shaped for clone3 already, so they are not shaped again
	// (dedupe on trace path and line); the other 18 distinct lines fill 8 slots.
	if allowed != maxCensusShapes {
		t.Errorf("allowed shapes kept: %d, want %d (shapes %d)", allowed, maxCensusShapes, len(a.shapes))
	}

	// Refusal slots: 8 across codes, 2 computations per code, 32 in all.
	r, a = newRun()
	for i := 1; i <= 20; i++ {
		r.noteShape(adir, tr, 0, size, "refusal:code"+strings.Repeat("x", i), i)
	}
	if len(a.shapes) != maxCensusShapes {
		t.Errorf("refusal slots: %d, want %d", len(a.shapes), maxCensusShapes)
	}

	// The same (trace, line) is never shaped twice, and costs no budget.
	r, a = newRun()
	r.noteShape(adir, tr, 0, size, "allowed:a", 3)
	r.noteShape(adir, tr, 0, size, "allowed:b", 3)
	if a.shapeCalls["allowed"] != 1 || a.shapesDropped != 0 {
		t.Errorf("calls %v dropped %d", a.shapeCalls, a.shapesDropped)
	}

	// The byte budget: 2x each trace's size is charged against 4 GiB; a trace
	// that would exceed it is counted, not scanned.
	r, a = newRun()
	a.shapeBytes = maxCensusShapeBytes - 1
	r.noteShape(adir, tr, 0, size, "allowed:a", 1)
	if a.shapesDropped != 1 || len(a.shapes) != 0 || a.shapeCalls["allowed"] != 0 {
		t.Errorf("dropped %d shapes %d", a.shapesDropped, len(a.shapes))
	}

	// The computation cap per class.
	r, a = newRun()
	a.shapeCalls["allowed"] = maxCensusShapeCallsAllow
	r.noteShape(adir, tr, 0, size, "allowed:a", 1)
	if a.shapesDropped != 1 {
		t.Errorf("an over-budget outline was not counted")
	}
}

// END TO END, hostile: a real trace full of secret-looking text, refused by the
// real parser and cross-check.  The summary shows a redacted outline and not one
// byte of the secrets, in JSON or Markdown.
func TestCensusShapeLeaksNothingFromARealRefusedTrace(t *testing.T) {
	f := censusDirFix(t)
	secrets := []string{"zqSecretPath", "zqSecretAnnot", "zqSecretWord", "zqSecretStr", "hunter2"}
	hostile := "1  openat(AT_FDCWD<zqSecretAnnot>, \"/home/zqSecretPath/zqSecretStr\", zqSecretWord|O_RDONLY, hunter2) = 3</home/zqSecretPath>\n" +
		"1  openat(AT_FDCWD<zqSecretAnnot, \"zqSecretStr\n"
	if err := os.WriteFile(filepath.Join(StepDir(f.p.dir, actA), "0.trace"), []byte(hostile), 0o600); err != nil {
		t.Fatal(err)
	}
	_, s, jsonPath, mdPath := runCensus(t, f)
	if s.CrossCheck.Refused == 0 || len(s.Shapes) == 0 {
		t.Fatalf("the hostile trace was not refused and shaped: cross %+v shapes %+v problems %v", s.CrossCheck, s.Shapes, s.Problems)
	}
	if s.Shapes[0].Lines[len(s.Shapes[0].Lines)-1].Line != 2 {
		t.Errorf("shape of line %+v, want line 2 (where the grammar stopped)", s.Shapes[0].Lines)
	}
	for _, p := range []string{jsonPath, mdPath} {
		data, _ := os.ReadFile(p)
		for _, secret := range secrets {
			if bytes.Contains(data, []byte(secret)) {
				t.Errorf("%s leaks %q", filepath.Base(p), secret)
			}
		}
	}
}

// Sub-codes and op classes come back out of census.json: kinds of the form
// trace-incomplete:unstable-path:<sub> are ordinary kinds, ops are a per-kind
// breakdown of host-read by the closed op classes, and an op class outside the
// list makes the file invalid.
func TestCensusReadsSubCodesAndOps(t *testing.T) {
	f := censusDirFix(t)
	putCensusDoc(t, f, actZ, CensusDoc{Total: 6, Entries: []tracedEntry{
		{Kind: "host-read", Path: "/var/x/a", Count: 3},
		{Kind: "host-read", Path: "/var/x/b", Count: 1},
		{Kind: "trace-incomplete:unstable-path:dotdot", Path: "/w/repo/x/..", Count: 1},
		{Kind: "trace-incomplete:exec-unreadable:under-tmp", Path: "$HOME/tool", Count: 1},
	}, Ops: map[string]map[string]int64{"host-read": {"stat": 2, "absent-probe": 1, "listing": 1}}})
	putCensusDoc(t, f, actA, CensusDoc{Total: 2, Entries: []tracedEntry{{Kind: "host-read", Path: "/var/x/c", Count: 2}},
		Ops: map[string]map[string]int64{"host-read": {"stat": 1, "open": 1}}})
	_, s, _, mdPath := runCensus(t, f)
	kinds := map[string]KindSummary{}
	for _, k := range s.Reasons {
		kinds[k.Kind] = k
	}
	hr := kinds["host-read"]
	want := map[string]int64{"stat": 3, "absent-probe": 1, "listing": 1, "open": 1}
	if len(hr.Ops) != len(want) {
		t.Errorf("ops %v, want %v", hr.Ops, want)
	}
	for c, n := range want {
		if hr.Ops[c] != n {
			t.Errorf("ops %v, want %v", hr.Ops, want)
			break
		}
	}
	if kinds["trace-incomplete:unstable-path:dotdot"].Occurrences != 1 || kinds["trace-incomplete:exec-unreadable:under-tmp"].TopPaths[0].Path != "$HOME/tool" {
		t.Errorf("sub-code kinds: %+v", kinds)
	}
	if kinds["trace-incomplete:unstable-path:dotdot"].Ops != nil {
		t.Error("a kind that is not split has ops")
	}
	md, _ := os.ReadFile(mdPath)
	if !strings.Contains(string(md), "by the kind of look that raised it: `absent-probe` 1; `listing` 1; `open` 1; `stat` 3;") {
		t.Errorf("the Markdown lacks the op-class breakdown")
	}

	// A class outside the closed list invalidates the file, and nothing of it is read.
	putCensusDoc(t, f, actZ, CensusDoc{Total: 1, Entries: []tracedEntry{{Kind: "host-read", Path: "/var/x/a", Count: 1}},
		Ops: map[string]map[string]int64{"host-read": {"::error::pwn": 1}}})
	_, s, _, _ = runCensus(t, f)
	if s.Problems["census_bad_entry"] != 1 || s.Actions.CensusInvalid != 1 {
		t.Errorf("problems %v actions %+v", s.Problems, s.Actions)
	}
}

// $HOME and $TMPDIR keep meaning the STEP's: the invoker's own directories get
// $RUNNER_HOME and $RUNNER_TMP unless they are the very directories the sealed
// host facts name.
func TestCensusLabelsTheInvokersDirectoriesDistinctly(t *testing.T) {
	f := censusDirFix(t)
	pre, err := VerifyPre(f.p.dir)
	if err != nil {
		t.Fatal(err)
	}
	stepHome, stepTmp := sealedHomeTmp(pre)
	if stepHome == "" || stepTmp == "" {
		t.Skipf("the fixture's host facts name no home (%q) or tmpdir (%q)", stepHome, stepTmp)
	}
	putCensusDoc(t, f, actZ, CensusDoc{Total: 3, Entries: []tracedEntry{
		{Kind: "host-read", Path: "/home/runner/.cache/x", Count: 1},
		{Kind: "host-read", Path: "/var/tmp/job7/out", Count: 1},
		{Kind: "host-read", Path: "/var/other/y", Count: 1},
	}})
	_, s, _, _ := runCensus(t, f, func(o *CensusRunOptions) {
		// The invoker's (different from the sealed facts), then the step's own
		// (the same directory: keeps its label).
		o.Redact = []Redaction{
			{From: "/home/runner", To: "$HOME"}, {From: "/var/tmp/job7", To: "$TMPDIR"},
			{From: "/var/other", To: "$WORKSPACE"},
		}
	})
	var paths []string
	for _, p := range s.Reasons[0].TopPaths {
		paths = append(paths, p.Path)
	}
	joined := strings.Join(paths, "\n")
	for _, want := range []string{"$RUNNER_HOME/.cache/x", "$RUNNER_TMP/out", "$WORKSPACE/y"} {
		if !strings.Contains(joined, want) {
			t.Errorf("no %q in %v", want, paths)
		}
	}
	if strings.Contains(joined, "$HOME") || strings.Contains(joined, "$TMPDIR") {
		t.Errorf("the invoker's directory kept the step's label: %v", paths)
	}

	got := relabelRunner([]Redaction{{From: stepHome, To: "$HOME"}, {From: stepTmp, To: "$TMPDIR"}, {From: "/x/y", To: "$HOME"}}, stepHome, stepTmp)
	if got[0].To != "$HOME" || got[1].To != "$TMPDIR" || got[2].To != "$RUNNER_HOME" {
		t.Errorf("relabelRunner: %+v", got)
	}
	// With no sealed facts every one is the invoker's.
	got = relabelRunner([]Redaction{{From: "/a/b", To: "$HOME"}, {From: "/c/d", To: "$TMPDIR"}, {From: "/e/f", To: "$WORKSPACE"}}, "", "")
	if got[0].To != "$RUNNER_HOME" || got[1].To != "$RUNNER_TMP" || got[2].To != "$WORKSPACE" {
		t.Errorf("relabelRunner without facts: %+v", got)
	}
}

// built_ok and emittable per language, and the shares over all actions and over
// built_ok.  "recordable" (parse ok + writes complete) is not "would emit".
func TestCensusLanguagesBuiltOKAndEmittable(t *testing.T) {
	f := censusDirFix(t)
	// Z: recordable but with a host-read reason: built ok, not emittable.
	putRecord(t, f, actZ, func(r *ClassifiedRecord) { r.Record.Reasons = []string{"host-read:/etc/x"} })
	// A: the canned fixture's record carries a reason (an exec-format one: its
	// "/bin/sh" is not an ELF image) although its census is EMPTY, which is
	// exactly why "clean" must not read as "would emit".  Clear it: built ok and
	// emittable.
	putRecord(t, f, actA, func(r *ClassifiedRecord) { r.Record.Reasons = []string{} })
	_, s, _, mdPath := runCensus(t, f)
	if len(s.Languages) != 1 {
		t.Fatalf("languages %+v", s.Languages)
	}
	l := s.Languages[0]
	if l.Actions != 2 || l.Recordable != 2 || l.BuiltOK != 2 || l.Emittable != 1 {
		t.Errorf("language %+v", l)
	}
	if l.Share != 1 || l.RecordableShareOfBuiltOK != 1 || l.EmittableShare != 0.5 || l.EmittableShareOfBuiltOK != 0.5 {
		t.Errorf("shares %+v", l)
	}
	if s.Actions.BuiltOK != 2 || s.Actions.Emittable != 1 || s.Actions.ShardVoid {
		t.Errorf("actions %+v", s.Actions)
	}
	md, _ := os.ReadFile(mdPath)
	for _, want := range []string{"## Share by language", "NOT necessarily emittable", "would emit, upper bound (zero reasons, no gate, keyed, shard not void) 1 = 50.0%"} {
		if !strings.Contains(string(md), want) {
			t.Errorf("the Markdown lacks %q:\n%s", want, md)
		}
	}
	for _, line := range strings.Split(string(md), "\n") {
		if strings.Contains(line, "built ok") && strings.Contains(line, "clean") {
			t.Errorf("the old 'clean' wording is on the language line: %s", line)
		}
	}

	// A failed build is not built_ok, even though its record parsed.
	putRecord(t, f, actZ, func(r *ClassifiedRecord) { r.Record.Reasons = []string{"failed"} })
	_, s, _, _ = runCensus(t, f)
	if l := s.Languages[0]; l.BuiltOK != 1 || l.Emittable != 1 || l.EmittableShareOfBuiltOK != 1 || l.EmittableShare != 0.5 {
		t.Errorf("language %+v", l)
	}
	// A record whose parse half failed: built_ok follows the sidecars (all exited
	// 0 here), and it is never emittable.
	putRecord(t, f, actA, func(r *ClassifiedRecord) {
		r.ParseOK, r.Record, r.WritesComplete = false, nil, false
		r.ParseReasons = []string{"step 0: trace-incomplete:unparsable-line"}
	})
	_, s, _, _ = runCensus(t, f)
	if l := s.Languages[0]; l.BuiltOK != 1 || l.Emittable != 0 {
		t.Errorf("language %+v", l)
	}
}

// A collect header that names a void reason gates every record: nothing is
// emittable, and the summary says why.
func TestCensusGatedShardEmitsNothing(t *testing.T) {
	a := newCensusAcc(nil)
	a.langs["go"] = &langAcc{actions: 4, recordable: 4, builtOK: 4, emittable: 4}
	a.actions.Emittable = 4
	a.collect = &CollectSummary{Void: []string{"post-inventory"}}
	s := a.summary(10)
	if !s.Actions.ShardVoid || s.Actions.Emittable != 0 || s.Languages[0].Emittable != 0 || s.Languages[0].EmittableShare != 0 {
		t.Errorf("gated summary: %+v %+v", s.Actions, s.Languages[0])
	}
	if md, _, _ := RenderCensusMarkdown(s); !strings.Contains(md, "SHARD VOID") {
		t.Error("the Markdown does not say the shard is gated")
	}
	a.collect = &CollectSummary{}
	if s := a.summary(10); s.Actions.ShardVoid || s.Languages[0].Emittable != 4 {
		t.Errorf("ungated summary: %+v", s.Actions)
	}
}
