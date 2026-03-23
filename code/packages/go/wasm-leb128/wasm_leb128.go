// Package wasmleb128 provides LEB128 variable-length integer encoding for WASM binary format.
//
// This package is part of the coding-adventures monorepo, a ground-up
// implementation of the computing stack from transistors to operating systems.
//
// # What Is LEB128?
//
// LEB128 stands for "Little-Endian Base-128." It is a variable-length encoding
// for integers invented at Hewlett-Packard and used prominently in:
//
//   - WebAssembly binary format (all integer values — function indices, types,
//     immediate operands, section sizes, etc.)
//   - DWARF debug information
//   - Android DEX files
//   - Protocol Buffers (a similar scheme called "varint")
//
// The motivation is simple: most integers in programs are small. A 32-bit field
// that always holds 0–127 wastes 3 bytes per value in a fixed-width format.
// LEB128 uses only 1 byte for values 0–127, 2 bytes for 128–16383, etc.
//
// # Byte Layout
//
// Each LEB128 byte carries 7 data bits and 1 control bit:
//
//	Bit 7 (high bit): continuation flag — 1 = more bytes follow, 0 = last byte
//	Bits 6–0:         data payload (7 bits of the integer value)
//
//	┌─────────────────────────────────────────────────────────────────────┐
//	│  Bit:  7   6   5   4   3   2   1   0                               │
//	│       ───────────────────────────────                               │
//	│  Role: C   d6  d5  d4  d3  d2  d1  d0                              │
//	│        ↑                                                            │
//	│        continuation flag                                            │
//	└─────────────────────────────────────────────────────────────────────┘
//
// # Encoding Example (unsigned 624485)
//
//	624485 in binary: 0010_0110_0001_1100_0110_0101
//
//	Split into 7-bit groups (least-significant first):
//	    bits  0– 6: 110 0101  = 0x65
//	    bits  7–13: 000 1110  = 0x0E
//	    bits 14–20: 010 0110  = 0x26
//
//	Set continuation bit on all but the last group:
//	    0x65 | 0x80 = 0xE5   (continuation)
//	    0x0E | 0x80 = 0x8E   (continuation)
//	    0x26        = 0x26   (last byte)
//
//	Encoded:  [0xE5, 0x8E, 0x26]
//
// # Signed LEB128 (Two's Complement)
//
// Signed LEB128 adds sign extension on top of unsigned LEB128:
//   - During decoding: after reading the final byte (continuation bit = 0),
//     if bit 6 of that byte is 1, the value is negative and must be sign-extended:
//     set all bits above position (shift) to 1.
//   - During encoding: keep emitting bytes until the remaining value (after
//     right-shifting) is either 0 (positive) or -1 (negative) AND bit 6 of the
//     last payload matches the sign.
//
// Encoding -2:
//
//	-2 in two's complement: ...111111111111110
//	Low 7 bits: 111 1110 = 0x7E. Shift right by 7 → -1.
//	Done? value == -1 and bit 6 of 0x7E is 1 → YES.
//	Encoded: [0x7E]
//
// Decoding [0x7E]:
//
//	payload = 0x7E & 0x7F = 0x7E = 126
//	No continuation bit → last byte.
//	Bit 6 = 1 → negative → sign-extend: 126 | -(1<<7) = 126 | -128 = -2 ✓
//
// # API
//
//	DecodeUnsigned(data []byte, offset int) (value uint64, bytesConsumed int, err error)
//	DecodeSigned(data []byte, offset int)   (value int64,  bytesConsumed int, err error)
//	EncodeUnsigned(value uint64) []byte
//	EncodeSigned(value int64)   []byte
package wasmleb128

import "fmt"

// maxBytes is the maximum number of bytes a 32-bit LEB128 value can span.
//
// A 32-bit integer requires at most ceil(32/7) = 5 bytes.
// We use this as a safety guard against malformed/infinite byte streams.
const maxBytes = 5

// LEB128Error is returned when decoding encounters malformed input.
//
// Fields:
//   - Message: human-readable description of what went wrong
//   - Offset:  byte position in the input slice where decoding started
//
// It implements the built-in error interface via the Error() method.
//
// Example:
//
//	_, _, err := DecodeUnsigned([]byte{0x80, 0x80}, 0)
//	if err != nil {
//	    e := err.(*LEB128Error)
//	    fmt.Println(e.Message, e.Offset)
//	}
type LEB128Error struct {
	Message string
	Offset  int
}

