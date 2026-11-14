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
//	1  oversize?   the action's traces together over 2 GiB: record.failed (each
//	               trace is also bounded by the parser's limits, see
//	               ClassifyLimits: 1 GiB and 8 million events a step)
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
//	4  write       traces/<sha256(action)>/record.json, atomically (written to a
//	               temporary name and link(2)ed into place, so it is whole or
//	               absent)
//
// Steps 2 to 4 run in a CHILD process (`kiln record classify-action`, see
// classify_child.go) with a memory bound and a 10 minute deadline the parent
// enforces. If the child panics, runs out of memory, is killed, exceeds the
// deadline, or fails for any other reason than a trace that does not parse,
// `record.failed` is written instead (atomically, with the reason as text). The
// epilogue can therefore add up to 10 minutes to the last step of an action.
// Nothing here changes the step's exit status: the build must see the BUILD
// line's own result, and `collect` reads the verdict from the files.
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

// A BUILD line over 4096 bytes can never be recorded: strace runs with `-s 4096`,
// so the step's `execve` argv is cut ("...") and cannot be decoded exactly. The
// recipe check and the digest check of the parse half of G7 then fail, and the
// action's write set is unknown.

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
	// SealedSHA256 is the digest of pre/SEALED the record was made under (Pre.
	// SealedSHA256): `collect` requires it to equal the sealed directory's own.
	SealedSHA256 string `json:"sealed_sha256"`
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
	if !hexDigest(c.BindingID) || !hexDigest(c.SealedSHA256) {
		return fmt.Errorf("classified record: binding_id and sealed_sha256 must be SHA-256 digests in lowercase hex")
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

// writeAtomic puts data at final so that final either does not exist or holds
// ALL of data: it is written to a temporary name (O_EXCL, fsynced) and then
// link(2)ed into place, which fails with EEXIST rather than replace anything.
// A kill in the middle leaves at most a stray `<final>.tmp-<pid>` file, never a
// half-written record.json for `collect` to read.
func writeAtomic(final string, data []byte) error {
	tmp := fmt.Sprintf("%s.tmp-%d", final, os.Getpid())
	f, err := os.OpenFile(tmp, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return err
	}
	_, err = f.Write(data)
	if err == nil {
		err = f.Sync()
	}
	if cerr := f.Close(); err == nil {
		err = cerr
	}
	if err == nil {
		err = os.Link(tmp, final)
	}
	os.Remove(tmp)
	return err
}

// classifyEpilogue runs the epilogue for the action whose step `step` has just
// ended. It returns the outcome and, for RecordFailed, the reason. It never
// returns an error: whatever goes wrong is a record.failed, and the step's exit
// status is the caller's business.
//
// The work is done by a CHILD process (`kiln record classify-action`, see
// classify_child.go), not by a goroutine of the supervisor. A goroutine can
// recover from a panic but not from running out of memory (the runtime aborts
// the whole process) or from any other fatal runtime error, and the supervisor
// must outlive all of that with the step's exit status intact. The child is a
// separate process with its own bounded memory and a deadline the parent
// enforces by killing it; any way it can end other than "exit 0 and a valid
// record.json" is a record.failed.
func classifyEpilogue(o StepOptions, pre *Pre, kilnPath, action string, step int) (outcome, why string) {
	fail := func(reason string) (string, string) {
		reason = traced.Sanitize(reason)
		if err := writeAtomic(RecordFailedFile(pre.Dir, action), []byte(reason)); err != nil && !errors.Is(err, os.ErrExist) {
			return RecordFailed, reason + " (and record.failed could not be written: " + err.Error() + ")"
		}
		return RecordFailed, reason
	}
	// 1. Oversize: all the action's traces together. (Each one is also bounded
	// by the parser's own limit, ClassifyMaxStepBytes.) The sizes are of the
	// files, not the sidecars' claims.
	var total int64
	for j := 0; j <= step; j++ {
		if st, err := os.Lstat(StepFile(pre.Dir, action, j, "trace")); err == nil {
			total += st.Size()
		}
	}
	if total > o.MaxTraceTotal {
		return fail(fmt.Sprintf("traces-too-large: %d bytes over %d", total, o.MaxTraceTotal))
	}
	// 2. The child does the rest.
	if reason := runClassifyChild(o, pre, kilnPath, action, step); reason != "" {
		return fail(reason)
	}
	// 3. The child exited cleanly: its record must be there and be a record of
	// THIS action under THIS sealed directory. The parent trusts the file no
	// more than `collect` will.
	data, err := os.ReadFile(RecordFile(pre.Dir, action))
	if err != nil {
		return fail("the classifier exited cleanly but left no record.json: " + err.Error())
	}
	rec, err := DecodeClassified(data)
	switch {
	case err != nil:
		return fail("the classifier's record.json does not decode: " + err.Error())
	case rec.Action != action || rec.BindingID != pre.BindingID || rec.SealedSHA256 != pre.SealedSHA256:
		return fail("the classifier's record.json is of another action or another sealed directory")
	}
	return RecordWritten, ""
}

// ClassifyDeps are the classifier's seams, for the child's tests; the zero value
// is the real thing.
type ClassifyDeps struct {
	Classify func(traced.Config, traced.Input) (*traced.RawRecord, error)
	DiskFS   traced.FS
	// Limits are the parser's caps; zero fields take ClassifyLimits.
	Limits trace.Limits
}

func (d ClassifyDeps) withDefaults() ClassifyDeps {
	if d.Classify == nil {
		d.Classify = traced.Classify
	}
	if d.DiskFS == nil {
		d.DiskFS = LiveFS{}
	}
	def := ClassifyLimits()
	if d.Limits.MaxLineBytes <= 0 {
		d.Limits.MaxLineBytes = def.MaxLineBytes
	}
	if d.Limits.MaxTotalBytes <= 0 {
		d.Limits.MaxTotalBytes = def.MaxTotalBytes
	}
	if d.Limits.MaxEvents <= 0 {
		d.Limits.MaxEvents = def.MaxEvents
	}
	return d
}

// ClassifyLimits are the parser's caps for one step's trace in the epilogue.
//
//	MaxTotalBytes  1 GiB, the supervisor's own cap on a trace (DefaultTraceCap):
//	               a trace the supervisor was willing to write is one the
//	               classifier is willing to read. A cap below that would turn a
//	               build the recorder accepted into an unknown write set (the
//	               parser's own default, 64 MiB, did exactly that).
//	MaxEvents      8 million. A kept record costs its source line plus about
//	               200 bytes of event, so at the 1 GiB cap and around 100 bytes
//	               a line a step can hold about 10 million records; 8 million
//	               events leave room for the calls the parser ignores. The
//	               memory this can take (about 1 GiB of lines and 1.6 GiB of
//	               events for one step, more for several) is what the child's
//	               memory bound (DefaultChildSoftMemory, DefaultChildDataLimit)
//	               is for: over it the child dies and the record is
//	               record.failed, never a false answer.
//	MaxLineBytes   1 MiB, the parser's default (a line is one system call).
func ClassifyLimits() trace.Limits {
	return trace.Limits{MaxLineBytes: 1 << 20, MaxTotalBytes: ClassifyMaxStepBytes, MaxEvents: ClassifyMaxEvents}
}

// The epilogue's limits.
const (
	ClassifyMaxStepBytes int64 = DefaultTraceCap
	ClassifyMaxEvents          = 8_000_000
)

// classifyAction is the parse and classify steps: parse every step's trace with
// the parse half of G7, then classify. It reads the disk as it is NOW and writes
// nothing.
func classifyAction(d ClassifyDeps, pre *Pre, action string, step int, last StepSidecar) (ClassifiedRecord, error) {
	d = d.withDefaults()
	planned, _ := pre.Action(action)
	rec := ClassifiedRecord{Schema: ClassifiedSchema, Action: action, BindingID: pre.BindingID, SealedSHA256: pre.SealedSHA256, ParseReasons: []string{}}
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
		// A tracer that was not clean (strace killed, cut short, noisy, pipes
		// held open, ...) has a trace nobody should believe, whatever it parses
		// to: the action's write set is unknown.
		for _, u := range sc.Unclean {
			reasons[fmt.Sprintf("step %d: unclean:%s", j, u)] = true
		}
		if sc.Started {
			rec.StepsRun++
		}
		netns = netns && sc.StepNetNS
		tr, why := parseStep(pre, action, j, sc, pkgDir, d.Limits)
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
	cfg, err := classifyInputs(d, pre, planned)
	if err != nil {
		return ClassifiedRecord{}, err
	}
	raw, err := d.Classify(cfg, traced.Input{Package: action, Traces: traces, Recipe: planned.Steps, StepNetNS: netns})
	if err != nil {
		return ClassifiedRecord{}, fmt.Errorf("classify: %w", err)
	}
	rec.ParseOK = true
	rec.Record = raw
	rec.WritesComplete = raw.WritesComplete
	return rec, nil
}

// readSidecar reads and strictly decodes a step's sidecar, and requires it to be
// the sidecar of that step of that action: its own fields must agree with the
// name it was found under.
func readSidecar(dir, action string, step int) (StepSidecar, error) {
	data, err := os.ReadFile(StepFile(dir, action, step, "json"))
	if err != nil {
		return StepSidecar{}, err
	}
	sc, err := DecodeStepSidecar(data)
	if err != nil {
		return StepSidecar{}, err
	}
	if sc.Action != action || sc.Step != step {
		return StepSidecar{}, fmt.Errorf("the sidecar found as step %d of %q is of step %d of %q", step, printable(action), sc.Step, printable(sc.Action))
	}
	return sc, nil
}

// parseStep parses one step's trace and applies the parse half of G7. It
// returns the trace, or a reason (never both).
func parseStep(pre *Pre, action string, step int, sc StepSidecar, pkgDir string, limits trace.Limits) (*trace.Trace, string) {
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
	tr, err := trace.ParseOptions(f, pkgDir, trace.Options{Limits: limits, HelperRoot: true, HelperExe: pre.Config.KilnExe})
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
func classifyInputs(d ClassifyDeps, pre *Pre, planned Action) (traced.Config, error) {
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
		Tree: NewDiskTree(tree, facts.Root), FS: d.DiskFS,
		HostRoots: &roots, Pinned: pinned, Tools: tools,
		Home: facts.Home, Tmpdir: facts.Tmpdir, JobStart: time.Unix(pre.Binding.JobStart, 0).UTC(),
	}, nil
}
