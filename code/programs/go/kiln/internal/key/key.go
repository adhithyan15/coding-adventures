// Package key computes Kiln action keys exactly as specified in
// code/specs/KILN02-key-encoding.md (key schema version 1).
//
// # The one question a key answers
//
// "Has exactly this work already passed?"  Two machines must compute the same
// 32 bytes for the same action, and two different actions must never share
// bytes.  Both properties come from the ENCODING, i.e. how an action is turned
// into the byte string ("preimage") that SHA-256 hashes.
//
// # Why every value is length-framed
//
// Joining fields with a separator is ambiguous:
//
//	target "a,b" + phase "test"   ->  "a,b,test"
//	target "a"   + phase "b,test" ->  "a,b,test"     (same bytes, different work)
//
// Framing fixes that.  bytes(b) = u64(len(b)) || b, so the two actions above
// become
//
//	00..03 'a' ',' 'b'  00..04 't' 'e' 's' 't'
//	00..01 'a'          00..06 'b' ',' 't' 'e' 's' 't'
//
// which differ from the very first length byte onward.  Nothing is ever
// inferred from a separator; each section also starts with its own name so a
// section can never be mistaken for a neighbour.
//
// # Why bytewise sort, and why refuse instead of repair
//
// Lists must be sorted so two implementations agree on order.  We compare raw
// UTF-8 bytes (bytes.Compare), never locale order, so 'B' (0x42) sorts before
// 'a' (0x61) everywhere.  And if input is NOT sorted, or has duplicates, or a
// malformed path, we return an error rather than fixing it: silently repairing
// would let two implementations disagree about which action they keyed, which
// is exactly the false-hit failure Kiln exists to avoid.  Callers must hand us
// canonical input; we only verify and encode it.
package key

import (
	"bytes"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"unicode/utf8"
)

// SchemaVersion is the key-schema version implemented by this package.
const SchemaVersion uint64 = 1

// Phase names.
const (
	PhaseFetch = "fetch"
	PhaseBuild = "build"
	PhaseTest  = "test"
)

// Entry kinds for inputs/reads.
const (
	KindFile uint8 = 1
	KindLink uint8 = 2
)

// Tool kinds (KILN02 section 3.8).
const (
	ToolAbsent   uint8 = 0 // optional and absent; identity empty
	ToolManaged  uint8 = 1 // managed download; identity = 32-byte archive SHA-256
	ToolHost     uint8 = 2 // supplied by the host; identity = check output, byte-exact
	ToolInRepo   uint8 = 3 // built in this repository; identity = 32-byte action key
	DigestLength       = 32
)

// Env origins.
const (
	OriginEnv  uint8 = 0
	OriginPass uint8 = 1
)

// FileEntry is one element of the "inputs" or "reads" section.
type FileEntry struct {
	Path   string
	Kind   uint8
	Exec   uint8
	Digest []byte // exactly 32 bytes
}

// Consumed is one element of the "consumed" section: the key of an upstream
// action whose output this action uses.
type Consumed struct {
	Target string
	Phase  string
	Key    []byte // exactly 32 bytes
}

// Tool is one element of the "tools" section.
type Tool struct {
	Name     string
	Version  string
	Kind     uint8
	Identity []byte
}

// EnvVar is one element of the "env" section.
type EnvVar struct {
	Name   string
	Value  string
	Origin uint8
}

// Action is the complete description of one unit of work.  The nine sections
// of the preimage map onto it in order: platform (OS, Arch), host
// (HostImageID), action (Target, Phase), recipe, inputs, reads, consumed,
// tools, env.
type Action struct {
	SemanticsVersion uint64 // KILN01 expansion semantics; 0 for legacy actions
	OS               string // linux | macos | windows
	Arch             string // x64 | arm64
	HostImageID      *string
	Target           string
	Phase            string
	Recipe           []byte
	Inputs           []FileEntry
	Reads            []FileEntry
	Consumed         []Consumed
	Tools            []Tool
	Env              []EnvVar
}

// ---------------------------------------------------------------------------
// Building blocks (KILN02 section 2).  Each appends to dst and returns it.
// ---------------------------------------------------------------------------