// Error implements the error interface. Returns the Message field as a string.
func (e *LEB128Error) Error() string {
	return e.Message
}

// DecodeUnsigned reads an unsigned LEB128 integer from data starting at offset.
//
// Each byte contributes 7 bits of the integer value. The high bit of each byte
// is the continuation flag: 1 means more bytes follow, 0 means this is the last.
//
//	Algorithm:
//	  result = 0, shift = 0
//	  for each byte at (offset, offset+1, ...):
//	      payload = byte & 0x7F          // strip continuation bit
//	      result |= uint64(payload) << shift
//	      shift += 7
//	      if byte & 0x80 == 0:          // last byte
//	          return (result, bytes_consumed, nil)
//	  // ran out of bytes → error
//
// Parameters:
//   - data:   byte slice containing the LEB128-encoded integer
//   - offset: starting position within data
//
// Returns:
//   - value:         decoded unsigned integer
//   - bytesConsumed: number of bytes read from data
//   - err:           *LEB128Error if data is unterminated or too short
//
// Examples:
//
//	DecodeUnsigned([]byte{0x00}, 0)               // → (0, 1, nil)
//	DecodeUnsigned([]byte{0x03}, 0)               // → (3, 1, nil)
//	DecodeUnsigned([]byte{0xE5, 0x8E, 0x26}, 0)   // → (624485, 3, nil)
//	DecodeUnsigned([]byte{0x80, 0x80}, 0)          // → error
func DecodeUnsigned(data []byte, offset int) (uint64, int, error) {
	var result uint64
	var shift uint
	start := offset

	for i := 0; i < maxBytes; i++ {
		// Guard against reading past the end of the slice.
		if offset >= len(data) {
			return 0, 0, &LEB128Error{
				Message: fmt.Sprintf(
					"unterminated LEB128 at offset %d: ran out of bytes after %d byte(s)",
					start, i,
				),
				Offset: start,
			}
		}

		b := data[offset]
		offset++

		// Extract the 7 data bits and place them at the correct bit position.
		//
		// byte = 0xE5 = 1110_0101  (i=0, shift=0)
		//               ^           continuation bit (stripped below)
		//                ^^^ ^^^^  7 data bits → 0x65
		//
		// payload = 0xE5 & 0x7F = 0x65
		// result  |= 0x65 << 0  = 0x00000065
		payload := uint64(b & 0x7F)
		result |= payload << shift
		shift += 7

		if b&0x80 == 0 {
			// Continuation bit is 0 → this was the last byte.
			return result, offset - start, nil
		}
	}

	// Consumed maxBytes without seeing a final byte. Malformed.
	return 0, 0, &LEB128Error{
		Message: fmt.Sprintf(
			"unterminated LEB128 at offset %d: continuation bit still set after %d bytes (max for 32-bit)",
			start, maxBytes,
		),
		Offset: start,
	}
}

// DecodeSigned reads a signed LEB128 integer from data starting at offset.
//
// Identical to DecodeUnsigned except: after reading the last byte, if bit 6 of
// that byte is set, the value is negative and must be sign-extended to int64.
//
// Sign extension in practice:
//
//	if lastByte & 0x40 != 0 {
//	    result |= -(int64(1) << shift)
//	}
//
// This works because -(1<<shift) in two's complement has all bits set from
// position shift upward, which is exactly the sign extension we need.
//
// Example — [0x7E] → -2:
//
//	payload = 0x7E & 0x7F = 0x7E = 126
//	shift   = 7
//	last byte, bit 6 = 1 → sign extend
//	result  = int64(126) | -(1<<7)
//	        = 126 | -128
//	        = 0x7E | 0xFFFFFFFFFFFFFF80
//	        = 0xFFFFFFFFFFFFFFFE = -2 ✓
//
// Parameters and return values mirror DecodeUnsigned but value is int64.
func DecodeSigned(data []byte, offset int) (int64, int, error) {
	var result int64
	var shift uint
	start := offset

	for i := 0; i < maxBytes; i++ {
		if offset >= len(data) {
			return 0, 0, &LEB128Error{
				Message: fmt.Sprintf(
					"unterminated LEB128 at offset %d: ran out of bytes after %d byte(s)",
					start, i,
				),
				Offset: start,
			}
		}

		b := data[offset]
		offset++

		payload := int64(b & 0x7F)
		result |= payload << shift
		shift += 7

		if b&0x80 == 0 {
			// Last byte. Check the sign bit (bit 6 of this byte = bit (shift-1) overall).
			//
			// Bit 6 of b is tested as: b & 0x40 != 0
			//
			// If set, the decoded value is negative in the original integer width
			// (32-bit for WASM), so we sign-extend to fill int64.
			if b&0x40 != 0 {
				result |= -(int64(1) << shift)
			}
			return result, offset - start, nil
		}
	}

	return 0, 0, &LEB128Error{
		Message: fmt.Sprintf(
			"unterminated LEB128 at offset %d: continuation bit still set after %d bytes (max for 32-bit)",
			start, maxBytes,
		),
		Offset: start,
	}
}

