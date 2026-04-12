// Package aes implements the AES (Advanced Encryption Standard) block cipher.
//
// # What Is AES?
//
// AES (FIPS 197, published 2001) is the most widely deployed symmetric cipher
// in the world. It replaced DES and is used in TLS/HTTPS, WPA2/WPA3, disk
// encryption (BitLocker, LUKS, FileVault), VPNs, and virtually every secure
// protocol. It was designed by Joan Daemen and Vincent Rijmen (the "Rijndael"
// algorithm, renamed AES after NIST's competition).
//
// This package is for educational use. It prioritises readability and explains
// the GF(2^8) mathematics underlying the algorithm. Production code should use
// AES-NI hardware instructions via crypto/aes in the Go standard library.
//
// # Architecture (Substitution-Permutation Network)
//
// Unlike DES's Feistel network (which only transforms half the state per round),
// AES's SPN transforms all 16 bytes of the 4×4 state on every round:
//
//	plaintext (16 bytes) loaded as 4×4 column-major state
//	     │
//	AddRoundKey(state, round_key[0])       ← XOR with first key material
//	     │
//	┌── Nr-1 full rounds ──────────────────────────────────────────────┐
//	│   SubBytes   — non-linear S-box substitution (GF(2^8) inverse)   │
//	│   ShiftRows  — cyclic row shifts (column-to-column diffusion)     │
//	│   MixColumns — GF(2^8) matrix multiply (full state mixing)       │
//	│   AddRoundKey — XOR with round key                               │
//	└───────────────────────────────────────────────────────────────────┘
//	     │
//	SubBytes + ShiftRows + AddRoundKey     ← final round (no MixColumns)
//	     │
//	ciphertext (16 bytes, column-major)
//
// The state is a 4×4 matrix of bytes indexed state[row][col], loaded
// column by column from the input bytes: state[row][col] = block[row + 4*col].
//
// # GF(2^8) Connection
//
// AES arithmetic lives in GF(2^8) with irreducible polynomial:
//
//	p(x) = x^8 + x^4 + x^3 + x + 1  =  0x11B
//
// This differs from Reed-Solomon's 0x11D. We use the parameterised gf256.Field
// to create an AES-specific field instance.
//
// # Key Sizes and Round Counts
//
//	Key size   Nk (words)   Nr (rounds)   Round keys
//	128 bits      4             10          11 × 16 bytes
//	192 bits      6             12          13 × 16 bytes
//	256 bits      8             14          15 × 16 bytes
package aes

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/gf256"
)

// =============================================================================
// AES GF(2^8) field — polynomial 0x11B = x^8 + x^4 + x^3 + x + 1
//
// This is distinct from the Reed-Solomon polynomial 0x11D used by gf256's
// module-level functions. We create a parameterised instance for AES.
// =============================================================================

var aesField = gf256.NewField(0x11B)

// =============================================================================
// S-box and inverse S-box
//
// SubBytes maps each byte b to:
//  1. inv = b^{-1} in GF(2^8) with polynomial 0x11B  (0 maps to 0)
//  2. affine transformation: apply the AES affine matrix over GF(2), XOR 0x63
//
// The two-step design ensures:
//   - Non-linearity (from GF inverse): resists linear cryptanalysis
//   - No fixed points (from XOR 0x63): SBOX[b] != b for all b
//
// The S-box and inverse S-box are built once at package init time.
// =============================================================================

// SBOX is the AES SubBytes substitution table, indexed by byte value.
// SBOX[0x00] = 0x63, SBOX[0x01] = 0x7c, ... (FIPS 197 Figure 7).
var SBOX [256]byte

// INV_SBOX is the inverse SubBytes table: INV_SBOX[SBOX[b]] = b for all b.
var INV_SBOX [256]byte

// affineTransform applies the AES affine transformation over GF(2):
//
//	s_i = b_i XOR b_{(i+4)%8} XOR b_{(i+5)%8} XOR b_{(i+6)%8} XOR b_{(i+7)%8} XOR c_i
//
// where c = 0x63 = 01100011.
//
// This is the circulant matrix M with first row 11110001 applied to the bit
// vector of b, then XOR'd with the constant vector 0x63.
func affineTransform(b byte) byte {
	var result byte
	for i := 0; i < 8; i++ {
		bit := ((b >> uint(i)) & 1) ^
			((b >> uint((i+4)%8)) & 1) ^
			((b >> uint((i+5)%8)) & 1) ^
			((b >> uint((i+6)%8)) & 1) ^
			((b >> uint((i+7)%8)) & 1) ^
			((0x63 >> uint(i)) & 1)
		result |= bit << uint(i)
	}
	return result
}