// putU8 appends a single byte: u8(1) = 01.
func putU8(dst []byte, n uint8) []byte { return append(dst, n) }

// putU64 appends eight big-endian bytes: u64(3) = 00 00 00 00 00 00 00 03.
func putU64(dst []byte, n uint64) []byte {
	return binary.BigEndian.AppendUint64(dst, n)
}

// putBytes appends u64(len(b)) || b: bytes("hi") = 00..02 68 69.
func putBytes(dst []byte, b []byte) []byte {
	return append(putU64(dst, uint64(len(b))), b...)
}

// putStr appends bytes(UTF-8 of s).  Go strings already hold raw bytes, so no
// normalisation, case folding or newline conversion can happen here.  The
// caller is responsible for having validated UTF-8 first.
func putStr(dst []byte, s string) []byte { return putBytes(dst, []byte(s)) }

// ---------------------------------------------------------------------------
// Validation (KILN02 section 4).  Errors, never repairs.
// ---------------------------------------------------------------------------

func checkUTF8(what, s string) error {
	if !utf8.ValidString(s) {
		return fmt.Errorf("%s is not valid UTF-8", what)
	}
	return nil
}

func checkOS(os string) error {
	switch os {
	case "linux", "macos", "windows":
		return nil
	}
	return fmt.Errorf("unknown os %q", os)
}

func checkArch(arch string) error {
	switch arch {
	case "x64", "arm64":
		return nil
	}
	return fmt.Errorf("unknown arch %q", arch)
}

func checkPhase(phase string) error {
	switch phase {
	case PhaseFetch, PhaseBuild, PhaseTest:
		return nil
	}
	return fmt.Errorf("unknown phase %q", phase)
}

// checkPath enforces: valid UTF-8, repository-relative, "/"-separated, no
// leading "/", no empty / "." / ".." segment, no trailing "/".  A backslash
// is an ordinary byte (KILN02 does not mention it), so "a\b" is one segment.
//
//	"src/a.go"  ok        "/src"   leading slash     "a//b"  empty segment
//	"a/./b"     "."       "a/../b" ".."              "a/"    trailing slash
func checkPath(p string) error {
	if err := checkUTF8("path", p); err != nil {
		return err
	}
	if p == "" {
		return fmt.Errorf("malformed path %q: empty", p)
	}
	if p[0] == '/' {
		return fmt.Errorf("malformed path %q: leading slash", p)
	}
	if p[len(p)-1] == '/' {
		return fmt.Errorf("malformed path %q: trailing slash", p)
	}
	start := 0
	for i := 0; i <= len(p); i++ {
		if i == len(p) || p[i] == '/' {
			switch seg := p[start:i]; seg {
			case "":
				return fmt.Errorf("malformed path %q: empty segment", p)
			case ".", "..":
				return fmt.Errorf("malformed path %q: %q segment", p, seg)
			}
			start = i + 1
		}
	}
	return nil
}

// ValidatePath reports whether p is a valid Kiln path (KILN02 section 3.5).
// Source implementations use it so every path they return is key-ready.
func ValidatePath(p string) error { return checkPath(p) }

func checkDigest(what string, d []byte) error {
	if len(d) != DigestLength {
		return fmt.Errorf("%s must be exactly %d bytes, got %d", what, DigestLength, len(d))
	}
	return nil
}

// checkOrder demands strictly ascending order: cmp < 0 between neighbours.
// Equal neighbours are duplicates, descending ones are unsorted.
func checkOrder(section string, prev, cur []byte) error {
	c := bytes.Compare(prev, cur)
	if c == 0 {
		return fmt.Errorf("%s: duplicate entry %q", section, cur)
	}
	if c > 0 {
		return fmt.Errorf("%s: entries not sorted bytewise: %q before %q", section, prev, cur)
	}
	return nil
}

