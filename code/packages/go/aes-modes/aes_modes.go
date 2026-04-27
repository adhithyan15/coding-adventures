// Package aesmodes implements AES modes of operation: ECB, CBC, CTR, and GCM.
//
// # Why Modes of Operation?
//
// AES is a block cipher that encrypts exactly 16 bytes at a time. Real messages
// are rarely exactly 16 bytes, so a "mode of operation" defines how to use the
// block cipher for arbitrary-length messages. The choice of mode critically
// affects security:
//
//   - ECB: INSECURE. Each block encrypted independently. Patterns leak through.
//   - CBC: Legacy. Chains blocks via XOR. Vulnerable to padding oracle attacks.
//   - CTR: Recommended. Stream cipher mode. No padding. Parallelizable.
//   - GCM: Best. CTR + GHASH authentication. Detects tampering. TLS 1.3 standard.
//
// # Dependencies
//
// This package wraps the AES block cipher from the sibling aes package, which
// provides EncryptBlock and DecryptBlock operating on 16-byte blocks.
//
// # Security Warning
//
// This is an educational implementation. It prioritizes clarity over performance
// and side-channel resistance. Do not use in production — use crypto/aes and
// crypto/cipher from the Go standard library instead.
package aesmodes

import (
	"encoding/binary"
	"errors"
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/aes"
)

const blockSize = 16

// =============================================================================
// PKCS#7 Padding
// =============================================================================
//
// Block ciphers in ECB and CBC mode require plaintext to be an exact multiple
// of the block size (16 bytes). PKCS#7 padding appends N bytes of value N,
// where N = blockSize - (len(data) % blockSize).
//
// Even if data is already aligned, a full block of padding (16 bytes of 0x10)
// is added. This ensures unpadding is always unambiguous.

// pkcs7Pad pads data to a multiple of 16 bytes using PKCS#7.
func pkcs7Pad(data []byte) []byte {
	padLen := blockSize - (len(data) % blockSize)
	padded := make([]byte, len(data)+padLen)
	copy(padded, data)
	for i := len(data); i < len(padded); i++ {
		padded[i] = byte(padLen)
	}
	return padded
}

// pkcs7Unpad removes PKCS#7 padding and validates it.
func pkcs7Unpad(data []byte) ([]byte, error) {
	if len(data) == 0 || len(data)%blockSize != 0 {
		return nil, fmt.Errorf("data length %d is not a positive multiple of %d", len(data), blockSize)
	}

	padLen := int(data[len(data)-1])
	if padLen < 1 || padLen > blockSize {
		return nil, errors.New("invalid PKCS#7 padding")
	}

	// Constant-time padding validation: accumulate differences with OR
	// so the loop always takes the same time regardless of which byte fails.
	var diff byte
	for i := 1; i <= padLen; i++ {
		diff |= data[len(data)-i] ^ byte(padLen)
	}
	if diff != 0 {
		return nil, errors.New("invalid PKCS#7 padding")
	}

	return data[:len(data)-padLen], nil
}

// xorBytes XORs two byte slices of equal length.
func xorBytes(a, b []byte) []byte {
	result := make([]byte, len(a))
	for i := range a {
		result[i] = a[i] ^ b[i]
	}
	return result
}

// =============================================================================
// ECB Mode (Electronic Codebook) — INSECURE
// =============================================================================
//
// Each 16-byte block is encrypted independently. Identical plaintext blocks
// produce identical ciphertext blocks, revealing patterns in the data.
// The famous "ECB penguin" demonstrates this clearly.
//
// ECB is included here as an anti-pattern. Never use it for real encryption.

// EncryptECB encrypts plaintext using AES in ECB mode (INSECURE — educational only).
//
// Pads with PKCS#7, then encrypts each 16-byte block independently.
func EncryptECB(plaintext, key []byte) ([]byte, error) {
	padded := pkcs7Pad(plaintext)
	ciphertext := make([]byte, 0, len(padded))

	for i := 0; i < len(padded); i += blockSize {
		block := padded[i : i+blockSize]
		encrypted, err := aes.EncryptBlock(block, key)
		if err != nil {
			return nil, fmt.Errorf("ECB encrypt block %d: %w", i/blockSize, err)
		}
		ciphertext = append(ciphertext, encrypted...)
	}

	return ciphertext, nil
}

