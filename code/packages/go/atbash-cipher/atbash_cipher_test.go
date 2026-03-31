package atbashcipher

// Comprehensive tests for the Atbash cipher implementation.
//
// These tests use Go's table-driven test pattern, which is idiomatic Go
// for testing functions with many input/output pairs. Each test case is
// a struct with a name, input, and expected output. The test runner loops
// over all cases and runs each as a subtest.

import "testing"

// TestEncrypt verifies Atbash encryption with known plaintext/ciphertext pairs.
//
// The table covers:
//   - Basic uppercase encryption
//   - Lowercase preservation
//   - Mixed case with punctuation
//   - Full alphabet reversal
//   - Digits-only (passthrough)
//   - Empty string
//   - Single characters
func TestEncrypt(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		// --- Core test cases ---
		{name: "uppercase HELLO", input: "HELLO", expected: "SVOOL"},
		{name: "lowercase hello", input: "hello", expected: "svool"},
		{name: "mixed case with punctuation", input: "Hello, World! 123", expected: "Svool, Dliow! 123"},
		{name: "full uppercase alphabet", input: "ABCDEFGHIJKLMNOPQRSTUVWXYZ", expected: "ZYXWVUTSRQPONMLKJIHGFEDCBA"},
		{name: "full lowercase alphabet", input: "abcdefghijklmnopqrstuvwxyz", expected: "zyxwvutsrqponmlkjihgfedcba"},

		// --- Non-alpha passthrough ---
		{name: "digits only", input: "12345", expected: "12345"},
		{name: "punctuation only", input: "!@#$%", expected: "!@#$%"},
		{name: "spaces only", input: "   ", expected: "   "},
		{name: "mixed alpha and digits", input: "A1B2C3", expected: "Z1Y2X3"},
		{name: "newlines and tabs", input: "A\nB\tC", expected: "Z\nY\tX"},

		// --- Edge cases ---
		{name: "empty string", input: "", expected: ""},
		{name: "single A", input: "A", expected: "Z"},
		{name: "single Z", input: "Z", expected: "A"},
		{name: "single a", input: "a", expected: "z"},
		{name: "single z", input: "z", expected: "a"},
		{name: "single M", input: "M", expected: "N"},
		{name: "single N", input: "N", expected: "M"},
		{name: "single digit", input: "5", expected: "5"},
		{name: "single space", input: " ", expected: " "},

		// --- Mixed case preservation ---
		{name: "alternating case", input: "AbCdEf", expected: "ZyXwVu"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := Encrypt(tt.input)
			if got != tt.expected {
				t.Errorf("Encrypt(%q) = %q, want %q", tt.input, got, tt.expected)
			}
		})
	}
}

// TestDecrypt verifies that Decrypt correctly reverses encryption.
func TestDecrypt(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{name: "SVOOL decrypts to HELLO", input: "SVOOL", expected: "HELLO"},
		{name: "svool decrypts to hello", input: "svool", expected: "hello"},
		{name: "mixed ciphertext", input: "Svool, Dliow! 123", expected: "Hello, World! 123"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := Decrypt(tt.input)
			if got != tt.expected {
				t.Errorf("Decrypt(%q) = %q, want %q", tt.input, got, tt.expected)
			}
		})
	}
}

// TestSelfInverse verifies the most important mathematical property:
// encrypt(encrypt(text)) == text for all inputs.
//
// This works because f(f(x)) = 25 - (25 - x) = x.
func TestSelfInverse(t *testing.T) {
	inputs := []string{
		"HELLO",
		"hello",
		"Hello, World! 123",
		"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
		"",
		"42",
		"The quick brown fox jumps over the lazy dog!",
	}

	for _, input := range inputs {
		t.Run(input, func(t *testing.T) {
			result := Encrypt(Encrypt(input))
			if result != input {
				t.Errorf("Encrypt(Encrypt(%q)) = %q, want %q", input, result, input)
			}
		})
	}
}

// TestNoLetterMapsToItself verifies that no letter in the alphabet
// maps to itself under Atbash.
//
// Mathematically, 25 - p == p only when p == 12.5, which is not an
// integer, so no letter position can satisfy this equation.
func TestNoLetterMapsToItself(t *testing.T) {
	for i := 0; i < 26; i++ {
		upper := string(rune('A' + i))
		if Encrypt(upper) == upper {
			t.Errorf("Letter %s maps to itself!", upper)
		}

		lower := string(rune('a' + i))
		if Encrypt(lower) == lower {
			t.Errorf("Letter %s maps to itself!", lower)
		}
	}
}

// TestEncryptDecryptEquivalence verifies that Encrypt and Decrypt
// produce identical output for the same input (since Atbash is self-inverse).
func TestEncryptDecryptEquivalence(t *testing.T) {
	inputs := []string{"HELLO", "svool", "Test!", "", "A1B2"}
	for _, input := range inputs {
		enc := Encrypt(input)
		dec := Decrypt(input)
		if enc != dec {
			t.Errorf("Encrypt(%q) = %q but Decrypt(%q) = %q", input, enc, input, dec)
		}
	}
}
