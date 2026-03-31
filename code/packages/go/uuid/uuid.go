// Package ca_uuid implements UUID v1/v3/v4/v5/v7 generation and parsing.
//
// # What Is a UUID?
//
// A UUID (Universally Unique Identifier), also called a GUID (Globally Unique
// Identifier), is a 128-bit label used to identify information in computer
// systems without requiring a central registration authority. The standard
// representation is 32 hexadecimal digits displayed in five groups separated by
// hyphens, in the form 8-4-4-4-12:
//
//	xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
//	^        ^    ^    ^    ^
//	|        |    |    |    12 hex = 6 bytes = node/random
//	|        |    |    4 hex = 2 bytes = clock_seq (N encodes variant)
//	|        |    4 hex = 2 bytes = time_hi (M encodes version)
//	|        4 hex = 2 bytes = time_mid
//	8 hex = 4 bytes = time_low
//
// The "M" nibble encodes the version (1, 3, 4, 5, 7 in this package).
// The top bits of "N" encode the variant — 10xxxxxx means RFC 4122 (most UUIDs).
//
// # UUID Versions
//
//   - v1: Time-based. Encodes the current time as 100-nanosecond intervals since
//     October 15, 1582 (the Gregorian calendar epoch), plus a random node ID.
//     Pros: monotonically ordered within a node. Cons: reveals the MAC address
//     (hence the random node alternative used here).
//
//   - v3: Name-based, MD5 hash. Deterministic: the same namespace + name always
//     produces the same UUID. Not collision-resistant (MD5 is broken).
//
//   - v4: Randomly generated. 122 bits of randomness, 6 bits used for version
//     and variant. The most common UUID type in practice.
//
//   - v5: Name-based, SHA-1 hash. Like v3 but using the stronger SHA-1. Still
//     deterministic from namespace + name. Preferred over v3.
//
//   - v7: Time-ordered random. Encodes a 48-bit Unix millisecond timestamp in
//     the most-significant bytes so that UUIDs sort chronologically. The
//     remaining 74 bits are random. Ideal for database primary keys.
//
// # Standards
//
// This package implements RFC 4122 (original UUID standard) and RFC 9562 (2024
// update that formally defines v7). The RFC test vectors for v3 and v5 are
// reproduced in uuid_test.go and must pass exactly.
//
// # Layout in Memory
//
// All UUIDs are stored as [16]byte in network byte order (big-endian). When we
// say "byte 6", we mean the byte at index 6 of that array.
//
//	Byte index:  0  1  2  3  |  4  5  |  6  7  |  8  9  | 10 11 12 13 14 15
//	Field:       time_low    | time_mid | time_hi+ver | clk_seq | node
//
// This matches the wire format defined in RFC 4122 section 4.1.2.
//
// # Operations
//
// Every public function and method is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery.
package uuid

import (
	"crypto/rand"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"regexp"
	"strings"
	"sync"
	"time"

	md5pkg "github.com/adhithyan15/coding-adventures/code/packages/go/md5"
	sha1pkg "github.com/adhithyan15/coding-adventures/code/packages/go/sha1"
)

// UUID is a 128-bit universally unique identifier stored as 16 bytes in network
// byte order (big-endian). Using a named array type gives us value semantics and
// lets us define methods directly on the UUID.
type UUID [16]byte

// ─── Error Type ──────────────────────────────────────────────────────────────

// UUIDError is the error type returned by this package. Wrapping it in a named
// type means callers can use errors.As to distinguish UUID errors from other
// errors in their applications.
type UUIDError struct{ msg string }

func (e *UUIDError) Error() string { return "uuid: " + e.msg }

// ─── Parsing ─────────────────────────────────────────────────────────────────

