// Package chacha20poly1305 implements the ChaCha20-Poly1305 AEAD cipher
// suite (RFC 8439) from scratch, using only ARX (Add, Rotate, XOR) operations.
//
// # What is ChaCha20-Poly1305?
//
// It combines two cryptographic primitives:
//
//   - ChaCha20: a stream cipher that generates pseudorandom keystream bytes
//     using only additions, rotations, and XORs on 32-bit words.
//
//   - Poly1305: a one-time message authentication code (MAC) that produces a
//     16-byte tag by evaluating a polynomial modulo the prime 2^130 - 5.
//
// Together they provide authenticated encryption: the ciphertext is both
// confidential and tamper-evident.
//
// # Why ChaCha20 Instead of AES?
//
// AES uses lookup tables (S-boxes) and Galois field arithmetic that are
// complex and vulnerable to cache-timing side-channel attacks in software.
// ChaCha20 uses only additions, rotations, and XORs -- operations that
// execute in constant time on all CPUs.
//
// Reference: RFC 8439 (https://www.rfc-editor.org/rfc/rfc8439)
package chacha20poly1305

import (
	"encoding/binary"
	"errors"
	"math/big"
)

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// The ChaCha20 state matrix begins with four constant words that spell out
// "expand 32-byte k" in ASCII. These ensure that even if an attacker
// controls the key, nonce, and counter, they cannot force the initial state
// into a degenerate configuration.
//
//	"expa" = 0x61707865
//	"nd 3" = 0x3320646e
//	"2-by" = 0x79622d32
//	"te k" = 0x6b206574
var chacha20Constants = [4]uint32{
	0x61707865, 0x3320646e, 0x79622d32, 0x6b206574,
}

// ErrAuthFailed is returned when AEAD decryption fails due to a tag mismatch,
// indicating the ciphertext has been tampered with or the wrong key/nonce/AAD
// was used.
var ErrAuthFailed = errors.New("chacha20poly1305: authentication failed: tag mismatch")

// ---------------------------------------------------------------------------
// Utility: 32-bit left rotation
// ---------------------------------------------------------------------------

// rotl32 rotates a 32-bit integer left by the given number of bits.
//
// In hardware this is a single instruction. In Go, we emulate it with two
// shifts and an OR. Go's uint32 type naturally wraps at 32 bits, so no
// masking is needed (unlike Python or Ruby which have arbitrary-precision ints).
func rotl32(v uint32, n uint) uint32 {
	return (v << n) | (v >> (32 - n))
}

// ---------------------------------------------------------------------------
// ChaCha20 Quarter Round
// ---------------------------------------------------------------------------

// quarterRound applies the ChaCha20 quarter round to four words in the state.
//
// The quarter round is the core mixing function. It takes four 32-bit words
// and mixes them using ARX operations in a cascade:
//
//	a += b;  d ^= a;  d <<<= 16    (mix a,b into d)
//	c += d;  b ^= c;  b <<<= 12    (mix c,d into b)
//	a += b;  d ^= a;  d <<<= 8     (mix a,b into d again)
//	c += d;  b ^= c;  b <<<= 7     (mix c,d into b again)
//
// The rotation amounts (16, 12, 8, 7) were chosen by Bernstein to maximize
// diffusion -- after 20 rounds, every output bit depends on every input bit.
func quarterRound(state *[16]uint32, a, b, c, d int) {
	// Step 1
	state[a] += state[b]
	state[d] ^= state[a]
	state[d] = rotl32(state[d], 16)

	// Step 2
	state[c] += state[d]
	state[b] ^= state[c]
	state[b] = rotl32(state[b], 12)

	// Step 3
	state[a] += state[b]
	state[d] ^= state[a]
	state[d] = rotl32(state[d], 8)

	// Step 4
	state[c] += state[d]
	state[b] ^= state[c]
	state[b] = rotl32(state[b], 7)
}

// ---------------------------------------------------------------------------
// ChaCha20 Block Function
// ---------------------------------------------------------------------------