func validateFiles(section string, es []FileEntry) error {
	for i, e := range es {
		if err := checkPath(e.Path); err != nil {
			return fmt.Errorf("%s: %w", section, err)
		}
		switch e.Kind {
		case KindFile, KindLink:
		default:
			return fmt.Errorf("%s: unknown kind %d for %q", section, e.Kind, e.Path)
		}
		if e.Exec > 1 {
			return fmt.Errorf("%s: exec must be 0 or 1 for %q", section, e.Path)
		}
		if e.Kind == KindLink && e.Exec != 0 {
			return fmt.Errorf("%s: link %q must have exec=0", section, e.Path)
		}
		if err := checkDigest(section+" digest for "+e.Path, e.Digest); err != nil {
			return err
		}
		if i > 0 {
			if err := checkOrder(section, []byte(es[i-1].Path), []byte(e.Path)); err != nil {
				return err
			}
		}
	}
	return nil
}

func validateConsumed(es []Consumed) error {
	for i, e := range es {
		if err := checkUTF8("consumed target", e.Target); err != nil {
			return err
		}
		if err := checkPhase(e.Phase); err != nil {
			return fmt.Errorf("consumed: %w", err)
		}
		if err := checkDigest("consumed key for "+e.Target, e.Key); err != nil {
			return err
		}
		if i > 0 {
			p := es[i-1]
			c := bytes.Compare([]byte(p.Target), []byte(e.Target))
			if c == 0 {
				c = bytes.Compare([]byte(p.Phase), []byte(e.Phase))
			}
			if c == 0 {
				return fmt.Errorf("consumed: duplicate entry (%q, %q)", e.Target, e.Phase)
			}
			if c > 0 {
				return fmt.Errorf("consumed: entries not sorted bytewise: (%q, %q) before (%q, %q)",
					p.Target, p.Phase, e.Target, e.Phase)
			}
		}
	}
	return nil
}

func validateTools(es []Tool) error {
	for i, e := range es {
		if err := checkUTF8("tool name", e.Name); err != nil {
			return err
		}
		if err := checkUTF8("tool version", e.Version); err != nil {
			return err
		}
		switch e.Kind {
		case ToolAbsent:
			if len(e.Identity) != 0 {
				return fmt.Errorf("tools: absent tool %q must have empty identity", e.Name)
			}
		case ToolManaged, ToolInRepo:
			if err := checkDigest("tool identity for "+e.Name, e.Identity); err != nil {
				return err
			}
		case ToolHost: // any bytes, byte-exact
		default:
			return fmt.Errorf("tools: unknown kind %d for %q", e.Kind, e.Name)
		}
		if i > 0 {
			if err := checkOrder("tools", []byte(es[i-1].Name), []byte(e.Name)); err != nil {
				return err
			}
		}
	}
	return nil
}

func validateEnv(es []EnvVar) error {
	for i, e := range es {
		if err := checkUTF8("env name", e.Name); err != nil {
			return err
		}
		if err := checkUTF8("env value", e.Value); err != nil {
			return err
		}
		if e.Origin > OriginPass {
			return fmt.Errorf("env: unknown origin %d for %q", e.Origin, e.Name)
		}
		if i > 0 {
			if err := checkOrder("env", []byte(es[i-1].Name), []byte(e.Name)); err != nil {
				return err
			}
		}
	}
	return nil
}

// Validate checks an action against KILN02 section 4 without encoding it.
func Validate(a Action) error {
	if err := checkOS(a.OS); err != nil {
		return err
	}
	if err := checkArch(a.Arch); err != nil {
		return err
	}
	if a.HostImageID != nil {
		if err := checkUTF8("host image id", *a.HostImageID); err != nil {
			return err
		}
	}
	if err := checkUTF8("target", a.Target); err != nil {
		return err
	}
	if err := checkPhase(a.Phase); err != nil {
		return err
	}
	if err := validateFiles("inputs", a.Inputs); err != nil {
		return err
	}
	if err := validateFiles("reads", a.Reads); err != nil {
		return err
	}
	if err := validateConsumed(a.Consumed); err != nil {
		return err
	}
	if err := validateTools(a.Tools); err != nil {
		return err
	}
	return validateEnv(a.Env)
}

// ---------------------------------------------------------------------------
// Encoding (KILN02 section 3)
// ---------------------------------------------------------------------------

