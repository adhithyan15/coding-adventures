// Package des implements the DES and Triple-DES (3DES/TDEA) block ciphers.
//
// # Historical Context
//
// DES (Data Encryption Standard) was published by NIST in 1977 and was the
// world's first openly standardized encryption algorithm. Its 56-bit key is
// completely broken by modern hardware -- a full key search takes under 24
// hours on consumer hardware, and specialized ASICs can crack it in seconds.
//
// This package is for educational use only. It implements:
//   - Single DES (EncryptBlock / DecryptBlock)
//   - ECB mode with PKCS#7 padding (ECBEncrypt / ECBDecrypt)
//   - Triple DES / 3TDEA (TDEAEncryptBlock / TDEADecryptBlock)
//
// Never use this to protect real data. Use AES-GCM (or ChaCha20-Poly1305)
// from the Go standard library for production encryption.
//
// # Architecture (Feistel Network)
//
//	plaintext (8 bytes)
//	     │
//	IP (initial permutation)       ← hardware alignment, no crypto value
//	     │
//	┌── 16 Feistel rounds ──────────────────────────────────────────────┐
//	│   L_i = R_{i-1}                                                   │
//	│   R_i = L_{i-1} XOR f(R_{i-1}, K_i)                             │
//	│                                                                   │
//	│   f(R, K):                                                        │
//	│     E(R)          32→48 bits (expansion permutation)              │
//	│     XOR K_i       48-bit round subkey                             │
//	│     S-boxes       8 × (6 bits → 4 bits) = 32 bits out            │
//	│     P             32→32 bit diffusion permutation                 │
//	└───────────────────────────────────────────────────────────────────┘
//	     │
//	FP (final permutation = IP⁻¹)
//	     │
//	ciphertext (8 bytes)
//
// Decryption reuses the same circuit with subkeys in reverse order (K16..K1),
// which is the self-inverse property of the Feistel network.
package des

import "fmt"

// =============================================================================
// Permutation and selection tables
//
// All DES standard tables use 1-indexed bit positions. We store them as-is
// and subtract 1 when indexing to convert to 0-based positions.
// =============================================================================

// ip is the Initial Permutation applied to the plaintext before rounds.
// Bit 58 of the input becomes bit 1 of the output, etc.
// This permutation was designed for 8-bit parallel bus loading on 1970s
// hardware; it has no cryptographic significance.
var ip = [64]int{
	58, 50, 42, 34, 26, 18, 10, 2,
	60, 52, 44, 36, 28, 20, 12, 4,
	62, 54, 46, 38, 30, 22, 14, 6,
	64, 56, 48, 40, 32, 24, 16, 8,
	57, 49, 41, 33, 25, 17, 9, 1,
	59, 51, 43, 35, 27, 19, 11, 3,
	61, 53, 45, 37, 29, 21, 13, 5,
	63, 55, 47, 39, 31, 23, 15, 7,
}

// fp is the Final Permutation (IP⁻¹). Undoes the initial permutation.
// It must satisfy: fp[ip[i]-1] = i+1 for all i.
var fp = [64]int{
	40, 8, 48, 16, 56, 24, 64, 32,
	39, 7, 47, 15, 55, 23, 63, 31,
	38, 6, 46, 14, 54, 22, 62, 30,
	37, 5, 45, 13, 53, 21, 61, 29,
	36, 4, 44, 12, 52, 20, 60, 28,
	35, 3, 43, 11, 51, 19, 59, 27,
	34, 2, 42, 10, 50, 18, 58, 26,
	33, 1, 41, 9, 49, 17, 57, 25,
}

// pc1 is Permuted Choice 1 for the key schedule.
// Drops the 8 parity bits (positions 8,16,24,32,40,48,56,64) and reorders
// the remaining 56 key bits into two 28-bit halves C and D.
var pc1 = [56]int{
	57, 49, 41, 33, 25, 17, 9,
	1, 58, 50, 42, 34, 26, 18,
	10, 2, 59, 51, 43, 35, 27,
	19, 11, 3, 60, 52, 44, 36,
	63, 55, 47, 39, 31, 23, 15,
	7, 62, 54, 46, 38, 30, 22,
	14, 6, 61, 53, 45, 37, 29,
	21, 13, 5, 28, 20, 12, 4,
}