// hexRE matches all valid UUID string formats accepted by Parse. We anchor with
// ^ and $ and allow optional leading/trailing whitespace so callers do not need
// to pre-trim strings.
//
// Accepted forms (case-insensitive):
//
//	Standard:   6ba7b810-9dad-11d1-80b4-00c04fd430c8
//	Uppercase:  6BA7B810-9DAD-11D1-80B4-00C04FD430C8
//	Compact:    6ba7b8109dad11d180b400c04fd430c8
//	Braced:     {6ba7b810-9dad-11d1-80b4-00c04fd430c8}
//	URN:        urn:uuid:6ba7b810-9dad-11d1-80b4-00c04fd430c8
//
// The five capture groups correspond to the five UUID fields (8-4-4-4-12).
// We strip hyphens to let the compact form work: the regex makes hyphens
// optional with -?.
var hexRE = regexp.MustCompile(
	`(?i)^\s*(?:urn:uuid:)?\{?` +
		`([0-9a-f]{8})-?` +
		`([0-9a-f]{4})-?` +
		`([0-9a-f]{4})-?` +
		`([0-9a-f]{4})-?` +
		`([0-9a-f]{12})` +
		`\}?\s*$`,
)

// Parse converts a UUID string to a UUID value. It accepts all common UUID
// representations:
//   - Standard hyphenated:     xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
//   - Uppercase:               XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX
//   - Compact (no hyphens):    xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
//   - Braced:                  {xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}
//   - URN:                     urn:uuid:xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
//
// Returns UUIDError if the string does not match any accepted format.
func Parse(s string) (UUID, error) {
	return StartNew[UUID]("uuid.Parse", UUID{},
		func(op *Operation[UUID], rf *ResultFactory[UUID]) *OperationResult[UUID] {
			op.AddProperty("s", s)

			// The regex captures exactly five hex groups. If it does not match, the
			// string is not a valid UUID in any of our accepted forms.
			m := hexRE.FindStringSubmatch(s)
			if m == nil {
				return rf.Fail(UUID{}, &UUIDError{msg: fmt.Sprintf("invalid UUID string: %q", s)})
			}

			// Reassemble the five groups into a flat 32-character hex string. This
			// normalises all formats into one canonical sequence we can decode with
			// hex.DecodeString.
			joined := m[1] + m[2] + m[3] + m[4] + m[5]
			b, err := hex.DecodeString(joined)
			if err != nil {
				// Should never happen given the regex already validated hex characters,
				// but we handle it defensively.
				return rf.Fail(UUID{}, &UUIDError{msg: fmt.Sprintf("hex decode failed: %v", err)})
			}

			var u UUID
			copy(u[:], b)
			return rf.Generate(true, false, u)
		}).GetResult()
}

// IsValid returns true if s is a valid UUID string in any format Parse accepts.
// It is a convenience wrapper around Parse for use in validation-only contexts
// where you do not need the UUID value.
func IsValid(s string) bool {
	result, _ := StartNew[bool]("uuid.IsValid", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("s", s)
			_, err := Parse(s)
			return rf.Generate(true, false, err == nil)
		}).GetResult()
	return result
}

// ─── Formatting ──────────────────────────────────────────────────────────────

// String returns the canonical RFC 4122 representation: 32 lowercase hex digits
// grouped as 8-4-4-4-12 with hyphens.
//
// Example: "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
//
// We build the string by hex-encoding each of the five UUID fields individually
// rather than hex-encoding all 16 bytes at once, to avoid a second pass for
// inserting hyphens.
func (u UUID) String() string {
	result, _ := StartNew[string]("uuid.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			// hex.EncodeToString allocates; for a hot path you would use a pre-allocated
			// buffer, but clarity wins here since this is a learning package.
			s := fmt.Sprintf("%08x-%04x-%04x-%04x-%012x",
				u[0:4],  // time_low  (4 bytes = 8 hex chars)
				u[4:6],  // time_mid  (2 bytes = 4 hex chars)
				u[6:8],  // time_hi   (2 bytes = 4 hex chars, includes version nibble)
				u[8:10], // clock_seq (2 bytes = 4 hex chars, includes variant bits)
				u[10:],  // node      (6 bytes = 12 hex chars)
			)
			return rf.Generate(true, false, s)
		}).GetResult()
	return result
}

// Bytes returns the UUID as a 16-byte slice. Callers receive a slice into the
// array, so mutations will affect the original UUID. Use copy if you need an
// independent copy.
func (u UUID) Bytes() []byte {
	result, _ := StartNew[[]byte]("uuid.Bytes", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			return rf.Generate(true, false, u[:])
		}).GetResult()
	return result
}

