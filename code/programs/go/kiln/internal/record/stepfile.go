package record

// The per-step sidecar (KILN04 section 4.8, decision P12)
// =======================================================
//
// `kiln record step` supervises one BUILD step under strace. The trace says
// what the BUILD did; it cannot say how the TRACER fared (was strace killed,
// did it complain, was the trace cut off because it grew too large), and the
// tracer's health is half of the completeness rule (G7). So the supervisor
// leaves, next to every trace, one small file of facts that only it could
// observe, and `collect` (the next PR) reads both:
//
//	traces/<sha256(action)>/0.trace   strace's output for step 0
//	traces/<sha256(action)>/0.json    this file
//	traces/<sha256(action)>/0.rerun   (only if the step was run twice)
//
// The action's name never becomes a path component (decision P4); its SHA-256
// does, exactly as in `pre/actions/`.
//
// The supervisor does NOT classify. It never parses the trace, never decides
// whether a write set is complete, and does not read the root's exit line.
// `strace_exit` is strace's own exit code; strace exits with the traced root's
// status, so it doubles as `root_status`, and `collect` compares it with the
// root's exit line in the trace (the other half of G7).

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"path/filepath"
	"strconv"
	"time"
)

// StepSchema is the schema of a step sidecar.
const StepSchema = "kiln-record-step/1"

// Why a trace was cut short (StepSidecar.TruncatedReason).
const (
	TruncTooLarge    = "trace-too-large"
	TruncWriteFailed = "trace-write-failed" // the file could not be written (a full disk, an I/O error)
	TruncDiskFloor   = "disk-floor"         // no longer produced (the floor is checked before a step starts); still accepted
)

// Why a tracer is not clean (entries of StepSidecar.Unclean).
const (
	UncleanNotStarted = "not-started"
	UncleanSignalled  = "strace-signalled"
	UncleanTruncated  = "truncated"
	UncleanStdout     = "strace-stdout-not-empty"
	UncleanStderr     = "strace-stderr-not-empty"
	UncleanPipesHeld  = "strace-pipes-held-open"
)

// StepSidecar is `traces/<sha256(action)>/<step>.json`.
type StepSidecar struct {
	Schema    string `json:"schema"`
	Action    string `json:"action"`
	Step      int    `json:"step"`
	StepCount int    `json:"step_count"`
	// BindingID is the SHA-256 of pre/binding.json this step was run under.
	BindingID string `json:"binding_id"`
	// LineSHA256 is the SHA-256 of the `sh -c` line that ran; LineInPlan says
	// it is byte for byte the line `prepare` planned for this step.
	LineSHA256 string `json:"line_sha256"`
	LineInPlan bool   `json:"line_in_plan"`
	// Started is false when strace could not be started at all (nothing ran).
	Started   bool   `json:"started"`
	StartedAt string `json:"started_at"`
	EndedAt   string `json:"ended_at"`
	// StraceExit is strace's exit code, or -1 when a signal killed it, which
	// StraceSignal then names (0 otherwise).
	StraceExit   int `json:"strace_exit"`
	StraceSignal int `json:"strace_signal"`
	// StraceStdoutEmpty and StraceStderrEmpty say that strace (and the root,
	// which shares them) wrote nothing to its own two standard streams: the
	// step's streams are separate pipes (S2), so anything here is not the
	// step's.
	StraceStdoutEmpty bool `json:"strace_stdout_empty"`
	StraceStderrEmpty bool `json:"strace_stderr_empty"`
	// RootStatus is the traced root's exit status as strace reports it (strace
	// exits with it), -1 when strace was signalled or never ran.
	RootStatus int `json:"root_status"`
	// Exit125 says the status is 125: the sandbox refused, or the step itself
	// exited 125. The supervisor never retries it; `collect` tells the two
	// apart by whether the step's execve is in the trace.
	Exit125 bool `json:"exit_125"`
	// Truncated says the trace is incomplete because the supervisor killed
	// strace at the cap, or could not write the file (TruncatedReason says
	// which); the trace then ends at an arbitrary place.
	Truncated       bool   `json:"truncated"`
	TruncatedReason string `json:"truncated_reason"`
	// TraceBytes and TraceSHA256 describe the bytes the supervisor wrote to the
	// trace file, hashed as they were written; `collect` re-hashes the file and
	// requires it to end in a newline.
	TraceBytes  int64  `json:"trace_bytes"`
	TraceSHA256 string `json:"trace_sha256"`
	// StepNetNS is true only when the step really ran under `kiln
	// sandbox-exec`, which unshares the network namespace (so an unbound
	// loopback or abstract peer is not another process's). It is the recorder's
	// own statement, the only source of traced.Input.StepNetNS.
	StepNetNS bool `json:"step_netns"`
	// Clean is the supervisor's half of the tracer-completeness rule (G7):
	// strace started, was not killed by a signal, was not cut short, and wrote
	// nothing to its own streams. Unclean lists what failed, and is empty
	// exactly when Clean is true.
	Clean   bool     `json:"clean"`
	Unclean []string `json:"unclean"`
}