// pc2 is Permuted Choice 2. Selects 48 of the 56 key bits to form each
// round subkey. The 8 dropped positions (9,18,22,25,35,38,43,54 in C∥D)
// are a compression step that increases the key schedule's non-linearity.
var pc2 = [48]int{
	14, 17, 11, 24, 1, 5,
	3, 28, 15, 6, 21, 10,
	23, 19, 12, 4, 26, 8,
	16, 7, 27, 20, 13, 2,
	41, 52, 31, 37, 47, 55,
	30, 40, 51, 45, 33, 48,
	44, 49, 39, 56, 34, 53,
	46, 42, 50, 36, 29, 32,
}

// eTable is the Expansion permutation. Expands the 32-bit right half to 48
// bits by copying border bits of each 4-bit group into adjacent groups.
// This expansion ensures each S-box input overlaps two 4-bit groups, giving
// every output bit of a round a chance to influence every S-box.
var eTable = [48]int{
	32, 1, 2, 3, 4, 5,
	4, 5, 6, 7, 8, 9,
	8, 9, 10, 11, 12, 13,
	12, 13, 14, 15, 16, 17,
	16, 17, 18, 19, 20, 21,
	20, 21, 22, 23, 24, 25,
	24, 25, 26, 27, 28, 29,
	28, 29, 30, 31, 32, 1,
}

// pTable is the post-S-box Permutation. Disperses the 32-bit S-box output
// so that every output bit of each S-box affects every S-box in the next
// round -- providing avalanche/diffusion.
var pTable = [32]int{
	16, 7, 20, 21, 29, 12, 28, 17,
	1, 15, 23, 26, 5, 18, 31, 10,
	2, 8, 24, 14, 32, 27, 3, 9,
	19, 13, 30, 6, 22, 11, 4, 25,
}

// shifts specifies the left-rotation amounts for the key halves C and D in
// each of the 16 rounds. Total across 16 rounds = 28 (one full rotation of
// a 28-bit register). Rounds 1,2,9,16 rotate by 1; all others by 2.
var shifts = [16]int{1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1}

// =============================================================================
// S-Boxes: the core non-linearity of DES
//
// Eight substitution boxes, each mapping 6 bits to 4 bits. Without S-boxes,
// DES would be entirely linear and breakable with Gaussian elimination over GF(2).
//
// Reading an S-box with 6 input bits b₁b₂b₃b₄b₅b₆:
//   row = 2·b₁ + b₆            (outer/border bits, range 0–3)
//   col = 8·b₂ + 4·b₃ + 2·b₄ + b₅  (inner 4 bits, range 0–15)
//   output = SBOXES[box][row*16 + col]
//
// These S-boxes were redesigned by the NSA from IBM's originals to resist
// differential cryptanalysis -- a technique the NSA had classified in 1974,
// over a decade before Biham and Shamir independently discovered it in 1990.
// =============================================================================

