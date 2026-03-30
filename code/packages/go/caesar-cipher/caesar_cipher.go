// Package caesarcipher implements the Caesar cipher — history's oldest known
// substitution cipher — along with brute-force and frequency-analysis attacks.
//
// This package is part of the coding-adventures monorepo, a ground-up
// implementation of the computing stack from transistors to operating systems.
//
// ─────────────────────────────────────────────────────────────────────────────
// WHAT IS A CAESAR CIPHER?
// ─────────────────────────────────────────────────────────────────────────────
//
// Named after Julius Caesar, who reportedly used it for military messages,
// the Caesar cipher is a *monoalphabetic substitution cipher*. Every letter
// in the plaintext is replaced by a letter a fixed number of positions down
// the alphabet. That fixed number is called the "shift" or "key".
//
// Example with shift = 3:
//
//	Plaintext alphabet:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
//	Ciphertext alphabet: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
//
// So "HELLO" becomes "KHOOR":
//
//	H → K  (H is position 7, +3 = 10, which is K)
//	E → H  (E is position 4, +3 = 7,  which is H)
//	L → O  (L is position 11, +3 = 14, which is O)
//	L → O  (same letter, same result)
//	O → R  (O is position 14, +3 = 17, which is R)
//
// ─────────────────────────────────────────────────────────────────────────────
// THE MATH: MODULAR ARITHMETIC
// ─────────────────────────────────────────────────────────────────────────────
//
// The cipher is really just addition modulo 26 (the number of letters in the
// English alphabet):
//
//	Encryption: E(x) = (x + shift) mod 26
//	Decryption: D(x) = (x - shift) mod 26  =  (x + (26 - shift)) mod 26
//
// where x is the zero-indexed position of a letter (A=0, B=1, ..., Z=25).
//
// Go's `%` operator can return negative values for negative operands, so we
// always add 26 before taking the modulus to ensure a positive result:
//
//	shifted = ((x + shift) % 26 + 26) % 26
//
// This double-mod trick is a common pattern in modular arithmetic:
//
//	(-3 % 26) in Go  →  -3   (not what we want)
//	((-3 % 26) + 26) % 26  →  23  (correct!)
//
// ─────────────────────────────────────────────────────────────────────────────
// CASE PRESERVATION AND NON-ALPHA PASSTHROUGH
// ─────────────────────────────────────────────────────────────────────────────
//
// Real-world text contains uppercase letters, lowercase letters, digits,
// spaces, and punctuation. Our implementation:
//
//   - Uppercase letters (A-Z): shifted within A-Z, remain uppercase
//   - Lowercase letters (a-z): shifted within a-z, remain lowercase
//   - Everything else (digits, spaces, punctuation): passed through unchanged
//
// This matches the traditional Caesar cipher behavior, which only operates on
// alphabetic characters.
//
// ─────────────────────────────────────────────────────────────────────────────
// BREAKING THE CIPHER
// ─────────────────────────────────────────────────────────────────────────────
//
// The Caesar cipher is trivially broken because it has only 25 possible keys
// (shifts 1–25; shift 0 is the identity). Two standard attacks:
//
// 1. **Brute Force**: Try all 25 shifts and read the results. A human can
//    spot the correct plaintext instantly.
//
// 2. **Frequency Analysis**: In English, letters appear with known frequencies
//    (E ≈ 12.7%, T ≈ 9.1%, A ≈ 8.2%, ...). We compute the letter frequency
//    distribution of the ciphertext for each possible shift and compare it to
//    the expected English distribution using the chi-squared (χ²) statistic.
//    The shift with the lowest χ² is the most likely key.
//
//    χ² = Σ (observed_i - expected_i)² / expected_i
//
//    where i ranges over each letter A-Z, observed_i is the count of letter i
//    in the decrypted text, and expected_i is the expected count based on
//    English frequencies and the total letter count.
package caesarcipher

import (
	"math"
	"unicode"
)

