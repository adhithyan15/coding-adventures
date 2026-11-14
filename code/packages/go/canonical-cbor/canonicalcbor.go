// Package canonicalcbor provides an in-memory, zero-dependency, deterministic
// codec for Concise Binary Object Representation (CBOR, RFC 8949) using the
// deterministic profile defined in RFC 8949 §4.2.3, "Length-First Map Key Ordering".
//
// # 1. The Need for Deterministic CBOR
//
// In general-purpose binary object encodings like standard CBOR or JSON, one logical
// value can be represented by multiple distinct byte sequences. For example, in
// standard CBOR:
//
//   - An integer such as 0 can be encoded inline in 1 byte (0x00) or expanded into
//     2 bytes (0x18 0x00), 3 bytes (0x19 0x00 0x00), 5 bytes (0x1A 0x00 0x00 0x00 0x00),
//     or 9 bytes (0x1B 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00).
//   - Map keys may appear in arbitrary, non-deterministic order.
//   - Byte strings, text, arrays, and maps may use indefinite-length framing with
//     streaming chunks closed by a break marker (0xFF).
//
// While flexible for general serialization, non-deterministic encoding compromises
// security-critical systems such as cryptographic vaults, verifiable audit trails,
// and authentication protocols:
//
//   - AEAD Associated Data (AAD) and content integrity: Authenticated encryption
//     tags depend on exact byte equality. If re-encoding a record produces a
//     different byte sequence, existing cryptographic tags fail verification.
//   - Conflict-free replication and delta sync: Comparing record revisions
//     byte-for-byte requires that identical states have identical encodings.
//   - Hardware and authenticator interop: Protocols like FIDO2 / WebAuthn-PRF (CTAP2)
//     and COSE (RFC 9052) require length-first map key ordering for consistent
//     credential validation.
//
// This package guarantees a strict bijection between logical canonical values and
// their wire bytes:
//
//	EncodeChecked(v) produces deterministic bytes.
//	Decode(bytes) enforces that bytes adhere to the single legal canonical spelling.
//
// # 2. The CBR01 Canonical Profile
//
// The encoder and decoder jointly enforce the following rules:
//
//	Rule                    Encoder                         Decoder
//	----                    -------                         -------
//	Definite lengths only   Always definite                 Rejects indefinite info (31)
//	Smallest-form arguments Shortest 0/1/2/4/8 byte form    Rejects expanded integers/lengths
//	Length-first map keys   Key length asc, unsigned lex    Requires strictly increasing keys
//	Unique map keys         Rejects duplicate keys          Rejects duplicate keys
//	Reserved info codes     Never emits 28, 29, 30          Rejects info 28, 29, 30
//	UTF-8 text validation   Encodes valid Go strings        Validates UTF-8 scalar bytes
//	Closed simple values    Emits false, true, null         Rejects all other simple values
//	No floats               Never emits floats              Rejects half, single, double floats
//	Exactly one item        Emits exactly one item          Rejects trailing bytes
//	Payload-blind errors    Returns static identifiers      Never reflects payload in error
//
// # 3. Resource Bounds & Security Invariants
//
// To prevent denial-of-service (DoS) attacks via hostile inputs:
//
//   - MaxNestingDepth (128): Call chains of nested arrays, maps, and tags are capped
//     at 128 levels (root is depth 0). Inputs exceeding this depth fail with ErrTooDeep
//     (decode) or ErrEncodeTooDeep (encode).
//   - MaxEncodedBytes (1,048,576 / 1 MiB): The complete serialized CBOR item is bounded
//     to at most 1 MiB. Attempting to encode larger structures fails with ErrEncodeTooLarge.
//   - Pre-allocation Bounds Check: Decoders verify that declared collection lengths
//     do not exceed the remaining bytes in the input stream before allocating memory.
//     Hostile inputs claiming 2^64 elements in a small buffer fail with ErrLengthTooLarge
//     without allocating any slice.
//   - Payload-Blind Diagnostics: All error messages begin with "canonical-cbor: " and
//     contain only static literal descriptions, ensuring secret keys or user payloads
//     are never echoed into log files or telemetry.
//   - Defensive Slicing: Constructors and accessor methods return defensive copies of
//     internal byte and element slices to preserve caller-slice mutation isolation.
package canonicalcbor

import (
	"bytes"
	"encoding/binary"
	"math"
	"sort"
	"unicode/utf8"
)

// ============================================================================
// 1. Constants & Limits
// ============================================================================

