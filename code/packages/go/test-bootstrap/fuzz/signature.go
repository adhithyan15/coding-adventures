package fuzz

import (
	"errors"
	"fmt"
	"slices"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/prng"
)

// -----------------------------------------------------------------------------
// Failure ids: a closed vocabulary (VER06 section 6)
// -----------------------------------------------------------------------------

// The registered fuzz failure ids.  They are prefixed FuzzFID because the pbt
// lane owns the bare names.  fuzz-leak and fuzz-panic are registered but this
// engine cannot produce them (VER06e section 9, item 9).
const (
	FuzzFIDCrash   = "fuzz-crash"
	FuzzFIDTimeout = "fuzz-timeout"
	FuzzFIDOOM     = "fuzz-oom"
	FuzzFIDLeak    = "fuzz-leak"
	FuzzFIDPanic   = "fuzz-panic"
)

// FuzzFailureIDs lists every registered id.
var FuzzFailureIDs = [5]string{FuzzFIDCrash, FuzzFIDTimeout, FuzzFIDOOM, FuzzFIDLeak, FuzzFIDPanic}

// IsFuzzFailureID reports whether candidate is one of the five registered ids.
//
//	IsFuzzFailureID("fuzz-crash")           -> true
//	IsFuzzFailureID("fuzz-arbitrary-error") -> false
func IsFuzzFailureID(candidate string) bool { return slices.Contains(FuzzFailureIDs[:], candidate) }

// ErrSigHash is returned for a signature hash that is not exactly 8 lowercase
// hex digits.
var ErrSigHash = errors.New("fuzz: sig hash must be exactly 8 lowercase hex digits")

// maxTargetLen caps the target id in a TAP line (bytes).
const maxTargetLen = 128

// ErrFailureID is returned for an id outside the closed vocabulary.
var ErrFailureID = errors.New("fuzz: failure id is not one of the registered FuzzFailureIDs")

// -----------------------------------------------------------------------------
// Signatures
// -----------------------------------------------------------------------------

// CrashSignature is the low 32 bits of fnv1a64(kind ":" frames joined by ":"),
// as 8 lowercase hex digits.
//
// Frames are NOT escaped (VER06 section 9.4 item 6): one frame "x:y" and two
// frames "x", "y" hash alike.  This lane never produces a frame with a colon
// (normaliseFrame), so the ambiguity cannot arise from Go targets.
func CrashSignature(kind string, frames []string) string {
	raw := kind + ":" + strings.Join(frames, ":")
	return fmt.Sprintf("%08x", prng.FNV1a64String(raw)&0xFFFFFFFF)
}

// IsDuplicateCrash reports whether two crash reports share one signature.
func IsDuplicateCrash(kindA string, framesA []string, kindB string, framesB []string) bool {
	return CrashSignature(kindA, framesA) == CrashSignature(kindB, framesB)
}

// -----------------------------------------------------------------------------
// TAP: the only public failure line
// -----------------------------------------------------------------------------

// token reduces text to the characters its grammar allows; every other byte
// becomes "_".  Without this a target id holding a newline would let a caller
// forge a second TAP line inside "an opaque token".  Multi-byte UTF-8 is
// replaced byte for byte, so the result is pure ASCII.
func token(text string, allowed func(byte) bool) string {
	out := []byte(text)
	for i, c := range out {
		if !allowed(c) {
			out[i] = '_'
		}
	}
	return string(out)
}

func isIDByte(c byte) bool {
	return c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c >= '0' && c <= '9' ||
		c == '.' || c == '_' || c == '/' || c == '-'
}

func isHexByte(c byte) bool { return c >= '0' && c <= '9' || c >= 'a' && c <= 'f' }