var sboxes = [8][64]int{
	// S1
	{
		14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7,
		0, 15, 7, 4, 14, 2, 13, 1, 10, 6, 12, 11, 9, 5, 3, 8,
		4, 1, 14, 8, 13, 6, 2, 11, 15, 12, 9, 7, 3, 10, 5, 0,
		15, 12, 8, 2, 4, 9, 1, 7, 5, 11, 3, 14, 10, 0, 6, 13,
	},
	// S2
	{
		15, 1, 8, 14, 6, 11, 3, 4, 9, 7, 2, 13, 12, 0, 5, 10,
		3, 13, 4, 7, 15, 2, 8, 14, 12, 0, 1, 10, 6, 9, 11, 5,
		0, 14, 7, 11, 10, 4, 13, 1, 5, 8, 12, 6, 9, 3, 2, 15,
		13, 8, 10, 1, 3, 15, 4, 2, 11, 6, 7, 12, 0, 5, 14, 9,
	},
	// S3
	{
		10, 0, 9, 14, 6, 3, 15, 5, 1, 13, 12, 7, 11, 4, 2, 8,
		13, 7, 0, 9, 3, 4, 6, 10, 2, 8, 5, 14, 12, 11, 15, 1,
		13, 6, 4, 9, 8, 15, 3, 0, 11, 1, 2, 12, 5, 10, 14, 7,
		1, 10, 13, 0, 6, 9, 8, 7, 4, 15, 14, 3, 11, 5, 2, 12,
	},
	// S4
	{
		7, 13, 14, 3, 0, 6, 9, 10, 1, 2, 8, 5, 11, 12, 4, 15,
		13, 8, 11, 5, 6, 15, 0, 3, 4, 7, 2, 12, 1, 10, 14, 9,
		10, 6, 9, 0, 12, 11, 7, 13, 15, 1, 3, 14, 5, 2, 8, 4,
		3, 15, 0, 6, 10, 1, 13, 8, 9, 4, 5, 11, 12, 7, 2, 14,
	},
	// S5
	{
		2, 12, 4, 1, 7, 10, 11, 6, 8, 5, 3, 15, 13, 0, 14, 9,
		14, 11, 2, 12, 4, 7, 13, 1, 5, 0, 15, 10, 3, 9, 8, 6,
		4, 2, 1, 11, 10, 13, 7, 8, 15, 9, 12, 5, 6, 3, 0, 14,
		11, 8, 12, 7, 1, 14, 2, 13, 6, 15, 0, 9, 10, 4, 5, 3,
	},
	// S6
	{
		12, 1, 10, 15, 9, 2, 6, 8, 0, 13, 3, 4, 14, 7, 5, 11,
		10, 15, 4, 2, 7, 12, 9, 5, 6, 1, 13, 14, 0, 11, 3, 8,
		9, 14, 15, 5, 2, 8, 12, 3, 7, 0, 4, 10, 1, 13, 11, 6,
		4, 3, 2, 12, 9, 5, 15, 10, 11, 14, 1, 7, 6, 0, 8, 13,
	},
	// S7
	{
		4, 11, 2, 14, 15, 0, 8, 13, 3, 12, 9, 7, 5, 10, 6, 1,
		13, 0, 11, 7, 4, 9, 1, 10, 14, 3, 5, 12, 2, 15, 8, 6,
		1, 4, 11, 13, 12, 3, 7, 14, 10, 15, 6, 8, 0, 5, 9, 2,
		6, 11, 13, 8, 1, 4, 10, 7, 9, 5, 0, 15, 14, 2, 3, 12,
	},
	// S8
	{
		13, 2, 8, 4, 6, 15, 11, 1, 10, 9, 3, 14, 5, 0, 12, 7,
		1, 15, 13, 8, 10, 3, 7, 4, 12, 5, 6, 11, 0, 14, 9, 2,
		7, 11, 4, 1, 9, 12, 14, 2, 0, 6, 10, 13, 15, 3, 5, 8,
		2, 1, 14, 7, 4, 10, 8, 13, 15, 12, 9, 0, 3, 5, 6, 11,
	},
}

// =============================================================================
// Bit manipulation helpers
// =============================================================================

// bytesToBits converts a byte slice to a slice of bits, MSB first within
// each byte. For example, byte 0xA0 = 10100000 becomes [1,0,1,0,0,0,0,0].
func bytesToBits(data []byte) []int {
	bits := make([]int, len(data)*8)
	for i, b := range data {
		for j := 0; j < 8; j++ {
			// Shift right by (7-j) to put the j-th bit (MSB=0) into position 0.
			bits[i*8+j] = int((b >> uint(7-j)) & 1)
		}
	}
	return bits
}

// bitsToBytes converts a slice of bits (MSB first) back to bytes.
// len(bits) must be a multiple of 8.
func bitsToBytes(bits []int) []byte {
	result := make([]byte, len(bits)/8)
	for i := range result {
		var b byte
		for j := 0; j < 8; j++ {
			b = (b << 1) | byte(bits[i*8+j])
		}
		result[i] = b
	}
	return result
}

// permute applies a 1-indexed permutation table to a bit slice and returns
// the permuted result. The output length equals len(table).
//
// Example: permute([1,0,1], [2,3,1]) returns [0,1,1] (bits at positions 2,3,1).
func permute(bits []int, table []int) []int {
	result := make([]int, len(table))
	for i, pos := range table {
		result[i] = bits[pos-1] // convert 1-indexed to 0-indexed
	}
	return result
}

// leftRotate cyclically shifts a bit slice left by n positions.
// Used in the key schedule to rotate the 28-bit C and D halves.
func leftRotate(half []int, n int) []int {
	l := len(half)
	result := make([]int, l)
	for i := 0; i < l; i++ {
		result[i] = half[(i+n)%l]
	}
	return result
}