// DecryptECB decrypts ciphertext that was encrypted with AES-ECB.
func DecryptECB(ciphertext, key []byte) ([]byte, error) {
	if len(ciphertext) == 0 || len(ciphertext)%blockSize != 0 {
		return nil, fmt.Errorf("ciphertext length %d is not a positive multiple of %d", len(ciphertext), blockSize)
	}

	plaintext := make([]byte, 0, len(ciphertext))

	for i := 0; i < len(ciphertext); i += blockSize {
		block := ciphertext[i : i+blockSize]
		decrypted, err := aes.DecryptBlock(block, key)
		if err != nil {
			return nil, fmt.Errorf("ECB decrypt block %d: %w", i/blockSize, err)
		}
		plaintext = append(plaintext, decrypted...)
	}

	return pkcs7Unpad(plaintext)
}

// =============================================================================
// CBC Mode (Cipher Block Chaining) — Legacy
// =============================================================================
//
// Chains blocks by XOR-ing each plaintext block with the previous ciphertext
// block before encryption. A random IV starts the chain.
//
//   C[0] = AES_encrypt(P[0] XOR IV, key)
//   C[i] = AES_encrypt(P[i] XOR C[i-1], key)
//
// Vulnerable to padding oracle attacks (POODLE, Lucky 13).

// EncryptCBC encrypts plaintext using AES in CBC mode.
// The IV must be exactly 16 bytes and should be random for each message.
func EncryptCBC(plaintext, key, iv []byte) ([]byte, error) {
	if len(iv) != blockSize {
		return nil, fmt.Errorf("IV must be %d bytes, got %d", blockSize, len(iv))
	}

	padded := pkcs7Pad(plaintext)
	ciphertext := make([]byte, 0, len(padded))
	prev := iv

	for i := 0; i < len(padded); i += blockSize {
		block := padded[i : i+blockSize]
		xored := xorBytes(block, prev)
		encrypted, err := aes.EncryptBlock(xored, key)
		if err != nil {
			return nil, fmt.Errorf("CBC encrypt block %d: %w", i/blockSize, err)
		}
		ciphertext = append(ciphertext, encrypted...)
		prev = encrypted
	}

	return ciphertext, nil
}

// DecryptCBC decrypts ciphertext that was encrypted with AES-CBC.
func DecryptCBC(ciphertext, key, iv []byte) ([]byte, error) {
	if len(iv) != blockSize {
		return nil, fmt.Errorf("IV must be %d bytes, got %d", blockSize, len(iv))
	}
	if len(ciphertext) == 0 || len(ciphertext)%blockSize != 0 {
		return nil, fmt.Errorf("ciphertext length %d is not a positive multiple of %d", len(ciphertext), blockSize)
	}

	plaintext := make([]byte, 0, len(ciphertext))
	prev := iv

	for i := 0; i < len(ciphertext); i += blockSize {
		block := ciphertext[i : i+blockSize]
		decrypted, err := aes.DecryptBlock(block, key)
		if err != nil {
			return nil, fmt.Errorf("CBC decrypt block %d: %w", i/blockSize, err)
		}
		plaintext = append(plaintext, xorBytes(decrypted, prev)...)
		prev = block
	}

	return pkcs7Unpad(plaintext)
}

// =============================================================================
// CTR Mode (Counter Mode) — Recommended
// =============================================================================
//
// Turns the block cipher into a stream cipher by encrypting a counter and
// XOR-ing the keystream with the plaintext:
//
//   keystream[i] = AES_encrypt(nonce || counter_i, key)
//   ciphertext[i] = plaintext[i] XOR keystream[i]
//
// The counter block is: [12-byte nonce] [4-byte big-endian counter].
// Counter starts at 1 (GCM reserves counter 0 for the tag).
//
// No padding needed. Encryption = decryption (XOR is self-inverse).
// CRITICAL: Never reuse a nonce with the same key.

// buildCounterBlock builds a 16-byte block: 12-byte nonce || 4-byte big-endian counter.
func buildCounterBlock(nonce []byte, counter uint32) []byte {
	block := make([]byte, 16)
	copy(block[:12], nonce)
	binary.BigEndian.PutUint32(block[12:], counter)
	return block
}

// EncryptCTR encrypts plaintext using AES in CTR mode.
// The nonce must be exactly 12 bytes and MUST be unique per message.
func EncryptCTR(plaintext, key, nonce []byte) ([]byte, error) {
	if len(nonce) != 12 {
		return nil, fmt.Errorf("nonce must be 12 bytes, got %d", len(nonce))
	}

	ciphertext := make([]byte, 0, len(plaintext))
	var counter uint32 = 1

	for i := 0; i < len(plaintext); i += blockSize {
		counterBlock := buildCounterBlock(nonce, counter)
		keystream, err := aes.EncryptBlock(counterBlock, key)
		if err != nil {
			return nil, fmt.Errorf("CTR encrypt counter %d: %w", counter, err)
		}

		end := i + blockSize
		if end > len(plaintext) {
			end = len(plaintext)
		}
		chunk := plaintext[i:end]
		ciphertext = append(ciphertext, xorBytes(keystream[:len(chunk)], chunk)...)
		counter++
	}

	return ciphertext, nil
}