const (
	// MaxNestingDepth defines the maximum recursion depth allowed during encoding
	// and decoding. The root item is at depth 0. Items requiring depth 129 are
	// rejected with ErrEncodeTooDeep or ErrTooDeep.
	MaxNestingDepth = 128

	// MaxEncodedBytes specifies the maximum size of a single canonical CBOR item
	// in bytes (1 MiB). Attempting to encode an item exceeding this size fails
	// with ErrEncodeTooLarge.
	MaxEncodedBytes = 1048576

	// MajorTypeShift is the bit shift applied to place the 3-bit major type into the high bits of a header byte.
	MajorTypeShift = 5

	// AdditionalInfoMask isolates the lower 5 bits of a header byte.
	AdditionalInfoMask = 0x1F

	// Major types defined by RFC 8949 Section 3.1:
	MajorUnsigned      byte = 0
	MajorNegative      byte = 1
	MajorBytes         byte = 2
	MajorText          byte = 3
	MajorArray         byte = 4
	MajorMap           byte = 5
	MajorTag           byte = 6
	MajorSimpleOrFloat byte = 7

	// Additional-info representation codes:
	InfoInlineMax  byte = 23
	Info1ByteArg   byte = 24
	Info2ByteArg   byte = 25
	Info4ByteArg   byte = 26
	Info8ByteArg   byte = 27
	InfoReserved28 byte = 28
	InfoReserved29 byte = 29
	InfoReserved30 byte = 30
	InfoIndefinite byte = 31

	// Integer boundary thresholds for minimal-width encoding (RFC 8949 §4.2.1):
	MaxInlineInteger uint64 = 23
	Max1ByteInteger  uint64 = 0xFF
	Max2ByteInteger  uint64 = 0xFFFF
	Max4ByteInteger  uint64 = 0xFFFFFFFF

	// Simple value codes for MajorSimpleOrFloat (RFC 8949 Section 3.3):
	SimpleFalse   byte = 20
	SimpleTrue    byte = 21
	SimpleNull    byte = 22
	SimpleFloat16 byte = 25
	SimpleFloat32 byte = 26
	SimpleFloat64 byte = 27

	// Exact wire bytes for canonical simple values:
	WireByteFalse byte = (MajorSimpleOrFloat << MajorTypeShift) | SimpleFalse // 0xF4
	WireByteTrue  byte = (MajorSimpleOrFloat << MajorTypeShift) | SimpleTrue  // 0xF5
	WireByteNull  byte = (MajorSimpleOrFloat << MajorTypeShift) | SimpleNull  // 0xF6

	// Minimum wire bytes required per unit for collection pre-allocation bounds checking:
	MinBytesPerElement  = 1
	MinBytesPerMapEntry = 2

	// Initial buffer capacities for efficient memory allocation:
	InitialOutputCapacity = 64
	InitialKeyCapacity    = 32
)

// ============================================================================
// 2. Closed Error Taxonomy
// ============================================================================

// ErrorCode is a closed set of 14 stable, payload-blind error identifiers defined
// in the CBR01 specification.
type ErrorCode string

const (
	// ErrUnexpectedEof indicates the input ended before completing a required item or argument.
	ErrUnexpectedEof ErrorCode = "unexpected-eof"
	// ErrTrailingBytes indicates bytes remained in the input after decoding one root item.
	ErrTrailingBytes ErrorCode = "trailing-bytes"
	// ErrReserved indicates an additional-info code in the reserved range (28, 29, 30).
	ErrReserved ErrorCode = "reserved"
	// ErrIndefinite indicates an indefinite-length item or 0xFF break marker.
	ErrIndefinite ErrorCode = "indefinite"
	// ErrNonMinimalInteger indicates an integer, length, or tag encoded in a non-minimal width.
	ErrNonMinimalInteger ErrorCode = "non-minimal-integer"
	// ErrInvalidUtf8 indicates a text string payload that is not valid UTF-8.
	ErrInvalidUtf8 ErrorCode = "invalid-utf8"
	// ErrNonCanonicalMapOrder indicates map keys that are not strictly increasing length-first.
	ErrNonCanonicalMapOrder ErrorCode = "non-canonical-map-order"
	// ErrUnsupportedSimple indicates a simple value other than false (20), true (21), or null (22).
	ErrUnsupportedSimple ErrorCode = "unsupported-simple"
	// ErrFloatNotSupported indicates an IEEE 754 half, single, or double float.
	ErrFloatNotSupported ErrorCode = "float-not-supported"
	// ErrTooDeep indicates nesting depth exceeding MaxNestingDepth during decoding.
	ErrTooDeep ErrorCode = "too-deep"
	// ErrLengthTooLarge indicates a declared length that exceeds remaining bytes or platform bounds.
	ErrLengthTooLarge ErrorCode = "length-too-large"
	// ErrDuplicateMapKey indicates two map keys that encode to identical canonical bytes.
	ErrDuplicateMapKey ErrorCode = "duplicate-map-key"
	// ErrEncodeTooDeep indicates nesting depth exceeding MaxNestingDepth during encoding.
	ErrEncodeTooDeep ErrorCode = "encode-too-deep"
	// ErrEncodeTooLarge indicates that an encoded item would exceed MaxEncodedBytes.
	ErrEncodeTooLarge ErrorCode = "encode-too-large"
)

