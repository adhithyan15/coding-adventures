// Package hkdf provides HKDF (HMAC-based Extract-and-Expand Key Derivation Function).
//
// # What Is HKDF?
//
// HKDF (RFC 5869) derives one or more cryptographically strong keys from a
// single piece of input keying material (IKM). It is the standard key
// derivation function used in TLS 1.3, Signal Protocol, WireGuard, and many
// other modern protocols.
//
// HKDF was designed by Hugo Krawczyk (the same person behind HMAC) and
// published as RFC 5869 in 2010. It is intentionally simple: just two calls
// to HMAC.
//
// # Why Do We Need Key Derivation?
//
// Raw key material — from a Diffie-Hellman exchange, a password, or a random
// source — is not always suitable for direct use. It might have:
//   - Non-uniform distribution (some bits more predictable)
//   - Wrong length (DH gives 32 bytes, but you need AES key + HMAC key + IV)
//   - Insufficient entropy concentration
//
// HKDF solves all three through a two-phase approach.
//
// # Phase 1: Extract
//
// Extract takes the raw IKM and concentrates its entropy into a fixed-length
// pseudorandom key (PRK):
//
//	PRK = HMAC-Hash(salt, IKM)
//
//	+------+     +------+
//	| salt |---->|      |
//	+------+     | HMAC |----> PRK (HashLen bytes)
//	| IKM  |---->|      |
//	+------+     +------+
//
// Note: salt is the HMAC key and IKM is the HMAC message.
// This follows RFC 5869 Section 2.2 exactly.
//
// # Phase 2: Expand
//
// Expand produces as many output bytes as needed by chaining HMAC calls:
//
//	T(0) = empty
//	T(i) = HMAC-Hash(PRK, T(i-1) || info || i)   for i = 1..N
//	OKM  = first L bytes of T(1) || T(2) || ... || T(N)
//
// The counter byte is a single octet (0x01..0xFF), so the maximum output
// is 255 × HashLen bytes.
//
// # Supported Hash Functions
//
//	Algorithm   HashLen   Max OKM
//	SHA-256     32        8160 bytes
//	SHA-512     64        16320 bytes
//
// # Example
//
//	okm, err := hkdf.HKDF([]byte("salt"), []byte("ikm"), []byte("info"), 32, hkdf.SHA256)
package hkdf

import (
	"errors"
	"fmt"

	ghmac "github.com/adhithyan15/coding-adventures/code/packages/go/hmac"
	gsha256 "github.com/adhithyan15/coding-adventures/code/packages/go/sha256"
	gsha512 "github.com/adhithyan15/coding-adventures/code/packages/go/sha512"
)

// ─── Hash Algorithm Selection ────────────────────────────────────────────────

// HashAlgorithm identifies which hash function HKDF should use.
type HashAlgorithm int

const (
	// SHA256 selects HMAC-SHA256 (32-byte output, 64-byte block).
	SHA256 HashAlgorithm = iota
	// SHA512 selects HMAC-SHA512 (64-byte output, 128-byte block).
	SHA512
)

// hashLen returns the output length of the hash function in bytes.
//
// This determines:
//   - The length of PRK from Extract
//   - The size of each T(i) block in Expand
//   - The default salt length when none is provided
func (h HashAlgorithm) hashLen() int {
	switch h {
	case SHA512:
		return 64
	default:
		return 32
	}
}

// blockSize returns the internal block size of the hash function.
//
// SHA-256 uses 64-byte blocks (32-bit words × 16 words).
// SHA-512 uses 128-byte blocks (64-bit words × 16 words).
func (h HashAlgorithm) blockSize() int {
	switch h {
	case SHA512:
		return 128
	default:
		return 64
	}
}

// hmacFn computes HMAC using the selected hash algorithm.
//
// It wraps the generic HMAC function from coding_adventures_hmac,
// selecting the appropriate hash function and block size.
func (h HashAlgorithm) hmacFn(key, message []byte) []byte {
	switch h {
	case SHA512:
		return ghmac.HMAC(func(d []byte) []byte {
			s := gsha512.Sum512(d)
			return s[:]
		}, 128, key, message)
	default:
		return ghmac.HMAC(func(d []byte) []byte {
			s := gsha256.Sum256(d)
			return s[:]
		}, 64, key, message)
	}
}

// ─── Errors ──────────────────────────────────────────────────────────────────

// ErrOutputTooLong is returned when the requested output length exceeds
// 255 × HashLen. The Expand phase uses a single-byte counter (0x01..0xFF),
// so it can produce at most 255 HMAC blocks.
var ErrOutputTooLong = errors.New("hkdf: output length exceeds 255 * HashLen")

// ErrOutputTooShort is returned when the requested output length is zero.
var ErrOutputTooShort = errors.New("hkdf: output length must be positive")

// ─── Extract ─────────────────────────────────────────────────────────────────

