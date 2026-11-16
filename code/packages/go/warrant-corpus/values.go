package warrantcorpus

import (
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"hash"
	"regexp"
	"strconv"
	"strings"
)

// # Warrant types and values, as this lane represents them
//
// A Warrant value crosses three boundaries, and each has its own encoding:
//
//	boundary       encoding
//	corpus JSON    tagged objects: {"type": "Bytes", "value": "00"}
//	cases.tfv      TFV tokens (see tfv.go)
//	stream digest  warrant-stream-v1 bytes (below)
//
// # Two forms of a value
//
// Inside the runner a value is CANONICAL: the same shape whatever the library
// under test prefers, so the model, the digests and the comparison never depend
// on a binding.
//
//	Warrant type   canonical form    library form (what an adapter receives)
//	Bool           bool              bool
//	U8             uint64            uint8
//	U64, Nat       uint64            uint64
//	Bytes          []byte            []byte (always the runner's own fresh copy)
//	Result<T,E>    Pair{"ok", v} / Pair{"err", "VARIANT"}
//
// U64 is a native uint64: parsed unsigned, never through a signed type, so
// 2^64-1 and 2^64-2 stay different values. U8 fits uint8 exactly, which is how a
// library naturally takes a byte.
//
// # Why exact types matter
//
// Go's interfaces make "right value, wrong type" a real possibility: an adapter
// that returns int(1) for a Warrant U64, or a named byte-slice type for Bytes,
// holds a value that prints the same and compares unequal under a type
// assertion. So a library result is first checked with checkValue, which asks
// for the exact Go type, and only then compared.
//
// # The stream encoding (VER04i section 2.6)
//
// Digests are over bytes that every lane can produce identically:
//
//	Bool         00 or 01
//	U8/U64/Nat   10 + 8 bytes little-endian (unsigned)
//	Bytes        20 + u32-LE length + the bytes
//	Ok(v)        50 + encoding of v
//	Err(E.V)     51 + u32-LE length + UTF-8 variant name
const (
	// maxWidth is VER04h's MAX_FIXED_WIDTH, which bounds every Bytes<N>.
	maxWidth = 4096
	u64Max   = ^uint64(0)
)

var (
	decimalRE   = regexp.MustCompile(`^(?:0|[1-9][0-9]*)$`)
	hexRE       = regexp.MustCompile(`^(?:[0-9a-f]{2})*$`)
	bytesTypeRE = regexp.MustCompile(`^Bytes(?:<([0-9]+|[A-Z][A-Za-z0-9_]*)>)?$`)
	errorNameRE = regexp.MustCompile(`^[A-Z][A-Za-z0-9_]*$`)
	digitsRE    = regexp.MustCompile(`^[0-9]+$`)
)

// The scalar and compound kinds of the closed warrant-corpus-v1 profile.
const (
	kindBool   = "Bool"
	kindU8     = "U8"
	kindU64    = "U64"
	kindNat    = "Nat"
	kindBytes  = "Bytes"
	kindResult = "Result"
)

// Pair is a canonical Result value, and also the shape of an `err` / `name` IR
// value: Tag "ok" carries a value in Body, Tag "err" or "name" carries a variant
// name (a string).
type Pair struct {
	Tag  string
	Body any
}

// Type is a parsed Warrant type.
//
// For Bytes<N>: HasWidth is set, and either WidthName is the parameter name
// ("N", in an operation signature) or Width is the literal width. For Result: Ok
// and Error are set.
type Type struct {
	Kind      string
	Text      string
	HasWidth  bool
	Width     int
	WidthName string
	Ok        *Type
	Error     string
}

// ErrorTypes maps each declared error type to its variant names.
type ErrorTypes map[string][]string

// Param is one named operation parameter.
type Param struct {
	Name string
	Ty   *Type
}

// OpSig is an operation's signature as the corpus declares it.
type OpSig struct {
	Name        string
	Params      []Param
	Ret         *Type
	WidthParams []string
	Fresh       bool
}

func isIntKind(kind string) bool { return kind == kindU8 || kind == kindU64 || kind == kindNat }

// upperBound is the largest value of an integer kind.
func upperBound(kind string) uint64 {
	if kind == kindU8 {
		return 255
	}
	return u64Max
}