// chacha20Block generates one 64-byte keystream block.
//
// The state is a 4x4 matrix of 32-bit words:
//
//	+----------+----------+----------+----------+
//	| const[0] | const[1] | const[2] | const[3] |  <- "expand 32-byte k"
//	+----------+----------+----------+----------+
//	|  key[0]  |  key[1]  |  key[2]  |  key[3]  |  <- first half of key
//	+----------+----------+----------+----------+
//	|  key[4]  |  key[5]  |  key[6]  |  key[7]  |  <- second half of key
//	+----------+----------+----------+----------+
//	| counter  | nonce[0] | nonce[1] | nonce[2] |  <- counter + nonce
//	+----------+----------+----------+----------+
//
// After 20 rounds of mixing (10 column + 10 diagonal quarter rounds), the
// original state is added back. This addition makes the function one-way:
// without it, the mixing would be invertible and the key recoverable.
func chacha20Block(key []byte, counter uint32, nonce []byte) [64]byte {
	var state [16]uint32

	// Initialize constants
	state[0] = chacha20Constants[0]
	state[1] = chacha20Constants[1]
	state[2] = chacha20Constants[2]
	state[3] = chacha20Constants[3]

	// Initialize key (8 little-endian uint32 words)
	for i := 0; i < 8; i++ {
		state[4+i] = binary.LittleEndian.Uint32(key[i*4:])
	}

	// Initialize counter and nonce
	state[12] = counter
	for i := 0; i < 3; i++ {
		state[13+i] = binary.LittleEndian.Uint32(nonce[i*4:])
	}

	// Save original state for final addition
	initial := state

	// 20 rounds = 10 double-rounds
	// Each double-round: 4 column quarter rounds + 4 diagonal quarter rounds
	//
	// Column indices:        Diagonal indices:
	//   (0,4,8,12)            (0,5,10,15)  main diagonal
	//   (1,5,9,13)            (1,6,11,12)  shifted by 1
	//   (2,6,10,14)           (2,7,8,13)   shifted by 2
	//   (3,7,11,15)           (3,4,9,14)   shifted by 3
	for i := 0; i < 10; i++ {
		// Column rounds
		quarterRound(&state, 0, 4, 8, 12)
		quarterRound(&state, 1, 5, 9, 13)
		quarterRound(&state, 2, 6, 10, 14)
		quarterRound(&state, 3, 7, 11, 15)
		// Diagonal rounds
		quarterRound(&state, 0, 5, 10, 15)
		quarterRound(&state, 1, 6, 11, 12)
		quarterRound(&state, 2, 7, 8, 13)
		quarterRound(&state, 3, 4, 9, 14)
	}

	// Add original state back
	for i := 0; i < 16; i++ {
		state[i] += initial[i]
	}

	// Serialize as 64 little-endian bytes
	var out [64]byte
	for i := 0; i < 16; i++ {
		binary.LittleEndian.PutUint32(out[i*4:], state[i])
	}
	return out
}

// ---------------------------------------------------------------------------
// ChaCha20 Stream Cipher
// ---------------------------------------------------------------------------

// ChaCha20Encrypt encrypts (or decrypts) data using the ChaCha20 stream cipher.
//
// ChaCha20 is a stream cipher: it generates a pseudorandom keystream and XORs
// it with the input. Because XOR is its own inverse, the same function works
// for both encryption and decryption:
//
//	ciphertext = plaintext XOR keystream
//	plaintext  = ciphertext XOR keystream
//
// The keystream is produced in 64-byte blocks, each with a different counter.
// Up to 2^32 * 64 = 256 GiB can be encrypted with one key/nonce pair.
func ChaCha20Encrypt(plaintext, key, nonce []byte, counter uint32) ([]byte, error) {
	if len(key) != 32 {
		return nil, errors.New("chacha20: key must be 32 bytes")
	}
	if len(nonce) != 12 {
		return nil, errors.New("chacha20: nonce must be 12 bytes")
	}

	result := make([]byte, len(plaintext))
	offset := 0

	for offset < len(plaintext) {
		// Generate one 64-byte keystream block
		keystream := chacha20Block(key, counter, nonce)

		// XOR plaintext with keystream (last block may be partial)
		end := offset + 64
		if end > len(plaintext) {
			end = len(plaintext)
		}
		for i := offset; i < end; i++ {
			result[i] = plaintext[i] ^ keystream[i-offset]
		}

		offset += 64
		counter++
	}

	return result, nil
}

