package canonicalcbor

import (
	"encoding/hex"
	"encoding/json"
	"strconv"
	"strings"
	"testing"
)

// CanonicalCborCasesJSON embeds the canonical fixture from
// code/specs/fixtures/canonical-cbor-v1/cases.json.
//
// BUILD verifies byte-for-byte zero drift between this embedded constant and
// the source fixture.
const CanonicalCborCasesJSON = `{
  "schema_version": 1,
  "profile": "rfc8949-section-4.2.3-length-first",
  "limits": {
    "max_nesting_depth": 128,
    "max_encoded_bytes": 1048576
  },
  "error_ids": [
    "unexpected-eof",
    "trailing-bytes",
    "reserved",
    "indefinite",
    "non-minimal-integer",
    "invalid-utf8",
    "non-canonical-map-order",
    "unsupported-simple",
    "float-not-supported",
    "too-deep",
    "length-too-large",
    "duplicate-map-key",
    "encode-too-deep",
    "encode-too-large"
  ],
  "cases": [
    { "id": "cbor-v1-unsigned-zero", "operation": "round-trip", "input": "00", "expected": "00" },
    { "id": "cbor-v1-unsigned-23", "operation": "round-trip", "input": "17", "expected": "17" },
    { "id": "cbor-v1-unsigned-24", "operation": "round-trip", "input": "1818", "expected": "1818" },
    { "id": "cbor-v1-unsigned-255", "operation": "round-trip", "input": "18ff", "expected": "18ff" },
    { "id": "cbor-v1-unsigned-256", "operation": "round-trip", "input": "190100", "expected": "190100" },
    { "id": "cbor-v1-unsigned-65535", "operation": "round-trip", "input": "19ffff", "expected": "19ffff" },
    { "id": "cbor-v1-unsigned-65536", "operation": "round-trip", "input": "1a00010000", "expected": "1a00010000" },
    { "id": "cbor-v1-unsigned-u32max", "operation": "round-trip", "input": "1affffffff", "expected": "1affffffff" },
    { "id": "cbor-v1-unsigned-u32plus1", "operation": "round-trip", "input": "1b0000000100000000", "expected": "1b0000000100000000" },
    { "id": "cbor-v1-unsigned-u64max", "operation": "round-trip", "input": "1bffffffffffffffff", "expected": "1bffffffffffffffff" },
    { "id": "cbor-v1-negative-one", "operation": "round-trip", "input": "20", "expected": "20" },
    { "id": "cbor-v1-negative-24", "operation": "round-trip", "input": "37", "expected": "37" },
    { "id": "cbor-v1-negative-25", "operation": "round-trip", "input": "3818", "expected": "3818" },
    { "id": "cbor-v1-bytes-empty", "operation": "round-trip", "input": "40", "expected": "40" },
    { "id": "cbor-v1-bytes-three", "operation": "round-trip", "input": "430001ff", "expected": "430001ff" },
    { "id": "cbor-v1-text-empty", "operation": "round-trip", "input": "60", "expected": "60" },
    { "id": "cbor-v1-text-utf8", "operation": "round-trip", "input": "63e29883", "expected": "63e29883" },
    { "id": "cbor-v1-array", "operation": "round-trip", "input": "8300f5f6", "expected": "8300f5f6" },
    { "id": "cbor-v1-map-wire", "operation": "round-trip", "input": "a300f460f56161f6", "expected": "a300f460f56161f6" },
    { "id": "cbor-v1-tag-zero", "operation": "round-trip", "input": "c0f6", "expected": "c0f6" },
    { "id": "cbor-v1-tag-24", "operation": "round-trip", "input": "d81800", "expected": "d81800" },
    { "id": "cbor-v1-simple-values", "operation": "round-trip", "input": "83f4f5f6", "expected": "83f4f5f6" },
    { "id": "cbor-v1-map-input-order", "operation": "encode-map", "input": "6162=f6;1818=f5;60=f4;00=01;6161=02", "expected": "a5000160f41818f56161026162f6" },
    { "id": "cbor-v1-depth-128", "operation": "generated-round-trip", "input": "nested-array:128", "expected": "wire:nested-array:128" },
    { "id": "cbor-v1-size-limit", "operation": "generated-round-trip", "input": "bytes-repeat:1048571:00", "expected": "wire:bytes-repeat:1048571:00" },
    { "id": "cbor-v1-empty-input", "operation": "decode-error", "input": "", "expected": "unexpected-eof" },
    { "id": "cbor-v1-truncated-argument", "operation": "decode-error", "input": "1a0001", "expected": "unexpected-eof" },
    { "id": "cbor-v1-trailing", "operation": "decode-error", "input": "0000", "expected": "trailing-bytes" },
    { "id": "cbor-v1-reserved-28", "operation": "decode-error", "input": "1c", "expected": "reserved" },
    { "id": "cbor-v1-reserved-30", "operation": "decode-error", "input": "1e", "expected": "reserved" },
    { "id": "cbor-v1-indefinite-bytes", "operation": "decode-error", "input": "5f", "expected": "indefinite" },
    { "id": "cbor-v1-indefinite-text", "operation": "decode-error", "input": "7f", "expected": "indefinite" },
    { "id": "cbor-v1-indefinite-array", "operation": "decode-error", "input": "9f", "expected": "indefinite" },
    { "id": "cbor-v1-indefinite-map", "operation": "decode-error", "input": "bf", "expected": "indefinite" },
    { "id": "cbor-v1-break", "operation": "decode-error", "input": "ff", "expected": "indefinite" },
    { "id": "cbor-v1-nonminimal-inline", "operation": "decode-error", "input": "1800", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-u8", "operation": "decode-error", "input": "1900ff", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-u16", "operation": "decode-error", "input": "1a0000ffff", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-u32", "operation": "decode-error", "input": "1b00000000ffffffff", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-length", "operation": "decode-error", "input": "5800", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-nonminimal-tag", "operation": "decode-error", "input": "d800f6", "expected": "non-minimal-integer" },
    { "id": "cbor-v1-invalid-utf8", "operation": "decode-error", "input": "61ff", "expected": "invalid-utf8" },
    { "id": "cbor-v1-invalid-utf8-overlong", "operation": "decode-error", "input": "62c080", "expected": "invalid-utf8" },
    { "id": "cbor-v1-map-out-of-order", "operation": "decode-error", "input": "a2616200616101", "expected": "non-canonical-map-order" },
    { "id": "cbor-v1-map-duplicate-wire", "operation": "decode-error", "input": "a2616100616101", "expected": "non-canonical-map-order" },
    { "id": "cbor-v1-unsupported-simple", "operation": "decode-error", "input": "f0", "expected": "unsupported-simple" },
    { "id": "cbor-v1-undefined", "operation": "decode-error", "input": "f7", "expected": "unsupported-simple" },
    { "id": "cbor-v1-float16", "operation": "decode-error", "input": "f90000", "expected": "float-not-supported" },
    { "id": "cbor-v1-float32", "operation": "decode-error", "input": "fa00000000", "expected": "float-not-supported" },
    { "id": "cbor-v1-float64", "operation": "decode-error", "input": "fb0000000000000000", "expected": "float-not-supported" },
    { "id": "cbor-v1-hostile-bytes-length", "operation": "decode-error", "input": "5bffffffffffffffff", "expected": "length-too-large" },
    { "id": "cbor-v1-depth-129-decode", "operation": "decode-error", "input": "nested-array-wire:129", "expected": "too-deep" },
    { "id": "cbor-v1-duplicate-encode", "operation": "encode-error", "input": "duplicate-map-key", "expected": "duplicate-map-key" },
    { "id": "cbor-v1-depth-129-encode", "operation": "encode-error", "input": "nested-array:129", "expected": "encode-too-deep" },
    { "id": "cbor-v1-size-limit-plus-one", "operation": "encode-error", "input": "bytes-repeat:1048572:00", "expected": "encode-too-large" }
  ]
}`