func init() {
	// Build SBOX: for each byte b, compute GF inverse then affine transform.
	// The inverse of 0 is defined as 0 (0 has no multiplicative inverse in GF).
	for b := 0; b < 256; b++ {
		var inv byte
		if b != 0 {
			inv = aesField.Inverse(byte(b))
		}
		SBOX[b] = affineTransform(inv)
	}
	// Build INV_SBOX by inverting: INV_SBOX[SBOX[b]] = b.
	for b := 0; b < 256; b++ {
		INV_SBOX[SBOX[b]] = byte(b)
	}
}

// =============================================================================
// Round constants (Rcon) for the key schedule
//
// Rcon[i] = x^{i-1} in GF(2^8) for i = 1..14.
// Used as [Rcon_i, 0, 0, 0] — only the first byte is non-zero.
// They break symmetry in the key schedule so that no two round keys are equal.
// =============================================================================

// rcon holds Rcon[0..14]; index 0 is unused (NIST uses 1-based indexing).
var rcon [15]byte

func init() {
	// rcon[1] = 0x01 = x^0; rcon[i] = 0x02 * rcon[i-1] in GF(2^8).
	val := byte(1)
	for i := 1; i <= 14; i++ {
		rcon[i] = val
		val = aesField.Multiply(val, 0x02)
	}
}

// =============================================================================
// Key schedule: ExpandKey
// =============================================================================

// ExpandKey expands a 16-, 24-, or 32-byte AES key into round keys.
//
// Returns a slice of (Nr+1) round keys. Each round key is represented as
// a [4][4]byte matrix (row-major: rk[row][col]). The round key at index 0
// is the initial AddRoundKey; round key Nr is the final AddRoundKey.
//
// Algorithm (FIPS 197 Section 5.2):
//
//	Nk = key length in 32-bit words (4, 6, or 8)
//	Nr = number of rounds (10, 12, or 14)
//	Total words W needed = 4 × (Nr + 1)
//
//	W[i] = W[i-1] XOR W[i-Nk]                          (general case)
//	W[i] = SubWord(RotWord(W[i-1])) XOR Rcon[i/Nk] XOR W[i-Nk]   (i mod Nk == 0)
//	W[i] = SubWord(W[i-1]) XOR W[i-Nk]                 (Nk==8, i mod Nk == 4)
func ExpandKey(key []byte) ([][][4]byte, error) {
	keyLen := len(key)
	if keyLen != 16 && keyLen != 24 && keyLen != 32 {
		return nil, fmt.Errorf("AES key must be 16, 24, or 32 bytes; got %d", keyLen)
	}

	nk := keyLen / 4 // number of 32-bit words in the key (4, 6, or 8)
	nrMap := map[int]int{4: 10, 6: 12, 8: 14}
	nr := nrMap[nk] // number of rounds
	totalWords := 4 * (nr + 1)

	// W is a flat list of 4-byte words.
	w := make([][4]byte, totalWords)
	for i := 0; i < nk; i++ {
		copy(w[i][:], key[i*4:i*4+4])
	}

	for i := nk; i < totalWords; i++ {
		temp := w[i-1]
		if i%nk == 0 {
			// RotWord: left-rotate the 4-byte word.
			temp = [4]byte{temp[1], temp[2], temp[3], temp[0]}
			// SubWord: apply S-box to each byte.
			for j := range temp {
				temp[j] = SBOX[temp[j]]
			}
			// XOR with the round constant (only first byte is non-zero).
			temp[0] ^= rcon[i/nk]
		} else if nk == 8 && i%nk == 4 {
			// Extra SubWord for AES-256 (additional non-linearity).
			for j := range temp {
				temp[j] = SBOX[temp[j]]
			}
		}
		// XOR with the word Nk positions back.
		for j := 0; j < 4; j++ {
			w[i][j] = w[i-nk][j] ^ temp[j]
		}
	}

	// Pack into (Nr+1) round keys. Each round key is 4 consecutive words.
	// Stored as state[row][col] in column-major order:
	//   rk_words[col][row] -> state[row][col]
	roundKeys := make([][][4]byte, nr+1)
	for r := 0; r <= nr; r++ {
		var rk [4][4]byte
		// Four words form this round key; words are column vectors.
		for col := 0; col < 4; col++ {
			for row := 0; row < 4; row++ {
				rk[row][col] = w[4*r+col][row]
			}
		}
		roundKeys[r] = rk[:]
	}
	return roundKeys, nil
}

