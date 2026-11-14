package record

// `collect`, part 1: one action's verdict (KILN04 section 4.8, P12 and P13)
// =========================================================================
//
// `kiln record step` leaves, for every action that ran, a directory
//
//	traces/<sha256(action)>/
//	    0.json 0.trace  1.json 1.trace ...   one sidecar and one trace per step
//	    0.rerun / 0.overlap                  markers (a step run twice; a lock refused)
//	    0.strace-stderr                      strace's own words, opaque
//	    record.json  or  record.failed       the verdict of the epilogue (P13)
//
// and `collect` turns the directory, the build tool's results and the sealed
// plan into ONE `recfmt.ShardRecord`. It classifies nothing and parses no
// trace: classification happened at the end of the action, while the disk was
// still the action's (P13). What is left for `collect`, and what this file
// does, is to prove that the files it is about to believe are the files the
// supervisor wrote, whole and in the right order:
//
//	scanAction     every entry of the directory is one the supervisor makes; a
//	               marker voids; a sidecar and its trace come as a pair
//	checkSidecars  each sidecar is of this action and step, under this binding,
//	               ran the planned line, and its trace hashes to what it says
//	checkSeq       the serial order: consecutive numbers within the action
//	readVerdict    record.failed overrides record.json; record.json is of this
//	               action under this sealed directory, and counts the steps that
//	               really started
//	crossCheck     the build tool saw the same number of steps start
//
// THE RULE: any doubt is an UNKNOWN WRITE SET. A record with `unknown_writes`
// set carries an empty raw record and `writes_complete: false`, and the merge
// voids the whole run for it (G1): one action whose writes are unknown could
// have written the very path another action, in another shard, found absent.
// So nothing here ever "repairs" a record, guesses a missing file, or reads the
// absence of a file as "nothing happened".
//
// What is untrusted. The directory is written by the supervisor, outside the
// sandbox, but it is read as if it were not: every file through a non-following
// handle, regular files only, bounded, strictly decoded. Text that came from the
// traced action (the classifier's reasons, record.failed) is made printable and
// short before it reaches a shard, because the shard's text ends up in a CI log
// where a `::` line is a workflow command. `<step>.strace-stderr` is never even
// opened: strace's words are whatever a step made it print.

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"hash"
	"io"
	"os"
	"strings"
	"time"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// maxSidecarBytes bounds a sidecar or a verdict marker we read: a real sidecar is
// under a kilobyte, so a megabyte is already a sign of tampering.
const maxSidecarBytes = 1 << 20

// maxFailedBytes bounds record.failed: it is a one-line reason.
const maxFailedBytes = 64 << 10

// maxUnknownText is how many bytes of a reason go into `unknown_writes` (the
// record format allows 200).
const maxUnknownText = 190

// reasonText makes text from a trace-derived file safe and short enough for a
// record's `unknown_writes` (200 bytes, printable): only printable runes (a
// file name can hold a newline, a bidi override or an escape) and a bounded
// length.
func reasonText(s string) string {
	s = traced.Sanitize(s)
	if len(s) <= maxUnknownText {
		return s
	}
	s = s[:maxUnknownText]
	for !utf8.ValidString(s) { // a cut inside a rune
		s = s[:len(s)-1]
	}
	return s
}

// seqUse is one sidecar's place in the serial order.
type seqUse struct {
	action string
	step   int
	seq    int
}

// actionOut is one action's assembled record and what the global checks need.
type actionOut struct {
	name string
	rec  recfmt.ShardRecord
	// seqs are the sidecars' numbers, in step order.
	seqs []seqUse
	// overlap says an `.overlap` marker was found: the whole shard is void.
	overlap bool
}

// unknownText joins the reasons an action's write set is unknown, in order.
func unknownText(bad []string) string {
	if len(bad) == 0 {
		return ""
	}
	return reasonText(strings.Join(bad, "; "))
}

// markUnknown turns a record into an unknown write set: the reason, an EMPTY raw
// record (never a partial one) and an explicit `writes_complete: false`. The
// steps run and their summaries are kept as they are (they are facts, not
// classifications), so the record stays well formed.
func markUnknown(rec *recfmt.ShardRecord, why string) {
	if rec.Unknown == "" {
		rec.Unknown = reasonText(why)
	}
	rec.Record = recfmt.Normalize(traced.RawRecord{Package: rec.Action})
	f := false
	rec.Complete = &f
}