// =============================================================================
// Key schedule: ExpandKey
// =============================================================================

// ExpandKey derives the 16 DES round subkeys from an 8-byte key.
//
// The DES key is 64 bits wide but only 56 bits are key material -- bits at
// positions 8, 16, 24, 32, 40, 48, 56, 64 are parity check bits and are
// dropped by PC-1. This function accepts any 8-byte key and ignores parity.
//
// Returns 16 subkeys, each 6 bytes (48 bits), in order K1..K16.
//
// Algorithm (FIPS 46-3 Section 3.3):
//
//  1. PC-1: 64-bit key → 56 bits (drop parity), split into C₀ (28) and D₀ (28)
//  2. For each round i = 1..16:
//     C_i = LeftRotate(C_{i-1}, SHIFTS[i])
//     D_i = LeftRotate(D_{i-1}, SHIFTS[i])
//     K_i = PC-2(C_i ∥ D_i)   (56 → 48 bits)
func ExpandKey(key []byte) ([][]byte, error) {
	if len(key) != 8 {
		return nil, fmt.Errorf("DES key must be exactly 8 bytes, got %d", len(key))
	}

	keyBits := bytesToBits(key)

	// PC-1: 64 bits → 56 bits, split into two 28-bit halves.
	permuted := permute(keyBits, pc1[:])
	c := permuted[:28]
	d := permuted[28:]

	subkeys := make([][]byte, 16)
	for round, shift := range shifts {
		c = leftRotate(c, shift)
		d = leftRotate(d, shift)

		// PC-2: select 48 bits from C∥D to form this round's subkey.
		cd := append(c, d...)
		subkeyBits := permute(cd, pc2[:])
		subkeys[round] = bitsToBytes(subkeyBits) // 48 bits = 6 bytes exactly
	}
	return subkeys, nil
}

// =============================================================================
// Round function f(R, K)
// =============================================================================

// feistelF computes the DES round function f(R, K):
//
//  1. E(R)   — expand 32-bit right half to 48 bits
//  2. XOR    — mix in the 48-bit round subkey
//  3. S      — 8 S-boxes, each 6 bits → 4 bits (32 bits total out)
//  4. P      — 32-bit diffusion permutation
//
// The S-boxes are the only non-linear step. Without them, every round would
// be a linear transformation over GF(2), solvable with matrix methods.
func feistelF(right []int, subkey []byte) []int {
	// Step 1: Expand R from 32 → 48 bits.
	expanded := permute(right, eTable[:])

	// Step 2: XOR with the 48-bit round subkey.
	subkeyBits := bytesToBits(subkey)
	xored := make([]int, 48)
	for i := 0; i < 48; i++ {
		xored[i] = expanded[i] ^ subkeyBits[i]
	}

	// Step 3: Apply 8 S-boxes. Each box maps 6 bits → 4 bits.
	//   row = outer bits (first and last of the 6-bit group)
	//   col = inner 4 bits
	sboxOut := make([]int, 0, 32)
	for boxIdx := 0; boxIdx < 8; boxIdx++ {
		chunk := xored[boxIdx*6 : boxIdx*6+6]
		row := (chunk[0] << 1) | chunk[5]    // b1 and b6
		col := (chunk[1] << 3) | (chunk[2] << 2) | (chunk[3] << 1) | chunk[4]
		val := sboxes[boxIdx][row*16+col]

		// Convert the 4-bit value to 4 bits (MSB first).
		for bitPos := 3; bitPos >= 0; bitPos-- {
			sboxOut = append(sboxOut, (val>>uint(bitPos))&1)
		}
	}

	// Step 4: P permutation disperses S-box outputs for diffusion.
	return permute(sboxOut, pTable[:])
}

// =============================================================================
// Core block cipher
// =============================================================================