// =============================================================================
// MixColumns helpers
//
// Each column of the 4×4 state is treated as a polynomial in GF(2^8) and
// multiplied by the fixed AES MixColumns matrix:
//
//	[2 3 1 1]   [s0]
//	[1 2 3 1] × [s1]
//	[1 1 2 3]   [s2]
//	[3 1 1 2]   [s3]
//
// where multiplication is in GF(2^8) with polynomial 0x11B.
// InvMixColumns uses the inverse matrix [14 11 13 9; 9 14 11 13; ...].
// =============================================================================

// xtime multiplies b by 2 in GF(2^8) with AES polynomial 0x11B.
// Equivalent to left-shift by 1, XOR 0x1B if bit 7 was set.
func xtime(b byte) byte {
	return aesField.Multiply(b, 0x02)
}

// mixCol applies the AES MixColumns transformation to one 4-byte column.
func mixCol(col [4]byte) [4]byte {
	s0, s1, s2, s3 := col[0], col[1], col[2], col[3]
	// 2·s = xtime(s), 3·s = xtime(s) XOR s
	return [4]byte{
		xtime(s0) ^ (xtime(s1) ^ s1) ^ s2 ^ s3,
		s0 ^ xtime(s1) ^ (xtime(s2) ^ s2) ^ s3,
		s0 ^ s1 ^ xtime(s2) ^ (xtime(s3) ^ s3),
		(xtime(s0) ^ s0) ^ s1 ^ s2 ^ xtime(s3),
	}
}

// invMixCol applies the AES InvMixColumns transformation to one 4-byte column.
func invMixCol(col [4]byte) [4]byte {
	s0, s1, s2, s3 := col[0], col[1], col[2], col[3]
	f := aesField.Multiply
	// Coefficients: 14=0x0e, 11=0x0b, 13=0x0d, 9=0x09
	return [4]byte{
		f(0x0e, s0) ^ f(0x0b, s1) ^ f(0x0d, s2) ^ f(0x09, s3),
		f(0x09, s0) ^ f(0x0e, s1) ^ f(0x0b, s2) ^ f(0x0d, s3),
		f(0x0d, s0) ^ f(0x09, s1) ^ f(0x0e, s2) ^ f(0x0b, s3),
		f(0x0b, s0) ^ f(0x0d, s1) ^ f(0x09, s2) ^ f(0x0e, s3),
	}
}

// =============================================================================
// State manipulation
// =============================================================================

// state represents the AES 4×4 byte matrix, indexed state[row][col].
// Loaded column-by-column: state[row][col] = block[row + 4*col].
type state [4][4]byte

// bytesToState converts 16 bytes to the AES state (column-major).
//
//	block[0]  block[4]  block[8]  block[12]
//	block[1]  block[5]  block[9]  block[13]
//	block[2]  block[6]  block[10] block[14]
//	block[3]  block[7]  block[11] block[15]
func bytesToState(block []byte) state {
	var s state
	for col := 0; col < 4; col++ {
		for row := 0; row < 4; row++ {
			s[row][col] = block[row+4*col]
		}
	}
	return s
}

// stateToBytes converts the AES state back to 16 bytes (column-major).
func stateToBytes(s state) []byte {
	out := make([]byte, 16)
	for col := 0; col < 4; col++ {
		for row := 0; row < 4; row++ {
			out[row+4*col] = s[row][col]
		}
	}
	return out
}

// addRoundKey XORs the state with a round key (AddRoundKey step).
func addRoundKey(s state, rk [][4]byte) state {
	var result state
	for row := 0; row < 4; row++ {
		for col := 0; col < 4; col++ {
			result[row][col] = s[row][col] ^ rk[row][col]
		}
	}
	return result
}

// subBytes replaces each byte with its SBOX value.
func subBytes(s state) state {
	var result state
	for row := 0; row < 4; row++ {
		for col := 0; col < 4; col++ {
			result[row][col] = SBOX[s[row][col]]
		}
	}
	return result
}

// invSubBytes applies the inverse S-box to each byte.
func invSubBytes(s state) state {
	var result state
	for row := 0; row < 4; row++ {
		for col := 0; col < 4; col++ {
			result[row][col] = INV_SBOX[s[row][col]]
		}
	}
	return result
}

