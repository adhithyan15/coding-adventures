// Package vigenerecipher implements the Vigenere polyalphabetic substitution
// cipher with full cryptanalysis capabilities.
//
// # What is the Vigenere Cipher?
//
// The Vigenere cipher shifts each letter by a different amount determined by
// a repeating keyword. Unlike Caesar (single shift), each position uses a
// different shift based on the corresponding keyword letter (A=0, B=1, ..., Z=25).
//
// It was considered "le chiffre indechiffrable" for 300 years until Friedrich
// Kasiski published a general attack in 1863.
//
// # How Encryption Works
//
// Given plaintext P and keyword K:
//
//  1. Repeat K cyclically to match the number of alphabetic characters in P.
//  2. For each letter P[i], shift forward by K[j] positions (j only advances on letters).
//  3. Non-alphabetic characters pass through unchanged.
//
// Example: Encrypt("ATTACKATDAWN", "LEMON")
//
//	Plaintext:  A  T  T  A  C  K  A  T  D  A  W  N
//	Keyword:    L  E  M  O  N  L  E  M  O  N  L  E
//	Shift:      11 4  12 14 13 11 4  12 14 13 11 4
//	Ciphertext: L  X  F  O  P  V  E  F  R  N  H  R
//
// # Cryptanalysis
//
// Breaking the cipher uses two statistical techniques:
//   - Index of Coincidence (IC): identifies the key length by measuring
//     how "English-like" letter distributions are in grouped subsets.
//   - Chi-squared test: recovers each key letter by finding which shift
//     produces frequencies closest to standard English.
//
// This package is part of the coding-adventures monorepo.
package vigenerecipher

import (
	"errors"
	"fmt"
	"math"
	"strings"
	"unicode"
)

// ErrInvalidKey is returned when the key is empty or contains non-alphabetic characters.
var ErrInvalidKey = errors.New("invalid key")

// EnglishFrequencies contains the standard English letter frequencies (A-Z).
// These are used by the cryptanalysis functions to identify English text.
//
// The frequencies are notably uneven -- E appears ~12.7% of the time while
// Z appears only ~0.07%. This non-uniformity is what makes frequency
// analysis possible.
var EnglishFrequencies = [26]float64{
	0.08167, // A
	0.01492, // B
	0.02782, // C
	0.04253, // D
	0.12702, // E -- most common
	0.02228, // F
	0.02015, // G
	0.06094, // H
	0.06966, // I
	0.00153, // J
	0.00772, // K
	0.04025, // L
	0.02406, // M
	0.06749, // N
	0.07507, // O
	0.01929, // P
	0.00095, // Q -- rarest
	0.05987, // R
	0.06327, // S
	0.09056, // T -- second most common
	0.02758, // U
	0.00978, // V
	0.02360, // W
	0.00150, // X
	0.01974, // Y
	0.00074, // Z
}

// validateKey checks that the key is non-empty and contains only letters.
func validateKey(key string) error {
	if key == "" {
		return fmt.Errorf("%w: key must not be empty", ErrInvalidKey)
	}
	for _, r := range key {
		if !unicode.IsLetter(r) {
			return fmt.Errorf("%w: key must contain only letters, got %q", ErrInvalidKey, key)
		}
	}
	return nil
}

// keyShifts converts a key string to a slice of shift values (0-25).
// The key is treated case-insensitively: both 'a' and 'A' give shift 0.
func keyShifts(key string) []int {
	shifts := make([]int, len(key))
	for i, r := range key {
		shifts[i] = int(unicode.ToUpper(r) - 'A')
	}
	return shifts
}

// Encrypt applies the Vigenere cipher to the given plaintext.
//
// Each letter is shifted forward by the corresponding keyword letter (A=0,
// B=1, ..., Z=25). Non-alphabetic characters pass through unchanged and do
// not advance the keyword position. Case is preserved.
//
// Returns an error if the key is empty or contains non-alphabetic characters.
//
// Examples:
//
//	Encrypt("ATTACKATDAWN", "LEMON")   // "LXFOPVEFRNHR", nil
//	Encrypt("Hello, World!", "key")    // "Rijvs, Uyvjn!", nil
func Encrypt(plaintext, key string) (string, error) {
	if err := validateKey(key); err != nil {
		return "", err
	}

	shifts := keyShifts(key)
	keyLen := len(shifts)

	var result strings.Builder
	result.Grow(len(plaintext))
	keyIndex := 0

	for _, ch := range plaintext {
		if unicode.IsLetter(ch) {
			// Determine the alphabetic base: 'A' for upper, 'a' for lower
			var base rune
			if unicode.IsUpper(ch) {
				base = 'A'
			} else {
				base = 'a'
			}

			// Shift forward: (letter_pos + key_shift) mod 26
			shifted := (ch - base + rune(shifts[keyIndex%keyLen])) % 26
			result.WriteRune(base + shifted)

			keyIndex++
		} else {
			// Non-alpha: pass through, don't advance key
			result.WriteRune(ch)
		}
	}

	return result.String(), nil
}

