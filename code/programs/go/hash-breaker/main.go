// hash-breaker — Demonstrating why MD5 is cryptographically broken.
//
// This program runs three attacks against MD5 to show, in concrete terms, why
// you must never use MD5 for security:
//
//  1. Known Collision Pairs — two different byte sequences with the same MD5
//  2. Length Extension Attack — forge a valid hash without knowing the secret
//  3. Birthday Attack — find a collision on a truncated hash via birthday paradox
//
// Each attack prints educational output explaining the cryptographic concept.
package main

import (
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"math"
	"math/rand"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/md5"
)

// ============================================================================
// ATTACK 1: Known MD5 Collision Pairs (Wang & Yu, 2004)
// ============================================================================
//
// In 2004, Xiaoyun Wang and Hongbo Yu found two 128-byte messages that produce
// the SAME MD5 hash despite differing at several byte positions. This was the
// first practical collision attack on MD5, proving it is unsuitable for digital
// signatures or any application requiring collision resistance.
//
// A collision means: md5(A) == md5(B) but A != B

func mustHex(s string) []byte {
	b, err := hex.DecodeString(s)
	if err != nil {
		panic(err)
	}
	return b
}

var collisionA = mustHex(
	"d131dd02c5e6eec4693d9a0698aff95c" +
		"2fcab58712467eab4004583eb8fb7f89" +
		"55ad340609f4b30283e488832571415a" +
		"085125e8f7cdc99fd91dbdf280373c5b" +
		"d8823e3156348f5bae6dacd436c919c6" +
		"dd53e2b487da03fd02396306d248cda0" +
		"e99f33420f577ee8ce54b67080a80d1e" +
		"c69821bcb6a8839396f9652b6ff72a70")

var collisionB = mustHex(
	"d131dd02c5e6eec4693d9a0698aff95c" +
		"2fcab50712467eab4004583eb8fb7f89" +
		"55ad340609f4b30283e4888325f1415a" +
		"085125e8f7cdc99fd91dbd7280373c5b" +
		"d8823e3156348f5bae6dacd436c919c6" +
		"dd53e23487da03fd02396306d248cda0" +
		"e99f33420f577ee8ce54b67080280d1e" +
		"c69821bcb6a8839396f965ab6ff72a70")

func hexDump(data []byte) string {
	var lines []string
	for i := 0; i < len(data); i += 16 {
		end := i + 16
		if end > len(data) {
			end = len(data)
		}
		lines = append(lines, fmt.Sprintf("  %s", hex.EncodeToString(data[i:end])))
	}
	return strings.Join(lines, "\n")
}

func attack1() {
	fmt.Println(strings.Repeat("=", 72))
	fmt.Println("ATTACK 1: Known MD5 Collision Pair (Wang & Yu, 2004)")
	fmt.Println(strings.Repeat("=", 72))
	fmt.Println()
	fmt.Println("Two different 128-byte messages that produce the SAME MD5 hash.")
	fmt.Println("This was the breakthrough that proved MD5 is broken for security.")
	fmt.Println()

	fmt.Println("Block A (hex):")
	fmt.Println(hexDump(collisionA))
	fmt.Println()

	fmt.Println("Block B (hex):")
	fmt.Println(hexDump(collisionB))
	fmt.Println()

	// Show byte differences
	var diffs []int
	for i := range collisionA {
		if collisionA[i] != collisionB[i] {
			diffs = append(diffs, i)
		}
	}
	fmt.Printf("Blocks differ at %d byte positions: %v\n", len(diffs), diffs)
	for _, pos := range diffs {
		fmt.Printf("  Byte %d: A=0x%02x  B=0x%02x\n", pos, collisionA[pos], collisionB[pos])
	}
	fmt.Println()

	hashA := md5.HexString(collisionA)
	hashB := md5.HexString(collisionB)
	fmt.Printf("MD5(A) = %s\n", hashA)
	fmt.Printf("MD5(B) = %s\n", hashB)
	match := "No (unexpected)"
	if hashA == hashB {
		match = "YES — COLLISION!"
	}
	fmt.Printf("Match?   %s\n", match)
	fmt.Println()
	fmt.Println("Lesson: MD5 collisions are REAL. Never use MD5 for integrity or auth.")
	fmt.Println()
}

// ============================================================================
// ATTACK 2: Length Extension Attack
// ============================================================================
//
// MD5 (and all Merkle-Damgard hashes) are vulnerable to length extension.
// Given md5(secret || message) and len(secret || message), an attacker can
// compute md5(secret || message || padding || evil_data) WITHOUT knowing
// the secret. This breaks naive MAC = md5(secret || message).
//
// How it works:
//  1. The MD5 hash IS the four 32-bit state words (A,B,C,D) in little-endian.
//  2. Extract the state from the known hash.
//  3. Compute the padding that was applied to the original message.
//  4. Continue hashing from that state with evil_data.
//  5. The result matches md5(secret || message || padding || evil_data).