// FormatTAPFailure is the only public failure line:
//
//	not ok N - target [failure-id:hex8]
//
// It carries no input bytes, seed, message or stack trace (VER00 7.3).
// failureID must be one of the five registered ids (ErrFailureID otherwise).
// The target and hash are reduced to what their grammar allows (target
// [A-Za-z0-9._/-], hash lowercase hex), every other byte becoming "_".
// sigHash must be exactly 8 lowercase hex digits (ErrSigHash otherwise): the
// field is a hash, never free text.  The target is truncated to 128 bytes.
// Well-formed arguments pass through unchanged, so the pinned vectors hold.
func FormatTAPFailure(testNum int, target, failureID, sigHash string) (string, error) {
	if !IsFuzzFailureID(failureID) {
		return "", ErrFailureID
	}
	if len(sigHash) != 8 || token(sigHash, isHexByte) != sigHash {
		return "", ErrSigHash
	}
	if len(target) > maxTargetLen {
		target = target[:maxTargetLen]
	}
	return fmt.Sprintf("not ok %d - %s [%s:%s]", testNum, token(target, isIDByte), failureID, sigHash), nil
}

// TAPFailures renders a report as public, opaque failure lines (none for a
// clean one): one per distinct crash signature, then one for timeouts and one
// for OOMs.  A timeout or OOM has no stack to hash, so its sig hash is the
// signature of the bare kind.
func TAPFailures(r *Report, target string, firstTest int) []string {
	var lines []string
	add := func(id, sig string) {
		// id is always registered here, so the error cannot occur.
		line, _ := FormatTAPFailure(firstTest+len(lines), target, id, sig)
		lines = append(lines, line)
	}
	for _, rec := range r.Crashes {
		add(FuzzFIDCrash, rec.Signature)
	}
	if r.Timeouts > 0 {
		add(FuzzFIDTimeout, CrashSignature("timeout", nil))
	}
	if r.OOMs > 0 {
		add(FuzzFIDOOM, CrashSignature("oom", nil))
	}
	return lines
}

// -----------------------------------------------------------------------------
// Frame normalisation (VER06e section 5)
// -----------------------------------------------------------------------------

const (
	// maxFrames keeps the innermost frames of a stack.
	maxFrames = 64
	// maxKindLen caps a crash kind.
	maxKindLen = 64
)

// safeName restricts text to [A-Za-z0-9._<>$/-]; anything else becomes "_".
func safeName(s string) string {
	return token(s, func(c byte) bool { return isIDByte(c) || c == '<' || c == '>' || c == '$' })
}

// normaliseFrame turns a runtime function name into "package.Function":
//
//	github.com/x/y/pkg.(*T).Method        -> pkg.T.Method
//	github.com/x/y/pkg.Gen[go.shape.int]  -> pkg.Gen
//	main.main.func1                       -> main.main.func1
//
// The import-path directory prefix is dropped; "(*T)" becomes "T"; generic
// type arguments are removed.  The result contains no ":" (safeName).
func normaliseFrame(fn string) string {
	// Drop the directory part, but only if the last "/" comes before the first
	// "(" or "[" (a "/" can appear inside generic arguments).
	cut := strings.IndexAny(fn, "([")
	if cut < 0 {
		cut = len(fn)
	}
	if i := strings.LastIndex(fn[:cut], "/"); i >= 0 {
		fn = fn[i+1:]
	}
	// Remove [...] groups (generic arguments), nesting-aware.
	var b strings.Builder
	depth := 0
	for i := 0; i < len(fn); i++ {
		switch c := fn[i]; {
		case c == '[':
			depth++
		case c == ']' && depth > 0:
			depth--
		case depth > 0:
		case c == '(' || c == ')' || c == '*':
		default:
			b.WriteByte(c)
		}
	}
	return safeName(b.String())
}

// normaliseKind turns the %T of a panic value into a signature kind.
func normaliseKind(typeName string) string {
	k := safeName(strings.TrimLeft(typeName, "*"))
	if len(k) > maxKindLen {
		k = k[:maxKindLen]
	}
	return k
}

// collapse removes consecutive duplicate frames, so a recursive panic hashes
// alike at any depth, and keeps the innermost maxFrames.
func collapse(frames []string) []string {
	out := frames[:0:0]
	for _, f := range frames {
		if len(out) == 0 || out[len(out)-1] != f {
			out = append(out, f)
		}
	}
	if len(out) > maxFrames {
		out = out[:maxFrames]
	}
	return out
}
