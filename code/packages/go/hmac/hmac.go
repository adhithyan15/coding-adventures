// Package hmac provides HMAC (Hash-based Message Authentication Code).
//
// # What Is HMAC?
//
// HMAC (RFC 2104 / FIPS 198-1) takes a secret key and a message and produces
// a fixed-size authentication tag that proves:
//   - Integrity — the message has not been altered
//   - Authenticity — the sender knows the secret key
//
// HMAC is used in TLS 1.2 PRF, JWT (HS256, HS512), WPA2 authentication,
// TOTP/HOTP one-time passwords, and AWS Signature Version 4.
//
// # Why Not hash(key || message)?
//
// Naively prepending the key is vulnerable to length extension attacks.
// Merkle-Damgård hashes (MD5, SHA-1, SHA-256, SHA-512) produce a digest
// equal to the internal state after the last block. An attacker who knows
// hash(key || message) can compute hash(key || message || padding || extra)
// without knowing key.
//
// HMAC defeats this with two nested calls under different padded keys:
//
//	HMAC(K, M) = H((K' XOR opad) || H((K' XOR ipad) || M))
//
// where ipad = 0x36 repeated and opad = 0x5C repeated to the block size.
//
// # The Algorithm (RFC 2104 §2)
//
//  1. Normalize key to block_size bytes:
//     len(key) > block_size → K' = H(key), then zero-pad to block_size
//     len(key) ≤ block_size → zero-pad to block_size
//
//  2. Derive padded keys:
//     inner_key = K' XOR (0x36 * block_size)
//     outer_key = K' XOR (0x5C * block_size)
//
//  3. Nested hashes:
//     inner = H(inner_key || message)
//     return H(outer_key || inner)
//
// # Block Sizes
//
//	MD5, SHA-1, SHA-256: 64-byte block
//	SHA-512:             128-byte block
//
// # RFC 4231 Test Vector (TC1, HMAC-SHA256)
//
//	HmacSHA256(bytes.Repeat([]byte{0x0b}, 20), []byte("Hi There"))
//	// b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7
package hmac

import (
	"encoding/hex"

	gmd5 "github.com/adhithyan15/coding-adventures/code/packages/go/md5"
	gsha1 "github.com/adhithyan15/coding-adventures/code/packages/go/sha1"
	gsha256 "github.com/adhithyan15/coding-adventures/code/packages/go/sha256"
	gsha512 "github.com/adhithyan15/coding-adventures/code/packages/go/sha512"
)

// HashFn is the type of a hash function: takes bytes, returns bytes.
type HashFn func(data []byte) []byte

// HMAC computes an HMAC tag using any hash function.
//
// Parameters:
//   - hashFn:    a function []byte -> []byte (e.g. sha256 one-shot hash)
//   - blockSize: internal block size of hashFn in bytes (64 or 128)
//   - key:       secret key, any length
//   - message:   data to authenticate, any length
//
// Returns the authentication tag as a byte slice.
func HMAC(hashFn HashFn, blockSize int, key, message []byte) []byte {
	// Step 1 — normalize key to exactly blockSize bytes
	keyPrime := normalizeKey(hashFn, blockSize, key)

	// Step 2 — derive inner and outer padded keys
	innerKey := xorBytes(keyPrime, 0x36)
	outerKey := xorBytes(keyPrime, 0x5C)

	// Step 3 — nested hashes
	inner := hashFn(append(innerKey, message...))
	return hashFn(append(outerKey, inner...))
}

// ===========================================================================
// Named variants
// ===========================================================================

// HmacMD5 returns a 16-byte HMAC-MD5 authentication tag.
//
// HMAC-MD5 remains secure as a MAC even though MD5 is collision-broken.
// It appears in legacy TLS cipher suites and some older protocols.
//
//	HmacMD5([]byte("Jefe"), []byte("what do ya want for nothing?"))
//	// 750c783e6ab0b503eaa86e310a5db738
func HmacMD5(key, message []byte) []byte {
	return HMAC(func(d []byte) []byte {
		s := gmd5.SumMD5(d)
		return s[:]
	}, 64, key, message)
}

// HmacSHA1 returns a 20-byte HMAC-SHA1 authentication tag.
//
// Used in WPA2 (PBKDF2-HMAC-SHA1), older TLS/SSH, and TOTP/HOTP.
//
//	HmacSHA1([]byte("Jefe"), []byte("what do ya want for nothing?"))
//	// effcdf6ae5eb2fa2d27416d5f184df9c259a7c79
func HmacSHA1(key, message []byte) []byte {
	return HMAC(func(d []byte) []byte {
		s := gsha1.Sum1(d)
		return s[:]
	}, 64, key, message)
}

// HmacSHA256 returns a 32-byte HMAC-SHA256 authentication tag.
//
// The modern default for TLS 1.3, JWT HS256, AWS Signature V4,
// and PBKDF2-HMAC-SHA256.
//
//	key := bytes.Repeat([]byte{0x0b}, 20)
//	HmacSHA256(key, []byte("Hi There"))
//	// b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7
func HmacSHA256(key, message []byte) []byte {
	return HMAC(func(d []byte) []byte {
		s := gsha256.Sum256(d)
		return s[:]
	}, 64, key, message)
}

// HmacSHA512 returns a 64-byte HMAC-SHA512 authentication tag.
//
// Used in JWT HS512 and high-security configurations.
// Note: SHA-512 has a 128-byte block size, so key normalization
// and ipad/opad use 128 bytes.
//
//	key := bytes.Repeat([]byte{0x0b}, 20)
//	HmacSHA512(key, []byte("Hi There"))
//	// 87aa7cdea5ef619d4ff0b4241a1d6cb0...
func HmacSHA512(key, message []byte) []byte {
	return HMAC(func(d []byte) []byte {
		s := gsha512.Sum512(d)
		return s[:]
	}, 128, key, message)
}

// ===========================================================================
// Hex-string variants
// ===========================================================================

// HmacMD5Hex returns the HMAC-MD5 tag as a 32-character lowercase hex string.
func HmacMD5Hex(key, message []byte) string {
	return hex.EncodeToString(HmacMD5(key, message))
}

// HmacSHA1Hex returns the HMAC-SHA1 tag as a 40-character lowercase hex string.
func HmacSHA1Hex(key, message []byte) string {
	return hex.EncodeToString(HmacSHA1(key, message))
}

// HmacSHA256Hex returns the HMAC-SHA256 tag as a 64-character lowercase hex string.
func HmacSHA256Hex(key, message []byte) string {
	return hex.EncodeToString(HmacSHA256(key, message))
}

// HmacSHA512Hex returns the HMAC-SHA512 tag as a 128-character lowercase hex string.
func HmacSHA512Hex(key, message []byte) string {
	return hex.EncodeToString(HmacSHA512(key, message))
}

// ===========================================================================
// Private helpers
// ===========================================================================

// normalizeKey brings key to exactly blockSize bytes.
// If len(key) > blockSize, key is hashed first.
// The result is always zero-padded on the right to blockSize.
func normalizeKey(hashFn HashFn, blockSize int, key []byte) []byte {
	if len(key) > blockSize {
		key = hashFn(key)
	}
	// Zero-pad to blockSize
	result := make([]byte, blockSize)
	copy(result, key)
	return result
}

// xorBytes XORs every byte in data with constant c.
// Used to derive the ipad-key (c=0x36) and opad-key (c=0x5C).
func xorBytes(data []byte, c byte) []byte {
	out := make([]byte, len(data))
	for i, b := range data {
		out[i] = b ^ c
	}
	return out
}