type fixtureDocument struct {
	SchemaVersion int           `json:"schema_version"`
	Profile       string        `json:"profile"`
	Limits        fixtureLimits `json:"limits"`
	ErrorIDs      []string      `json:"error_ids"`
	Cases         []fixtureCase `json:"cases"`
}

type fixtureLimits struct {
	MaxNestingDepth int `json:"max_nesting_depth"`
	MaxEncodedBytes int `json:"max_encoded_bytes"`
}

type fixtureCase struct {
	ID        string `json:"id"`
	Operation string `json:"operation"`
	Input     string `json:"input"`
	Expected  string `json:"expected"`
}

func parseHex(t *testing.T, s string) []byte {
	t.Helper()
	if len(s) == 0 {
		return []byte{}
	}
	b, err := hex.DecodeString(s)
	if err != nil {
		t.Fatalf("failed to decode hex %q: %v", s, err)
	}
	return b
}

func buildNestedArray(depth int) Value {
	v := Null()
	for i := 0; i < depth; i++ {
		v = NewArray([]Value{v})
	}
	return v
}

func buildNestedArrayWire(depth int) []byte {
	out := make([]byte, depth+1)
	for i := 0; i < depth; i++ {
		out[i] = 0x81
	}
	out[depth] = 0xF6
	return out
}