// DecryptCTR decrypts ciphertext encrypted with AES-CTR.
// Identical to EncryptCTR because XOR is its own inverse.
func DecryptCTR(ciphertext, key, nonce []byte) ([]byte, error) {
	return EncryptCTR(ciphertext, key, nonce)
}

// =============================================================================
// GCM Mode (Galois/Counter Mode) — Recommended with Authentication
// =============================================================================
//
// GCM combines CTR encryption with GHASH authentication:
//
//   H = AES_encrypt(0^128, key)         — hash subkey
//   J0 = IV || 0x00000001               — initial counter
//   Ciphertext = CTR(plaintext, counter=2)
//   Tag = GHASH(H, AAD, CT) XOR AES(J0)
//
// GHASH uses multiplication in GF(2^128) with reducing polynomial:
//   R(x) = x^128 + x^7 + x^2 + x + 1  (0xE1 << 120 in reflected form)

// gf128Mul multiplies two 128-bit elements in GF(2^128) with the GCM polynomial.
//
// Uses the "shift-and-add" algorithm in the reflected bit convention:
//   - Process bits of Y from MSB to LSB
//   - If bit is 1, XOR current V into result Z
//   - Right-shift V; if carry, XOR with reducing polynomial R = 0xE1 << 120
//
// The inputs and output are 16-byte big-endian representations.
func gf128Mul(x, y []byte) []byte {
	// Convert to two 64-bit halves for efficient computation
	xHi := binary.BigEndian.Uint64(x[:8])
	xLo := binary.BigEndian.Uint64(x[8:])
	yHi := binary.BigEndian.Uint64(y[:8])
	yLo := binary.BigEndian.Uint64(y[8:])

	// Reducing polynomial R = 0xE100000000000000 (high 64 bits)
	const rHi uint64 = 0xE100000000000000

	// Working copies
	vHi, vLo := xHi, xLo
	var zHi, zLo uint64

	// Process all 128 bits of Y
	for i := 0; i < 128; i++ {
		// Check bit i of Y (MSB first)
		var yBit uint64
		if i < 64 {
			yBit = (yHi >> (63 - uint(i))) & 1
		} else {
			yBit = (yLo >> (63 - uint(i-64))) & 1
		}

		if yBit == 1 {
			zHi ^= vHi
			zLo ^= vLo
		}

		// Check if LSB of the 128-bit V is set (the LSB of vLo)
		carry := vLo & 1

		// Right-shift the 128-bit V by 1
		vLo = (vLo >> 1) | ((vHi & 1) << 63)
		vHi >>= 1

		// If there was a carry from the LSB, XOR with reducing polynomial
		if carry == 1 {
			vHi ^= rHi
		}
	}

	result := make([]byte, 16)
	binary.BigEndian.PutUint64(result[:8], zHi)
	binary.BigEndian.PutUint64(result[8:], zLo)
	return result
}

// ghash computes GHASH over the concatenated data using hash subkey H.
func ghash(h, data []byte) []byte {
	y := make([]byte, 16)

	for i := 0; i < len(data); i += 16 {
		end := i + 16
		block := make([]byte, 16)
		if end <= len(data) {
			copy(block, data[i:end])
		} else {
			copy(block, data[i:])
		}
		y = gf128Mul(xorBytes(y, block), h)
	}

	return y
}

// padTo16 zero-pads data to a multiple of 16 bytes.
func padTo16(data []byte) []byte {
	remainder := len(data) % 16
	if remainder == 0 {
		return data
	}
	padded := make([]byte, len(data)+16-remainder)
	copy(padded, data)
	return padded
}