// ToInt returns the UUID as a pair of uint64 values representing the high and
// low 64-bit halves of the 128-bit integer, in big-endian order.
//
// This is useful when you need to store a UUID in two 64-bit database columns
// or compare UUIDs as integers.
func (u UUID) ToInt() (hi, lo uint64) {
	type toIntResult struct {
		hi uint64
		lo uint64
	}
	result, _ := StartNew[toIntResult]("uuid.ToInt", toIntResult{},
		func(op *Operation[toIntResult], rf *ResultFactory[toIntResult]) *OperationResult[toIntResult] {
			h := binary.BigEndian.Uint64(u[0:8])
			l := binary.BigEndian.Uint64(u[8:16])
			return rf.Generate(true, false, toIntResult{hi: h, lo: l})
		}).GetResult()
	return result.hi, result.lo
}

// ─── Metadata ────────────────────────────────────────────────────────────────

// Version returns the version number embedded in bits 76-79 (byte 6, high
// nibble). RFC 4122 defines versions 1-5; RFC 9562 adds versions 6-8.
//
// The version nibble sits at the top 4 bits of byte 6:
//
//	byte 6: Vvvv xxxx
//	        ^^^^ = version nibble
//	             ^^^^ = high 4 bits of time_hi (v1) or random (v4)
func (u UUID) Version() int {
	result, _ := StartNew[int]("uuid.Version", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			// Shift right by 4 to move the high nibble to the low nibble position, then
			// mask to ensure we only keep the 4 version bits (in case of sign extension
			// on platforms where byte is signed, though Go's byte is always unsigned).
			return rf.Generate(true, false, int(u[6]>>4)&0xF)
		}).GetResult()
	return result
}

// Variant returns a human-readable string describing the UUID variant field.
// The variant occupies the top 1-3 bits of byte 8:
//
//	0xxxxxxx = NCS backward compatibility (historical, rarely seen today)
//	10xxxxxx = RFC 4122 / RFC 9562 — the standard used by this package
//	110xxxxx = Microsoft COM / DCOM GUIDs
//	111xxxxx = Reserved for future use
//
// All UUIDs generated by this package will return "rfc4122".
func (u UUID) Variant() string {
	result, _ := StartNew[string]("uuid.Variant", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			b := u[8]
			var variant string
			switch {
			case b>>7 == 0: // 0xxxxxxx
				variant = "ncs"
			case b>>6 == 0b10: // 10xxxxxx
				variant = "rfc4122"
			case b>>5 == 0b110: // 110xxxxx
				variant = "microsoft"
			default: // 111xxxxx
				variant = "reserved"
			}
			return rf.Generate(true, false, variant)
		}).GetResult()
	return result
}

// IsNil returns true if the UUID is the nil UUID (all 128 bits zero). The nil
// UUID is the UUID equivalent of a null pointer — it signals "no UUID assigned".
func (u UUID) IsNil() bool {
	result, _ := StartNew[bool]("uuid.IsNil", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, u == Nil)
		}).GetResult()
	return result
}

// IsMax returns true if the UUID is the max UUID (all 128 bits one, i.e., all
// bytes 0xFF). Defined in RFC 9562 as a complement to the nil UUID.
func (u UUID) IsMax() bool {
	result, _ := StartNew[bool]("uuid.IsMax", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, u == Max)
		}).GetResult()
	return result
}

// ─── Sentinel Values ─────────────────────────────────────────────────────────

// Nil is the nil UUID: all 128 bits zero.
//
//	00000000-0000-0000-0000-000000000000
var Nil = UUID{}

// Max is the max UUID: all 128 bits one.
//
//	ffffffff-ffff-ffff-ffff-ffffffffffff
var Max = UUID{
	0xff, 0xff, 0xff, 0xff,
	0xff, 0xff,
	0xff, 0xff,
	0xff, 0xff,
	0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
}

// ─── Namespace Constants ──────────────────────────────────────────────────────

// RFC 4122 section 4.3 defines four well-known namespace UUIDs for use with v3
// and v5 name-based UUIDs. Using a namespace prevents name collisions across
// different naming systems: "www.example.com" in the DNS namespace yields a
// different UUID than "www.example.com" in the URL namespace.

