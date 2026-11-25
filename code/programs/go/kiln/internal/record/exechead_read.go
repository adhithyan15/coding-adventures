package record

// Reading the exec-head sidecar back (KILN04 D48, step 5)
// ======================================================
//
// `kiln record step --exec-head` leaves `<step>.exechead` beside the trace
// (exechead_step.go) and binds it to the step sidecar with a nonce, a digest and
// a length.  The classifier will now BELIEVE the heads in it (internal/traced,
// exechead.go), so before it is handed anything this file checks everything that
// only the recorder can:
//
//	step sidecar says                file on disk                verdict
//	-------------------------------  --------------------------  ---------------------------
//	no exechead fields (no           absent                      no file: behaviour of today
//	  supervisor ran)
//	no exechead fields               PRESENT                     refuse the action: a file
//	                                                             nobody claims is a second file
//	nonce, sha256, bytes             missing / not a regular     refuse the action
//	                                 file / a link
//	nonce, sha256, bytes             over MaxFileBytes           refuse the action (oversize)
//	nonce, sha256, bytes             length or digest differs    refuse the action
//	nonce, sha256, bytes             does not decode strictly,   refuse the action
//	                                 or names another nonce,
//	                                 action or step
//	nonce, sha256, bytes             matches                     decoded, handed to Classify
//
// "Refuse the action" is one more reason in the record's parse_reasons, exactly
// as an unparsable trace is: the action's write set is unknown, never "no
// sidecar, carry on".  The reason texts are FIXED phrases, never text taken from
// the file (the census maps each to a code, census_explain.go).
//
// The digest check is what makes the per-line `sha256` (self-attested) enough:
// the file is the one the supervisor wrote and synced before the step sidecar,
// and the nonce in its header is the step sidecar's.

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"io"
	"os"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
)

// The parse reasons of an exec-head file that cannot be trusted.  Each is a
// fixed phrase; parseReasonCode maps them (census_explain.go).
const (
	ReasonExecHeadMissing   = "exechead-missing: the step sidecar names an exec-head file that cannot be read"
	ReasonExecHeadUnclaimed = "exechead-unclaimed: an exec-head file exists that the step sidecar does not name"
	ReasonExecHeadOversize  = "exechead-oversize: the exec-head file is over its cap"
	ReasonExecHeadDigest    = "exechead-digest: the exec-head file is not the one the step sidecar binds"
	ReasonExecHeadMalformed = "exechead-malformed: the exec-head file does not decode"
)

// readExecHead reads and checks step's exec-head file.  It returns the decoded
// file, or nil with a reason (never both).  A step that ran without the
// supervisor (no nonce in its sidecar, no file) is nil with no reason.
func readExecHead(dir, action string, step int, sc StepSidecar) (*exechead.File, string) {
	path := StepFile(dir, action, step, "exechead")
	if sc.ExecHeadNonce == "" {
		if _, err := os.Lstat(path); err == nil {
			return nil, ReasonExecHeadUnclaimed
		} else if !errors.Is(err, os.ErrNotExist) {
			return nil, ReasonExecHeadUnclaimed // cannot tell: fail closed
		}
		return nil, ""
	}
	f, err := openNoFollow(path)
	if err != nil {
		return nil, ReasonExecHeadMissing
	}
	defer f.Close()
	st, err := f.Stat()
	if err != nil || !st.Mode().IsRegular() {
		return nil, ReasonExecHeadMissing
	}
	// Bound BEFORE reading: the file is supervisor output, but the classifier's
	// child holds it in memory.
	if st.Size() > exechead.MaxFileBytes {
		return nil, ReasonExecHeadOversize
	}
	data, err := io.ReadAll(io.LimitReader(f, exechead.MaxFileBytes+1))
	if err != nil {
		return nil, ReasonExecHeadMissing
	}
	if int64(len(data)) > exechead.MaxFileBytes {
		return nil, ReasonExecHeadOversize
	}
	sum := sha256.Sum256(data)
	if int64(len(data)) != sc.ExecHeadBytes || hex.EncodeToString(sum[:]) != sc.ExecHeadSHA256 {
		return nil, ReasonExecHeadDigest
	}
	act := sha256.Sum256([]byte(action))
	file, err := exechead.Decode(data, exechead.Limits{}, exechead.Expect{
		Nonce: sc.ExecHeadNonce, ActionSHA256: hex.EncodeToString(act[:]), Step: step,
	})
	if err != nil {
		return nil, ReasonExecHeadMalformed
	}
	return file, ""
}
