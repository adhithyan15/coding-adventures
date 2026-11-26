package record

// The exec-head sidecar of a step (KILN04 D48, step 4)
// ====================================================
//
// `kiln record step` is the supervisor of the step's exec notifications
// (internal/exechead).  This file is the recorder's side of that: it creates the
// `<step>.exechead` file exclusively before the step starts, draws the nonce the
// header and the step sidecar share, and tells the supervisor which directories
// are the step's fresh tmpfs.
//
//	traces/<sha256(action)>/0.trace      strace's output
//	traces/<sha256(action)>/0.exechead   the supervisor's lines (JSON Lines)
//	traces/<sha256(action)>/0.json       the step sidecar: exechead_nonce,
//	                                     exechead_sha256 and exechead_bytes bind
//	                                     the file to this step
//
// Nothing here changes what the classifier believes.  Consuming the file is
// KILN04 D48 step 5; until then `collect` only checks that the file is a regular
// file, and the classifier never opens it.

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"os"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// newExecHead creates the step's exechead file (O_EXCL, in a directory the
// sandboxed step cannot see) and the supervisor that writes it.  It returns the
// supervisor, the open file (the caller closes it) and the nonce.
func newExecHead(o StepOptions, pre *Pre, action string, step int) (*exechead.Supervisor, *os.File, string, error) {
	f, err := os.OpenFile(StepFile(o.Dir, action, step, "exechead"), os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return nil, nil, "", fmt.Errorf("the exec-head file: %w", err)
	}
	var raw [16]byte
	if _, err := rand.Read(raw[:]); err != nil {
		f.Close()
		return nil, nil, "", fmt.Errorf("the exec-head nonce: %w", err)
	}
	nonce := hex.EncodeToString(raw[:])
	sum := sha256.Sum256([]byte(action))
	sup, err := exechead.New(exechead.Config{
		Out:    f,
		Header: exechead.Header{V: exechead.Version, Nonce: nonce, ActionSHA256: hex.EncodeToString(sum[:]), Step: step},
		Tmpfs:  execHeadTmpfs(pre.Config),
		Timing: o.ExecHeadTiming,
	})
	if err != nil {
		f.Close()
		return nil, nil, "", err
	}
	return sup, f, nonce, nil
}

// execHeadTmpfs are the directories of the step's fresh tmpfs, as the step sees
// them: $HOME, $TMPDIR and the scratch directories (the classifier's `underTmp`
// set).  The supervisor measures their devices through /proc/<tgid>/root.
func execHeadTmpfs(cfg *sandbox.Config) []string {
	var out []string
	for _, d := range []string{cfg.Home, cfg.Tmpdir} {
		if d != "" {
			out = append(out, d)
		}
	}
	return append(out, SandboxScratchDirs()...)
}
