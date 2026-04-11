// Package pbkdf2 implements PBKDF2 (Password-Based Key Derivation Function 2)
// as defined in RFC 8018 (formerly RFC 2898) and PKCS#5 v2.1.
//
// # What Is PBKDF2?
//
// PBKDF2 derives a cryptographic key from a password by applying a pseudorandom
// function (PRF) — typically HMAC — many thousands of times. The iteration count
// is the computational cost: every password guess during a brute-force attack
// requires running the same number of iterations.
//
// Real-world deployments:
//   - WPA2 Wi-Fi — PBKDF2-HMAC-SHA1, 4096 iterations
//   - Django password hasher — PBKDF2-HMAC-SHA256, 720,000 iterations (2024)
//   - macOS Keychain — PBKDF2-HMAC-SHA256
//   - LUKS disk encryption — PBKDF2 with configurable hash
//
// # Algorithm (RFC 8018 § 5.2)
//
//	DK = T_1 || T_2 || ... || T_ceil(dkLen/hLen)    (first dkLen bytes)
//
//	T_i = U_1 XOR U_2 XOR ... XOR U_c
//
//	U_1   = PRF(Password, Salt || INT_32_BE(i))
//	U_j   = PRF(Password, U_{j-1})   for j = 2..c
//
// The block index i is encoded as a 4-byte big-endian integer appended to the
// salt. This ensures each block's seed is unique even when the salt is reused.
//
// # Security Notes
//
// OWASP 2023 minimum iteration counts:
//   - HMAC-SHA256: 600,000
//   - HMAC-SHA1:   1,300,000
//
// For new systems, prefer Argon2id (memory-hard) when available.
package pbkdf2

import (
	"encoding/binary"
	"errors"
	"fmt"

	hmacpkg "github.com/adhithyan15/coding-adventures/code/packages/go/hmac"
)

// ErrEmptyPassword is returned when an empty password is supplied.
// An empty password provides no entropy and is almost certainly a bug.
var ErrEmptyPassword = errors.New("pbkdf2: password must not be empty")

// ErrInvalidIterations is returned when iterations is zero or negative.
var ErrInvalidIterations = errors.New("pbkdf2: iterations must be positive")

// ErrInvalidKeyLength is returned when key_length is zero or negative.
var ErrInvalidKeyLength = errors.New("pbkdf2: key_length must be positive")

// prf is the type of a pseudorandom function: PRF(key, message) → ([]byte, error).
// In PBKDF2, the password is the key and the iterated data is the message.
// Returning an error allows the PRF to propagate HMAC failures rather than
// silently returning a zero-value byte slice.
type prf func(key, message []byte) ([]byte, error)

// pbkdf2Core is the generic PBKDF2 loop used by all public functions.
//
// Parameters:
//   - fn:         PRF(key, msg) → ([]byte, error), output length = hLen
//   - hLen:       output byte length of fn (20 for SHA-1, 32 for SHA-256, …)
//   - password:   secret being stretched — becomes the HMAC key
//   - salt:       unique random value per credential (≥16 bytes recommended)
//   - iterations: number of PRF calls per block
//   - keyLength:  number of derived bytes to return
func pbkdf2Core(fn prf, hLen int, password, salt []byte, iterations, keyLength int) ([]byte, error) {
	if len(password) == 0 {
		return nil, ErrEmptyPassword
	}
	if iterations <= 0 {
		return nil, ErrInvalidIterations
	}
	if keyLength <= 0 {
		return nil, ErrInvalidKeyLength
	}

	// How many hLen-sized blocks are needed?
	numBlocks := (keyLength + hLen - 1) / hLen
	dk := make([]byte, 0, numBlocks*hLen)

	// blockIdx is reused across blocks to avoid allocations.
	blockIdx := make([]byte, 4)

	for i := 1; i <= numBlocks; i++ {
		// Encode block number as big-endian uint32 and append to salt.
		binary.BigEndian.PutUint32(blockIdx, uint32(i))
		seed := append(append([]byte(nil), salt...), blockIdx...)

		// U_1 = PRF(Password, Salt || INT_32_BE(i))
		u, err := fn(password, seed)
		if err != nil {
			return nil, fmt.Errorf("pbkdf2: PRF failed on block %d: %w", i, err)
		}

		// t accumulates the XOR of all U values.
		t := make([]byte, hLen)
		copy(t, u)

		// U_j = PRF(Password, U_{j-1})  for j = 2..c
		for j := 1; j < iterations; j++ {
			u, err = fn(password, u)
			if err != nil {
				return nil, fmt.Errorf("pbkdf2: PRF failed on block %d iteration %d: %w", i, j+1, err)
			}
			for k := range t {
				t[k] ^= u[k]
			}
		}

		dk = append(dk, t...)
	}

	return dk[:keyLength], nil
}

