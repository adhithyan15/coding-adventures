package record

// The classification epilogue of `kiln record step` (KILN04 section 4.8,
// decision P13)
// ====================================================================
//
// Why it is here. `traced.Classify` resolves the paths strace printed against
// the DISK. If it ran after the whole build, an action that ran later could have
// replaced a link, a directory or a file an earlier action resolved through, and
// the earlier action's writes and observations would be placed wrongly:
//
//	action A    ln -s d1 L ; echo x > L/x         the write is at d1/x
//	action B    rm L ; ln -s d2 L                 L now points at d2
//	classify A  after B:  L/x resolves to d2/x    WRONG, and nothing says so
//
// The merge's cross-action rules would catch some of this in the strict verdict
// and none of it in the relaxed one. So each action is classified the moment it
// ends, by the supervisor that ran its last step, while recording is serial
// (the `traces/.serial` lock, P13): no other action can have touched the disk
// since the action's last command returned.
//
// When it runs. After the sidecar of the action's LAST step, or of a step that
// did not start, or that exited non-zero (the build tool stops an action at its
// first failing step, so that is where the action ends), and still under the
// serial lock. Never earlier: a classification of half an action is worthless.
//
// What it does, in order (all under one time budget):
//
//	1  oversize?   traces of the action together over 2 GiB: record.failed
//	2  parse       each step's trace, in the sandbox-helper mode, plus the
//	               parse half of the completeness rule (G7):
//	                 - the trace parses;
//	                 - the root's exit line equals the sidecar's root_status;
//	                 - the traced `execve` of the init carries --sha256, and the
//	                   root's carries --config-sha256, both equal to the
//	                   binding's sandbox_config_sha256 (the configuration the
//	                   step really ran under is the sealed one);
//	                 - a status of 125 is "the sandbox refused" exactly when the
//	                   step's own `execve` is absent from the trace, otherwise it
//	                   is a step that exited 125.
//	               Any failure: the record says so (parse_ok false, the reasons),
//	               and carries NO raw record. A failed parse is never replaced by
//	               an empty record: `collect` turns it into unknown_writes.
//	3  classify    traced.Classify with the sealed tree, the live no-follow disk
//	               view, and the sealed policy (see classifyInputs)
//	4  write       traces/<sha256(action)>/record.json, O_EXCL
//
// If the work panics, or exceeds 10 minutes, or fails for any other reason than
// a trace that does not parse, `record.failed` is written instead (O_EXCL, with
// the reason as text). Nothing here changes the step's exit status: the build
// must see the BUILD line's own result, and `collect` reads the verdict from the
// files.
//
// Why it cannot trust the files it reads. Everything it reads from `pre/` is
// re-hashed against SEALED as it is read (Pre.ReadSealed); the traces are
// supervisor output on disk the sandbox cannot reach; the live disk is read
// only through LiveFS, which follows no link and opens nothing but regular
// files.

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"sort"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// ClassifiedSchema is the schema of record.json.
const ClassifiedSchema = "kiln-record-classified/1"

// The default limits of the epilogue (decision P13).
const (
	DefaultClassifyTimeout       = 10 * time.Minute
	DefaultMaxTraceTotal   int64 = 2 << 30
)

// ClassifiedRecord is `traces/<sha256(action)>/record.json`: what the epilogue
// concluded about one action, and what `collect` needs to build a
// recfmt.ShardRecord without parsing a trace itself.
type ClassifiedRecord struct {
	Schema    string `json:"schema"`
	Action    string `json:"action"`
	BindingID string `json:"binding_id"`
	// StepsRun is how many steps of the action started (had a traced run).
	StepsRun int `json:"steps_run"`
	// StepNetNS is traced.Input.StepNetNS: every step ran under the sandbox
	// helper that unshares the network namespace.
	StepNetNS bool `json:"step_netns"`
	// ParseOK says every trace passed the parse half of G7. When it is false,
	// ParseReasons says why and Record is null: the action's write set is
	// unknown.
	ParseOK      bool     `json:"parse_ok"`
	ParseReasons []string `json:"parse_reasons"`
	// WritesComplete is the classifier's answer (RawRecord.WritesComplete, which
	// is not part of the raw record's own JSON); false whenever Record is null.
	WritesComplete bool              `json:"writes_complete"`
	Record         *traced.RawRecord `json:"record"`
}

