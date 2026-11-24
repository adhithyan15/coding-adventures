//go:build unix

package record

// KILN04 D48 step 5, the recorder's half of consumption: before the classifier
// is handed a step's exec-head sidecar, readExecHead checks everything only the
// recorder can (the file is the one the step sidecar binds, whole, strict,
// and of this step), and a file that fails refuses the WHOLE action.

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

const (
	ehAction = "code/p/a"
	ehNonce  = "0123456789abcdef0123456789abcdef"
)

func ehActionDigest(action string) string {
	s := sha256.Sum256([]byte(action))
	return hex.EncodeToString(s[:])
}

func ehHeader(nonce, action string, step int) exechead.Header {
	return exechead.Header{V: exechead.Version, Kernel: "7.0.0-1014-aws", Nonce: nonce, ActionSHA256: ehActionDigest(action), Step: step}
}

func ehConfirmedLine(path string, head []byte) exechead.Line {
	s := sha256.Sum256(head)
	return exechead.Line{Tid: 21, Tgid: 21, Syscall: exechead.SyscallExecve, Path: path, Status: exechead.StatusConfirmed,
		Dev: 58, Ino: 9, Head: head, SHA256: hex.EncodeToString(s[:])}
}

// ehBytes encodes a whole sidecar.
func ehBytes(t *testing.T, h exechead.Header, lines ...exechead.Line) []byte {
	t.Helper()
	var buf bytes.Buffer
	w, err := exechead.NewWriter(&buf, h, exechead.Limits{})
	if err != nil {
		t.Fatal(err)
	}
	for _, l := range lines {
		if err := w.Append(l); err != nil {
			t.Fatal(err)
		}
	}
	return buf.Bytes()
}