// shiftRows cyclically shifts row i left by i positions.
//
//	Row 0: no shift     [a b c d] → [a b c d]
//	Row 1: shift left 1 [a b c d] → [b c d a]
//	Row 2: shift left 2 [a b c d] → [c d a b]
//	Row 3: shift left 3 [a b c d] → [d a b c]
//
// After MixColumns, every output column depends on all four input columns.
func shiftRows(s state) state {
	var result state
	for row := 0; row < 4; row++ {
		for col := 0; col < 4; col++ {
			result[row][col] = s[row][(col+row)%4]
		}
	}
	return result
}

// invShiftRows shifts row i right by i positions (undoes ShiftRows).
func invShiftRows(s state) state {
	var result state
	for row := 0; row < 4; row++ {
		for col := 0; col < 4; col++ {
			result[row][col] = s[row][(col-row+4)%4]
		}
	}
	return result
}

// mixColumns applies MixColumns to each of the 4 columns.
func mixColumns(s state) state {
	var result state
	for col := 0; col < 4; col++ {
		var column [4]byte
		for row := 0; row < 4; row++ {
			column[row] = s[row][col]
		}
		mixed := mixCol(column)
		for row := 0; row < 4; row++ {
			result[row][col] = mixed[row]
		}
	}
	return result
}

// invMixColumns applies InvMixColumns to each of the 4 columns.
func invMixColumns(s state) state {
	var result state
	for col := 0; col < 4; col++ {
		var column [4]byte
		for row := 0; row < 4; row++ {
			column[row] = s[row][col]
		}
		mixed := invMixCol(column)
		for row := 0; row < 4; row++ {
			result[row][col] = mixed[row]
		}
	}
	return result
}

// =============================================================================
// Core block cipher
// =============================================================================

// EncryptBlock encrypts a single 128-bit (16-byte) block with AES.
//
// Supports all three key sizes:
//   - 16 bytes (AES-128): 10 rounds
//   - 24 bytes (AES-192): 12 rounds
//   - 32 bytes (AES-256): 14 rounds
//
// Algorithm (FIPS 197 Section 5.1):
//
//	AddRoundKey(state, round_key[0])
//	for round = 1 to Nr-1:
//	  SubBytes → ShiftRows → MixColumns → AddRoundKey
//	SubBytes → ShiftRows → AddRoundKey  (final round: no MixColumns)
func EncryptBlock(block, key []byte) ([]byte, error) {
	if len(block) != 16 {
		return nil, fmt.Errorf("AES block must be 16 bytes, got %d", len(block))
	}
	roundKeys, err := ExpandKey(key)
	if err != nil {
		return nil, err
	}
	nr := len(roundKeys) - 1

	s := bytesToState(block)
	s = addRoundKey(s, roundKeys[0])

	for rnd := 1; rnd < nr; rnd++ {
		s = subBytes(s)
		s = shiftRows(s)
		s = mixColumns(s)
		s = addRoundKey(s, roundKeys[rnd])
	}

	// Final round: SubBytes + ShiftRows + AddRoundKey (no MixColumns).
	s = subBytes(s)
	s = shiftRows(s)
	s = addRoundKey(s, roundKeys[nr])

	return stateToBytes(s), nil
}

// DecryptBlock decrypts a single 128-bit (16-byte) block with AES.
//
// Unlike DES (Feistel), AES decryption is NOT the same circuit as encryption.
// It uses the inverses of each operation, applied in reverse order:
// InvShiftRows → InvSubBytes → AddRoundKey → InvMixColumns.
//
// (AddRoundKey is its own inverse since XOR is self-inverse; the key order
// is still reversed compared to encryption.)
func DecryptBlock(block, key []byte) ([]byte, error) {
	if len(block) != 16 {
		return nil, fmt.Errorf("AES block must be 16 bytes, got %d", len(block))
	}
	roundKeys, err := ExpandKey(key)
	if err != nil {
		return nil, err
	}
	nr := len(roundKeys) - 1

	s := bytesToState(block)
	s = addRoundKey(s, roundKeys[nr])

	for rnd := nr - 1; rnd >= 1; rnd-- {
		s = invShiftRows(s)
		s = invSubBytes(s)
		s = addRoundKey(s, roundKeys[rnd])
		s = invMixColumns(s)
	}

	// Final (initial encryption) round.
	s = invShiftRows(s)
	s = invSubBytes(s)
	s = addRoundKey(s, roundKeys[0])

	return stateToBytes(s), nil
}