// Error represents a deterministic CBOR codec error. It encapsulates a stable
// ErrorCode. In accordance with the CBR01 threat model, Error() produces a
// static string literal prefixed with "canonical-cbor: " with zero reflection of
// payload contents, byte values, lengths, or stream offsets.
type Error struct {
	Code ErrorCode
}

// Error returns the static human-readable description for this error.
func (e *Error) Error() string {
	switch e.Code {
	case ErrUnexpectedEof:
		return "canonical-cbor: unexpected end of input"
	case ErrTrailingBytes:
		return "canonical-cbor: trailing bytes after decoded item"
	case ErrReserved:
		return "canonical-cbor: reserved additional-info value (28/29/30)"
	case ErrIndefinite:
		return "canonical-cbor: indefinite-length items rejected by canonical profile"
	case ErrNonMinimalInteger:
		return "canonical-cbor: integer not in smallest-form encoding"
	case ErrInvalidUtf8:
		return "canonical-cbor: text string is not valid UTF-8"
	case ErrNonCanonicalMapOrder:
		return "canonical-cbor: map keys not in length-first canonical order or duplicated"
	case ErrUnsupportedSimple:
		return "canonical-cbor: unsupported simple value (only true / false / null permitted)"
	case ErrFloatNotSupported:
		return "canonical-cbor: floats are not supported in this version"
	case ErrTooDeep:
		return "canonical-cbor: nesting depth exceeded the configured maximum"
	case ErrLengthTooLarge:
		return "canonical-cbor: length / count exceeds the remaining bytes or platform usize"
	case ErrDuplicateMapKey:
		return "canonical-cbor: duplicate canonical map key"
	case ErrEncodeTooDeep:
		return "canonical-cbor: encode nesting depth exceeded the configured maximum"
	case ErrEncodeTooLarge:
		return "canonical-cbor: encoded item exceeds the configured maximum"
	default:
		return "canonical-cbor: unknown error"
	}
}

// ID returns the stable wire-level identifier of this error code.
func (e *Error) ID() string {
	return string(e.Code)
}

// Is reports whether this error matches target for errors.Is matching.
func (e *Error) Is(target error) bool {
	if t, ok := target.(*Error); ok {
		return e.Code == t.Code
	}
	return false
}

// ============================================================================
// 3. Value Model & Algebraic Types
// ============================================================================

// Kind represents the neutral value kind of a canonical CBOR item.
type Kind int

const (
	// KindUnsigned represents an unsigned 64-bit integer (CBOR major type 0).
	KindUnsigned Kind = iota
	// KindNegative represents a negative integer -1 - n (CBOR major type 1).
	KindNegative
	// KindBytes represents an arbitrary byte sequence (CBOR major type 2).
	KindBytes
	// KindText represents a valid UTF-8 scalar sequence (CBOR major type 3).
	KindText
	// KindArray represents an ordered sequence of CBOR values (CBOR major type 4).
	KindArray
	// KindMap represents a collection of key-value pairs (CBOR major type 5).
	KindMap
	// KindTag represents a tagged CBOR value with a 64-bit tag (CBOR major type 6).
	KindTag
	// KindBool represents a boolean true or false (CBOR major type 7, simple 20/21).
	KindBool
	// KindNull represents a null value (CBOR major type 7, simple 22).
	KindNull
)

// String returns the name of the kind.
func (k Kind) String() string {
	switch k {
	case KindUnsigned:
		return "unsigned"
	case KindNegative:
		return "negative"
	case KindBytes:
		return "bytes"
	case KindText:
		return "text"
	case KindArray:
		return "array"
	case KindMap:
		return "map"
	case KindTag:
		return "tag"
	case KindBool:
		return "bool"
	case KindNull:
		return "null"
	default:
		return "unknown"
	}
}

// MapEntry represents a key-value pair in a CBOR map.
type MapEntry struct {
	Key   Value
	Value Value
}

// Value represents an immutable CBOR value under the canonical profile.
// Internal slice fields are protected via defensive copying on constructors and accessors.
type Value struct {
	kind     Kind
	unsigned uint64
	negative uint64
	bytes    []byte
	text     string
	array    []Value
	entries  []MapEntry
	tag      uint64
	tagVal   *Value
	boolean  bool
}