// desBlock encrypts or decrypts a single 8-byte block using the provided
// subkey list. Pass subkeys K1..K16 for encryption, K16..K1 for decryption.
//
// The beauty of the Feistel structure is that decryption requires no inverse
// round function -- only reversed subkeys. The same circuit handles both.
func desBlock(block []byte, subkeys [][]byte) ([]byte, error) {
	if len(block) != 8 {
		return nil, fmt.Errorf("DES block must be exactly 8 bytes, got %d", len(block))
	}

	bits := bytesToBits(block)

	// Initial permutation (scatters bits for historical bus alignment).
	bits = permute(bits, ip[:])

	// Split into left (L₀) and right (R₀) halves.
	left := make([]int, 32)
	right := make([]int, 32)
	copy(left, bits[:32])
	copy(right, bits[32:])

	// 16 Feistel rounds: L_i = R_{i-1}, R_i = L_{i-1} XOR f(R_{i-1}, K_i).
	for _, subkey := range subkeys {
		fOut := feistelF(right, subkey)
		newRight := make([]int, 32)
		for i := 0; i < 32; i++ {
			newRight[i] = left[i] ^ fOut[i]
		}
		left = right
		right = newRight
	}

	// Standard DES step: swap halves before the final permutation.
	// Combined as R₁₆ ∥ L₁₆ (right then left).
	combined := append(right, left...)

	// Final permutation (IP⁻¹) produces the ciphertext.
	return bitsToBytes(permute(combined, fp[:])), nil
}

// EncryptBlock encrypts a single 64-bit (8-byte) block with DES.
//
// Args:
//   - block: 8 bytes of plaintext
//   - key:   8 bytes (64 bits; 56 are key material, 8 are parity)
//
// Returns 8 bytes of ciphertext, or an error if inputs are the wrong size.
//
// For variable-length data, use ECBEncrypt (with PKCS#7 padding). Note that
// ECB mode is insecure for most real uses -- see ECBEncrypt for details.
func EncryptBlock(block, key []byte) ([]byte, error) {
	subkeys, err := ExpandKey(key)
	if err != nil {
		return nil, err
	}
	return desBlock(block, subkeys)
}

// DecryptBlock decrypts a single 64-bit (8-byte) block with DES.
//
// Decryption is encryption with subkeys in reverse order (K16..K1) -- a
// direct consequence of the Feistel structure's self-inverse property.
//
// Args:
//   - block: 8 bytes of ciphertext
//   - key:   8 bytes (same key used for encryption)
//
// Returns 8 bytes of plaintext.
func DecryptBlock(block, key []byte) ([]byte, error) {
	subkeys, err := ExpandKey(key)
	if err != nil {
		return nil, err
	}
	// Reverse subkeys for decryption.
	reversed := make([][]byte, len(subkeys))
	for i, sk := range subkeys {
		reversed[len(subkeys)-1-i] = sk
	}
	return desBlock(block, reversed)
}

// =============================================================================
// ECB mode (educational only)
// =============================================================================

// pkcs7Pad appends N bytes each with value N, where N is the number of bytes
// needed to reach the next block boundary (1 ≤ N ≤ blockSize).
//
// If the data is already block-aligned, a full padding block is appended so
// that unpadding is always unambiguous. This is the PKCS#7 / RFC 5652 scheme.
//
// Example: 5 bytes, blockSize=8 → append 3 bytes of value 0x03.
func pkcs7Pad(data []byte, blockSize int) []byte {
	padLen := blockSize - (len(data) % blockSize)
	result := make([]byte, len(data)+padLen)
	copy(result, data)
	for i := len(data); i < len(result); i++ {
		result[i] = byte(padLen)
	}
	return result
}

// pkcs7Unpad removes PKCS#7 padding and returns the unpadded data.
// Returns an error if the padding is invalid.
func pkcs7Unpad(data []byte) ([]byte, error) {
	if len(data) == 0 {
		return nil, fmt.Errorf("cannot unpad empty data")
	}
	padLen := int(data[len(data)-1])
	if padLen == 0 || padLen > 8 {
		return nil, fmt.Errorf("invalid PKCS#7 padding byte: %d", padLen)
	}
	if len(data) < padLen {
		return nil, fmt.Errorf("padding length %d exceeds data length %d", padLen, len(data))
	}
	for i := len(data) - padLen; i < len(data); i++ {
		if data[i] != byte(padLen) {
			return nil, fmt.Errorf("invalid PKCS#7 padding (bytes do not match)")
		}
	}
	return data[:len(data)-padLen], nil
}