func buildBytesRepeat(t *testing.T, spec string) Value {
	t.Helper()
	parts := strings.Split(spec, ":")
	if len(parts) != 2 {
		t.Fatalf("invalid bytes-repeat spec: %q", spec)
	}
	length, err := strconv.Atoi(parts[0])
	if err != nil {
		t.Fatalf("invalid length in %q: %v", spec, err)
	}
	b := parseHex(t, parts[1])
	payload := make([]byte, length)
	for i := range payload {
		payload[i] = b[0]
	}
	return NewBytes(payload)
}

func buildWireRepeat(t *testing.T, spec string) []byte {
	t.Helper()
	parts := strings.Split(spec, ":")
	if len(parts) != 2 {
		t.Fatalf("invalid wire bytes-repeat spec: %q", spec)
	}
	length, err := strconv.Atoi(parts[0])
	if err != nil {
		t.Fatalf("invalid length in %q: %v", spec, err)
	}
	b := parseHex(t, parts[1])

	var wire []byte
	switch {
	case length <= 23:
		wire = []byte{0x40 | byte(length)}
	case length <= 0xFF:
		wire = []byte{0x58, byte(length)}
	case length <= 0xFFFF:
		wire = []byte{0x59, byte(length >> 8), byte(length)}
	default:
		wire = []byte{0x5A, byte(length >> 24), byte(length >> 16), byte(length >> 8), byte(length)}
	}
	payload := make([]byte, length)
	for i := range payload {
		payload[i] = b[0]
	}
	return append(wire, payload...)
}

func buildMapFromSpec(t *testing.T, spec string) Value {
	t.Helper()
	fragments := strings.Split(spec, ";")
	entries := make([]MapEntry, len(fragments))
	for i, frag := range fragments {
		parts := strings.Split(frag, "=")
		if len(parts) != 2 {
			t.Fatalf("invalid map entry fragment: %q", frag)
		}
		kBytes := parseHex(t, parts[0])
		vBytes := parseHex(t, parts[1])

		kVal, err := Decode(kBytes)
		if err != nil {
			t.Fatalf("failed to decode map key %q: %v", parts[0], err)
		}
		vVal, err := Decode(vBytes)
		if err != nil {
			t.Fatalf("failed to decode map val %q: %v", parts[1], err)
		}
		entries[i] = MapEntry{Key: kVal, Value: vVal}
	}
	return NewMap(entries)
}

