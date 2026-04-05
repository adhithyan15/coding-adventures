// Package scytalecipher implements the Scytale transposition cipher, one of
// the oldest known ciphers from ancient Sparta (~700 BCE).
//
// # What is the Scytale Cipher?
//
// Unlike substitution ciphers (Caesar, Atbash) which replace characters,
// the Scytale rearranges the positions of characters. It is a transposition
// cipher — think of it as shuffling a deck of cards rather than replacing
// the cards with different ones.
//
// The physical Scytale was a wooden rod. A strip of leather was wrapped
// around it, the message was written along the rod's length, and the strip
// was unwrapped. Only someone with a rod of the same diameter could read it.
//
// # How Encryption Works
//
// Given plaintext and a key (number of columns):
//
//  1. Write text row-by-row into a grid with `key` columns.
//  2. Pad the last row with spaces if needed.
//  3. Read the grid column-by-column to produce ciphertext.
//
// Example: Encrypt("HELLO WORLD", 3)
//
//	Grid (4 rows x 3 cols):
//	    H E L
//	    L O ' '
//	    W O R
//	    L D ' '
//
//	Columns: HLWL + EOOD + L R  = "HLWLEOODL R "
//
// # How Decryption Works
//
//  1. Calculate rows = ceil(len / key).
//  2. Write ciphertext column-by-column into the grid.
//  3. Read row-by-row and strip trailing padding spaces.
//
// # Why It's Insecure
//
// The key space is tiny: only about n/2 possible keys for a message of
// length n. BruteForce() demonstrates this by trying every key.
//
// This package is part of the coding-adventures monorepo.
package scytalecipher

import (
	"errors"
	"fmt"
	"strings"
)

// ErrInvalidKey is returned when the key is out of the valid range.
var ErrInvalidKey = errors.New("invalid key")

// Encrypt applies the Scytale transposition cipher to the given text.
//
// The text is written row-by-row into a grid with `key` columns, then
// read column-by-column. All characters are preserved (spaces, punctuation,
// digits). The last row is padded with spaces if needed.
//
// Returns an error if key < 2 or key > len(text).
//
// Examples:
//
//	Encrypt("HELLO WORLD", 3)  // returns "HLWLEOODL R ", nil
//	Encrypt("ABCDEF", 2)       // returns "ACEBDF", nil
func Encrypt(text string, key int) (string, error) {
	if text == "" {
		return "", nil
	}

	runes := []rune(text)
	n := len(runes)

	if key < 2 {
		return "", fmt.Errorf("%w: key must be >= 2, got %d", ErrInvalidKey, key)
	}
	if key > n {
		return "", fmt.Errorf("%w: key must be <= text length (%d), got %d", ErrInvalidKey, n, key)
	}

	// Calculate grid dimensions and pad
	numRows := (n + key - 1) / key // ceil(n / key)
	paddedLen := numRows * key

	// Pad the text with spaces to fill the grid
	padded := make([]rune, paddedLen)
	copy(padded, runes)
	for i := n; i < paddedLen; i++ {
		padded[i] = ' '
	}

	// Read column-by-column
	// Column c contains runes at positions: c, c+key, c+2*key, ...
	result := make([]rune, paddedLen)
	idx := 0
	for col := 0; col < key; col++ {
		for row := 0; row < numRows; row++ {
			result[idx] = padded[row*key+col]
			idx++
		}
	}

	return string(result), nil
}

// Decrypt reverses the Scytale transposition cipher.
//
// The ciphertext is written column-by-column into a grid, then read
// row-by-row. Trailing padding spaces are stripped.
//
// Returns an error if key < 2 or key > len(text).
//
// Examples:
//
//	Decrypt("HLWLEOODL R ", 3)  // returns "HELLO WORLD", nil
//	Decrypt("ACEBDF", 2)        // returns "ABCDEF", nil
func Decrypt(text string, key int) (string, error) {
	if text == "" {
		return "", nil
	}

	runes := []rune(text)
	n := len(runes)

	if key < 2 {
		return "", fmt.Errorf("%w: key must be >= 2, got %d", ErrInvalidKey, key)
	}
	if key > n {
		return "", fmt.Errorf("%w: key must be <= text length (%d), got %d", ErrInvalidKey, n, key)
	}

	// Calculate grid dimensions
	numRows := (n + key - 1) / key

	// When n is not a multiple of key (e.g. during brute-force with a
	// "wrong" key), not all columns have the same length. The first
	// (n % key) columns have numRows chars; the rest have (numRows - 1).
	// If n % key == 0, all columns have numRows chars.
	fullCols := n % key
	if fullCols == 0 {
		fullCols = key
	}

	// Compute column start indices and lengths
	colStarts := make([]int, key)
	colLens := make([]int, key)
	offset := 0
	for c := 0; c < key; c++ {
		colStarts[c] = offset
		if n%key == 0 || c < fullCols {
			colLens[c] = numRows
		} else {
			colLens[c] = numRows - 1
		}
		offset += colLens[c]
	}

	// Read row-by-row
	var result []rune
	for row := 0; row < numRows; row++ {
		for col := 0; col < key; col++ {
			if row < colLens[col] {
				result = append(result, runes[colStarts[col]+row])
			}
		}
	}

	// Strip trailing padding spaces
	return strings.TrimRight(string(result), " "), nil
}

// BruteForceResult holds one candidate decryption from a brute-force attempt.
type BruteForceResult struct {
	Key  int
	Text string
}

// BruteForce tries all possible Scytale keys from 2 to len(text)/2 and
// returns the decrypted text for each key.
//
// The Scytale has a very small key space (roughly n/2 possibilities),
// making it trivially breakable. This function demonstrates that weakness.
//
// Example:
//
//	results := BruteForce("ACEBDF")
//	// results[0] = BruteForceResult{Key: 2, Text: "ABCDEF"}
func BruteForce(text string) []BruteForceResult {
	runes := []rune(text)
	n := len(runes)
	if n < 4 {
		return nil
	}

	maxKey := n / 2
	results := make([]BruteForceResult, 0, maxKey-1)

	for candidateKey := 2; candidateKey <= maxKey; candidateKey++ {
		decrypted, err := Decrypt(text, candidateKey)
		if err != nil {
			continue
		}
		results = append(results, BruteForceResult{
			Key:  candidateKey,
			Text: decrypted,
		})
	}

	return results
}