// dirScan is what an action's trace directory holds.
type dirScan struct {
	exists               bool
	sidecars, traces     map[int]bool
	reruns, overlaps     int
	hasRecord, hasFailed bool
	bad                  []string
}

// scanAction lists the action's directory and files every entry under what the
// supervisor makes. It reads no file.
func scanAction(dir string) dirScan {
	s := dirScan{sidecars: map[int]bool{}, traces: map[int]bool{}}
	st, err := os.Lstat(dir)
	switch {
	case errors.Is(err, os.ErrNotExist):
		return s
	case err != nil:
		s.exists = true
		s.bad = append(s.bad, "the action's trace directory: "+err.Error())
		return s
	case st.Mode()&os.ModeSymlink != 0 || !st.IsDir():
		s.exists = true
		s.bad = append(s.bad, "the action's trace directory is not a real directory")
		return s
	}
	s.exists = true
	entries, err := os.ReadDir(dir)
	if err != nil {
		s.bad = append(s.bad, "the action's trace directory cannot be listed: "+err.Error())
		return s
	}
	for _, e := range entries {
		name := e.Name()
		switch {
		case name == "record.json":
			s.hasRecord = true
			continue
		case name == "record.failed":
			s.hasFailed = true
			continue
		case strings.HasPrefix(name, "record.json.tmp-") || strings.HasPrefix(name, "record.failed.tmp-"):
			// A kill between the write and the link(2): a record that may have been
			// half made. It is never "no record".
			s.bad = append(s.bad, "a stray "+reasonText(name))
			continue
		}
		n, ext, ok := strings.Cut(name, ".")
		j, err := canonicalInt("", n)
		if !ok || err != nil {
			s.bad = append(s.bad, "an unexpected entry "+reasonText(name))
			continue
		}
		// Whatever a step's file is, it must be a regular file: the supervisor
		// makes nothing else, and an irregular one is not its work.
		regular := e.Type().IsRegular()
		switch ext {
		case "json":
			s.sidecars[j] = true
		case "trace":
			s.traces[j] = true
		case "rerun":
			s.reruns++
		case "overlap":
			s.overlaps++
		case "strace-stderr":
			// Opaque, never read (see the file comment).
			continue
		default:
			s.bad = append(s.bad, "an unexpected entry "+reasonText(name))
			continue
		}
		if !regular {
			s.bad = append(s.bad, reasonText(name)+" is not a regular file")
		}
	}
	return s
}