// NewUnsigned constructs a major type 0 unsigned integer value.
func NewUnsigned(n uint64) Value {
	return Value{kind: KindUnsigned, unsigned: n}
}

// NewNegative constructs a major type 1 negative integer representing -1 - n.
// For example, NewNegative(0) is -1, NewNegative(41) is -42, and NewNegative(math.MaxUint64) is -2^64.
func NewNegative(n uint64) Value {
	return Value{kind: KindNegative, negative: n}
}

// NewBytes constructs a major type 2 byte string value.
// A defensive copy of the input slice is made to guarantee caller-slice isolation.
func NewBytes(b []byte) Value {
	var copied []byte
	if b != nil {
		copied = make([]byte, len(b))
		copy(copied, b)
	} else {
		copied = []byte{}
	}
	return Value{kind: KindBytes, bytes: copied}
}

// NewText constructs a major type 3 UTF-8 text string value.
func NewText(s string) Value {
	return Value{kind: KindText, text: s}
}

// NewArray constructs a major type 4 array value.
// A defensive copy of the items slice is made to preserve immutability.
func NewArray(items []Value) Value {
	var copied []Value
	if items != nil {
		copied = make([]Value, len(items))
		copy(copied, items)
	} else {
		copied = []Value{}
	}
	return Value{kind: KindArray, array: copied}
}

// NewMap constructs a major type 5 map value.
// A defensive copy of the entries slice is made. Map keys are sorted length-first
// during encoding; callers may pass entries in any initial order.
func NewMap(entries []MapEntry) Value {
	var copied []MapEntry
	if entries != nil {
		copied = make([]MapEntry, len(entries))
		copy(copied, entries)
	} else {
		copied = []MapEntry{}
	}
	return Value{kind: KindMap, entries: copied}
}

// NewTag constructs a major type 6 tagged value with the given 64-bit tag.
func NewTag(tag uint64, val Value) Value {
	valCopy := val
	return Value{kind: KindTag, tag: tag, tagVal: &valCopy}
}

// NewBool constructs a major type 7 boolean value (simple 20 for false, 21 for true).
func NewBool(b bool) Value {
	return Value{kind: KindBool, boolean: b}
}

// Null constructs a major type 7 null value (simple 22).
func Null() Value {
	return Value{kind: KindNull}
}

// Kind returns the neutral kind of this Value.
func (v Value) Kind() Kind {
	return v.kind
}

// Unsigned returns the 64-bit unsigned integer payload for KindUnsigned.
func (v Value) Unsigned() uint64 {
	return v.unsigned
}

// Negative returns the 64-bit negative offset n for KindNegative (representing -1 - n).
func (v Value) Negative() uint64 {
	return v.negative
}

// Bytes returns a defensive copy of the byte slice for KindBytes.
func (v Value) Bytes() []byte {
	if v.bytes == nil {
		return nil
	}
	out := make([]byte, len(v.bytes))
	copy(out, v.bytes)
	return out
}

// Text returns the string payload for KindText.
func (v Value) Text() string {
	return v.text
}

// Array returns a defensive copy of the elements slice for KindArray.
func (v Value) Array() []Value {
	if v.array == nil {
		return nil
	}
	out := make([]Value, len(v.array))
	copy(out, v.array)
	return out
}

// Map returns a defensive copy of the entries slice for KindMap.
func (v Value) Map() []MapEntry {
	if v.entries == nil {
		return nil
	}
	out := make([]MapEntry, len(v.entries))
	copy(out, v.entries)
	return out
}

// Tag returns the tag number and inner value for KindTag.
func (v Value) Tag() (uint64, Value) {
	if v.tagVal == nil {
		return v.tag, Null()
	}
	return v.tag, *v.tagVal
}

// Bool returns the boolean payload for KindBool.
func (v Value) Bool() bool {
	return v.boolean
}

// IsNull reports whether this value is KindNull.
func (v Value) IsNull() bool {
	return v.kind == KindNull
}