// Validate checks what a record's own fields promise.
func (c *ClassifiedRecord) Validate() error {
	if c.Schema != ClassifiedSchema {
		return fmt.Errorf("classified record: schema %q", printable(c.Schema))
	}
	if err := recfmt.CheckPath(c.Action); err != nil {
		return fmt.Errorf("classified record: action: %v", err)
	}
	if !hexDigest(c.BindingID) {
		return fmt.Errorf("classified record: binding_id is not a SHA-256 in lowercase hex")
	}
	if c.StepsRun < 0 {
		return fmt.Errorf("classified record: steps_run %d", c.StepsRun)
	}
	if c.ParseReasons == nil {
		return fmt.Errorf("classified record: parse_reasons is null")
	}
	for i := 1; i < len(c.ParseReasons); i++ {
		if c.ParseReasons[i-1] >= c.ParseReasons[i] {
			return fmt.Errorf("classified record: parse_reasons are not strictly sorted")
		}
	}
	if c.ParseOK != (len(c.ParseReasons) == 0) {
		return fmt.Errorf("classified record: parse_ok is %v with %d reasons", c.ParseOK, len(c.ParseReasons))
	}
	if c.ParseOK != (c.Record != nil) {
		return fmt.Errorf("classified record: parse_ok is %v but the raw record is %v", c.ParseOK, c.Record != nil)
	}
	if c.Record == nil && c.WritesComplete {
		return fmt.Errorf("classified record: writes_complete without a raw record")
	}
	if c.Record != nil && c.Record.Package != c.Action {
		return fmt.Errorf("classified record: the raw record is of another package")
	}
	return nil
}

// EncodeClassified validates and renders record.json (compact, no trailing
// newline).
func EncodeClassified(c ClassifiedRecord) ([]byte, error) {
	if c.ParseReasons == nil {
		c.ParseReasons = []string{}
	}
	if err := c.Validate(); err != nil {
		return nil, err
	}
	return marshalJSON(c)
}

// DecodeClassified reads record.json strictly.
func DecodeClassified(data []byte) (ClassifiedRecord, error) {
	var c ClassifiedRecord
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&c); err != nil {
		return ClassifiedRecord{}, fmt.Errorf("classified record: %w", err)
	}
	if _, err := dec.Token(); err != io.EOF {
		return ClassifiedRecord{}, fmt.Errorf("classified record: unexpected data after the JSON value")
	}
	return c, c.Validate()
}

// RecordFile and RecordFailedFile are the epilogue's two possible outputs for
// an action.
func RecordFile(dir, action string) string       { return StepDir(dir, action) + "/record.json" }
func RecordFailedFile(dir, action string) string { return StepDir(dir, action) + "/record.failed" }

// Outcomes of the epilogue (StepResult.Record).
const (
	RecordWritten = "record.json"
	RecordFailed  = "record.failed"
)