// ECBEncrypt encrypts variable-length plaintext with DES in ECB mode,
// applying PKCS#7 padding so any length is supported.
//
// WARNING: ECB (Electronic Code Book) mode is insecure for most purposes.
// Identical 8-byte plaintext blocks always produce identical ciphertext
// blocks, leaking data patterns. The canonical demonstration is the
// "ECB penguin": encrypting a bitmap in ECB mode leaves the image structure
// visible in the ciphertext. Use CBC or CTR mode for real data.
//
// This function exists for:
//   - Compatibility with historical systems
//   - Demonstrating ECB's weakness
//   - Understanding why modes of operation exist
func ECBEncrypt(plaintext, key []byte) ([]byte, error) {
	subkeys, err := ExpandKey(key)
	if err != nil {
		return nil, err
	}
	padded := pkcs7Pad(plaintext, 8)
	result := make([]byte, 0, len(padded))
	for i := 0; i < len(padded); i += 8 {
		ct, err := desBlock(padded[i:i+8], subkeys)
		if err != nil {
			return nil, err
		}
		result = append(result, ct...)
	}
	return result, nil
}

// ECBDecrypt decrypts variable-length DES ECB ciphertext and removes PKCS#7 padding.
//
// Args:
//   - ciphertext: bytes (must be a non-empty multiple of 8)
//   - key: 8 bytes
//
// Returns plaintext with padding removed, or an error for invalid inputs.
func ECBDecrypt(ciphertext, key []byte) ([]byte, error) {
	if len(ciphertext) == 0 {
		return nil, fmt.Errorf("ciphertext must not be empty")
	}
	if len(ciphertext)%8 != 0 {
		return nil, fmt.Errorf("DES ECB ciphertext length must be a multiple of 8 bytes, got %d", len(ciphertext))
	}
	subkeys, err := ExpandKey(key)
	if err != nil {
		return nil, err
	}
	// Reverse subkeys for decryption.
	reversed := make([][]byte, len(subkeys))
	for i, sk := range subkeys {
		reversed[len(subkeys)-1-i] = sk
	}
	result := make([]byte, 0, len(ciphertext))
	for i := 0; i < len(ciphertext); i += 8 {
		pt, err := desBlock(ciphertext[i:i+8], reversed)
		if err != nil {
			return nil, err
		}
		result = append(result, pt...)
	}
	return pkcs7Unpad(result)
}

// =============================================================================
// Triple DES (3DES / TDEA) — NIST SP 800-67
// =============================================================================

// TDEAEncryptBlock encrypts one 8-byte block with Triple DES (3TDEA / EDE mode).
//
// Algorithm (NIST SP 800-67): C = E_K1(D_K2(E_K3(P)))
//
// Applied right-to-left:
//  1. Encrypt with K3
//  2. Decrypt with K2
//  3. Encrypt with K1
//
// The EDE (Encrypt-Decrypt-Encrypt) structure provides backward compatibility:
// if K1 = K2 = K3 = K, then 3DES reduces to single DES:
//
//	E(K, D(K, E(K, P))) = E(K, P)    since D(K, E(K, x)) = x
//
// Effective security: ~112 bits (168-bit key space reduced by meet-in-the-middle).
//
// NIST deprecated 3DES for new applications in 2017 and disallowed it entirely
// in 2023 (SP 800-131A Rev 2) due to the SWEET32 birthday attack on 64-bit blocks.
func TDEAEncryptBlock(block, k1, k2, k3 []byte) ([]byte, error) {
	step1, err := EncryptBlock(block, k3) // E_K3(P)
	if err != nil {
		return nil, err
	}
	step2, err := DecryptBlock(step1, k2) // D_K2(E_K3(P))
	if err != nil {
		return nil, err
	}
	return EncryptBlock(step2, k1) // E_K1(D_K2(E_K3(P)))
}

// TDEADecryptBlock decrypts one 8-byte block with Triple DES (3TDEA / EDE mode).
//
// Algorithm (NIST SP 800-67): P = D_K3(E_K2(D_K1(C)))
//
// Applied right-to-left:
//  1. Decrypt with K1
//  2. Encrypt with K2
//  3. Decrypt with K3
func TDEADecryptBlock(block, k1, k2, k3 []byte) ([]byte, error) {
	step1, err := DecryptBlock(block, k1) // D_K1(C)
	if err != nil {
		return nil, err
	}
	step2, err := EncryptBlock(step1, k2) // E_K2(D_K1(C))
	if err != nil {
		return nil, err
	}
	return DecryptBlock(step2, k3) // D_K3(E_K2(D_K1(C)))
}