// EncodeUnsigned encodes a non-negative integer as unsigned LEB128.
//
// Algorithm:
//
//	loop (at least once, to handle 0):
//	    payload = value & 0x7F   // low 7 bits
//	    value >>= 7
//	    if value != 0:
//	        emit payload | 0x80  // more bytes follow
//	    else:
//	        emit payload         // final byte
//	        break
//
// Example — encode 624485:
//
//	iter 1: payload = 0x98765 & 0x7F = 0x65, value = 0x98765>>7 = 0x130E (≠0)
//	        emit 0x65 | 0x80 = 0xE5
//	iter 2: payload = 0x130E & 0x7F = 0x0E, value = 0x130E>>7 = 0x26 (≠0)
//	        emit 0x0E | 0x80 = 0x8E
//	iter 3: payload = 0x26 & 0x7F = 0x26, value = 0x26>>7 = 0 (=0)
//	        emit 0x26
//	result: [0xE5, 0x8E, 0x26] ✓
func EncodeUnsigned(value uint64) []byte {
	var out []byte

	for {
		payload := byte(value & 0x7F) // low 7 bits
		value >>= 7
		if value != 0 {
			// More groups remain → set continuation bit.
			out = append(out, payload|0x80)
		} else {
			// Final group → no continuation bit.
			out = append(out, payload)
			break
		}
	}

	return out
}

// EncodeSigned encodes a signed integer as signed LEB128 (two's complement).
//
// Similar to EncodeUnsigned but the termination condition accounts for the sign
// bit of the last byte:
//
//	done when: (value == 0  AND bit 6 of payload == 0)  ← positive value fully encoded
//	        OR (value == -1 AND bit 6 of payload == 1)  ← negative value fully encoded
//
// The second condition ensures the decoder's sign extension will reconstruct the
// correct negative value. Without it, a value like +64 (0x40) would be written as
// a single byte 0x40, but bit 6 = 1 → decoder would sign-extend to -64!
//
// Example — encode +64 (requires 2 bytes despite fitting in 7 bits):
//
//	iter 1: payload = 64 & 0x7F = 0x40 (bit 6 = 1!), value = 64>>7 = 0
//	        done? value==0 and bit6==0? NO (bit 6 is 1, would be misread as negative)
//	        emit 0x40 | 0x80 = 0xC0
//	iter 2: payload = 0 & 0x7F = 0x00 (bit 6 = 0), value = 0>>7 = 0
//	        done? value==0 and bit6==0? YES
//	        emit 0x00
//	result: [0xC0, 0x00] — decodes to +64 correctly ✓
//
// Example — encode -2:
//
//	iter 1: payload = (-2) & 0x7F = 0x7E (bit 6 = 1), value = (-2)>>7 = -1
//	        done? value==-1 and bit6!=0? YES
//	        emit 0x7E
//	result: [0x7E] ✓
func EncodeSigned(value int64) []byte {
	var out []byte

	for {
		payload := byte(value & 0x7F) // low 7 bits
		value >>= 7                   // arithmetic right shift (preserves sign)

		// Check done condition.
		done := (value == 0 && payload&0x40 == 0) ||
			(value == -1 && payload&0x40 != 0)

		if done {
			out = append(out, payload) // final byte — no continuation bit
			break
		}
		out = append(out, payload|0x80) // more bytes follow
	}

	return out
}