// NamespaceDNS is used when names are DNS domain names (e.g., "python.org").
var NamespaceDNS = mustParse("6ba7b810-9dad-11d1-80b4-00c04fd430c8")

// NamespaceURL is used when names are URLs (e.g., "https://example.com/path").
var NamespaceURL = mustParse("6ba7b811-9dad-11d1-80b4-00c04fd430c8")

// NamespaceOID is used when names are ISO Object Identifiers.
var NamespaceOID = mustParse("6ba7b812-9dad-11d1-80b4-00c04fd430c8")

// NamespaceX500 is used when names are X.500 Distinguished Names (DNs) in DER
// or a text output format.
var NamespaceX500 = mustParse("6ba7b814-9dad-11d1-80b4-00c04fd430c8")

// mustParse is a package-internal helper used to initialise the namespace
// constants at startup. It panics on invalid input because the namespace strings
// are compile-time constants and a parse failure indicates a programming error,
// not a runtime condition.
func mustParse(s string) UUID {
	u, err := Parse(s)
	if err != nil {
		panic(err)
	}
	return u
}

// ─── Version / Variant Stamping ──────────────────────────────────────────────

// setVersionVariant stamps the version nibble and RFC 4122 variant bits into a
// raw 16-byte slice. This is called by V3, V5 (and indirectly V4, V1, V7
// through their own direct byte manipulation).
//
// Version nibble: byte 6 stores [VERSION_NIBBLE | high4_of_time_hi].
// We zero the top nibble with & 0x0F, then OR in the version shifted to the
// top nibble with byte(version << 4).
//
//	Before: xxxx yyyy   (x = old version nibble, y = data nibble)
//	Mask:   0000 yyyy   (clear version bits)
//	OR ver: VVVV yyyy   (stamp new version)
//
// Variant bits: byte 8 stores [10 | clock_seq_hi_res].
// RFC 4122 mandates the top two bits of byte 8 be "10" (binary).
// We zero the top two bits with & 0x3F, then OR in 0x80 (= 1000 0000) to set
// exactly the "10" pattern.
//
//	Before: xx yyy yyy   (x = old variant, y = clock data)
//	Mask:   00 yyy yyy   (clear variant bits)
//	OR 0x80: 10 yyy yyy  (stamp RFC 4122 variant)
func setVersionVariant(raw []byte, version int) {
	raw[6] = (raw[6] & 0x0F) | byte(version<<4)
	raw[8] = (raw[8] & 0x3F) | 0x80
}

// ─── UUID v4: Random ─────────────────────────────────────────────────────────

// V4 generates a version 4 (randomly generated) UUID.
//
// Structure of a v4 UUID:
//
//	Bytes 0-5:   random
//	Byte  6:     version nibble (0100) | 4 random bits
//	Byte  7:     random
//	Byte  8:     variant bits (10) | 6 random bits
//	Bytes 9-15:  random
//
// Total randomness: 6 bytes + 4 bits + 1 byte + 6 bits + 7 bytes
//               = 122 bits of randomness.
//
// We use crypto/rand (operating-system CSPRNG) rather than math/rand so that
// UUIDs are unpredictable. On Linux this is /dev/urandom via getrandom(2); on
// Windows it is CryptGenRandom.
func V4() (UUID, error) {
	return StartNew[UUID]("uuid.V4", UUID{},
		func(op *Operation[UUID], rf *ResultFactory[UUID]) *OperationResult[UUID] {
			var u UUID

			// Fill all 16 bytes with cryptographically secure random data.
			if _, err := rand.Read(u[:]); err != nil {
				return rf.Fail(UUID{}, &UUIDError{msg: fmt.Sprintf("crypto/rand failed: %v", err)})
			}

			// Stamp version and variant into the appropriate bytes, overwriting the
			// random bits that occupied those positions.
			setVersionVariant(u[:], 4)
			return rf.Generate(true, false, u)
		}).GetResult()
}

// ─── UUID v5: SHA-1 Name-Based ───────────────────────────────────────────────