// md5Padding computes the MD5 padding for a message of the given byte length.
func md5Padding(messageLen int) []byte {
	remainder := messageLen % 64
	padLen := (55 - remainder) % 64
	if padLen < 0 {
		padLen += 64
	}
	padding := make([]byte, 1+padLen+8)
	padding[0] = 0x80
	binary.LittleEndian.PutUint64(padding[1+padLen:], uint64(messageLen)*8)
	return padding
}

// ── Inline MD5 compression for length extension attack ──────────────────────
//
// We need access to MD5's internal compression function to demonstrate the
// length extension attack. Since the md5 package doesn't export compress(),
// we implement it here. This is the SAME algorithm — 64 rounds of mixing
// using the four auxiliary functions F, G, H, I.

// MD5 per-round sine-derived constants (T-table).
var t [64]uint32

func init() {
	for i := 0; i < 64; i++ {
		t[i] = uint32(math.Floor(math.Abs(math.Sin(float64(i+1))) * (1 << 32)))
	}
}

// MD5 per-round left-rotation amounts.
var shifts = [64]uint32{
	7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
	5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
	4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
	6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
}

func rotl32(x, n uint32) uint32 {
	return (x << n) | (x >> (32 - n))
}

// compress folds one 64-byte block into the four-word state.
func compress(state [4]uint32, block []byte) [4]uint32 {
	var m [16]uint32
	for i := 0; i < 16; i++ {
		m[i] = binary.LittleEndian.Uint32(block[i*4:])
	}
	a, b, c, d := state[0], state[1], state[2], state[3]
	a0, b0, c0, d0 := a, b, c, d

	for i := 0; i < 64; i++ {
		var f uint32
		var g int
		switch {
		case i < 16:
			f = (b & c) | (^b & d)
			g = i
		case i < 32:
			f = (d & b) | (^d & c)
			g = (5*i + 1) % 16
		case i < 48:
			f = b ^ c ^ d
			g = (3*i + 5) % 16
		default:
			f = c ^ (b | ^d)
			g = (7 * i) % 16
		}
		temp := d
		d = c
		c = b
		b = b + rotl32(a+f+t[i]+m[g], shifts[i])
		a = temp
	}

	return [4]uint32{a0 + a, b0 + b, c0 + c, d0 + d}
}

func attack2() {
	fmt.Println(strings.Repeat("=", 72))
	fmt.Println("ATTACK 2: Length Extension Attack")
	fmt.Println(strings.Repeat("=", 72))
	fmt.Println()
	fmt.Println("Given md5(secret + message) and len(secret + message), we can forge")
	fmt.Println("md5(secret + message + padding + evil_data) WITHOUT knowing the secret!")
	fmt.Println()

	secret := []byte("supersecretkey!!")
	message := []byte("amount=100&to=alice")
	originalData := append(append([]byte{}, secret...), message...)
	originalHash := md5.SumMD5(originalData)
	originalHex := hex.EncodeToString(originalHash[:])

	fmt.Printf("Secret (unknown to attacker): %q\n", secret)
	fmt.Printf("Message:                      %q\n", message)
	fmt.Printf("MAC = md5(secret || message): %s\n", originalHex)
	fmt.Printf("Length of (secret || message): %d bytes\n", len(originalData))
	fmt.Println()

	evilData := []byte("&amount=1000000&to=mallory")
	fmt.Printf("Evil data to append: %q\n", evilData)
	fmt.Println()

	// Step 1: Extract internal state from the hash
	a := binary.LittleEndian.Uint32(originalHash[0:4])
	b := binary.LittleEndian.Uint32(originalHash[4:8])
	c := binary.LittleEndian.Uint32(originalHash[8:12])
	d := binary.LittleEndian.Uint32(originalHash[12:16])

	fmt.Println("Step 1: Extract MD5 internal state from the hash")
	fmt.Printf("  A = 0x%08x, B = 0x%08x, C = 0x%08x, D = 0x%08x\n", a, b, c, d)
	fmt.Println()

	// Step 2: Compute padding for the original message
	padding := md5Padding(len(originalData))
	fmt.Println("Step 2: Compute MD5 padding for the original message")
	fmt.Printf("  Padding (%d bytes): %s\n", len(padding), hex.EncodeToString(padding))
	fmt.Println()

	processedLen := len(originalData) + len(padding)
	fmt.Printf("Step 3: Total bytes processed so far: %d\n", processedLen)
	fmt.Println()

	// Step 4: Forge by compressing evil_data from the extracted state.
	// Build forged_input = evil_data + padding for total length (processedLen + len(evilData))
	forgedInput := append([]byte{}, evilData...)
	forgedInput = append(forgedInput, md5Padding(processedLen+len(evilData))...)

	state := [4]uint32{a, b, c, d}
	for i := 0; i < len(forgedInput); i += 64 {
		end := i + 64
		if end > len(forgedInput) {
			break
		}
		state = compress(state, forgedInput[i:end])
	}

	var forgedHash [16]byte
	binary.LittleEndian.PutUint32(forgedHash[0:], state[0])
	binary.LittleEndian.PutUint32(forgedHash[4:], state[1])
	binary.LittleEndian.PutUint32(forgedHash[8:], state[2])
	binary.LittleEndian.PutUint32(forgedHash[12:], state[3])
	forgedHex := hex.EncodeToString(forgedHash[:])

	fmt.Println("Step 4: Initialize hasher with extracted state, feed evil_data")
	fmt.Printf("  Forged hash: %s\n", forgedHex)
	fmt.Println()

	// Step 5: Verify
	actualFull := append(append(append([]byte{}, originalData...), padding...), evilData...)
	actualHash := md5.SumMD5(actualFull)
	actualHex := hex.EncodeToString(actualHash[:])

	fmt.Println("Step 5: Verify — compute actual md5(secret || message || padding || evil_data)")
	fmt.Printf("  Actual hash: %s\n", actualHex)
	match := "No (bug)"
	if forgedHex == actualHex {
		match = "YES — FORGED!"
	}
	fmt.Printf("  Match?       %s\n", match)
	fmt.Println()
	fmt.Println("The attacker forged a valid MAC without knowing the secret!")
	fmt.Println()
	fmt.Println("Why HMAC fixes this:")
	fmt.Println("  HMAC = md5(key XOR opad || md5(key XOR ipad || message))")
	fmt.Println("  The outer hash prevents length extension because the attacker")
	fmt.Println("  cannot extend past the outer md5() boundary.")
	fmt.Println()
}