// ehPlace writes data as the step's exec-head file and returns the step sidecar
// that binds it (nonce, digest and length of the bytes as written).
func ehPlace(t *testing.T, dir string, data []byte) StepSidecar {
	t.Helper()
	if err := os.MkdirAll(StepDir(dir, ehAction), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(StepFile(dir, ehAction, 0, "exechead"), data, 0o600); err != nil {
		t.Fatal(err)
	}
	sum := sha256.Sum256(data)
	sc := execHeadSidecar()
	sc.Action = ehAction
	sc.ExecHeadNonce, sc.ExecHeadSHA256, sc.ExecHeadBytes = ehNonce, hex.EncodeToString(sum[:]), int64(len(data))
	return sc
}

func TestReadExecHead(t *testing.T) {
	good := func(t *testing.T) []byte {
		return ehBytes(t, ehHeader(ehNonce, ehAction, 0),
			exechead.Line{Tid: 20, Tgid: 20, Syscall: exechead.SyscallExecve, Path: "/usr/bin/sh", Status: exechead.StatusOutside},
			ehConfirmedLine("/tmp/x/t.test", []byte("\x7fELFhead")))
	}
	t.Run("a good file is decoded", func(t *testing.T) {
		dir := t.TempDir()
		sc := ehPlace(t, dir, good(t))
		f, why := readExecHead(dir, ehAction, 0, sc)
		if why != "" || f == nil || len(f.Lines) != 2 || f.Lines[1].Status != exechead.StatusConfirmed || string(f.Lines[1].Head) != "\x7fELFhead" {
			t.Fatalf("%v %q", f, why)
		}
	})
	t.Run("no supervisor, no file: nothing, no reason", func(t *testing.T) {
		dir := t.TempDir()
		if f, why := readExecHead(dir, ehAction, 0, execHeadSidecar()); f != nil || why != "" {
			t.Fatalf("%v %q", f, why)
		}
	})
	t.Run("a file the step sidecar does not name is a second file", func(t *testing.T) {
		dir := t.TempDir()
		sc := ehPlace(t, dir, good(t))
		sc.ExecHeadNonce, sc.ExecHeadSHA256, sc.ExecHeadBytes = "", "", 0
		if f, why := readExecHead(dir, ehAction, 0, sc); f != nil || why != ReasonExecHeadUnclaimed {
			t.Fatalf("%v %q", f, why)
		}
	})
	t.Run("the sidecar names a file that is missing", func(t *testing.T) {
		dir := t.TempDir()
		sc := ehPlace(t, dir, good(t))
		os.Remove(StepFile(dir, ehAction, 0, "exechead"))
		if f, why := readExecHead(dir, ehAction, 0, sc); f != nil || why != ReasonExecHeadMissing {
			t.Fatalf("%v %q", f, why)
		}
	})
	t.Run("a link and a directory are not the supervisor's file", func(t *testing.T) {
		dir := t.TempDir()
		sc := ehPlace(t, dir, good(t))
		real := StepFile(dir, ehAction, 0, "exechead")
		os.Rename(real, real+".real")
		if err := os.Symlink(real+".real", real); err != nil {
			t.Fatal(err)
		}
		if f, why := readExecHead(dir, ehAction, 0, sc); f != nil || why != ReasonExecHeadMissing {
			t.Fatalf("a link: %v %q", f, why)
		}
		os.Remove(real)
		if err := os.Mkdir(real, 0o755); err != nil {
			t.Fatal(err)
		}
		if f, why := readExecHead(dir, ehAction, 0, sc); f != nil || why != ReasonExecHeadMissing {
			t.Fatalf("a directory: %v %q", f, why)
		}
	})
	t.Run("a flipped byte or another length is a digest mismatch", func(t *testing.T) {
		dir := t.TempDir()
		data := good(t)
		sc := ehPlace(t, dir, data)
		flipped := append([]byte(nil), data...)
		flipped[len(flipped)-3] ^= 1
		os.WriteFile(StepFile(dir, ehAction, 0, "exechead"), flipped, 0o600)
		if f, why := readExecHead(dir, ehAction, 0, sc); f != nil || why != ReasonExecHeadDigest {
			t.Fatalf("flipped: %v %q", f, why)
		}
		os.WriteFile(StepFile(dir, ehAction, 0, "exechead"), data, 0o600)
		sc.ExecHeadBytes++
		if f, why := readExecHead(dir, ehAction, 0, sc); f != nil || why != ReasonExecHeadDigest {
			t.Fatalf("length: %v %q", f, why)
		}
	})
	t.Run("an oversize file is refused before it is read", func(t *testing.T) {
		dir := t.TempDir()
		sc := ehPlace(t, dir, good(t))
		if err := os.Truncate(StepFile(dir, ehAction, 0, "exechead"), exechead.MaxFileBytes+1); err != nil {
			t.Skipf("cannot make a sparse file: %v", err)
		}
		if f, why := readExecHead(dir, ehAction, 0, sc); f != nil || why != ReasonExecHeadOversize {
			t.Fatalf("%v %q", f, why)
		}
	})
	// The digest is right for the bytes, but the bytes are not a file of this step.
	for name, tc := range map[string]struct {
		build func(*testing.T) []byte
	}{
		"another nonce in the header": {func(t *testing.T) []byte {
			return ehBytes(t, ehHeader(strings.Repeat("f", 32), ehAction, 0), ehConfirmedLine("/tmp/x", []byte("h")))
		}},
		"another action": {func(t *testing.T) []byte {
			return ehBytes(t, ehHeader(ehNonce, "code/p/other", 0), ehConfirmedLine("/tmp/x", []byte("h")))
		}},
		"another step": {func(t *testing.T) []byte {
			return ehBytes(t, ehHeader(ehNonce, ehAction, 3), ehConfirmedLine("/tmp/x", []byte("h")))
		}},
		"garbage": {func(t *testing.T) []byte { return []byte("not json\n") }},
		"empty":   {func(t *testing.T) []byte { return []byte{} }},
		"a duplicated line": {func(t *testing.T) []byte {
			// The same line twice: its seq repeats, which the decoder refuses (a gap and
			// a repeat are detected by seq, never by comparing text).
			data := ehBytes(t, ehHeader(ehNonce, ehAction, 0), ehConfirmedLine("/tmp/x", []byte("h")))
			lines := bytes.SplitAfter(data, []byte("\n"))
			return append(append([]byte(nil), data...), lines[1]...)
		}},
		"no trailing newline": {func(t *testing.T) []byte {
			d := ehBytes(t, ehHeader(ehNonce, ehAction, 0), ehConfirmedLine("/tmp/x", []byte("h")))
			return d[:len(d)-1]
		}},
	} {
		t.Run(name, func(t *testing.T) {
			dir := t.TempDir()
			data := tc.build(t)
			sc := ehPlace(t, dir, data)
			if len(data) == 0 {
				sc.ExecHeadBytes = 1 // Validate wants a positive length; the file is empty
				if f, why := readExecHead(dir, ehAction, 0, sc); f != nil || why == "" {
					t.Fatalf("%v %q", f, why)
				}
				return
			}
			if f, why := readExecHead(dir, ehAction, 0, sc); f != nil || why != ReasonExecHeadMalformed {
				t.Fatalf("%v %q", f, why)
			}
		})
	}
}

// ---------------------------------------------------------------- the epilogue

// The epilogue hands the classifier the decoded file of every step that has one,
// and refuses the whole action when a file cannot be trusted.  The classifier is
// a stub, so this pins the wiring and not the classification (internal/traced).
func TestEpilogueHandsTheClassifierTheExecHeadsAndRefusesABadFile(t *testing.T) {
	line := playLine("", "a", 0)
	f := newStepFixLinks(t, planOf([]string{line}, nil, nil), nil, nil, true)
	f.give("a", actA, line, 0, cannedOpts{})
	if res := f.must(actA, 0, 1, line); res.ExitCode != 0 || res.Record != RecordWritten {
		t.Fatalf("%d %q %q", res.ExitCode, res.Record, res.RecordWhy)
	}
	base := f.sidecar(actA, 0)
	dir := f.p.dir

	var got *traced.Input
	deps := ClassifyDeps{Classify: func(cfg traced.Config, in traced.Input) (*traced.RawRecord, error) {
		got = &in
		return &traced.RawRecord{Package: actA}, nil
	}}
	place := func(data []byte, action string) StepSidecar {
		t.Helper()
		if err := os.WriteFile(StepFile(dir, action, 0, "exechead"), data, 0o600); err != nil {
			t.Fatal(err)
		}
		sc := base
		sum := sha256.Sum256(data)
		sc.ExecHeadNonce, sc.ExecHeadSHA256, sc.ExecHeadBytes = ehNonce, hex.EncodeToString(sum[:]), int64(len(data))
		return sc
	}
	remove := func() { os.Remove(StepFile(dir, actA, 0, "exechead")) }

	// No supervisor and no file: the classifier gets no ExecHeads at all.
	got = nil
	rec, err := classifyAction(deps, f.pre, actA, 0, base)
	if err != nil || !rec.ParseOK || got == nil || len(got.ExecHeads) != 0 {
		t.Fatalf("%v %+v %+v", err, rec, got)
	}

	// A good file: one entry per trace, the decoded file.
	h := ehHeader(ehNonce, actA, 0)
	sc := place(ehBytes(t, h, ehConfirmedLine("/tmp/x/t.test", []byte("\x7fELF"))), actA)
	got = nil
	rec, err = classifyAction(deps, f.pre, actA, 0, sc)
	if err != nil || !rec.ParseOK || got == nil || len(got.ExecHeads) != 1 || got.ExecHeads[0] == nil ||
		len(got.ExecHeads[0].Lines) != 1 || got.ExecHeads[0].Lines[0].Path != "/tmp/x/t.test" {
		t.Fatalf("%v %+v %+v", err, rec, got)
	}

	// A file whose bytes are not what the sidecar binds refuses the whole action:
	// parse_ok false, no raw record, and the classifier is never called.
	bad := sc
	bad.ExecHeadSHA256 = strings.Repeat("0", 64)
	got = nil
	rec, err = classifyAction(deps, f.pre, actA, 0, bad)
	if err != nil || rec.ParseOK || rec.Record != nil || got != nil || len(rec.ParseReasons) != 1 ||
		rec.ParseReasons[0] != "step 0: "+ReasonExecHeadDigest {
		t.Fatalf("%v %+v called=%v", err, rec, got != nil)
	}
	// The census files it under a fixed code.
	if c := parseReasonCode(rec.ParseReasons[0]); c != "exechead-digest" {
		t.Fatalf("code %q", c)
	}

	// A file the sidecar does not name (the supervisor "did not run") is refused too.
	unclaimed := base
	got = nil
	rec, err = classifyAction(deps, f.pre, actA, 0, unclaimed)
	if err != nil || rec.ParseOK || got != nil || len(rec.ParseReasons) != 1 || rec.ParseReasons[0] != "step 0: "+ReasonExecHeadUnclaimed {
		t.Fatalf("%v %+v", err, rec)
	}
	remove()
}

func TestExecHeadReasonsAndUncleanCodesAreInTheCensusVocabulary(t *testing.T) {
	for _, r := range []string{ReasonExecHeadMissing, ReasonExecHeadUnclaimed, ReasonExecHeadOversize, ReasonExecHeadDigest, ReasonExecHeadMalformed} {
		want, _, _ := strings.Cut(r, ":")
		if got := parseReasonCode("step 2: " + r); got != want {
			t.Errorf("parseReasonCode(%q) = %q, want %q", r, got, want)
		}
	}
	for _, u := range []string{exechead.UncleanHandover, exechead.UncleanReceiver, exechead.UncleanFailed, exechead.UncleanOverflow, exechead.UncleanStuck, exechead.UncleanStall} {
		if got := parseReasonCode("step 1: unclean:" + u); got != "unclean:"+u {
			t.Errorf("unclean %q filed as %q", u, got)
		}
	}
	if got := parseReasonCode("step 1: unclean:exechead-nonsense"); got != "unclean:other" {
		t.Errorf("an unknown exechead unclean reason is %q", got)
	}
	_ = filepath.Join // keep the import for platforms that trim it
}