func splitTopLevelComma(text string) (string, string) {
	depth := 0
	for i := 0; i < len(text); i++ {
		switch text[i] {
		case '<':
			depth++
		case '>':
			depth--
		case ',':
			if depth == 0 {
				return text[:i], text[i+1:]
			}
		}
	}
	fail("unsupported-type", "malformed Result type")
	return "", ""
}

// parseType parses a type name; anything outside the closed profile is an error.
func parseType(text string) *Type {
	switch text {
	case kindBool, kindU8, kindU64, kindNat:
		return &Type{Kind: text, Text: text}
	}
	if match := bytesTypeRE.FindStringSubmatch(text); match != nil {
		ty := &Type{Kind: kindBytes, Text: text}
		if match[1] == "" {
			return ty
		}
		ty.HasWidth = true
		if digitsRE.MatchString(match[1]) {
			width, err := strconv.Atoi(match[1])
			need(err == nil && width <= maxWidth, "unsupported-type", "Bytes width exceeds the fixed-width bound")
			ty.Width = width
		} else {
			ty.WidthName = match[1]
		}
		return ty
	}
	if strings.HasPrefix(text, "Result<") && strings.HasSuffix(text, ">") {
		okText, errName := splitTopLevelComma(text[len("Result<") : len(text)-1])
		need(errorNameRE.MatchString(errName), "unsupported-type", "Result error type must be a declared error name")
		return &Type{Kind: kindResult, Text: text, Ok: parseType(okText), Error: errName}
	}
	fail("unsupported-type", "type is outside the warrant-corpus-v1 profile")
	return nil
}

// ---------------------------------------------------------------------------
// Comparison and hex
// ---------------------------------------------------------------------------

// fromHex decodes lowercase even-length hex (the caller has already validated it).
func fromHex(text string) []byte {
	out, err := hex.DecodeString(text)
	need(err == nil, "value", "bad hex")
	return out
}

func bytesEqual(left, right []byte) bool {
	if len(left) != len(right) {
		return false
	}
	for i := range left {
		if left[i] != right[i] {
			return false
		}
	}
	return true
}

// valuesEqual is structural equality of two CANONICAL values.
//
// Strict: a bool never equals a uint64, and a []byte equals only another []byte
// with the same bytes. Go's == on interface values would compare dynamic types
// too, but would panic on a slice, so each case is spelled out.
func valuesEqual(left, right any) bool {
	switch l := left.(type) {
	case bool:
		r, ok := right.(bool)
		return ok && l == r
	case uint64:
		r, ok := right.(uint64)
		return ok && l == r
	case string:
		r, ok := right.(string)
		return ok && l == r
	case []byte:
		r, ok := right.([]byte)
		return ok && bytesEqual(l, r)
	case Pair:
		r, ok := right.(Pair)
		return ok && l.Tag == r.Tag && valuesEqual(l.Body, r.Body)
	}
	return false
}

// ---------------------------------------------------------------------------
// Corpus JSON  ->  canonical value
// ---------------------------------------------------------------------------

// isTagged: an object with exactly the keys "type" and "value", whose type is text.
func isTagged(raw any, text string) (map[string]any, bool) {
	object, ok := raw.(map[string]any)
	if !ok || len(object) != 2 {
		return nil, false
	}
	tag, hasType := object["type"]
	_, hasValue := object["value"]
	if !hasType || !hasValue || tag != text {
		return nil, false
	}
	return object, true
}

func canonicalInt(text any, upper uint64, what string) uint64 {
	s, ok := text.(string)
	need(ok && decimalRE.MatchString(s), "value", "%s must be canonical decimal text", what)
	n, err := strconv.ParseUint(s, 10, 64)
	need(err == nil && n <= upper, "value", "%s is out of range", what)
	return n
}