// classifyEpilogue runs the epilogue for the action whose step `step` has just
// ended (sidecar last). It returns the outcome and, for RecordFailed, the
// reason. It never returns an error: whatever goes wrong is a record.failed,
// and the step's exit status is the caller's business.
func classifyEpilogue(o StepOptions, pre *Pre, action string, step int, last StepSidecar) (outcome, why string) {
	fail := func(reason string) (string, string) {
		reason = traced.Sanitize(reason)
		if err := writeExcl(RecordFailedFile(pre.Dir, action), []byte(reason)); err != nil && !errors.Is(err, os.ErrExist) {
			return RecordFailed, reason + " (and the marker could not be written: " + err.Error() + ")"
		}
		return RecordFailed, reason
	}
	// 1. Oversize. The sizes are of the files, not the sidecars' claims.
	var total int64
	for j := 0; j <= step; j++ {
		if st, err := os.Lstat(StepFile(pre.Dir, action, j, "trace")); err == nil {
			total += st.Size()
		}
	}
	if total > o.MaxTraceTotal {
		return fail(fmt.Sprintf("traces-too-large: %d bytes over %d", total, o.MaxTraceTotal))
	}
	// 2 and 3, under the time budget, with a panic caught.
	type result struct {
		rec ClassifiedRecord
		err error
	}
	done := make(chan result, 1)
	go func() {
		defer func() {
			if r := recover(); r != nil {
				done <- result{err: fmt.Errorf("panic: %v", r)}
			}
		}()
		rec, err := classifyAction(o, pre, action, step, last)
		done <- result{rec: rec, err: err}
	}()
	timer := time.NewTimer(o.ClassifyTimeout)
	defer timer.Stop()
	var res result
	select {
	case res = <-done:
	case <-timer.C:
		// The work goroutine is abandoned; the process exits right after, which
		// ends it. It writes nothing.
		return fail(fmt.Sprintf("timeout: classification took longer than %s", o.ClassifyTimeout))
	}
	if res.err != nil {
		return fail(res.err.Error())
	}
	data, err := EncodeClassified(res.rec)
	if err != nil {
		return fail("encode: " + err.Error())
	}
	if err := writeExcl(RecordFile(pre.Dir, action), data); err != nil {
		if errors.Is(err, os.ErrExist) {
			// A second classification of one action means a step ran after the
			// action's end: what is on disk is for fewer steps than ran.
			return fail("classified-twice: record.json already exists")
		}
		return fail("record.json: " + err.Error())
	}
	return RecordWritten, ""
}

// classifyAction is steps 2 and 3: parse every step's trace with the parse half
// of G7, then classify. It reads the disk as it is NOW and writes nothing.
func classifyAction(o StepOptions, pre *Pre, action string, step int, last StepSidecar) (ClassifiedRecord, error) {
	planned, _ := pre.Action(action)
	rec := ClassifiedRecord{Schema: ClassifiedSchema, Action: action, BindingID: pre.BindingID, ParseReasons: []string{}}
	pkgDir := pre.Config.PackageDir(action)
	reasons := map[string]bool{}
	var traces []*trace.Trace
	netns := true
	for j := 0; j <= step; j++ {
		sc := last
		if j != step {
			var err error
			if sc, err = readSidecar(pre.Dir, action, j); err != nil {
				reasons[fmt.Sprintf("step %d: its sidecar is unreadable", j)] = true
				netns = false
				continue
			}
		}
		if sc.BindingID != pre.BindingID {
			reasons[fmt.Sprintf("step %d: run under another binding", j)] = true
		}
		if sc.Started {
			rec.StepsRun++
		}
		netns = netns && sc.StepNetNS
		tr, why := parseStep(pre, action, j, sc, pkgDir)
		if why != "" {
			reasons[fmt.Sprintf("step %d: %s", j, why)] = true
			continue
		}
		traces = append(traces, tr)
	}
	rec.StepNetNS = netns
	if len(reasons) > 0 {
		for r := range reasons {
			rec.ParseReasons = append(rec.ParseReasons, r)
		}
		sort.Strings(rec.ParseReasons)
		return rec, nil
	}
	cfg, err := classifyInputs(o, pre, planned)
	if err != nil {
		return ClassifiedRecord{}, err
	}
	raw, err := o.Classify(cfg, traced.Input{Package: action, Traces: traces, Recipe: planned.Steps, StepNetNS: netns})
	if err != nil {
		return ClassifiedRecord{}, fmt.Errorf("classify: %w", err)
	}
	rec.ParseOK = true
	rec.Record = raw
	rec.WritesComplete = raw.WritesComplete
	return rec, nil
}

// readSidecar reads and strictly decodes an earlier step's sidecar.
func readSidecar(dir, action string, step int) (StepSidecar, error) {
	data, err := os.ReadFile(StepFile(dir, action, step, "json"))
	if err != nil {
		return StepSidecar{}, err
	}
	return DecodeStepSidecar(data)
}