// readBoundedFile reads a small file the supervisor wrote: a non-following
// open, a regular file, at most max bytes (checked on the size BEFORE reading,
// and again while reading).
func readBoundedFile(path string, max int64) ([]byte, error) {
	f, err := openNoFollow(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	st, err := f.Stat()
	if err != nil {
		return nil, err
	}
	if !st.Mode().IsRegular() {
		return nil, errors.New("not a regular file")
	}
	if st.Size() > max {
		return nil, fmt.Errorf("%d bytes, over %d", st.Size(), max)
	}
	data, err := io.ReadAll(io.LimitReader(f, max+1))
	if err != nil {
		return nil, err
	}
	if int64(len(data)) > max {
		return nil, fmt.Errorf("grew past %d bytes", max)
	}
	return data, nil
}

// lastByte remembers the last byte written through it.
type lastByte struct {
	b byte
	n int64
}

func (l *lastByte) Write(p []byte) (int, error) {
	if len(p) > 0 {
		l.b = p[len(p)-1]
		l.n += int64(len(p))
	}
	return len(p), nil
}

// checkTrace re-hashes one step's trace and compares it with its sidecar: the
// file is a regular file of exactly `trace_bytes` bytes (no more: the read stops
// one byte past, so a file that grew is caught without reading a gigabyte more),
// its SHA-256 equals `trace_sha256`, and it ends in a newline (strace writes
// whole lines; a trace that does not end in one was cut). It returns "" when the
// trace is the one the supervisor wrote.
func checkTrace(path string, sc StepSidecar) string {
	f, err := openNoFollow(path)
	if err != nil {
		return "the trace cannot be opened"
	}
	defer f.Close()
	st, err := f.Stat()
	if err != nil || !st.Mode().IsRegular() {
		return "the trace is not a regular file"
	}
	if st.Size() != sc.TraceBytes {
		return fmt.Sprintf("the trace is %d bytes, the sidecar says %d", st.Size(), sc.TraceBytes)
	}
	var h hash.Hash = sha256.New()
	last := &lastByte{}
	n, err := io.Copy(io.MultiWriter(h, last), io.LimitReader(f, sc.TraceBytes+1))
	if err != nil {
		return "the trace cannot be read"
	}
	switch {
	case n != sc.TraceBytes:
		return fmt.Sprintf("the trace changed size while it was read (%d of %d bytes)", n, sc.TraceBytes)
	case hex.EncodeToString(h.Sum(nil)) != sc.TraceSHA256:
		return "the trace does not hash to the sidecar's trace_sha256"
	case n == 0 || last.b != '\n':
		return "the trace does not end in a newline"
	}
	return ""
}

// wallMS is a step's wall time in milliseconds, 0 when the clock went back or a
// stamp does not parse (the sidecar validated both).
func wallMS(sc StepSidecar) int64 {
	a, err1 := time.Parse(time.RFC3339Nano, sc.StartedAt)
	b, err2 := time.Parse(time.RFC3339Nano, sc.EndedAt)
	if err1 != nil || err2 != nil || b.Before(a) {
		return 0
	}
	return b.Sub(a).Milliseconds()
}

// sidecarOK are the facts of one action's sidecars the rest needs.
type sidecarOK struct {
	started    int
	summaries  []recfmt.Step
	seqs       []seqUse
	allZero    bool // every started step exited 0
	allDecoded bool
}

// checkSidecars reads every sidecar of the action, in step order, and checks it
// against its trace and the plan. It appends a reason to bad for each failure
// and never stops at the first, so the report says everything.
func checkSidecars(c *collector, name string, planned Action, s dirScan, bad *[]string) sidecarOK {
	out := sidecarOK{allZero: true, allDecoded: true}
	// n is how many steps the directory shows: one past the highest number that
	// has a sidecar or a trace. Indices are walked in order (never by ranging over
	// the maps) so the reasons, and the text built from them, are the same on
	// every run: the merge's output is compared byte for byte.
	n := 0
	for j := range s.sidecars {
		n = max(n, j+1)
	}
	for j := range s.traces {
		n = max(n, j+1)
	}
	// The sidecars are exactly steps 0..n-1, and every one has its trace and the
	// reverse: a step that took a number and was killed has a trace and no
	// sidecar; a deleted trace leaves a sidecar alone.
	for j := 0; j < n; j++ {
		switch {
		case s.sidecars[j] && !s.traces[j]:
			*bad = append(*bad, fmt.Sprintf("step %d has a sidecar and no trace", j))
		case !s.sidecars[j] && s.traces[j]:
			*bad = append(*bad, fmt.Sprintf("step %d has a trace and no sidecar (it started and was not accounted for)", j))
			out.allDecoded = false
		case !s.sidecars[j]:
			*bad = append(*bad, fmt.Sprintf("step %d has no sidecar but a later one does", j))
			out.allDecoded = false
		}
	}
	if n > len(planned.Steps) {
		*bad = append(*bad, fmt.Sprintf("%d steps ran, the plan has %d", n, len(planned.Steps)))
		out.allDecoded = false
		return out
	}
	for j := 0; j < n; j++ {
		if !s.sidecars[j] {
			continue
		}
		data, err := readBoundedFile(StepFile(c.dir, name, j, "json"), maxSidecarBytes)
		if err != nil {
			*bad = append(*bad, fmt.Sprintf("step %d: its sidecar: %v", j, err))
			out.allDecoded = false
			continue
		}
		sc, err := DecodeStepSidecar(data)
		if err != nil {
			*bad = append(*bad, fmt.Sprintf("step %d: %v", j, err))
			out.allDecoded = false
			continue
		}
		switch {
		case sc.Action != name || sc.Step != j:
			*bad = append(*bad, fmt.Sprintf("step %d: the sidecar is of another action or step", j))
		case sc.StepCount != len(planned.Steps):
			*bad = append(*bad, fmt.Sprintf("step %d: the sidecar counts %d steps, the plan has %d", j, sc.StepCount, len(planned.Steps)))
		case sc.BindingID != c.pre.BindingID:
			*bad = append(*bad, fmt.Sprintf("step %d: run under another binding", j))
		}
		// The recipe check (P12 step 4: the supervisor records it, `collect` is
		// the judge): the line that ran is the planned one, byte for byte. The
		// sidecar's own claim is not enough, so the digest is recomputed from the
		// sealed plan.
		want := sha256.Sum256([]byte(planned.Steps[j]))
		if !sc.LineInPlan || sc.LineSHA256 != hex.EncodeToString(want[:]) {
			*bad = append(*bad, fmt.Sprintf("step %d: recipe-mismatch (the line that ran is not the planned one)", j))
		}
		if sc.TruncatedReason == TruncWriteFailed {
			*bad = append(*bad, fmt.Sprintf("step %d: trace-write-failed", j))
		}
		for _, u := range sc.Unclean {
			if u == UncleanPipesHeld {
				*bad = append(*bad, fmt.Sprintf("step %d: strace-pipes-held-open", j))
			}
		}
		if why := checkTrace(StepFile(c.dir, name, j, "trace"), sc); why != "" {
			*bad = append(*bad, fmt.Sprintf("step %d: %s", j, why))
		}
		out.seqs = append(out.seqs, seqUse{action: name, step: j, seq: sc.Seq})
		if sc.Started {
			out.started++
			out.summaries = append(out.summaries, recfmt.Step{WallMS: wallMS(sc), TraceBytes: sc.TraceBytes, Clean: sc.Clean})
			if sc.RootStatus != 0 {
				out.allZero = false
			}
		}
	}
	// The serial order within the action: the steps took consecutive numbers,
	// because nothing else ran between them (the serial lock, P13). A gap means
	// another action's step ran in the middle.
	for i := 1; i < len(out.seqs); i++ {
		if out.seqs[i].seq != out.seqs[i-1].seq+1 {
			*bad = append(*bad, fmt.Sprintf("serial-violated: step %d has seq %d after seq %d", out.seqs[i].step, out.seqs[i].seq, out.seqs[i-1].seq))
			break
		}
	}
	return out
}

// verdict is what the action's record.json says, when it can be believed.
type verdict struct {
	rec     *ClassifiedRecord
	present bool // a record.json or record.failed exists
}

// readVerdict applies the epilogue's rules (P13): record.failed overrides
// record.json (both present is a failure); a missing verdict for an action that
// ran a step is an unknown write set; record.json must decode, be of this action
// under this binding and sealed directory, count the steps that really started,
// and carry a raw record (parse_ok).
func readVerdict(c *collector, name string, s dirScan, started int, bad *[]string) verdict {
	v := verdict{present: s.hasRecord || s.hasFailed}
	if s.hasFailed {
		why := "record.failed"
		if data, err := readBoundedFile(RecordFailedFile(c.dir, name), maxFailedBytes); err != nil {
			why += " (unreadable: " + err.Error() + ")"
		} else if len(data) > 0 {
			// The classifier's own reason, made printable and short; it is
			// diagnostic text, never interpreted.
			why += ": " + reasonText(string(data))
		}
		if s.hasRecord {
			why += " (and a record.json: the failure overrides it)"
		}
		*bad = append(*bad, "no usable record: "+why)
		return v
	}
	if !s.hasRecord {
		return v
	}
	data, err := readRecordBounded(RecordFile(c.dir, name), DefaultMaxRecordBytes)
	if err != nil {
		*bad = append(*bad, "record.json: "+err.Error())
		return v
	}
	rec, err := DecodeClassified(data)
	if err != nil {
		*bad = append(*bad, "record.json does not decode: "+err.Error())
		return v
	}
	switch {
	case rec.Action != name:
		*bad = append(*bad, "record.json is of another action")
	case rec.BindingID != c.pre.BindingID:
		*bad = append(*bad, "record.json was made under another binding")
	case rec.SealedSHA256 != c.pre.SealedSHA256:
		*bad = append(*bad, "record.json was made from another sealed directory")
	case rec.StepsRun != started:
		*bad = append(*bad, fmt.Sprintf("record.json counts %d started steps, the sidecars %d", rec.StepsRun, started))
	case !rec.ParseOK:
		// Includes "sandbox-refused": a refused step ran nothing, but the sandbox
		// may have failed after it had changed the disk, so the write set is
		// unknown, not empty.
		*bad = append(*bad, "the trace did not pass the parse half of G7: "+reasonText(strings.Join(rec.ParseReasons, ", ")))
	default:
		v.rec = &rec
	}
	return v
}

// assembleAction builds one action's record and its seq numbers.
func (c *collector) assembleAction(name string, planned Action) actionOut {
	out := actionOut{name: name}
	res, inResults := c.results[name]
	status := "not-run"
	if inResults {
		status = res.Status
	}
	rec := recfmt.ShardRecord{
		BindingID: c.pre.BindingID, Action: name, Language: planned.Language, Status: status,
		StepsTotal: len(planned.Steps), Steps: []recfmt.Step{},
		// The sealed plan's lines, whose every sidecar line was just compared with
		// them: what the merge's B6 compares with its own build tool's lines.
		StepsSHA: recfmt.StepsDigest(planned.Steps),
		Weak:     planned.Weak, Refusal: planned.Refusal,
		Record: recfmt.Normalize(traced.RawRecord{Package: name}),
	}
	var bad []string
	s := scanAction(StepDir(c.dir, name))
	bad = append(bad, s.bad...)
	if s.overlaps > 0 {
		out.overlap = true
		bad = append(bad, "serial-violated: an overlap marker (a step was refused the serial lock)")
	}
	if s.reruns > 0 {
		bad = append(bad, "a step was run twice or out of order (a .rerun marker)")
	}
	sc := checkSidecars(c, name, planned, s, &bad)
	out.seqs = sc.seqs
	v := readVerdict(c, name, s, sc.started, &bad)

	// The build tool's own count and status (the cross-check, section 7).
	switch {
	case inResults && res.StepsTotal != len(planned.Steps):
		bad = append(bad, fmt.Sprintf("the build tool says %d steps, the plan has %d", res.StepsTotal, len(planned.Steps)))
	case inResults && res.StepsRun != sc.started:
		bad = append(bad, fmt.Sprintf("the build tool started %d steps, %d have a traced run", res.StepsRun, sc.started))
	case !inResults && len(s.sidecars)+len(s.traces) > 0:
		bad = append(bad, "steps ran for an action the build tool's results do not list")
	}
	if status == recfmt.StatusBuilt {
		if len(planned.Steps) == 0 || len(s.sidecars) == 0 {
			bad = append(bad, "built without a traced step: nothing was classified")
		}
		if !sc.allZero {
			bad = append(bad, "built, but a step exited non-zero")
		}
	}
	if len(s.sidecars) > 0 && !v.present {
		bad = append(bad, "no record.json for an action that ran: the epilogue did not finish")
	}
	if len(s.sidecars) == 0 && v.present {
		bad = append(bad, "a verdict for an action with no step")
	}

	// The steps as facts, when every sidecar could be read (a record's summaries
	// are one per started step, exactly).
	if sc.allDecoded {
		rec.StepsRun, rec.Steps = sc.started, sc.summaries
		if rec.Steps == nil {
			rec.Steps = []recfmt.Step{}
		}
	}
	if len(bad) > 0 || v.rec == nil {
		if v.rec == nil && len(bad) == 0 && len(s.sidecars) == 0 {
			// The action ran nothing (not run, skipped, refused by the plan): it
			// was never classified, so the key is an explicit false that the
			// format accepts without a raw record (CheckComplete), and the merge
			// charges it as a truncated execution, not as an unknown write set.
			f := false
			rec.Complete = &f
			out.rec = rec
			return out
		}
		markUnknown(&rec, unknownText(bad))
		if rec.Unknown == "" {
			markUnknown(&rec, "no usable record")
		}
		out.rec = rec
		return out
	}
	// A classified record: the classifier's own answer, never a default. The
	// flag is not part of the raw record's JSON, so it comes from the record
	// file's own field and goes through CompleteOf like every record the writer
	// emits.
	raw := *v.rec.Record
	raw.WritesComplete = v.rec.WritesComplete
	rec.Record = recfmt.Normalize(raw)
	rec.Complete = recfmt.CompleteOf(rec.Record)
	out.rec = rec
	return out
}