// decodeJSON decodes a corpus JSON value of declared type ty.
//
// errors maps each declared error type to its variant names, so a Result can only
// carry a variant the Warrant module declared. Integers and byte strings are
// tagged objects, never bare JSON numbers: a U64 must not lose its low bits to a
// double.
func decodeJSON(raw any, ty *Type, errors ErrorTypes) any {
	if ty.Kind == kindBool {
		b, ok := raw.(bool)
		need(ok, "value", "Bool value must be a JSON boolean")
		return b
	}
	if ty.Kind == kindResult {
		tagged, ok := isTagged(raw, ty.Text)
		need(ok, "value", "Result value must be tagged with its type")
		body, ok := tagged["value"].(map[string]any)
		need(ok && len(body) == 1, "value", "invalid Result body")
		if inner, isOk := body["ok"]; isOk {
			return Pair{"ok", decodeJSON(inner, ty.Ok, errors)}
		}
		variant, isString := body["err"].(string)
		need(isString && containsString(errors[ty.Error], variant), "value", "Result error is not a declared variant")
		return Pair{"err", variant}
	}
	tagged, ok := isTagged(raw, ty.Text)
	need(ok, "value", "%s value must be a tagged object", ty.Text)
	text := tagged["value"]
	if ty.Kind == kindBytes {
		s, isString := text.(string)
		need(isString && hexRE.MatchString(s), "value", "Bytes value must be lowercase even-length hex")
		data := fromHex(s)
		if ty.HasWidth && ty.WidthName == "" {
			need(len(data) == ty.Width, "value", "Bytes width mismatch")
		}
		return data
	}
	return canonicalInt(text, upperBound(ty.Kind), ty.Kind)
}

func containsString(list []string, item string) bool {
	for _, candidate := range list {
		if candidate == item {
			return true
		}
	}
	return false
}

// ---------------------------------------------------------------------------
// Canonical <-> library form
// ---------------------------------------------------------------------------

// lower converts a canonical argument to the form a library takes: a U8 is a
// uint8, Bytes a FRESH copy (with capacity equal to length, so an append cannot
// write into spare room we own).
func lower(value any, ty *Type) any {
	switch ty.Kind {
	case kindU8:
		return uint8(value.(uint64))
	case kindBytes:
		source := value.([]byte)
		copied := make([]byte, len(source))
		copy(copied, source)
		return copied
	}
	return value
}

// lift converts a library result back to canonical form. The value has already
// passed checkValue.
func lift(value any, ty *Type) any {
	switch ty.Kind {
	case kindU8:
		return uint64(value.(uint8))
	case kindResult:
		pair := value.(Pair)
		if pair.Tag == "ok" {
			return Pair{"ok", lift(pair.Body, ty.Ok)}
		}
		return pair
	}
	return value
}

// ---------------------------------------------------------------------------
// Strict type check of a *result* (the thing a library returned)
// ---------------------------------------------------------------------------

// checkValue reports whether value is exactly a ty, in library form.
//
// "Exactly" is the point: an int is not a U64, a named byte-slice type is not
// Bytes, an untyped nil is not anything, and a Result must carry a declared
// variant.
func checkValue(value any, ty *Type, errors ErrorTypes) bool {
	switch ty.Kind {
	case kindBool:
		_, ok := value.(bool)
		return ok
	case kindBytes:
		_, ok := value.([]byte)
		return ok
	case kindU8:
		_, ok := value.(uint8)
		return ok
	case kindU64, kindNat:
		_, ok := value.(uint64)
		return ok
	case kindResult:
		pair, ok := value.(Pair)
		if !ok {
			return false
		}
		if pair.Tag == "ok" {
			return checkValue(pair.Body, ty.Ok, errors)
		}
		variant, isString := pair.Body.(string)
		return pair.Tag == "err" && isString && containsString(errors[ty.Error], variant)
	}
	return false
}

// ---------------------------------------------------------------------------
// Stream encoding
// ---------------------------------------------------------------------------

const hasherChunk = 64 * 1024

// StreamHasher is a SHA-256 over the warrant-stream-v1 bytes of a sequence of
// values.
//
// Hashing 500,000 tiny values one Write call at a time is slower than it needs to
// be, so values are encoded into a 64 KiB buffer that is flushed when full.
type StreamHasher struct {
	h      hash.Hash
	buffer []byte
}

func newStreamHasher(prefix []byte) *StreamHasher {
	hasher := &StreamHasher{h: sha256.New(), buffer: make([]byte, 0, hasherChunk)}
	hasher.h.Write(prefix)
	return hasher
}

func (s *StreamHasher) flush() {
	s.h.Write(s.buffer)
	s.buffer = s.buffer[:0]
}

func (s *StreamHasher) reserve(count int) {
	if len(s.buffer)+count > hasherChunk {
		s.flush()
	}
}