// parseStep parses one step's trace and applies the parse half of G7. It
// returns the trace, or a reason (never both).
func parseStep(pre *Pre, action string, step int, sc StepSidecar, pkgDir string) (*trace.Trace, string) {
	if !sc.Started {
		return nil, "the step did not start"
	}
	f, err := openNoFollow(StepFile(pre.Dir, action, step, "trace"))
	if err != nil {
		return nil, "the trace cannot be opened"
	}
	defer f.Close()
	if st, err := f.Stat(); err != nil || !st.Mode().IsRegular() {
		return nil, "the trace is not a regular file"
	}
	tr, err := trace.ParseOptions(f, pkgDir, trace.Options{HelperRoot: true, HelperExe: pre.Config.KilnExe})
	if err != nil {
		var te *trace.Error
		// Exit status 125 with no step exec in the trace is the sandbox refusing
		// to start the step: the parser says "helper-no-init" when the root died
		// before it made the init child and "helper-no-exec" when the init died
		// before the step.
		if sc.Exit125 && errors.As(err, &te) && (te.Reason == "helper-no-exec" || te.Reason == "helper-no-init") {
			return nil, "sandbox-refused"
		}
		return nil, traced.Sanitize(err.Error())
	}
	if tr.StepEvent() == nil {
		if sc.Exit125 {
			return nil, "sandbox-refused"
		}
		return nil, "the step's own execve is not in the trace"
	}
	if root := tr.Procs[tr.RootPid]; root == nil || root.Exit == nil || root.Exit.Killed || root.Exit.Code != sc.RootStatus {
		return nil, "the root's exit line is not the sidecar's root_status"
	}
	want := pre.Binding.SandboxConfigSHA256
	if got, ok := flagValue(tr.RootExecArgv, "--config-sha256"); !ok {
		return nil, "the root's --config-sha256 cannot be read from the trace"
	} else if got != want {
		return nil, "the root ran with another --config-sha256 than the sealed configuration's"
	}
	if got, ok := flagValue(tr.InitExecArgv, "--sha256"); !ok {
		return nil, "the init's --sha256 cannot be read from the trace"
	} else if got != want {
		return nil, "the init read another configuration than the sealed one"
	}
	return tr, ""
}

// flagValue finds `name` among the arguments before the first "--" and returns
// the argument after it. It must appear exactly once.
func flagValue(argv []string, name string) (string, bool) {
	found, val := 0, ""
	for i := 0; i < len(argv); i++ {
		if argv[i] == "--" {
			break
		}
		if argv[i] == name && i+1 < len(argv) {
			found++
			val = argv[i+1]
			i++
		}
	}
	if found != 1 {
		return "", false
	}
	return val, true
}

// classifyInputs builds the traced.Config from the sealed files and the live
// disk: nothing here asks Git, and nothing is built from the repository's own
// policy files (they were read by `prepare`, which sealed what they said).
func classifyInputs(o StepOptions, pre *Pre, planned Action) (traced.Config, error) {
	tree, err := pre.Tree()
	if err != nil {
		return traced.Config{}, err
	}
	graphBytes, err := pre.ReadSealed("graph.json")
	if err != nil {
		return traced.Config{}, err
	}
	graph, err := legacy.ParseGraph(graphBytes)
	if err != nil {
		return traced.Config{}, fmt.Errorf("pre/graph.json: %w", err)
	}
	factsBytes, err := pre.ReadSealed("host-facts.json")
	if err != nil {
		return traced.Config{}, err
	}
	facts, err := recfmt.DecodeHostFacts(factsBytes)
	if err != nil {
		return traced.Config{}, fmt.Errorf("pre/host-facts.json: %w", err)
	}
	rels := make([]string, 0, len(graph.Packages))
	for _, p := range graph.Packages {
		rels = append(rels, p.RelPath)
	}
	roots := pre.Classify.HostRoots
	pinned := map[string]bool{}
	for _, e := range pre.Classify.Pinned {
		pinned[e] = true
	}
	tools := map[string]bool{}
	for _, t := range planned.Tools {
		tools[t] = true
	}
	return traced.Config{
		Root: facts.Root, Packages: traced.NewPackageIndex(rels),
		Tree: NewDiskTree(tree, facts.Root), FS: o.DiskFS,
		HostRoots: &roots, Pinned: pinned, Tools: tools,
		Home: facts.Home, Tmpdir: facts.Tmpdir, JobStart: time.Unix(pre.Binding.JobStart, 0).UTC(),
	}, nil
}