func TestConformanceVectors(t *testing.T) {
	var doc fixtureDocument
	if err := json.Unmarshal([]byte(CanonicalCborCasesJSON), &doc); err != nil {
		t.Fatalf("failed to parse fixture json: %v", err)
	}

	if doc.Limits.MaxNestingDepth != MaxNestingDepth {
		t.Fatalf("max nesting depth mismatch: got %d, want %d", MaxNestingDepth, doc.Limits.MaxNestingDepth)
	}
	if doc.Limits.MaxEncodedBytes != MaxEncodedBytes {
		t.Fatalf("max encoded bytes mismatch: got %d, want %d", MaxEncodedBytes, doc.Limits.MaxEncodedBytes)
	}

	for _, c := range doc.Cases {
		c := c
		t.Run(c.ID, func(t *testing.T) {
			switch c.Operation {
			case "round-trip":
				wire := parseHex(t, c.Input)
				val, err := Decode(wire)
				if err != nil {
					t.Fatalf("Decode failed: %v", err)
				}
				encoded, err := EncodeChecked(val)
				if err != nil {
					t.Fatalf("EncodeChecked failed: %v", err)
				}
				encodedHex := hex.EncodeToString(encoded)
				if encodedHex != c.Expected {
					t.Fatalf("round-trip mismatch: got %s, want %s", encodedHex, c.Expected)
				}

			case "decode-error":
				var wire []byte
				if strings.HasPrefix(c.Input, "nested-array-wire:") {
					depthStr := strings.TrimPrefix(c.Input, "nested-array-wire:")
					depth, err := strconv.Atoi(depthStr)
					if err != nil {
						t.Fatalf("invalid depth in %q: %v", c.Input, err)
					}
					wire = buildNestedArrayWire(depth)
				} else {
					wire = parseHex(t, c.Input)
				}

				val, err := Decode(wire)
				if err == nil {
					t.Fatalf("expected decode error %q, got success: %+v", c.Expected, val)
				}
				cborErr, ok := err.(*Error)
				if !ok {
					t.Fatalf("expected *Error type, got %T: %v", err, err)
				}
				if cborErr.ID() != c.Expected {
					t.Fatalf("error id mismatch: got %s, want %s", cborErr.ID(), c.Expected)
				}
				if !strings.HasPrefix(cborErr.Error(), "canonical-cbor: ") {
					t.Fatalf("error message does not have canonical-cbor prefix: %q", cborErr.Error())
				}

			case "encode-map":
				val := buildMapFromSpec(t, c.Input)
				encoded, err := EncodeChecked(val)
				if err != nil {
					t.Fatalf("EncodeChecked failed: %v", err)
				}
				encodedHex := hex.EncodeToString(encoded)
				if encodedHex != c.Expected {
					t.Fatalf("encode-map mismatch: got %s, want %s", encodedHex, c.Expected)
				}

			case "generated-round-trip":
				var val Value
				var expectedWire []byte
				if strings.HasPrefix(c.Input, "nested-array:") {
					depth, _ := strconv.Atoi(strings.TrimPrefix(c.Input, "nested-array:"))
					val = buildNestedArray(depth)
				} else if strings.HasPrefix(c.Input, "bytes-repeat:") {
					val = buildBytesRepeat(t, strings.TrimPrefix(c.Input, "bytes-repeat:"))
				}

				if strings.HasPrefix(c.Expected, "wire:nested-array:") {
					depth, _ := strconv.Atoi(strings.TrimPrefix(c.Expected, "wire:nested-array:"))
					expectedWire = buildNestedArrayWire(depth)
				} else if strings.HasPrefix(c.Expected, "wire:bytes-repeat:") {
					expectedWire = buildWireRepeat(t, strings.TrimPrefix(c.Expected, "wire:bytes-repeat:"))
				}

				encoded, err := EncodeChecked(val)
				if err != nil {
					t.Fatalf("EncodeChecked failed: %v", err)
				}
				if hex.EncodeToString(encoded) != hex.EncodeToString(expectedWire) {
					t.Fatalf("generated encode mismatch: got len %d, want len %d", len(encoded), len(expectedWire))
				}

				decoded, err := Decode(expectedWire)
				if err != nil {
					t.Fatalf("Decode failed: %v", err)
				}
				if !decoded.Equal(val) {
					t.Fatalf("decoded value not equal to generated value")
				}

			case "encode-error":
				var val Value
				if c.Input == "duplicate-map-key" {
					val = buildMapFromSpec(t, "6161=00;6161=01")
				} else if strings.HasPrefix(c.Input, "nested-array:") {
					depth, _ := strconv.Atoi(strings.TrimPrefix(c.Input, "nested-array:"))
					val = buildNestedArray(depth)
				} else if strings.HasPrefix(c.Input, "bytes-repeat:") {
					val = buildBytesRepeat(t, strings.TrimPrefix(c.Input, "bytes-repeat:"))
				} else {
					t.Fatalf("unhandled encode-error input: %q", c.Input)
				}

				encoded, err := EncodeChecked(val)
				if err == nil {
					t.Fatalf("expected encode error %q, got success: len %d", c.Expected, len(encoded))
				}
				cborErr, ok := err.(*Error)
				if !ok {
					t.Fatalf("expected *Error, got %T: %v", err, err)
				}
				if cborErr.ID() != c.Expected {
					t.Fatalf("error id mismatch: got %s, want %s", cborErr.ID(), c.Expected)
				}
				if !strings.HasPrefix(cborErr.Error(), "canonical-cbor: ") {
					t.Fatalf("error message missing prefix: %q", cborErr.Error())
				}

				// Verify atomic rollback on EncodeIntoChecked
				dst := []byte{0xAA}
				errRollback := EncodeIntoChecked(val, &dst)
				if errRollback == nil {
					t.Fatalf("expected EncodeIntoChecked error")
				}
				if len(dst) != 1 || dst[0] != 0xAA {
					t.Fatalf("atomic rollback violated: dst modified to %v", dst)
				}

			default:
				t.Fatalf("unknown operation %q", c.Operation)
			}
		})
	}
}

func TestAllErrorIDsCovered(t *testing.T) {
	var doc fixtureDocument
	if err := json.Unmarshal([]byte(CanonicalCborCasesJSON), &doc); err != nil {
		t.Fatalf("failed to parse fixture json: %v", err)
	}

	exercised := make(map[string]bool)
	for _, c := range doc.Cases {
		if c.Operation == "decode-error" || c.Operation == "encode-error" {
			exercised[c.Expected] = true
		}
	}

	for _, expectedID := range doc.ErrorIDs {
		if !exercised[expectedID] {
			t.Errorf("fixture error ID %q was not exercised by any error case", expectedID)
		}
	}
}