// write encodes value (canonical form) of type ty into the stream.
func (s *StreamHasher) write(value any, ty *Type) {
	switch ty.Kind {
	case kindBool:
		s.reserve(1)
		if value.(bool) {
			s.buffer = append(s.buffer, 1)
		} else {
			s.buffer = append(s.buffer, 0)
		}
	case kindU8, kindU64, kindNat:
		s.reserve(9)
		s.buffer = append(s.buffer, 0x10)
		s.buffer = binary.LittleEndian.AppendUint64(s.buffer, value.(uint64))
	case kindBytes:
		data := value.([]byte)
		s.reserve(5)
		s.buffer = append(s.buffer, 0x20)
		s.buffer = binary.LittleEndian.AppendUint32(s.buffer, uint32(len(data)))
		if len(data) > hasherChunk {
			s.flush()
			s.h.Write(data)
		} else {
			s.reserve(len(data))
			s.buffer = append(s.buffer, data...)
		}
	case kindResult:
		pair := value.(Pair)
		if pair.Tag == "ok" {
			s.reserve(1)
			s.buffer = append(s.buffer, 0x50)
			s.write(pair.Body, ty.Ok)
			return
		}
		name := pair.Body.(string)
		s.reserve(5)
		s.buffer = append(s.buffer, 0x51)
		s.buffer = binary.LittleEndian.AppendUint32(s.buffer, uint32(len(name)))
		s.flush()
		s.h.Write([]byte(name))
	}
}

// hexdigest is the lowercase hex digest of everything written.
func (s *StreamHasher) hexdigest() string {
	s.flush()
	return hex.EncodeToString(s.h.Sum(nil))
}

// streamPrefix is the context both digests start with (VER04i 2.6):
// "warrant-stream-v1\0", the length-prefixed suite id, the binder count, each
// binder's canonical type name, then the result type name. Because the context
// fixes every type, the per-case bytes need no delimiters.
func streamPrefix(suiteID string, binderTypes []string, resultType string) []byte {
	out := []byte("warrant-stream-v1\x00")
	field := func(text string) {
		out = binary.LittleEndian.AppendUint32(out, uint32(len(text)))
		out = append(out, text...)
	}
	field(suiteID)
	out = binary.LittleEndian.AppendUint32(out, uint32(len(binderTypes)))
	for _, name := range binderTypes {
		field(name)
	}
	field(resultType)
	return out
}

// ---------------------------------------------------------------------------
// TFV value  ->  canonical value
// ---------------------------------------------------------------------------

// TfvValue is a parsed TFV value. Kind is one of "null", "bool", "int", "bytes",
// "text", "list" or "map"; the field that matches the kind is the payload.
// Integers keep their decimal text so no width is imposed until a declared type
// asks for one.
type TfvValue struct {
	Kind  string
	Bool  bool
	Text  string
	Bytes []byte
	List  []TfvValue
	Pairs [][2]TfvValue
}

// decodeTFV decodes a parsed TFV value of declared type ty (VER04i section 2.7).
//
// Bool is T/F, integers are I<n>, bytes are X<hex>, and a Result is a one-entry
// map whose key is the text `ok` (value) or `err` (the VARIANT NAME as text).
func decodeTFV(raw TfvValue, ty *Type, errors ErrorTypes) any {
	switch ty.Kind {
	case kindBool:
		need(raw.Kind == "bool", "tfv-value", "expected a TFV boolean")
		return raw.Bool
	case kindBytes:
		need(raw.Kind == "bytes", "tfv-value", "expected TFV bytes")
		if ty.HasWidth && ty.WidthName == "" {
			need(len(raw.Bytes) == ty.Width, "tfv-value", "TFV Bytes width mismatch")
		}
		return raw.Bytes
	case kindResult:
		need(raw.Kind == "map" && len(raw.Pairs) == 1, "tfv-value", "expected a one-entry TFV map")
		key, body := raw.Pairs[0][0], raw.Pairs[0][1]
		need(key.Kind == "text", "tfv-value", "Result key must be text")
		if key.Text == "ok" {
			return Pair{"ok", decodeTFV(body, ty.Ok, errors)}
		}
		need(key.Text == "err" && body.Kind == "text" && containsString(errors[ty.Error], body.Text),
			"tfv-value", "Result error is not a declared variant")
		return Pair{"err", body.Text}
	}
	need(raw.Kind == "int", "tfv-value", "TFV integer out of range")
	n, err := strconv.ParseUint(raw.Text, 10, 64)
	need(err == nil && n <= upperBound(ty.Kind), "tfv-value", "TFV integer out of range")
	return n
}