// V5 generates a version 5 (SHA-1 name-based) UUID.
//
// Algorithm (RFC 4122 section 4.3):
//  1. Concatenate the namespace UUID bytes and the name bytes.
//  2. Hash with SHA-1 → 20-byte digest.
//  3. Take the first 16 bytes of the digest.
//  4. Stamp version=5 and RFC 4122 variant.
//
// The same (namespace, name) pair always produces the same UUID. This makes v5
// useful for generating stable identifiers for well-known resources (e.g., DNS
// names, URLs) without coordination.
//
// SHA-1 is not collision-resistant for adversarial inputs, but for name-based
// UUIDs the inputs are controlled, so the deterministic property matters more
// than the cryptographic strength.
//
// RFC 4122 test vector:
//
//	V5(NamespaceDNS, "python.org") = "886313e1-3b8a-5372-9b90-0c9aee199e5d"
func V5(namespace UUID, name string) UUID {
	result, _ := StartNew[UUID]("uuid.V5", UUID{},
		func(op *Operation[UUID], rf *ResultFactory[UUID]) *OperationResult[UUID] {
			op.AddProperty("name", name)

			// Step 1: Build the hash input = namespace_bytes || name_bytes.
			// append creates a new slice, so namespace is not mutated.
			data := append(namespace[:], []byte(name)...)

			// Step 2: SHA-1 hash. Sum1 returns [20]byte.
			digest := sha1pkg.Sum1(data)

			// Step 3: Take the first 16 bytes.
			var raw [16]byte
			copy(raw[:], digest[:16])

			// Step 4: Stamp version 5 and RFC 4122 variant.
			setVersionVariant(raw[:], 5)
			return rf.Generate(true, false, UUID(raw))
		}).GetResult()
	return result
}

// ─── UUID v3: MD5 Name-Based ─────────────────────────────────────────────────

// V3 generates a version 3 (MD5 name-based) UUID.
//
// Algorithm (RFC 4122 section 4.3) — identical structure to v5 but with MD5:
//  1. Concatenate namespace UUID bytes and name bytes.
//  2. Hash with MD5 → 16-byte digest (note: MD5 output is exactly 16 bytes,
//     so no truncation is needed unlike v5's 20-byte SHA-1).
//  3. Stamp version=3 and RFC 4122 variant.
//
// Important: MD5 is cryptographically broken. Use v5 (SHA-1) for new systems,
// or better yet use a proper HMAC if security is a concern. V3 is included
// because it is part of RFC 4122 and some legacy systems require it.
//
// RFC 4122 test vector:
//
//	V3(NamespaceDNS, "python.org") = "6fa459ea-ee8a-3ca4-894e-db77e160355e"
func V3(namespace UUID, name string) UUID {
	result, _ := StartNew[UUID]("uuid.V3", UUID{},
		func(op *Operation[UUID], rf *ResultFactory[UUID]) *OperationResult[UUID] {
			op.AddProperty("name", name)

			// Step 1: Build the hash input.
			data := append(namespace[:], []byte(name)...)

			// Step 2: MD5 hash. SumMD5 returns [16]byte — exactly the size we need.
			digest := md5pkg.SumMD5(data)

			// Step 3: Stamp version 3 and RFC 4122 variant directly into the digest.
			// We can do this in-place because digest is a value (array), not a slice
			// pointing to shared memory.
			setVersionVariant(digest[:], 3)
			return rf.Generate(true, false, UUID(digest))
		}).GetResult()
	return result
}

// ─── UUID v1: Time-Based ─────────────────────────────────────────────────────

// gregorianOffset is the number of 100-nanosecond intervals between the
// Gregorian calendar epoch (October 15, 1582 00:00:00 UTC) and the Unix epoch
// (January 1, 1970 00:00:00 UTC).
//
// Calculation:
//
//	Years from 1582-10-15 to 1970-01-01: 387 years, 3 months, 17 days
//	= 122,192,928,000,000,000 × 100 ns intervals
//
// This constant is the magic number specified in RFC 4122 section 4.1.4.
const gregorianOffset = uint64(122192928000000000)

// v1Mu protects v1Clock and ensures no two concurrent V1() calls get the same
// timestamp + clock_seq combination (monotonicity guarantee).
var (
	v1Mu    sync.Mutex
	v1Clock uint16   // 14-bit clock sequence, randomly initialized once
	v1Once  sync.Once // ensures v1Clock is initialized exactly once
)