// PBKDF2HmacSHA1 derives a key using PBKDF2 with HMAC-SHA1 as the PRF.
//
// HMAC-SHA1 is used in WPA2 (4096 iterations) and older PKCS#12 files.
// For new systems prefer PBKDF2HmacSHA256 or Argon2id.
//
// hLen = 20 (160-bit SHA-1 output).
//
// RFC 6070 test vector:
//
//	PBKDF2HmacSHA1([]byte("password"), []byte("salt"), 1, 20)
//	→ 0c60c80f961f0e71f3a9b524af6012062fe037a6
func PBKDF2HmacSHA1(password, salt []byte, iterations, keyLength int) ([]byte, error) {
	fn := func(key, msg []byte) ([]byte, error) {
		return hmacpkg.HmacSHA1(key, msg)
	}
	return pbkdf2Core(fn, 20, password, salt, iterations, keyLength)
}

// PBKDF2HmacSHA256 derives a key using PBKDF2 with HMAC-SHA256 as the PRF.
//
// Recommended for new systems (OWASP 2023). Use at least 600,000 iterations.
//
// hLen = 32 (256-bit SHA-256 output).
//
// RFC 7914 Appendix B test vector:
//
//	PBKDF2HmacSHA256([]byte("passwd"), []byte("salt"), 1, 64)
//	→ 55ac046e56e3089fec1691c22544b605...
func PBKDF2HmacSHA256(password, salt []byte, iterations, keyLength int) ([]byte, error) {
	fn := func(key, msg []byte) ([]byte, error) {
		return hmacpkg.HmacSHA256(key, msg)
	}
	return pbkdf2Core(fn, 32, password, salt, iterations, keyLength)
}

// PBKDF2HmacSHA512 derives a key using PBKDF2 with HMAC-SHA512 as the PRF.
//
// Suitable for high-security applications. hLen = 64 (512-bit output).
func PBKDF2HmacSHA512(password, salt []byte, iterations, keyLength int) ([]byte, error) {
	fn := func(key, msg []byte) ([]byte, error) {
		return hmacpkg.HmacSHA512(key, msg)
	}
	return pbkdf2Core(fn, 64, password, salt, iterations, keyLength)
}

// PBKDF2HmacSHA1Hex is like PBKDF2HmacSHA1 but returns a lowercase hex string.
func PBKDF2HmacSHA1Hex(password, salt []byte, iterations, keyLength int) (string, error) {
	dk, err := PBKDF2HmacSHA1(password, salt, iterations, keyLength)
	if err != nil {
		return "", err
	}
	return fmt.Sprintf("%x", dk), nil
}

// PBKDF2HmacSHA256Hex is like PBKDF2HmacSHA256 but returns a lowercase hex string.
func PBKDF2HmacSHA256Hex(password, salt []byte, iterations, keyLength int) (string, error) {
	dk, err := PBKDF2HmacSHA256(password, salt, iterations, keyLength)
	if err != nil {
		return "", err
	}
	return fmt.Sprintf("%x", dk), nil
}

// PBKDF2HmacSHA512Hex is like PBKDF2HmacSHA512 but returns a lowercase hex string.
func PBKDF2HmacSHA512Hex(password, salt []byte, iterations, keyLength int) (string, error) {
	dk, err := PBKDF2HmacSHA512(password, salt, iterations, keyLength)
	if err != nil {
		return "", err
	}
	return fmt.Sprintf("%x", dk), nil
}