// ============================================================================
// ATTACK 3: Birthday Attack (Truncated Hash)
// ============================================================================
//
// The birthday paradox: with N possible values, you expect a collision after
// roughly sqrt(N) random samples. For 32-bit truncated hash: 2^16 = 65536.
//
// Full MD5 (128-bit) would need ~2^64 tries — too slow to demo. Truncating
// to 4 bytes (32 bits) lets us demonstrate the principle in milliseconds.

func attack3() {
	fmt.Println(strings.Repeat("=", 72))
	fmt.Println("ATTACK 3: Birthday Attack on Truncated MD5 (32-bit)")
	fmt.Println(strings.Repeat("=", 72))
	fmt.Println()
	fmt.Println("The birthday paradox: with N possible hash values, expect a collision")
	fmt.Println("after ~sqrt(N) random inputs. For 32-bit hash: sqrt(2^32) = 2^16 = 65536.")
	fmt.Println()

	rng := rand.New(rand.NewSource(42))
	seen := make(map[[4]byte][]byte)

	for attempts := 1; ; attempts++ {
		msg := make([]byte, 8)
		rng.Read(msg)
		hash := md5.SumMD5(msg)
		truncated := [4]byte{hash[0], hash[1], hash[2], hash[3]}

		if other, ok := seen[truncated]; ok {
			if string(other) != string(msg) {
				fmt.Printf("COLLISION FOUND after %d attempts!\n\n", attempts)
				fmt.Printf("  Message 1: %s\n", hex.EncodeToString(other))
				fmt.Printf("  Message 2: %s\n", hex.EncodeToString(msg))
				fmt.Printf("  Truncated MD5 (4 bytes): %s\n", hex.EncodeToString(truncated[:]))
				fmt.Printf("  Full MD5 of msg1: %s\n", md5.HexString(other))
				fmt.Printf("  Full MD5 of msg2: %s\n", md5.HexString(msg))
				fmt.Println()
				fmt.Printf("  Expected ~65536 attempts (2^16), got %d\n", attempts)
				fmt.Printf("  Ratio: %.2fx the theoretical expectation\n", float64(attempts)/65536.0)
				break
			}
		} else {
			seen[truncated] = msg
		}
	}

	fmt.Println()
	fmt.Println("This is a GENERIC attack — it works against any hash function.")
	fmt.Println("The defense is a longer hash: SHA-256 has 2^128 birthday bound,")
	fmt.Println("while MD5 has only 2^64 (and dedicated attacks are even faster).")
	fmt.Println()
}

func main() {
	fmt.Println()
	fmt.Println("======================================================================")
	fmt.Println("           MD5 HASH BREAKER — Why MD5 Is Broken")
	fmt.Println("======================================================================")
	fmt.Println("  Three attacks showing MD5 must NEVER be used for security:")
	fmt.Println("    1. Known collision pairs (Wang & Yu, 2004)")
	fmt.Println("    2. Length extension attack (forge MAC without secret)")
	fmt.Println("    3. Birthday attack on truncated hash (birthday paradox)")
	fmt.Println("======================================================================")
	fmt.Println()

	attack1()
	attack2()
	attack3()

	fmt.Println(strings.Repeat("=", 72))
	fmt.Println("CONCLUSION")
	fmt.Println(strings.Repeat("=", 72))
	fmt.Println()
	fmt.Println("MD5 is broken in three distinct ways:")
	fmt.Println("  1. COLLISION RESISTANCE: known pairs exist (and can be generated)")
	fmt.Println("  2. LENGTH EXTENSION: Merkle-Damgard structure leaks internal state")
	fmt.Println("  3. BIRTHDAY BOUND: only 2^64 (and dedicated attacks beat even that)")
	fmt.Println()
	fmt.Println("Use SHA-256 or SHA-3 for security. Use HMAC (not raw hash) for MACs.")
	fmt.Println()
}