// Equal returns true if v and other are structurally equal in kind and value.
func (v Value) Equal(other Value) bool {
	if v.kind != other.kind {
		return false
	}
	switch v.kind {
	case KindUnsigned:
		return v.unsigned == other.unsigned
	case KindNegative:
		return v.negative == other.negative
	case KindBytes:
		return bytes.Equal(v.bytes, other.bytes)
	case KindText:
		return v.text == other.text
	case KindArray:
		if len(v.array) != len(other.array) {
			return false
		}
		for i := range v.array {
			if !v.array[i].Equal(other.array[i]) {
				return false
			}
		}
		return true
	case KindMap:
		if len(v.entries) != len(other.entries) {
			return false
		}
		for i := range v.entries {
			if !v.entries[i].Key.Equal(other.entries[i].Key) ||
				!v.entries[i].Value.Equal(other.entries[i].Value) {
				return false
			}
		}
		return true
	case KindTag:
		if v.tag != other.tag {
			return false
		}
		if (v.tagVal == nil) != (other.tagVal == nil) {
			return false
		}
		if v.tagVal != nil {
			return v.tagVal.Equal(*other.tagVal)
		}
		return true
	case KindBool:
		return v.boolean == other.boolean
	case KindNull:
		return true
	default:
		return false
	}
}

// ============================================================================
// 4. Checked Encoder
// ============================================================================

// EncodeChecked encodes a Value into deterministic, canonical CBOR bytes.
//
// Invariants enforced:
//   - Smallest-form integer and length headers.
//   - Length-first then unsigned bytewise map key sorting (RFC 8949 §4.2.3).
//   - Duplicate map key rejection (ErrDuplicateMapKey).
//   - Maximum recursion depth capped at MaxNestingDepth (ErrEncodeTooDeep).
//   - Maximum total output size capped at MaxEncodedBytes (ErrEncodeTooLarge).
//
// If validation fails at any point, no bytes are returned.
func EncodeChecked(v Value) ([]byte, error) {
	out := make([]byte, 0, InitialOutputCapacity)
	if err := encodeIntoBuffer(v, &out, 0); err != nil {
		return nil, err
	}
	return out, nil
}

// EncodeIntoChecked appends the canonical CBOR encoding of v to dst.
//
// Atomic rollback guarantee: if encoding fails, *dst is guaranteed to remain
// unchanged.
func EncodeIntoChecked(v Value, dst *[]byte) error {
	if dst == nil {
		return &Error{Code: ErrLengthTooLarge}
	}
	encoded, err := EncodeChecked(v)
	if err != nil {
		return err
	}
	*dst = append(*dst, encoded...)
	return nil
}

// Encode is a convenience wrapper around EncodeChecked for callers with
// trusted values. It panics with a static error string if encoding fails.
func Encode(v Value) []byte {
	bytes, err := EncodeChecked(v)
	if err != nil {
		panic("canonical-cbor: checked encoding failed")
	}
	return bytes
}

// EncodeInto is a convenience wrapper around EncodeIntoChecked that panics
// on error.
func EncodeInto(v Value, dst *[]byte) {
	if err := EncodeIntoChecked(v, dst); err != nil {
		panic("canonical-cbor: checked encoding failed")
	}
}

func encodeIntoBuffer(v Value, out *[]byte, depth int) error {
	if depth > MaxNestingDepth {
		return &Error{Code: ErrEncodeTooDeep}
	}

	switch v.kind {
	case KindUnsigned:
		return writeTypeAndArgument(out, MajorUnsigned, v.unsigned)
	case KindNegative:
		return writeTypeAndArgument(out, MajorNegative, v.negative)
	case KindBytes:
		if err := writeTypeAndArgument(out, MajorBytes, uint64(len(v.bytes))); err != nil {
			return err
		}
		return appendChecked(out, v.bytes)
	case KindText:
		textBytes := []byte(v.text)
		if err := writeTypeAndArgument(out, MajorText, uint64(len(textBytes))); err != nil {
			return err
		}
		return appendChecked(out, textBytes)
	case KindArray:
		if err := writeTypeAndArgument(out, MajorArray, uint64(len(v.array))); err != nil {
			return err
		}
		for _, item := range v.array {
			if err := encodeIntoBuffer(item, out, depth+1); err != nil {
				return err
			}
		}
		return nil
	case KindMap:
		// Map canonicalization (RFC 8949 §4.2.3):
		// 1. Encode every key independently at depth + 1.
		// 2. Sort key-value pairs by key encoding: length ascending, then bytewise lex.
		// 3. Verify no adjacent keys have identical encodings (reject duplicate keys).
		// 4. Emit map header, followed by key bytes and encoded values.
		type encodedEntry struct {
			encodedKey []byte
			val        Value
		}
		entries := make([]encodedEntry, len(v.entries))
		for i, entry := range v.entries {
			keyBuf := make([]byte, 0, InitialKeyCapacity)
			if err := encodeIntoBuffer(entry.Key, &keyBuf, depth+1); err != nil {
				return err
			}
			entries[i] = encodedEntry{encodedKey: keyBuf, val: entry.Value}
		}

		sort.Slice(entries, func(i, j int) bool {
			li, lj := len(entries[i].encodedKey), len(entries[j].encodedKey)
			if li != lj {
				return li < lj
			}
			return bytes.Compare(entries[i].encodedKey, entries[j].encodedKey) < 0
		})

		for i := 1; i < len(entries); i++ {
			if bytes.Equal(entries[i-1].encodedKey, entries[i].encodedKey) {
				return &Error{Code: ErrDuplicateMapKey}
			}
		}

		if err := writeTypeAndArgument(out, MajorMap, uint64(len(entries))); err != nil {
			return err
		}
		for _, e := range entries {
			if err := appendChecked(out, e.encodedKey); err != nil {
				return err
			}
			if err := encodeIntoBuffer(e.val, out, depth+1); err != nil {
				return err
			}
		}
		return nil
	case KindTag:
		if err := writeTypeAndArgument(out, MajorTag, v.tag); err != nil {
			return err
		}
		inner := Null()
		if v.tagVal != nil {
			inner = *v.tagVal
		}
		return encodeIntoBuffer(inner, out, depth+1)
	case KindBool:
		if !v.boolean {
			return pushByteChecked(out, WireByteFalse)
		}
		return pushByteChecked(out, WireByteTrue)
	case KindNull:
		return pushByteChecked(out, WireByteNull)
	default:
		return &Error{Code: ErrUnsupportedSimple}
	}
}