// Decrypt reverses the Vigenere cipher.
//
// Each letter is shifted backward by the corresponding keyword letter.
// This is the exact inverse of Encrypt.
//
// Examples:
//
//	Decrypt("LXFOPVEFRNHR", "LEMON")   // "ATTACKATDAWN", nil
//	Decrypt("Rijvs, Uyvjn!", "key")    // "Hello, World!", nil
func Decrypt(ciphertext, key string) (string, error) {
	if err := validateKey(key); err != nil {
		return "", err
	}

	shifts := keyShifts(key)
	keyLen := len(shifts)

	var result strings.Builder
	result.Grow(len(ciphertext))
	keyIndex := 0

	for _, ch := range ciphertext {
		if unicode.IsLetter(ch) {
			var base rune
			if unicode.IsUpper(ch) {
				base = 'A'
			} else {
				base = 'a'
			}

			// Shift backward: (letter_pos - key_shift + 26) mod 26
			// The +26 prevents negative values before the modulo.
			shifted := (ch - base - rune(shifts[keyIndex%keyLen]) + 26) % 26
			result.WriteRune(base + shifted)

			keyIndex++
		} else {
			result.WriteRune(ch)
		}
	}

	return result.String(), nil
}

// extractAlphaUpper extracts only alphabetic characters and converts to uppercase.
// This preprocessing step is needed for cryptanalysis.
func extractAlphaUpper(text string) string {
	var b strings.Builder
	for _, r := range text {
		if unicode.IsLetter(r) {
			b.WriteRune(unicode.ToUpper(r))
		}
	}
	return b.String()
}

// indexOfCoincidence calculates the IC of a string of uppercase letters.
//
// IC measures how "English-like" a letter distribution is:
//   - English text: IC ~ 0.0667
//   - Random text:  IC ~ 0.0385 (= 1/26)
//
// Formula: IC = sum(f_i * (f_i - 1)) / (N * (N - 1))
func indexOfCoincidence(text string) float64 {
	n := len(text)
	if n < 2 {
		return 0.0
	}

	var counts [26]int
	for _, ch := range text {
		counts[ch-'A']++
	}

	numerator := 0
	for _, f := range counts {
		numerator += f * (f - 1)
	}

	return float64(numerator) / float64(n*(n-1))
}

// chiSquared computes the chi-squared statistic between observed letter
// counts and expected English frequencies.
//
// Lower chi-squared = closer match to English.
//
// Formula: chi2 = sum((O_i - E_i)^2 / E_i)
func chiSquared(counts [26]int, total int) float64 {
	if total == 0 {
		return math.Inf(1)
	}

	chi2 := 0.0
	for i := 0; i < 26; i++ {
		expected := float64(total) * EnglishFrequencies[i]
		if expected > 0 {
			diff := float64(counts[i]) - expected
			chi2 += (diff * diff) / expected
		}
	}
	return chi2
}

// FindKeyLength estimates the key length using Index of Coincidence analysis.
//
// For each candidate length k (2..maxLength), the ciphertext is split into
// k groups (every k-th letter). If k matches the actual key length, each
// group is a Caesar cipher and its IC will be close to English (~0.0667).
//
// Returns the k with the highest average IC.
func FindKeyLength(ciphertext string, maxLength int) int {
	letters := extractAlphaUpper(ciphertext)

	// Compute average IC for each candidate key length
	type icScore struct {
		k  int
		ic float64
	}
	scores := make([]icScore, 0, maxLength-1)

	for k := 2; k <= maxLength; k++ {
		// Split into k groups
		groups := make([]strings.Builder, k)
		for i, ch := range letters {
			groups[i%k].WriteRune(ch)
		}

		// Average IC across groups
		totalIC := 0.0
		for _, g := range groups {
			totalIC += indexOfCoincidence(g.String())
		}
		avgIC := totalIC / float64(k)
		scores = append(scores, icScore{k, avgIC})
	}

	// Find best IC
	bestIC := 0.0
	for _, s := range scores {
		if s.ic > bestIC {
			bestIC = s.ic
		}
	}

	// Among all key lengths within 5% of the best IC, choose the shortest.
	// This avoids selecting multiples of the true key length (e.g., 12
	// instead of 6), since multiples also produce high IC.
	threshold := bestIC * 0.95
	for _, s := range scores {
		if s.ic >= threshold {
			return s.k
		}
	}

	return 2
}

// FindKey determines each letter of the key using chi-squared analysis.
//
// For each position in the key, it extracts the group of letters at that
// position, tries all 26 shifts, and picks the shift with the lowest
// chi-squared against English frequencies.
func FindKey(ciphertext string, keyLength int) string {
	letters := extractAlphaUpper(ciphertext)

	keyChars := make([]byte, keyLength)

	for pos := 0; pos < keyLength; pos++ {
		// Extract every keyLength-th letter starting at pos
		var group []byte
		for i := pos; i < len(letters); i += keyLength {
			group = append(group, letters[i])
		}

		// Try all 26 shifts, find lowest chi-squared
		bestShift := 0
		bestChi2 := math.Inf(1)

		for shift := 0; shift < 26; shift++ {
			var counts [26]int
			for _, ch := range group {
				decrypted := (int(ch-'A') - shift + 26) % 26
				counts[decrypted]++
			}

			chi2 := chiSquared(counts, len(group))
			if chi2 < bestChi2 {
				bestChi2 = chi2
				bestShift = shift
			}
		}

		keyChars[pos] = byte('A') + byte(bestShift)
	}

	return string(keyChars)
}

// BreakCipher automatically breaks a Vigenere cipher.
//
// It combines FindKeyLength + FindKey + Decrypt into a single call.
// Works best on ciphertexts of 200+ characters.
//
// Returns the recovered key and decrypted plaintext.
func BreakCipher(ciphertext string) (string, string, error) {
	keyLength := FindKeyLength(ciphertext, 20)
	key := FindKey(ciphertext, keyLength)
	plaintext, err := Decrypt(ciphertext, key)
	if err != nil {
		return "", "", err
	}
	return key, plaintext, nil
}