// Preimage returns the exact bytes that are hashed to make the key, or an
// error if the action is not canonical.
//
// Worked example: a minimal action (linux/x64, target "t", phase "build",
// recipe "r", all lists empty) begins
//
//	00..08 "kiln-key"   00..01 (schema 1)   00..00 (semantics 0)
//	00..08 "platform"   00..05 "linux"      00..03 "x64"
//	00..04 "host"       00                  (no host fingerprint)
//	...
func Preimage(a Action) ([]byte, error) {
	if err := Validate(a); err != nil {
		return nil, err
	}
	b := putStr(nil, "kiln-key")
	b = putU64(b, SchemaVersion)
	b = putU64(b, a.SemanticsVersion)

	b = putStr(b, "platform")
	b = putStr(b, a.OS)
	b = putStr(b, a.Arch)

	b = putStr(b, "host")
	if a.HostImageID == nil {
		b = putU8(b, 0)
	} else {
		b = putU8(b, 1)
		b = putStr(b, *a.HostImageID)
	}

	b = putStr(b, "action")
	b = putStr(b, a.Target)
	b = putStr(b, a.Phase)

	b = putStr(b, "recipe")
	b = putBytes(b, a.Recipe)

	b = putFiles(b, "inputs", a.Inputs)
	b = putFiles(b, "reads", a.Reads)

	b = putStr(b, "consumed")
	b = putU64(b, uint64(len(a.Consumed)))
	for _, e := range a.Consumed {
		b = putStr(b, e.Target)
		b = putStr(b, e.Phase)
		b = putBytes(b, e.Key)
	}

	b = putStr(b, "tools")
	b = putU64(b, uint64(len(a.Tools)))
	for _, e := range a.Tools {
		b = putStr(b, e.Name)
		b = putStr(b, e.Version)
		b = putU8(b, e.Kind)
		b = putBytes(b, e.Identity)
	}

	b = putStr(b, "env")
	b = putU64(b, uint64(len(a.Env)))
	for _, e := range a.Env {
		b = putStr(b, e.Name)
		b = putStr(b, e.Value)
		b = putU8(b, e.Origin)
	}
	return b, nil
}

func putFiles(b []byte, section string, es []FileEntry) []byte {
	b = putStr(b, section)
	b = putU64(b, uint64(len(es)))
	for _, e := range es {
		b = putStr(b, e.Path)
		b = putU8(b, e.Kind)
		b = putU8(b, e.Exec)
		b = putBytes(b, e.Digest)
	}
	return b
}

// Key is SHA-256(Preimage(a)).
func Key(a Action) ([32]byte, error) {
	p, err := Preimage(a)
	if err != nil {
		return [32]byte{}, err
	}
	return KeyOfPreimage(p), nil
}

// KeyOfPreimage hashes an already-built preimage.  It exists so a caller that
// has just obtained a preimage successfully (the vector verifier) can derive
// the key without calling Key and having to discard an error that cannot
// occur: Key fails only when Preimage fails, and that has been ruled out.
func KeyOfPreimage(preimage []byte) [32]byte { return sha256.Sum256(preimage) }

// Hex renders a key as 64 lowercase hex characters.
func Hex(k [32]byte) string { return hex.EncodeToString(k[:]) }

// LegacyRecipe builds the canonical recipe of a legacy shell action
// (KILN02 section 3.4):
//
//	str("legacy-shell") || list([ file(BUILD), file(BUILD_windows)? ])
//	file(name) = str(name) || bytes(contents)
//
// buildWindows is nil when BUILD_windows does not exist.  An existing but
// empty file is a non-nil empty slice and IS included: absent and empty are
// different recipes.
func LegacyRecipe(build []byte, buildWindows []byte) []byte {
	count := uint64(1)
	if buildWindows != nil {
		count = 2
	}
	b := putStr(nil, "legacy-shell")
	b = putU64(b, count)
	b = putStr(b, "BUILD")
	b = putBytes(b, build)
	if buildWindows != nil {
		b = putStr(b, "BUILD_windows")
		b = putBytes(b, buildWindows)
	}
	return b
}