// ─────────────────────────────────────────────────────────────────────────────
// ENGLISH LETTER FREQUENCIES
// ─────────────────────────────────────────────────────────────────────────────
//
// These frequencies come from analysis of large English text corpora. They
// represent the percentage probability that a randomly chosen letter from
// English text is a given letter. For example, 'E' appears about 12.7% of
// the time.
//
// Source: https://en.wikipedia.org/wiki/Letter_frequency
//
//	E ████████████▋         12.70%
//	T █████████▏            9.06%
//	A ████████▏             8.17%
//	O ███████▌              7.51%
//	I ██████▉               6.97%
//	N ██████▋               6.75%
//	S ██████▎               6.33%
//	H ██████▏               6.09%
//	R █████▉                5.99%
//	D ████▎                 4.25%
//	L ████                  4.03%
//	C ██▊                   2.78%
//	U ██▊                   2.76%
//	M ██▍                   2.41%
//	W ██▍                   2.36%
//	F ██▎                   2.23%
//	G ██                    2.02%
//	Y ██                    1.97%
//	P █▉                    1.93%
//	B █▌                    1.49%
//	V █                     0.98%
//	K ▊                     0.77%
//	J ▎                     0.15%
//	X ▎                     0.15%
//	Q ▏                     0.10%
//	Z ▏                     0.07%

var EnglishFrequencies = map[rune]float64{
	'A': 0.0817, 'B': 0.0149, 'C': 0.0278, 'D': 0.0425,
	'E': 0.1270, 'F': 0.0223, 'G': 0.0202, 'H': 0.0609,
	'I': 0.0697, 'J': 0.0015, 'K': 0.0077, 'L': 0.0403,
	'M': 0.0241, 'N': 0.0675, 'O': 0.0751, 'P': 0.0193,
	'Q': 0.0010, 'R': 0.0599, 'S': 0.0633, 'T': 0.0906,
	'U': 0.0276, 'V': 0.0098, 'W': 0.0236, 'X': 0.0015,
	'Y': 0.0197, 'Z': 0.0007,
}

// ─────────────────────────────────────────────────────────────────────────────
// CORE SHIFT FUNCTION
// ─────────────────────────────────────────────────────────────────────────────
//
// Both encryption and decryption are the same operation — they just differ
// in the sign of the shift. Rather than duplicating logic, we factor out
// a single shiftRune helper.
//
// The algorithm for a single character:
//
//	1. Is it a letter? If not, return it unchanged.
//	2. Determine the base: 'A' for uppercase, 'a' for lowercase.
//	3. Compute the zero-indexed position: pos = char - base
//	4. Apply the shift with modular arithmetic: newPos = ((pos + shift) % 26 + 26) % 26
//	5. Convert back to a character: result = base + newPos
//
// The `+26) % 26` trick ensures we never get a negative modulus.
//
// Truth table for a few values (uppercase, base = 'A' = 65):
//
//	char | pos | shift | (pos+shift)%26 | result
//	─────┼─────┼───────┼────────────────┼───────
//	 'A' |  0  |   3   |       3        |  'D'
//	 'Z' | 25  |   1   |       0        |  'A'   ← wraps around!
//	 'H' |  7  |   3   |      10        |  'K'
//	 'K' | 10  |  -3   |       7        |  'H'   ← decryption

// shiftRune shifts a single rune by the given amount. Non-letter runes are
// returned unchanged. Case is preserved.
func shiftRune(r rune, shift int) rune {
	// Non-letter characters pass through untouched. This includes digits,
	// spaces, punctuation, emoji, and any other Unicode character.
	if !unicode.IsLetter(r) {
		return r
	}

	// We only shift ASCII letters. Non-ASCII letters (accented characters,
	// Cyrillic, etc.) pass through unchanged to avoid unexpected behavior.
	if r < 'A' || (r > 'Z' && r < 'a') || r > 'z' {
		return r
	}

	// Determine the base: 'A' (65) for uppercase, 'a' (97) for lowercase.
	// This lets us work with zero-indexed positions within each case range.
	var base rune
	if unicode.IsUpper(r) {
		base = 'A'
	} else {
		base = 'a'
	}

	// Compute zero-indexed position, apply shift with modular arithmetic.
	//
	//   pos = r - base              → 0..25
	//   shifted = (pos + shift)     → could be negative or > 25
	//   normalized = (shifted % 26 + 26) % 26  → always 0..25
	//
	pos := int(r - base)
	shifted := ((pos+shift)%26 + 26) % 26

	return base + rune(shifted)
}