// Extract performs HKDF-Extract: concentrating entropy from IKM into a
// pseudorandom key (PRK).
//
// Implements RFC 5869 Section 2.2:
//
//	PRK = HMAC-Hash(salt, IKM)
//
// If salt is nil or empty, a string of HashLen zero bytes is used (per the RFC).
//
// Parameters:
//   - salt: optional salt value; pass nil or empty for no salt
//   - ikm: input keying material
//   - algorithm: which hash function to use (SHA256 or SHA512)
//
// Returns the pseudorandom key (PRK), exactly HashLen bytes.
//
// Example:
//
//	prk := hkdf.Extract([]byte{0x00, 0x01, ...}, ikm, hkdf.SHA256)
func Extract(salt, ikm []byte, algorithm HashAlgorithm) []byte {
	// RFC 5869 Section 2.2: "if not provided, [salt] is set to a string
	// of HashLen zeros."
	//
	// When salt is empty, we create a slice of HashLen zero bytes.
	// This ensures a deterministic, well-defined PRK even when the
	// caller omits the salt.
	if len(salt) == 0 {
		salt = make([]byte, algorithm.hashLen())
	}

	// Note: salt is the HMAC *key*, IKM is the *message*.
	// This follows RFC 5869 exactly.
	return algorithm.hmacFn(salt, ikm)
}

// ─── Expand ──────────────────────────────────────────────────────────────────

// Expand performs HKDF-Expand: deriving output keying material from a PRK.
//
// Implements RFC 5869 Section 2.3:
//
//	N = ceil(L / HashLen)
//	T(0) = empty
//	T(i) = HMAC-Hash(PRK, T(i-1) || info || i)   for i = 1..N
//	OKM  = first L bytes of T(1) || ... || T(N)
//
// Parameters:
//   - prk: pseudorandom key (typically from Extract)
//   - info: context string binding the derived key to its purpose
//   - length: desired output length in bytes (1..255*HashLen)
//   - algorithm: which hash function to use
//
// Returns the output keying material (OKM), exactly length bytes.
// Returns an error if length is out of range.
//
// Example:
//
//	okm, err := hkdf.Expand(prk, []byte("tls13 derived"), 32, hkdf.SHA256)
func Expand(prk, info []byte, length int, algorithm HashAlgorithm) ([]byte, error) {
	hashLen := algorithm.hashLen()
	maxLength := 255 * hashLen

	// Validate output length.
	if length <= 0 {
		return nil, ErrOutputTooShort
	}
	if length > maxLength {
		return nil, fmt.Errorf(
			"%w: requested %d, maximum %d (255 * %d)",
			ErrOutputTooLong, length, maxLength, hashLen,
		)
	}

	// Number of HMAC blocks needed: ceil(length / hashLen).
	n := (length + hashLen - 1) / hashLen

	// Build OKM block by block.
	//
	// Each block T(i) = HMAC-Hash(PRK, T(i-1) || info || counter_byte)
	// where PRK is the HMAC key and the concatenation is the HMAC message.
	// T(0) is the empty slice.
	okm := make([]byte, 0, n*hashLen)
	var tPrev []byte // T(0) = empty

	for i := 1; i <= n; i++ {
		// Build the HMAC message: T(i-1) || info || counter_byte
		//
		// We allocate a fresh buffer each iteration to avoid mutation bugs.
		// The counter is a single byte, 1-indexed. Since n <= 255 and i >= 1,
		// the cast to byte is always safe.
		msgLen := len(tPrev) + len(info) + 1
		message := make([]byte, 0, msgLen)
		message = append(message, tPrev...)
		message = append(message, info...)
		message = append(message, byte(i))

		tPrev = algorithm.hmacFn(prk, message)
		okm = append(okm, tPrev...)
	}

	// Truncate to exactly the requested length.
	return okm[:length], nil
}

// ─── Combined: Extract-then-Expand ──────────────────────────────────────────

// HKDF performs the standard extract-then-expand key derivation (RFC 5869 Section 2):
//
//	OKM = HKDF-Expand(HKDF-Extract(salt, IKM), info, L)
//
// Parameters:
//   - salt: optional salt; pass nil or empty for no salt
//   - ikm: input keying material
//   - info: context string
//   - length: desired output length in bytes
//   - algorithm: SHA256 or SHA512
//
// Returns the output keying material (OKM), exactly length bytes.
// Returns an error if the output length is invalid.
//
// Example:
//
//	// RFC 5869 Test Case 1
//	ikm, _ := hex.DecodeString("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
//	salt, _ := hex.DecodeString("000102030405060708090a0b0c")
//	info, _ := hex.DecodeString("f0f1f2f3f4f5f6f7f8f9")
//	okm, err := hkdf.HKDF(salt, ikm, info, 42, hkdf.SHA256)
func HKDF(salt, ikm, info []byte, length int, algorithm HashAlgorithm) ([]byte, error) {
	prk := Extract(salt, ikm, algorithm)
	return Expand(prk, info, length, algorithm)
}