// writeTypeAndArgument emits a CBOR header byte and argument using the minimal-width
// representation required by canonical CBOR.
func writeTypeAndArgument(out *[]byte, major byte, arg uint64) error {
	mt := major << MajorTypeShift
	if arg <= MaxInlineInteger {
		return pushByteChecked(out, mt|byte(arg))
	} else if arg <= Max1ByteInteger {
		if err := pushByteChecked(out, mt|Info1ByteArg); err != nil {
			return err
		}
		return pushByteChecked(out, byte(arg))
	} else if arg <= Max2ByteInteger {
		if err := pushByteChecked(out, mt|Info2ByteArg); err != nil {
			return err
		}
		var b [2]byte
		binary.BigEndian.PutUint16(b[:], uint16(arg))
		return appendChecked(out, b[:])
	} else if arg <= Max4ByteInteger {
		if err := pushByteChecked(out, mt|Info4ByteArg); err != nil {
			return err
		}
		var b [4]byte
		binary.BigEndian.PutUint32(b[:], uint32(arg))
		return appendChecked(out, b[:])
	} else {
		if err := pushByteChecked(out, mt|Info8ByteArg); err != nil {
			return err
		}
		var b [8]byte
		binary.BigEndian.PutUint64(b[:], arg)
		return appendChecked(out, b[:])
	}
}

func pushByteChecked(out *[]byte, b byte) error {
	if len(*out) >= MaxEncodedBytes {
		return &Error{Code: ErrEncodeTooLarge}
	}
	*out = append(*out, b)
	return nil
}

func appendChecked(out *[]byte, data []byte) error {
	if len(*out)+len(data) > MaxEncodedBytes {
		return &Error{Code: ErrEncodeTooLarge}
	}
	*out = append(*out, data...)
	return nil
}

// ============================================================================
// 5. Strict Decoder
// ============================================================================

type cursor struct {
	data []byte
	pos  int
}

func (c *cursor) remaining() int {
	return len(c.data) - c.pos
}

func (c *cursor) readU8() (byte, error) {
	if c.pos >= len(c.data) {
		return 0, &Error{Code: ErrUnexpectedEof}
	}
	b := c.data[c.pos]
	c.pos++
	return b, nil
}

func (c *cursor) readN(n int) ([]byte, error) {
	if n < 0 {
		return nil, &Error{Code: ErrLengthTooLarge}
	}
	end := c.pos + n
	if end < c.pos {
		return nil, &Error{Code: ErrLengthTooLarge}
	}
	if end > len(c.data) {
		return nil, &Error{Code: ErrUnexpectedEof}
	}
	s := c.data[c.pos:end]
	c.pos = end
	return s, nil
}

func checkLength(declared uint64, rem int, minPerUnit int) (int, error) {
	if declared > uint64(math.MaxInt) {
		return 0, &Error{Code: ErrLengthTooLarge}
	}
	count := int(declared)
	if minPerUnit > 0 {
		if count > rem/minPerUnit {
			return 0, &Error{Code: ErrLengthTooLarge}
		}
	}
	return count, nil
}

