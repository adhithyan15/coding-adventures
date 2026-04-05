package scytalecipher

// Comprehensive tests for the Scytale cipher implementation.
//
// These tests use Go's table-driven test pattern to verify encryption,
// decryption, round-trip correctness, key validation, and brute-force
// decryption.

import (
	"errors"
	"testing"
)

// TestEncrypt verifies Scytale encryption with known plaintext/key/ciphertext triples.
func TestEncrypt(t *testing.T) {
	tests := []struct {
		name     string
		text     string
		key      int
		expected string
	}{
		{name: "HELLO WORLD key=3", text: "HELLO WORLD", key: 3, expected: "HLWLEOODL R "},
		{name: "ABCDEF key=2", text: "ABCDEF", key: 2, expected: "ACEBDF"},
		{name: "ABCDEF key=3", text: "ABCDEF", key: 3, expected: "ADBECF"},
		{name: "ABCDEFGH key=4", text: "ABCDEFGH", key: 4, expected: "AEBFCGDH"},
		{name: "key equals length", text: "ABCD", key: 4, expected: "ABCD"},
		{name: "all spaces", text: "    ", key: 2, expected: "    "},
		{name: "with digits", text: "A1B2C3", key: 2, expected: "ABC123"},
		{name: "empty string", text: "", key: 2, expected: ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := Encrypt(tt.text, tt.key)
			if err != nil {
				t.Fatalf("Encrypt(%q, %d) returned unexpected error: %v", tt.text, tt.key, err)
			}
			if got != tt.expected {
				t.Errorf("Encrypt(%q, %d) = %q, want %q", tt.text, tt.key, got, tt.expected)
			}
		})
	}
}

// TestDecrypt verifies Scytale decryption with known ciphertext/key/plaintext triples.
func TestDecrypt(t *testing.T) {
	tests := []struct {
		name     string
		text     string
		key      int
		expected string
	}{
		{name: "HELLO WORLD key=3", text: "HLWLEOODL R ", key: 3, expected: "HELLO WORLD"},
		{name: "ACEBDF key=2", text: "ACEBDF", key: 2, expected: "ABCDEF"},
		{name: "ADBECF key=3", text: "ADBECF", key: 3, expected: "ABCDEF"},
		{name: "key equals length", text: "ABCD", key: 4, expected: "ABCD"},
		{name: "empty string", text: "", key: 2, expected: ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := Decrypt(tt.text, tt.key)
			if err != nil {
				t.Fatalf("Decrypt(%q, %d) returned unexpected error: %v", tt.text, tt.key, err)
			}
			if got != tt.expected {
				t.Errorf("Decrypt(%q, %d) = %q, want %q", tt.text, tt.key, got, tt.expected)
			}
		})
	}
}

// TestRoundTrip verifies decrypt(encrypt(text, key), key) == text.
func TestRoundTrip(t *testing.T) {
	cases := []struct {
		text string
		key  int
	}{
		{"HELLO WORLD", 3},
		{"ABCDEF", 2},
		{"ABCDEF", 3},
		{"The quick brown fox", 4},
		{"12345", 2},
		{"AB", 2},
		{"ABCDEFGHIJKLMNOP", 4},
		{"Test with spaces and 123!", 5},
	}

	for _, tc := range cases {
		t.Run(tc.text, func(t *testing.T) {
			ct, err := Encrypt(tc.text, tc.key)
			if err != nil {
				t.Fatalf("Encrypt error: %v", err)
			}
			pt, err := Decrypt(ct, tc.key)
			if err != nil {
				t.Fatalf("Decrypt error: %v", err)
			}
			if pt != tc.text {
				t.Errorf("Round trip failed: got %q, want %q", pt, tc.text)
			}
		})
	}
}

// TestRoundTripAllKeys tests round-trip for all valid keys on a fixed string.
func TestRoundTripAllKeys(t *testing.T) {
	text := "The quick brown fox jumps over the lazy dog!"
	n := len([]rune(text))
	for key := 2; key <= n/2; key++ {
		ct, err := Encrypt(text, key)
		if err != nil {
			t.Fatalf("key=%d: Encrypt error: %v", key, err)
		}
		pt, err := Decrypt(ct, key)
		if err != nil {
			t.Fatalf("key=%d: Decrypt error: %v", key, err)
		}
		if pt != text {
			t.Errorf("key=%d: got %q, want %q", key, pt, text)
		}
	}
}

// TestEncryptInvalidKey checks that invalid keys return errors.
func TestEncryptInvalidKey(t *testing.T) {
	tests := []struct {
		name string
		text string
		key  int
	}{
		{name: "key=0", text: "HELLO", key: 0},
		{name: "key=1", text: "HELLO", key: 1},
		{name: "key=-1", text: "HELLO", key: -1},
		{name: "key > length", text: "HI", key: 3},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := Encrypt(tt.text, tt.key)
			if err == nil {
				t.Errorf("Encrypt(%q, %d) should have returned an error", tt.text, tt.key)
			}
			if !errors.Is(err, ErrInvalidKey) {
				t.Errorf("Expected ErrInvalidKey, got: %v", err)
			}
		})
	}
}

// TestDecryptInvalidKey checks that invalid keys return errors for decryption.
func TestDecryptInvalidKey(t *testing.T) {
	_, err := Decrypt("HELLO", 0)
	if err == nil {
		t.Error("Decrypt with key=0 should error")
	}
	_, err = Decrypt("HI", 3)
	if err == nil {
		t.Error("Decrypt with key > length should error")
	}
}

// TestBruteForce verifies the brute-force function.
func TestBruteForce(t *testing.T) {
	t.Run("finds original text", func(t *testing.T) {
		original := "HELLO WORLD"
		key := 3
		ct, _ := Encrypt(original, key)
		results := BruteForce(ct)

		found := false
		for _, r := range results {
			if r.Key == key && r.Text == original {
				found = true
				break
			}
		}
		if !found {
			t.Error("BruteForce did not find the original text with the correct key")
		}
	})

	t.Run("returns all keys 2 to n/2", func(t *testing.T) {
		results := BruteForce("ABCDEFGHIJ") // 10 chars
		if len(results) != 4 {              // keys 2,3,4,5
			t.Errorf("Expected 4 results, got %d", len(results))
		}
		expectedKeys := []int{2, 3, 4, 5}
		for i, r := range results {
			if r.Key != expectedKeys[i] {
				t.Errorf("Result %d: expected key %d, got %d", i, expectedKeys[i], r.Key)
			}
		}
	})

	t.Run("short text returns nil", func(t *testing.T) {
		results := BruteForce("AB")
		if results != nil {
			t.Errorf("Expected nil for short text, got %v", results)
		}
	})
}

// TestPadding verifies padding behavior.
func TestPadding(t *testing.T) {
	t.Run("no padding needed", func(t *testing.T) {
		ct, _ := Encrypt("ABCDEF", 2)
		if len(ct) != 6 {
			t.Errorf("Expected length 6, got %d", len(ct))
		}
	})

	t.Run("padding added", func(t *testing.T) {
		ct, _ := Encrypt("HELLO", 3) // 5 chars -> 6 padded
		if len(ct) != 6 {
			t.Errorf("Expected length 6, got %d", len(ct))
		}
	})

	t.Run("padding stripped on decrypt", func(t *testing.T) {
		ct, _ := Encrypt("HELLO", 3)
		pt, _ := Decrypt(ct, 3)
		if pt != "HELLO" {
			t.Errorf("Expected HELLO, got %q", pt)
		}
	})
}