// ---------------------------------------------------------------------------
// Poly1305 Message Authentication Code
// ---------------------------------------------------------------------------

// Poly1305Mac computes a Poly1305 one-time MAC tag.
//
// Poly1305 evaluates a polynomial over a prime field to produce a 16-byte
// authentication tag. It is provably secure when each key is used exactly
// once. Reusing a Poly1305 key allows tag forgery.
//
// Algorithm:
//  1. Split the 32-byte key into r (16 bytes, clamped) and s (16 bytes).
//  2. Process message in 16-byte chunks. For each chunk:
//     a. Interpret as little-endian integer with 0x01 appended
//     b. acc = ((acc + chunk) * r) mod (2^130 - 5)
//  3. tag = (acc + s) mod 2^128
//
// The prime 2^130 - 5 was chosen because it is Mersenne-like, enabling fast
// modular reduction, and just barely larger than 128 bits so each block fits.
//
// Go does not have native big integers in the language, so we use math/big
// for the 130+ bit arithmetic required by Poly1305.
func Poly1305Mac(message, key []byte) ([]byte, error) {
	if len(key) != 32 {
		return nil, errors.New("poly1305: key must be 32 bytes")
	}

	// Split key into r (first 16 bytes) and s (last 16 bytes)
	rBytes := make([]byte, 16)
	copy(rBytes, key[:16])
	sBytes := key[16:]

	// Clamp r: clear specific bits for the security proof to hold.
	//   bytes 3, 7, 11, 15: clear top 4 bits (& 0x0f)
	//   bytes 4, 8, 12: clear bottom 2 bits (& 0xfc)
	rBytes[3] &= 0x0f
	rBytes[7] &= 0x0f
	rBytes[11] &= 0x0f
	rBytes[15] &= 0x0f
	rBytes[4] &= 0xfc
	rBytes[8] &= 0xfc
	rBytes[12] &= 0xfc

	// Convert r and s to big.Int (little-endian)
	r := new(big.Int).SetBytes(reverseBytes(rBytes))
	s := new(big.Int).SetBytes(reverseBytes(sBytes))

	// p = 2^130 - 5
	p := new(big.Int).Sub(
		new(big.Int).Lsh(big.NewInt(1), 130),
		big.NewInt(5),
	)

	// Process message in 16-byte blocks
	acc := new(big.Int)
	tmp := new(big.Int)

	for i := 0; i < len(message); i += 16 {
		end := i + 16
		if end > len(message) {
			end = len(message)
		}
		chunk := message[i:end]

		// Convert chunk to little-endian integer, then set bit 8*len(chunk).
		// The sentinel bit distinguishes trailing zeros from padding.
		chunkLE := reverseBytes(chunk)
		n := new(big.Int).SetBytes(chunkLE)
		n.SetBit(n, 8*len(chunk), 1)

		// acc = ((acc + n) * r) mod p
		acc.Add(acc, n)
		acc.Mul(acc, r)
		tmp.Mod(acc, p)
		acc.Set(tmp)
	}

	// tag = (acc + s) mod 2^128
	acc.Add(acc, s)
	mod128 := new(big.Int).Lsh(big.NewInt(1), 128)
	acc.Mod(acc, mod128)

	// Convert to 16 little-endian bytes
	tagBytes := acc.Bytes()
	tag := make([]byte, 16)
	// big.Int.Bytes() is big-endian; reverse to little-endian
	for i, b := range tagBytes {
		tag[len(tagBytes)-1-i] = b
	}

	return tag, nil
}

// reverseBytes returns a new slice with bytes in reverse order.
// This is used to convert between little-endian byte arrays and big-endian
// big.Int representation.
func reverseBytes(b []byte) []byte {
	out := make([]byte, len(b))
	for i, v := range b {
		out[len(b)-1-i] = v
	}
	return out
}

// ---------------------------------------------------------------------------
// Pad16 Helper
// ---------------------------------------------------------------------------

// pad16 returns zero-padding bytes to make the total length a multiple of 16.
// If the data length is already a multiple of 16, returns nil.
func pad16(data []byte) []byte {
	rem := len(data) % 16
	if rem == 0 {
		return nil
	}
	return make([]byte, 16-rem)
}