// EncryptGCM encrypts and authenticates using AES-GCM.
//
// Returns (ciphertext, 16-byte tag). The IV must be exactly 12 bytes
// and MUST be unique per message with the same key.
func EncryptGCM(plaintext, key, iv, aad []byte) ([]byte, []byte, error) {
	if len(iv) != 12 {
		return nil, nil, fmt.Errorf("IV must be 12 bytes, got %d", len(iv))
	}

	// Step 1: Hash subkey H = AES(0^128, key)
	zeroBlock := make([]byte, 16)
	h, err := aes.EncryptBlock(zeroBlock, key)
	if err != nil {
		return nil, nil, fmt.Errorf("GCM compute H: %w", err)
	}

	// Step 2: Initial counter J0 = IV || 0x00000001
	j0 := buildCounterBlock(iv, 1)

	// Step 3: CTR encryption starting at counter=2
	ciphertext := make([]byte, 0, len(plaintext))
	var counter uint32 = 2
	for i := 0; i < len(plaintext); i += blockSize {
		counterBlock := buildCounterBlock(iv, counter)
		keystream, encErr := aes.EncryptBlock(counterBlock, key)
		if encErr != nil {
			return nil, nil, fmt.Errorf("GCM CTR counter %d: %w", counter, encErr)
		}
		end := i + blockSize
		if end > len(plaintext) {
			end = len(plaintext)
		}
		chunk := plaintext[i:end]
		ciphertext = append(ciphertext, xorBytes(keystream[:len(chunk)], chunk)...)
		counter++
	}

	// Step 4: Compute authentication tag
	// GHASH input: pad(AAD) || pad(CT) || len_aad_bits || len_ct_bits
	lenBlock := make([]byte, 16)
	binary.BigEndian.PutUint64(lenBlock[:8], uint64(len(aad)*8))
	binary.BigEndian.PutUint64(lenBlock[8:], uint64(len(ciphertext)*8))

	ghashInput := append(padTo16(aad), padTo16(ciphertext)...)
	ghashInput = append(ghashInput, lenBlock...)
	s := ghash(h, ghashInput)

	// Tag = GHASH_result XOR AES(J0, key)
	encJ0, err := aes.EncryptBlock(j0, key)
	if err != nil {
		return nil, nil, fmt.Errorf("GCM encrypt J0: %w", err)
	}
	tag := xorBytes(s, encJ0)

	return ciphertext, tag, nil
}

// DecryptGCM decrypts and verifies using AES-GCM.
//
// Returns the plaintext if the tag is valid. Returns an error if the tag
// does not match (indicating the ciphertext was tampered with).
func DecryptGCM(ciphertext, key, iv, aad, tag []byte) ([]byte, error) {
	if len(iv) != 12 {
		return nil, fmt.Errorf("IV must be 12 bytes, got %d", len(iv))
	}
	if len(tag) != 16 {
		return nil, fmt.Errorf("tag must be 16 bytes, got %d", len(tag))
	}

	// Recompute hash subkey and expected tag
	zeroBlock := make([]byte, 16)
	h, err := aes.EncryptBlock(zeroBlock, key)
	if err != nil {
		return nil, fmt.Errorf("GCM compute H: %w", err)
	}

	j0 := buildCounterBlock(iv, 1)

	// Compute expected tag
	lenBlock := make([]byte, 16)
	binary.BigEndian.PutUint64(lenBlock[:8], uint64(len(aad)*8))
	binary.BigEndian.PutUint64(lenBlock[8:], uint64(len(ciphertext)*8))

	ghashInput := append(padTo16(aad), padTo16(ciphertext)...)
	ghashInput = append(ghashInput, lenBlock...)
	s := ghash(h, ghashInput)

	encJ0, err := aes.EncryptBlock(j0, key)
	if err != nil {
		return nil, fmt.Errorf("GCM encrypt J0: %w", err)
	}
	expectedTag := xorBytes(s, encJ0)

	// Constant-time tag verification: OR together all byte differences so an
	// attacker cannot tell which byte (if any) differs from the timing.
	var diff byte
	for i := range tag {
		diff |= tag[i] ^ expectedTag[i]
	}
	if diff != 0 {
		return nil, errors.New("authentication tag mismatch — ciphertext may have been tampered with")
	}

	// Decrypt using CTR starting at counter=2
	plaintext := make([]byte, 0, len(ciphertext))
	var counter uint32 = 2
	for i := 0; i < len(ciphertext); i += blockSize {
		counterBlock := buildCounterBlock(iv, counter)
		keystream, encErr := aes.EncryptBlock(counterBlock, key)
		if encErr != nil {
			return nil, fmt.Errorf("GCM CTR decrypt counter %d: %w", counter, encErr)
		}
		end := i + blockSize
		if end > len(ciphertext) {
			end = len(ciphertext)
		}
		chunk := ciphertext[i:end]
		plaintext = append(plaintext, xorBytes(keystream[:len(chunk)], chunk)...)
		counter++
	}

	return plaintext, nil
}