// V1 generates a version 1 (time-based) UUID.
//
// The 60-bit timestamp is the count of 100-nanosecond intervals since the
// Gregorian epoch (Oct 15, 1582). This timestamp is split across three UUID
// fields in a somewhat counter-intuitive order (low, mid, high) for historical
// compatibility with the original DCE RPC specification.
//
// Layout:
//
//	Bytes 0-3:   time_low (32 low-order bits of timestamp)
//	Bytes 4-5:   time_mid (16 middle bits of timestamp)
//	Bytes 6-7:   time_hi_and_version (12 high bits of timestamp | version 1)
//	Byte  8:     clock_seq_hi_res (variant 10 | high 6 bits of clock_seq)
//	Byte  9:     clock_seq_low (low 8 bits of clock_seq)
//	Bytes 10-15: node (6 random bytes with multicast bit set)
//
// Node ID: RFC 4122 allows using the MAC address as the node ID. However,
// leaking the machine's MAC address is a privacy concern. Instead, we generate
// a random 48-bit node ID and set the multicast bit (bit 0 of byte 10) to
// signal that this is a random node, not an actual IEEE 802 address.
func V1() (UUID, error) {
	return StartNew[UUID]("uuid.V1", UUID{},
		func(op *Operation[UUID], rf *ResultFactory[UUID]) *OperationResult[UUID] {
			// Initialize the clock sequence once with random data. The clock sequence
			// exists to guarantee uniqueness even if the clock goes backwards (e.g., NTP
			// adjustment) or if two processes generate UUIDs within the same 100ns tick.
			v1Once.Do(func() {
				var buf [2]byte
				if _, err := rand.Read(buf[:]); err != nil {
					// In the unlikely event of CSPRNG failure, use a non-zero default.
					v1Clock = 0x1234
					return
				}
				// Only 14 bits are used for the clock sequence (the top 2 bits of byte 8
				// are overwritten by the variant stamp). Mask to 14 bits.
				v1Clock = binary.BigEndian.Uint16(buf[:]) & 0x3FFF
			})

			v1Mu.Lock()
			clock := v1Clock
			v1Mu.Unlock()

			// Compute the 60-bit timestamp: nanoseconds since Unix epoch → 100ns ticks
			// → shift to Gregorian epoch.
			now := uint64(time.Now().UnixNano()/100) + gregorianOffset

			// Split the 60-bit timestamp into three fields:
			//   time_low:  bits  0-31  (least significant 32 bits)
			//   time_mid:  bits 32-47
			//   time_hi:   bits 48-59  (most significant 12 bits)
			timeLow := uint32(now & 0xFFFFFFFF)
			timeMid := uint16((now >> 32) & 0xFFFF)
			timeHi := uint16((now >> 48) & 0x0FFF)

			// Stamp version 1 into time_hi. The version nibble occupies bits 12-15 of
			// the time_hi_and_version field. Setting 0x1000 = 0001 0000 0000 0000
			// places a '1' in the 13th bit position (0-indexed from 0).
			timeHiAndVersion := 0x1000 | timeHi

			// Pack into UUID bytes using big-endian byte order (network byte order).
			var u UUID
			binary.BigEndian.PutUint32(u[0:4], timeLow)
			binary.BigEndian.PutUint16(u[4:6], timeMid)
			binary.BigEndian.PutUint16(u[6:8], timeHiAndVersion)

			// Clock sequence: the top 2 bits of byte 8 encode the RFC 4122 variant (10).
			// Byte 8: 1 0 | clock[13] clock[12] ... clock[8]   (6 high bits of clock)
			// Byte 9: clock[7] ... clock[0]                     (8 low bits of clock)
			u[8] = 0x80 | byte(clock>>8)&0x3F // variant=10, clock_seq high 6 bits
			u[9] = byte(clock & 0xFF)         // clock_seq low 8 bits

			// Generate a 6-byte random node ID.
			if _, err := rand.Read(u[10:]); err != nil {
				return rf.Fail(UUID{}, &UUIDError{msg: fmt.Sprintf("crypto/rand failed for node: %v", err)})
			}

			// Set the multicast bit (LSB of first byte of node) to signal this is a
			// randomly generated node ID, not a real IEEE 802 MAC address.
			// RFC 4122 section 4.5: "Set the multicast bit of the node ID."
			u[10] |= 0x01

			return rf.Generate(true, false, u)
		}).GetResult()
}