// ---------------------------------------------------------------------------
// AEAD Encryption (RFC 8439 Section 2.8)
// ---------------------------------------------------------------------------

// AEADEncrypt encrypts and authenticates data using ChaCha20-Poly1305 AEAD.
//
// The construction (RFC 8439 Section 2.8):
//  1. Generate Poly1305 key: first 32 bytes of ChaCha20(key, nonce, counter=0)
//  2. Encrypt plaintext with ChaCha20(key, nonce, counter=1)
//  3. MAC input: AAD || pad16(AAD) || CT || pad16(CT) || le64(len(AAD)) || le64(len(CT))
//  4. tag = Poly1305(polyKey, macInput)
func AEADEncrypt(plaintext, key, nonce, aad []byte) (ciphertext, tag []byte, err error) {
	if len(key) != 32 {
		return nil, nil, errors.New("aead: key must be 32 bytes")
	}
	if len(nonce) != 12 {
		return nil, nil, errors.New("aead: nonce must be 12 bytes")
	}

	// Step 1: Generate one-time Poly1305 key from ChaCha20 block 0
	polyKeyBlock := chacha20Block(key, 0, nonce)
	polyKey := polyKeyBlock[:32]

	// Step 2: Encrypt plaintext starting at counter=1
	ciphertext, err = ChaCha20Encrypt(plaintext, key, nonce, 1)
	if err != nil {
		return nil, nil, err
	}

	// Step 3: Construct MAC input and compute tag
	macData := buildMACData(aad, ciphertext)
	tag, err = Poly1305Mac(macData, polyKey)
	if err != nil {
		return nil, nil, err
	}

	return ciphertext, tag, nil
}

// ---------------------------------------------------------------------------
// AEAD Decryption (RFC 8439 Section 2.8)
// ---------------------------------------------------------------------------

// AEADDecrypt decrypts and verifies data using ChaCha20-Poly1305 AEAD.
//
// If the tag does not match, ErrAuthFailed is returned and no plaintext is
// provided. This prevents chosen-ciphertext attacks that exploit partial
// decryption of tampered data.
func AEADDecrypt(ciphertext, key, nonce, aad, tag []byte) ([]byte, error) {
	if len(key) != 32 {
		return nil, errors.New("aead: key must be 32 bytes")
	}
	if len(nonce) != 12 {
		return nil, errors.New("aead: nonce must be 12 bytes")
	}
	if len(tag) != 16 {
		return nil, errors.New("aead: tag must be 16 bytes")
	}

	// Step 1: Generate one-time Poly1305 key
	polyKeyBlock := chacha20Block(key, 0, nonce)
	polyKey := polyKeyBlock[:32]

	// Step 2: Recompute expected tag
	macData := buildMACData(aad, ciphertext)
	expectedTag, err := Poly1305Mac(macData, polyKey)
	if err != nil {
		return nil, err
	}

	// Step 3: Constant-time tag comparison
	if !constantTimeCompare(expectedTag, tag) {
		return nil, ErrAuthFailed
	}

	// Step 4: Decrypt
	return ChaCha20Encrypt(ciphertext, key, nonce, 1)
}

// buildMACData constructs the Poly1305 input for the AEAD construction:
//
//	AAD || pad16(AAD) || ciphertext || pad16(ciphertext) ||
//	le64(len(AAD)) || le64(len(ciphertext))
func buildMACData(aad, ciphertext []byte) []byte {
	var data []byte
	data = append(data, aad...)
	data = append(data, pad16(aad)...)
	data = append(data, ciphertext...)
	data = append(data, pad16(ciphertext)...)

	var lengths [16]byte
	binary.LittleEndian.PutUint64(lengths[:8], uint64(len(aad)))
	binary.LittleEndian.PutUint64(lengths[8:], uint64(len(ciphertext)))
	data = append(data, lengths[:]...)

	return data
}

// constantTimeCompare compares two byte slices in constant time.
//
// A naive comparison short-circuits on the first differing byte, leaking
// timing information. This function always examines every byte.
func constantTimeCompare(a, b []byte) bool {
	if len(a) != len(b) {
		return false
	}
	var result byte
	for i := range a {
		result |= a[i] ^ b[i]
	}
	return result == 0
}