func readHeader(c *cursor) (major byte, info byte, arg uint64, err error) {
	b, err := c.readU8()
	if err != nil {
		return 0, 0, 0, err
	}
	major = b >> MajorTypeShift
	info = b & AdditionalInfoMask
	enforceMinimal := (major != MajorSimpleOrFloat)

	switch {
	case info <= InfoInlineMax:
		arg = uint64(info)
	case info == Info1ByteArg:
		v, err := c.readU8()
		if err != nil {
			return 0, 0, 0, err
		}
		arg = uint64(v)
		if enforceMinimal && arg <= MaxInlineInteger {
			return 0, 0, 0, &Error{Code: ErrNonMinimalInteger}
		}
	case info == Info2ByteArg:
		bs, err := c.readN(2)
		if err != nil {
			return 0, 0, 0, err
		}
		arg = uint64(binary.BigEndian.Uint16(bs))
		if enforceMinimal && arg <= Max1ByteInteger {
			return 0, 0, 0, &Error{Code: ErrNonMinimalInteger}
		}
	case info == Info4ByteArg:
		bs, err := c.readN(4)
		if err != nil {
			return 0, 0, 0, err
		}
		arg = uint64(binary.BigEndian.Uint32(bs))
		if enforceMinimal && arg <= Max2ByteInteger {
			return 0, 0, 0, &Error{Code: ErrNonMinimalInteger}
		}
	case info == Info8ByteArg:
		bs, err := c.readN(8)
		if err != nil {
			return 0, 0, 0, err
		}
		arg = binary.BigEndian.Uint64(bs)
		if enforceMinimal && arg <= Max4ByteInteger {
			return 0, 0, 0, &Error{Code: ErrNonMinimalInteger}
		}
	case info >= InfoReserved28 && info <= InfoReserved30:
		return 0, 0, 0, &Error{Code: ErrReserved}
	case info == InfoIndefinite:
		return 0, 0, 0, &Error{Code: ErrIndefinite}
	}
	return major, info, arg, nil
}

// Decode parses a single canonical CBOR item from data.
//
// Invariants enforced:
//   - The slice must contain exactly one valid item; trailing bytes return ErrTrailingBytes.
//   - Indefinite-length collections and 0xFF break markers return ErrIndefinite.
//   - Non-minimal integer/length representations return ErrNonMinimalInteger.
//   - Map keys must be strictly increasing in length-first order; out-of-order or duplicate keys return ErrNonCanonicalMapOrder.
//   - Declared lengths exceeding remaining input or platform width return ErrLengthTooLarge before allocation.
//   - Text strings must contain valid UTF-8 (RFC 3629); invalid UTF-8 returns ErrInvalidUtf8.
//   - Recursion depth beyond MaxNestingDepth returns ErrTooDeep.
//   - Floating-point encodings return ErrFloatNotSupported.
//   - Unassigned simple values return ErrUnsupportedSimple.
func Decode(data []byte) (Value, error) {
	c := &cursor{data: data, pos: 0}
	v, err := readValue(c, 0)
	if err != nil {
		return Value{}, err
	}
	if c.pos != len(data) {
		return Value{}, &Error{Code: ErrTrailingBytes}
	}
	return v, nil
}

func readValue(c *cursor, depth int) (Value, error) {
	if depth > MaxNestingDepth {
		return Value{}, &Error{Code: ErrTooDeep}
	}

	major, info, arg, err := readHeader(c)
	if err != nil {
		return Value{}, err
	}

	switch major {
	case MajorUnsigned:
		return NewUnsigned(arg), nil
	case MajorNegative:
		return NewNegative(arg), nil
	case MajorBytes:
		length, err := checkLength(arg, c.remaining(), MinBytesPerElement)
		if err != nil {
			return Value{}, err
		}
		bs, err := c.readN(length)
		if err != nil {
			return Value{}, err
		}
		return NewBytes(bs), nil
	case MajorText:
		length, err := checkLength(arg, c.remaining(), MinBytesPerElement)
		if err != nil {
			return Value{}, err
		}
		bs, err := c.readN(length)
		if err != nil {
			return Value{}, err
		}
		if !utf8.Valid(bs) {
			return Value{}, &Error{Code: ErrInvalidUtf8}
		}
		return NewText(string(bs)), nil
	case MajorArray:
		count, err := checkLength(arg, c.remaining(), MinBytesPerElement)
		if err != nil {
			return Value{}, err
		}
		items := make([]Value, count)
		for i := 0; i < count; i++ {
			item, err := readValue(c, depth+1)
			if err != nil {
				return Value{}, err
			}
			items[i] = item
		}
		return Value{kind: KindArray, array: items}, nil
	case MajorMap:
		count, err := checkLength(arg, c.remaining(), MinBytesPerMapEntry)
		if err != nil {
			return Value{}, err
		}
		entries := make([]MapEntry, count)
		var prevKeyBytes []byte
		for i := 0; i < count; i++ {
			keyStart := c.pos
			key, err := readValue(c, depth+1)
			if err != nil {
				return Value{}, err
			}
			curKeyBytes := c.data[keyStart:c.pos]
			if prevKeyBytes != nil {
				if !keyStrictlyLess(prevKeyBytes, curKeyBytes) {
					return Value{}, &Error{Code: ErrNonCanonicalMapOrder}
				}
			}
			prevKeyBytes = curKeyBytes

			val, err := readValue(c, depth+1)
			if err != nil {
				return Value{}, err
			}
			entries[i] = MapEntry{Key: key, Value: val}
		}
		return Value{kind: KindMap, entries: entries}, nil
	case MajorTag:
		inner, err := readValue(c, depth+1)
		if err != nil {
			return Value{}, err
		}
		return NewTag(arg, inner), nil
	case MajorSimpleOrFloat:
		switch info {
		case SimpleFalse:
			return NewBool(false), nil
		case SimpleTrue:
			return NewBool(true), nil
		case SimpleNull:
			return Null(), nil
		case SimpleFloat16, SimpleFloat32, SimpleFloat64:
			return Value{}, &Error{Code: ErrFloatNotSupported}
		default:
			return Value{}, &Error{Code: ErrUnsupportedSimple}
		}
	default:
		return Value{}, &Error{Code: ErrUnsupportedSimple}
	}
}