// ─────────────────────────────────────────────────────────────────────────────
// ENCRYPT
// ─────────────────────────────────────────────────────────────────────────────
//
// Encrypt takes a plaintext string and a shift value, and returns the
// ciphertext. Each letter is shifted forward by `shift` positions in the
// alphabet, wrapping around from Z to A.
//
// Examples:
//
//	Encrypt("HELLO", 3)       → "KHOOR"
//	Encrypt("Hello, World!", 3) → "Khoor, Zruog!"
//	Encrypt("abc", 1)         → "bcd"
//	Encrypt("xyz", 3)         → "abc"   (wraps around)
//	Encrypt("Hi!", 0)         → "Hi!"   (shift 0 is identity)
//
// The shift can be any integer — positive, negative, or zero. Large values
// are automatically reduced modulo 26. Negative shifts shift backward
// (equivalent to decrypting).
func Encrypt(text string, shift int) string {
	// Pre-allocate a byte slice for efficiency. We process rune-by-rune
	// to correctly handle multi-byte UTF-8 characters.
	result := make([]rune, 0, len(text))

	for _, r := range text {
		result = append(result, shiftRune(r, shift))
	}

	return string(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// DECRYPT
// ─────────────────────────────────────────────────────────────────────────────
//
// Decrypt is the inverse of Encrypt. It shifts each letter *backward* by
// `shift` positions. Mathematically:
//
//	Decrypt(text, shift) = Encrypt(text, -shift)
//
// This works because:
//
//	E(x) = (x + shift) mod 26
//	D(y) = (y - shift) mod 26 = (y + (-shift)) mod 26 = E(y, -shift)
//
// Example round-trip:
//
//	plaintext  = "HELLO"
//	ciphertext = Encrypt("HELLO", 3) = "KHOOR"
//	recovered  = Decrypt("KHOOR", 3) = "HELLO"  ✓
func Decrypt(text string, shift int) string {
	return Encrypt(text, -shift)
}

// ─────────────────────────────────────────────────────────────────────────────
// ROT13
// ─────────────────────────────────────────────────────────────────────────────
//
// ROT13 is a special case of the Caesar cipher where shift = 13. Because
// 13 is exactly half of 26, ROT13 is its own inverse:
//
//	ROT13(ROT13(text)) = text
//
// This is because shifting by 13 twice gives a total shift of 26, which is
// equivalent to shift 0 (the identity).
//
//	A B C D E F G H I J K L M ← first half (13 letters)
//	N O P Q R S T U V W X Y Z ← second half (13 letters)
//
// Each letter maps to the letter 13 positions away:
//
//	A↔N  B↔O  C↔P  D↔Q  E↔R  F↔S  G↔T
//	H↔U  I↔V  J↔W  K↔X  L↔Y  M↔Z
//
// ROT13 was famously used on Usenet to hide spoilers and punchlines — the
// reader had to consciously apply ROT13 to read the hidden text.
//
// Example:
//
//	Rot13("Hello")  → "Uryyb"
//	Rot13("Uryyb")  → "Hello"   ← self-inverse!
func Rot13(text string) string {
	return Encrypt(text, 13)
}

// ─────────────────────────────────────────────────────────────────────────────
// BRUTE FORCE ATTACK
// ─────────────────────────────────────────────────────────────────────────────
//
// Since the Caesar cipher has only 25 possible non-trivial keys (shifts 1
// through 25), we can simply try all of them and let a human pick the one
// that produces readable English.
//
// This is the simplest possible cryptanalysis: exhaustive key search.
//
// For example, given ciphertext "KHOOR":
//
//	Shift  1 → JGNNQ
//	Shift  2 → IFMMP
//	Shift  3 → HELLO  ← readable English!
//	Shift  4 → GDKKN
//	...
//	Shift 25 → LIPPS

// BruteForceResult holds one candidate decryption from a brute-force attack.
// Each result pairs a shift value with the plaintext it produces.
type BruteForceResult struct {
	Shift     int    // The shift value used for decryption (1-25)
	Plaintext string // The resulting plaintext when decrypted with this shift
}

// BruteForce tries all 25 possible shifts (1 through 25) to decrypt the
// given ciphertext. It returns a slice of 25 BruteForceResult values, one
// for each possible key.
//
// Shift 0 is omitted because it produces the original ciphertext unchanged.
//
// Usage:
//
//	results := BruteForce("KHOOR")
//	for _, r := range results {
//	    fmt.Printf("Shift %2d: %s\n", r.Shift, r.Plaintext)
//	}
func BruteForce(ciphertext string) []BruteForceResult {
	results := make([]BruteForceResult, 0, 25)

	for shift := 1; shift <= 25; shift++ {
		results = append(results, BruteForceResult{
			Shift:     shift,
			Plaintext: Decrypt(ciphertext, shift),
		})
	}

	return results
}

// ─────────────────────────────────────────────────────────────────────────────
// FREQUENCY ANALYSIS
// ─────────────────────────────────────────────────────────────────────────────
//
// Frequency analysis is a more sophisticated attack. Instead of requiring a
// human to inspect 25 candidates, it automatically identifies the most likely
// shift by comparing letter frequency distributions.
//
// The idea: English text has a very distinctive letter frequency "fingerprint".
// 'E' is the most common letter (~12.7%), followed by 'T' (~9.1%), 'A' (~8.2%),
// etc. When text is Caesar-shifted, the frequencies stay the same but move to
// different letters. For example, with shift 3, the frequency of 'E' moves to
// 'H'. By trying each shift and measuring how well the resulting frequencies
// match English, we find the correct shift.
//
// We use the chi-squared (χ²) goodness-of-fit test to measure how close an
// observed frequency distribution is to the expected English distribution:
//
//	         26
//	χ² = Σ (observed_i - expected_i)²  /  expected_i
//	        i=1
//
// where:
//   - observed_i = count of letter i in the candidate plaintext
//   - expected_i = EnglishFrequencies[i] × totalLetterCount
//
// The shift that produces the LOWEST χ² value is the best match.
//
// Note: This works best on longer texts (50+ characters). Short texts may
// not have enough statistical signal for accurate detection.

// FrequencyAnalysis attempts to automatically determine the Caesar shift
// used to encrypt the given ciphertext by comparing letter frequencies
// against known English letter frequencies.
//
// It returns the best-guess shift and the corresponding plaintext.
//
// For texts with no alphabetic characters, it returns (0, ciphertext).
//
// Example:
//
//	shift, plaintext := FrequencyAnalysis("KHOOR ZRUOG")
//	// shift = 3, plaintext = "HELLO WORLD"
func FrequencyAnalysis(ciphertext string) (int, string) {
	// Count total alphabetic characters. If there are none, we can't
	// do frequency analysis — return the input unchanged.
	totalLetters := 0
	for _, r := range ciphertext {
		if unicode.IsLetter(r) && ((r >= 'A' && r <= 'Z') || (r >= 'a' && r <= 'z')) {
			totalLetters++
		}
	}

	if totalLetters == 0 {
		return 0, ciphertext
	}

	bestShift := 0
	bestChi2 := math.MaxFloat64

	// Try each of the 26 possible shifts (including 0).
	for shift := 0; shift < 26; shift++ {
		candidate := Decrypt(ciphertext, shift)

		// Count letter occurrences in the candidate plaintext.
		// We fold everything to uppercase for counting.
		var counts [26]int
		for _, r := range candidate {
			if r >= 'A' && r <= 'Z' {
				counts[r-'A']++
			} else if r >= 'a' && r <= 'z' {
				counts[r-'a']++
			}
		}

		// Compute chi-squared statistic against English frequencies.
		//
		// For each letter i (A=0, B=1, ..., Z=25):
		//   expected = EnglishFrequencies[letter] * totalLetters
		//   observed = counts[i]
		//   χ² += (observed - expected)² / expected
		//
		// A lower χ² means the distribution is closer to English.
		chi2 := 0.0
		for i := 0; i < 26; i++ {
			letter := rune('A' + i)
			expected := EnglishFrequencies[letter] * float64(totalLetters)
			observed := float64(counts[i])

			if expected > 0 {
				chi2 += (observed - expected) * (observed - expected) / expected
			}
		}

		if chi2 < bestChi2 {
			bestChi2 = chi2
			bestShift = shift
		}
	}

	return bestShift, Decrypt(ciphertext, bestShift)
}