// StepDir is the directory of an action's traces under the record directory.
func StepDir(dir, action string) string {
	sum := sha256.Sum256([]byte(action))
	return filepath.Join(dir, "traces", hex.EncodeToString(sum[:]))
}

// StepFile is the path of one step's file with the given extension ("trace",
// "json" or "rerun").
func StepFile(dir, action string, step int, ext string) string {
	return filepath.Join(StepDir(dir, action), strconv.Itoa(step)+"."+ext)
}

// Validate checks everything a sidecar's own fields promise.
func (s *StepSidecar) Validate() error {
	if s.Schema != StepSchema {
		return fmt.Errorf("step sidecar: schema %q", printable(s.Schema))
	}
	if s.Step < 0 || s.StepCount < 1 || s.Step >= s.StepCount {
		return fmt.Errorf("step sidecar: step %d of %d", s.Step, s.StepCount)
	}
	for _, d := range []struct{ what, v string }{{"binding_id", s.BindingID}, {"line_sha256", s.LineSHA256}, {"trace_sha256", s.TraceSHA256}} {
		if !hexDigest(d.v) {
			return fmt.Errorf("step sidecar: %s is not a SHA-256 in lowercase hex", d.what)
		}
	}
	for _, t := range []struct{ what, v string }{{"started_at", s.StartedAt}, {"ended_at", s.EndedAt}} {
		if _, err := time.Parse(time.RFC3339Nano, t.v); err != nil {
			return fmt.Errorf("step sidecar: %s: %v", t.what, err)
		}
	}
	if s.StraceExit < -1 || s.StraceExit > 255 || s.StraceSignal < 0 || s.StraceSignal > 64 {
		return fmt.Errorf("step sidecar: strace status %d, signal %d", s.StraceExit, s.StraceSignal)
	}
	if (s.StraceExit == -1) != (s.StraceSignal != 0) && s.Started {
		return fmt.Errorf("step sidecar: strace_exit and strace_signal disagree")
	}
	if s.RootStatus != s.StraceExit || s.Exit125 != (s.StraceExit == 125) {
		return fmt.Errorf("step sidecar: root_status and exit_125 do not follow from strace_exit")
	}
	if s.TraceBytes < 0 {
		return fmt.Errorf("step sidecar: trace_bytes %d", s.TraceBytes)
	}
	if s.Truncated != (s.TruncatedReason != "") || (s.Truncated && s.TruncatedReason != TruncTooLarge && s.TruncatedReason != TruncDiskFloor && s.TruncatedReason != TruncWriteFailed) {
		return fmt.Errorf("step sidecar: truncated %v with reason %q", s.Truncated, printable(s.TruncatedReason))
	}
	if s.StepNetNS != s.Started {
		return fmt.Errorf("step sidecar: step_netns must equal started (the step ran under the sandbox exactly when strace started)")
	}
	if s.Unclean == nil {
		return fmt.Errorf("step sidecar: unclean is null")
	}
	if s.Clean != (len(s.Unclean) == 0) {
		return fmt.Errorf("step sidecar: clean is %v with %d unclean reasons", s.Clean, len(s.Unclean))
	}
	if s.Clean && (!s.Started || s.StraceExit < 0 || s.Truncated || !s.StraceStdoutEmpty || !s.StraceStderrEmpty) {
		return fmt.Errorf("step sidecar: clean, but the fields say the tracer was not")
	}
	return nil
}

// EncodeStepSidecar validates and renders a sidecar: compact, no HTML
// escaping, no trailing newline.
func EncodeStepSidecar(s StepSidecar) ([]byte, error) {
	if s.Unclean == nil {
		s.Unclean = []string{}
	}
	if err := s.Validate(); err != nil {
		return nil, err
	}
	return marshalJSON(s)
}

// DecodeStepSidecar reads a sidecar strictly: no unknown key, no trailing
// data, and the same validation as the encoder.
func DecodeStepSidecar(data []byte) (StepSidecar, error) {
	var s StepSidecar
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&s); err != nil {
		return StepSidecar{}, fmt.Errorf("step sidecar: %w", err)
	}
	if _, err := dec.Token(); err != io.EOF {
		return StepSidecar{}, fmt.Errorf("step sidecar: unexpected data after the JSON value")
	}
	return s, s.Validate()
}