// keyStrictlyLess evaluates strict length-first, then unsigned bytewise lexicographical
// order between two canonical CBOR key encodings. Duplicate keys return false.
func keyStrictlyLess(a, b []byte) bool {
	if len(a) != len(b) {
		return len(a) < len(b)
	}
	return bytes.Compare(a, b) < 0
}

// ============================================================================
// 6. Span-Reporting Map Decoder
// ============================================================================

// Span represents a half-open byte index range [Start, End) within an input slice.
type Span struct {
	Start int
	End   int
}

// SpannedMapEntry represents one key-value pair of a decoded map alongside the
// byte range occupied by its value in the input slice.
type SpannedMapEntry struct {
	Key       Value
	Value     Value
	ValueSpan Span
}

// DecodeMapSpans decodes a single canonical CBOR map from data and reports the
// exact byte span each entry's value occupied in data.
//
// Shape Mismatch vs Profile Violation:
//   - If data contains a valid canonical CBOR item that is not a map, DecodeMapSpans
//     returns (nil, false, nil). This reflects a document shape condition, not an error.
//   - If data violates the canonical CBOR profile or contains trailing bytes, it
//     returns (nil, false, err).
//   - If data is a valid canonical map, it returns (entries, true, nil).
//
// Invariant:
//
//	For every entry e in entries, data[e.ValueSpan.Start:e.ValueSpan.End] == EncodeChecked(e.Value)
//	whenever EncodeChecked(e.Value) succeeds.
func DecodeMapSpans(data []byte) ([]SpannedMapEntry, bool, error) {
	if len(data) == 0 {
		return nil, false, &Error{Code: ErrUnexpectedEof}
	}
	// Check the major type of the first byte:
	if data[0]>>MajorTypeShift != MajorMap {
		// Verify full item validity via standard Decode before declaring shape mismatch:
		_, err := Decode(data)
		if err != nil {
			return nil, false, err
		}
		return nil, false, nil
	}

	c := &cursor{data: data, pos: 0}
	_, _, arg, err := readHeader(c)
	if err != nil {
		return nil, false, err
	}
	count, err := checkLength(arg, c.remaining(), MinBytesPerMapEntry)
	if err != nil {
		return nil, false, err
	}

	entries := make([]SpannedMapEntry, count)
	var prevKeyBytes []byte
	for i := 0; i < count; i++ {
		keyStart := c.pos
		key, err := readValue(c, 1)
		if err != nil {
			return nil, false, err
		}
		curKeyBytes := c.data[keyStart:c.pos]
		if prevKeyBytes != nil {
			if !keyStrictlyLess(prevKeyBytes, curKeyBytes) {
				return nil, false, &Error{Code: ErrNonCanonicalMapOrder}
			}
		}
		prevKeyBytes = curKeyBytes

		valStart := c.pos
		val, err := readValue(c, 1)
		if err != nil {
			return nil, false, err
		}
		valEnd := c.pos

		entries[i] = SpannedMapEntry{
			Key:       key,
			Value:     val,
			ValueSpan: Span{Start: valStart, End: valEnd},
		}
	}

	if c.pos != len(data) {
		return nil, false, &Error{Code: ErrTrailingBytes}
	}

	return entries, true, nil
}