// ─── UUID v7: Time-Ordered Random ────────────────────────────────────────────

// V7 generates a version 7 (time-ordered random) UUID as defined in RFC 9562.
//
// V7 was designed to fix the database performance problem with v4: since v4 is
// purely random, inserting many v4 UUIDs into a B-tree index causes page
// splits at random positions, degrading performance. V7 encodes a Unix
// millisecond timestamp in the most-significant bits so that UUIDs generated
// close in time sort close together — just like auto-increment integers, but
// globally unique.
//
// Layout (RFC 9562 section 5.7):
//
//	Bits 0-47:   unix_ts_ms (48-bit big-endian Unix timestamp in milliseconds)
//	Bits 48-51:  version = 7 (0111)
//	Bits 52-63:  rand_a (12 random bits)
//	Bits 64-65:  variant = 10 (RFC 4122)
//	Bits 66-127: rand_b (62 random bits)
//
// Byte-level breakdown:
//
//	Byte 0: ts[47:40]
//	Byte 1: ts[39:32]
//	Byte 2: ts[31:24]
//	Byte 3: ts[23:16]
//	Byte 4: ts[15:8]
//	Byte 5: ts[7:0]
//	Byte 6: 0111 | rand_a[11:8]   (version nibble + 4 random bits)
//	Byte 7: rand_a[7:0]           (8 random bits)
//	Byte 8: 10 | rand_b[61:56]    (variant + 6 random bits)
//	Bytes 9-15: rand_b[55:0]      (56 random bits = 7 bytes)
func V7() (UUID, error) {
	return StartNew[UUID]("uuid.V7", UUID{},
		func(op *Operation[UUID], rf *ResultFactory[UUID]) *OperationResult[UUID] {
			// Unix timestamp in milliseconds (48 bits, good until year 10895).
			tsMs := uint64(time.Now().UnixNano() / 1_000_000)

			// Generate 10 bytes of randomness for rand_a (12 bits) and rand_b (62 bits).
			// We need bytes 6-15: that is 10 bytes. We generate them all at once for
			// efficiency.
			var rnd [10]byte
			if _, err := rand.Read(rnd[:]); err != nil {
				return rf.Fail(UUID{}, &UUIDError{msg: fmt.Sprintf("crypto/rand failed: %v", err)})
			}

			var u UUID

			// Bytes 0-5: 48-bit big-endian millisecond timestamp.
			// We write the timestamp as a 64-bit value and take the high 6 bytes, which
			// corresponds to the most-significant 48 bits.
			u[0] = byte(tsMs >> 40)
			u[1] = byte(tsMs >> 32)
			u[2] = byte(tsMs >> 24)
			u[3] = byte(tsMs >> 16)
			u[4] = byte(tsMs >> 8)
			u[5] = byte(tsMs)

			// Byte 6: version nibble (0111 = 7) in the high 4 bits, plus 4 random bits
			// from rand_a in the low 4 bits.
			u[6] = 0x70 | (rnd[0] & 0x0F)

			// Byte 7: 8 random bits (low 8 bits of rand_a).
			u[7] = rnd[1]

			// Byte 8: variant bits (10) in top 2 bits, plus 6 random bits from rand_b.
			u[8] = 0x80 | (rnd[2] & 0x3F)

			// Bytes 9-15: 56 random bits (7 bytes of rand_b).
			copy(u[9:], rnd[3:10])

			return rf.Generate(true, false, u)
		}).GetResult()
}

// ─── Comparison ──────────────────────────────────────────────────────────────

// Compare returns an integer comparing two UUIDs lexicographically by their
// 16 bytes (which for v7 is also chronological order).
//
//	-1 if u < v
//	 0 if u == v
//	+1 if u > v
func Compare(u, v UUID) int {
	result, _ := StartNew[int]("uuid.Compare", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, strings.Compare(string(u[:]), string(v[:])))
		}).GetResult()
	return result
}
