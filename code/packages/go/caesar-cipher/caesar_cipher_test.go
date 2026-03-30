package caesarcipher

import (
	"strings"
	"testing"
)

// ─────────────────────────────────────────────────────────────────────────────
// TABLE-DRIVEN ENCRYPT TESTS
// ─────────────────────────────────────────────────────────────────────────────
//
// Go's idiomatic testing style is "table-driven tests": define a slice of
// test cases, each with input and expected output, then loop over them.
// This makes it easy to add new cases and keeps the test logic DRY.

func TestEncrypt(t *testing.T) {
	tests := []struct {
		name     string
		text     string
		shift    int
		expected string
	}{
		// ── Classic examples ───────────────────────────────────────────
		{
			name:     "classic HELLO shift 3",
			text:     "HELLO",
			shift:    3,
			expected: "KHOOR",
		},
		{
			name:     "full alphabet shift 1",
			text:     "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
			shift:    1,
			expected: "BCDEFGHIJKLMNOPQRSTUVWXYZA",
		},
		{
			name:     "lowercase shift 3",
			text:     "hello",
			shift:    3,
			expected: "khoor",
		},

		// ── Case preservation ─────────────────────────────────────────
		{
			name:     "mixed case preserves casing",
			text:     "Hello, World!",
			shift:    3,
			expected: "Khoor, Zruog!",
		},
		{
			name:     "alternating case",
			text:     "AbCdEf",
			shift:    1,
			expected: "BcDeFg",
		},

		// ── Non-alpha passthrough ─────────────────────────────────────
		{
			name:     "digits pass through",
			text:     "ABC123",
			shift:    1,
			expected: "BCD123",
		},
		{
			name:     "punctuation and spaces pass through",
			text:     "Hello, World! 123.",
			shift:    5,
			expected: "Mjqqt, Btwqi! 123.",
		},
		{
			name:     "only non-alpha characters",
			text:     "12345!@#$%",
			shift:    7,
			expected: "12345!@#$%",
		},

		// ── Edge cases ────────────────────────────────────────────────
		{
			name:     "empty string",
			text:     "",
			shift:    5,
			expected: "",
		},
		{
			name:     "shift 0 is identity",
			text:     "HELLO",
			shift:    0,
			expected: "HELLO",
		},
		{
			name:     "shift 26 is identity (full rotation)",
			text:     "HELLO",
			shift:    26,
			expected: "HELLO",
		},
		{
			name:     "shift 52 is identity (double rotation)",
			text:     "HELLO",
			shift:    52,
			expected: "HELLO",
		},

		// ── Negative shifts ───────────────────────────────────────────
		{
			name:     "negative shift",
			text:     "KHOOR",
			shift:    -3,
			expected: "HELLO",
		},
		{
			name:     "shift -26 is identity",
			text:     "HELLO",
			shift:    -26,
			expected: "HELLO",
		},
		{
			name:     "large negative shift",
			text:     "HELLO",
			shift:    -29,
			expected: "EBIIL",
		},

		// ── Wrapping ──────────────────────────────────────────────────
		{
			name:     "Z wraps to A with shift 1",
			text:     "XYZ",
			shift:    1,
			expected: "YZA",
		},
		{
			name:     "z wraps to a with shift 1",
			text:     "xyz",
			shift:    1,
			expected: "yza",
		},
		{
			name:     "large shift wraps correctly",
			text:     "ABC",
			shift:    27,
			expected: "BCD",
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := Encrypt(tc.text, tc.shift)
			if got != tc.expected {
				t.Errorf("Encrypt(%q, %d) = %q, want %q", tc.text, tc.shift, got, tc.expected)
			}
		})
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// DECRYPT TESTS
// ─────────────────────────────────────────────────────────────────────────────

func TestDecrypt(t *testing.T) {
	tests := []struct {
		name     string
		text     string
		shift    int
		expected string
	}{
		{
			name:     "classic KHOOR shift 3",
			text:     "KHOOR",
			shift:    3,
			expected: "HELLO",
		},
		{
			name:     "lowercase",
			text:     "khoor",
			shift:    3,
			expected: "hello",
		},
		{
			name:     "mixed case with punctuation",
			text:     "Khoor, Zruog!",
			shift:    3,
			expected: "Hello, World!",
		},
		{
			name:     "empty string",
			text:     "",
			shift:    5,
			expected: "",
		},
		{
			name:     "shift 0",
			text:     "HELLO",
			shift:    0,
			expected: "HELLO",
		},
		{
			name:     "negative shift decrypts forward",
			text:     "HELLO",
			shift:    -3,
			expected: "KHOOR",
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := Decrypt(tc.text, tc.shift)
			if got != tc.expected {
				t.Errorf("Decrypt(%q, %d) = %q, want %q", tc.text, tc.shift, got, tc.expected)
			}
		})
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// ROUND-TRIP TESTS
// ─────────────────────────────────────────────────────────────────────────────
//
// The most important property: encrypting then decrypting with the same shift
// must always return the original text. We test this for many shift values.

func TestEncryptDecryptRoundTrip(t *testing.T) {
	plaintext := "The Quick Brown Fox Jumps Over The Lazy Dog! 123"

	for shift := -30; shift <= 30; shift++ {
		t.Run("", func(t *testing.T) {
			encrypted := Encrypt(plaintext, shift)
			decrypted := Decrypt(encrypted, shift)
			if decrypted != plaintext {
				t.Errorf("Round-trip failed for shift %d: got %q, want %q", shift, decrypted, plaintext)
			}
		})
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// ROT13 TESTS
// ─────────────────────────────────────────────────────────────────────────────

func TestRot13(t *testing.T) {
	tests := []struct {
		name     string
		text     string
		expected string
	}{
		{
			name:     "Hello becomes Uryyb",
			text:     "Hello",
			expected: "Uryyb",
		},
		{
			name:     "all uppercase",
			text:     "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
			expected: "NOPQRSTUVWXYZABCDEFGHIJKLM",
		},
		{
			name:     "non-alpha passthrough",
			text:     "Hello, World! 123",
			expected: "Uryyb, Jbeyq! 123",
		},
		{
			name:     "empty string",
			text:     "",
			expected: "",
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := Rot13(tc.text)
			if got != tc.expected {
				t.Errorf("Rot13(%q) = %q, want %q", tc.text, got, tc.expected)
			}
		})
	}
}

// TestRot13SelfInverse verifies the crucial property of ROT13: applying it
// twice returns the original text. This is because 13 + 13 = 26, which is
// a full rotation (identity).
func TestRot13SelfInverse(t *testing.T) {
	texts := []string{
		"Hello, World!",
		"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
		"The quick brown fox jumps over the lazy dog",
		"12345 !@#$%",
		"",
		"a",
		"Z",
	}

	for _, text := range texts {
		got := Rot13(Rot13(text))
		if got != text {
			t.Errorf("Rot13(Rot13(%q)) = %q, want %q", text, got, text)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// BRUTE FORCE TESTS
// ─────────────────────────────────────────────────────────────────────────────

func TestBruteForceReturns25Results(t *testing.T) {
	results := BruteForce("KHOOR")
	if len(results) != 25 {
		t.Fatalf("BruteForce returned %d results, want 25", len(results))
	}
}

func TestBruteForceShiftValues(t *testing.T) {
	results := BruteForce("KHOOR")

	// Verify shifts are 1 through 25 in order.
	for i, r := range results {
		expectedShift := i + 1
		if r.Shift != expectedShift {
			t.Errorf("results[%d].Shift = %d, want %d", i, r.Shift, expectedShift)
		}
	}
}

func TestBruteForceContainsCorrectPlaintext(t *testing.T) {
	// "KHOOR" was encrypted with shift 3, so decrypting with shift 3
	// should yield "HELLO".
	results := BruteForce("KHOOR")

	found := false
	for _, r := range results {
		if r.Shift == 3 && r.Plaintext == "HELLO" {
			found = true
			break
		}
	}

	if !found {
		t.Error("BruteForce(\"KHOOR\") did not contain {Shift: 3, Plaintext: \"HELLO\"}")
	}
}

func TestBruteForceEmptyString(t *testing.T) {
	results := BruteForce("")
	if len(results) != 25 {
		t.Fatalf("BruteForce(\"\") returned %d results, want 25", len(results))
	}
	for _, r := range results {
		if r.Plaintext != "" {
			t.Errorf("BruteForce(\"\") shift %d produced %q, want \"\"", r.Shift, r.Plaintext)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// FREQUENCY ANALYSIS TESTS
// ─────────────────────────────────────────────────────────────────────────────
//
// Frequency analysis needs enough text to be statistically meaningful. We
// use a passage long enough that letter frequencies approximate English.

func TestFrequencyAnalysisLongText(t *testing.T) {
	// A long English plaintext with realistic letter distribution.
	plaintext := "The quick brown fox jumps over the lazy dog. " +
		"Pack my box with five dozen liquor jugs. " +
		"How vexingly quick daft zebras jump. " +
		"The five boxing wizards jump quickly. " +
		"Sphinx of black quartz judge my vow. " +
		"Two driven jocks help fax my big quiz. " +
		"The jay pig fox dwelt on a kumquat shrub. " +
		"Watch Jeopardy Alex Trebeks fun TV quiz game. " +
		"By Jove my quick study of lexicography won a prize."

	// Try several shifts and verify frequency analysis recovers the correct one.
	for _, shift := range []int{1, 3, 7, 13, 19, 25} {
		ciphertext := Encrypt(plaintext, shift)
		detectedShift, recovered := FrequencyAnalysis(ciphertext)

		if detectedShift != shift {
			t.Errorf("FrequencyAnalysis with shift %d: detected shift %d", shift, detectedShift)
		}
		if recovered != plaintext {
			t.Errorf("FrequencyAnalysis with shift %d: recovered text doesn't match plaintext", shift)
		}
	}
}

func TestFrequencyAnalysisShift0(t *testing.T) {
	// Unshifted English text should be detected as shift 0.
	plaintext := "The quick brown fox jumps over the lazy dog several times in this sentence to give enough data"
	shift, recovered := FrequencyAnalysis(plaintext)
	if shift != 0 {
		t.Errorf("FrequencyAnalysis on unshifted text: detected shift %d, want 0", shift)
	}
	if recovered != plaintext {
		t.Errorf("FrequencyAnalysis on unshifted text: recovered text doesn't match")
	}
}

func TestFrequencyAnalysisNoAlpha(t *testing.T) {
	// Text with no alphabetic characters should return shift 0 and the
	// original text unchanged.
	text := "12345 !@#$%"
	shift, recovered := FrequencyAnalysis(text)
	if shift != 0 {
		t.Errorf("FrequencyAnalysis on non-alpha text: shift = %d, want 0", shift)
	}
	if recovered != text {
		t.Errorf("FrequencyAnalysis on non-alpha text: recovered %q, want %q", recovered, text)
	}
}

func TestFrequencyAnalysisEmptyString(t *testing.T) {
	shift, recovered := FrequencyAnalysis("")
	if shift != 0 {
		t.Errorf("FrequencyAnalysis on empty string: shift = %d, want 0", shift)
	}
	if recovered != "" {
		t.Errorf("FrequencyAnalysis on empty string: recovered %q, want \"\"", recovered)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// ENGLISH FREQUENCIES TESTS
// ─────────────────────────────────────────────────────────────────────────────

func TestEnglishFrequenciesHas26Entries(t *testing.T) {
	if len(EnglishFrequencies) != 26 {
		t.Errorf("EnglishFrequencies has %d entries, want 26", len(EnglishFrequencies))
	}
}

func TestEnglishFrequenciesSumToApproximatelyOne(t *testing.T) {
	sum := 0.0
	for _, freq := range EnglishFrequencies {
		sum += freq
	}
	// Frequencies should sum to approximately 1.0 (within rounding tolerance).
	if sum < 0.99 || sum > 1.01 {
		t.Errorf("EnglishFrequencies sum to %f, want approximately 1.0", sum)
	}
}

func TestEnglishFrequenciesEIsHighest(t *testing.T) {
	eFreq := EnglishFrequencies['E']
	for letter, freq := range EnglishFrequencies {
		if letter != 'E' && freq > eFreq {
			t.Errorf("Letter %c (%.4f) has higher frequency than E (%.4f)", letter, freq, eFreq)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// SHIFT RUNE TESTS (internal, but same package)
// ─────────────────────────────────────────────────────────────────────────────

func TestShiftRuneNonAlpha(t *testing.T) {
	// Non-alphabetic runes should pass through unchanged regardless of shift.
	nonAlpha := []rune{'0', '9', ' ', '!', '@', '#', '.', ',', '-', '\n', '\t'}
	for _, r := range nonAlpha {
		got := shiftRune(r, 5)
		if got != r {
			t.Errorf("shiftRune(%q, 5) = %q, want %q", r, got, r)
		}
	}
}

func TestShiftRuneUppercase(t *testing.T) {
	// 'A' + 3 = 'D'
	if got := shiftRune('A', 3); got != 'D' {
		t.Errorf("shiftRune('A', 3) = %c, want D", got)
	}
	// 'Z' + 1 = 'A' (wrap)
	if got := shiftRune('Z', 1); got != 'A' {
		t.Errorf("shiftRune('Z', 1) = %c, want A", got)
	}
}

func TestShiftRuneLowercase(t *testing.T) {
	// 'a' + 3 = 'd'
	if got := shiftRune('a', 3); got != 'd' {
		t.Errorf("shiftRune('a', 3) = %c, want d", got)
	}
	// 'z' + 1 = 'a' (wrap)
	if got := shiftRune('z', 1); got != 'a' {
		t.Errorf("shiftRune('z', 1) = %c, want a", got)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// SINGLE CHARACTER TESTS
// ─────────────────────────────────────────────────────────────────────────────

func TestSingleCharEncrypt(t *testing.T) {
	tests := []struct {
		char     string
		shift    int
		expected string
	}{
		{"A", 0, "A"},
		{"A", 1, "B"},
		{"A", 25, "Z"},
		{"Z", 1, "A"},
		{"a", 1, "b"},
		{"z", 1, "a"},
		{"M", 13, "Z"},
		{"N", 13, "A"},
	}

	for _, tc := range tests {
		got := Encrypt(tc.char, tc.shift)
		if got != tc.expected {
			t.Errorf("Encrypt(%q, %d) = %q, want %q", tc.char, tc.shift, got, tc.expected)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// UNICODE HANDLING
// ─────────────────────────────────────────────────────────────────────────────
//
// Non-ASCII letters (accented, Cyrillic, etc.) pass through unchanged.
// Only A-Z and a-z are shifted.

func TestNonASCIILettersPassThrough(t *testing.T) {
	// These contain non-ASCII Unicode letters that should not be shifted.
	text := "cafe\u0301 nai\u0308ve" // café naïve with combining characters
	got := Encrypt(text, 3)
	// The ASCII letters should be shifted, combining characters pass through.
	// c→f, a→d, f→i, e→h, n→q, a→d, i→l, v→y, e→h
	expected := "fdih\u0301 qdl\u0308yh"
	if got != expected {
		t.Errorf("Encrypt(%q, 3) = %q, want %q", text, got, expected)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// LONG STRING TEST
// ─────────────────────────────────────────────────────────────────────────────

func TestLongStringRoundTrip(t *testing.T) {
	// Build a long string with all printable ASCII characters repeated.
	var b strings.Builder
	for i := 0; i < 1000; i++ {
		b.WriteRune(rune(32 + (i % 95))) // printable ASCII range
	}
	longText := b.String()

	for _, shift := range []int{1, 13, 25} {
		encrypted := Encrypt(longText, shift)
		decrypted := Decrypt(encrypted, shift)
		if decrypted != longText {
			t.Errorf("Round-trip failed for long string with shift %d", shift)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// BENCHMARK
// ─────────────────────────────────────────────────────────────────────────────

func BenchmarkEncrypt(b *testing.B) {
	text := "The Quick Brown Fox Jumps Over The Lazy Dog"
	for i := 0; i < b.N; i++ {
		Encrypt(text, 13)
	}
}

func BenchmarkBruteForce(b *testing.B) {
	ciphertext := "Wkh Txlfn Eurzq Ira Mxpsv Ryhu Wkh Odcb Grj"
	for i := 0; i < b.N; i++ {
		BruteForce(ciphertext)
	}
}

func BenchmarkFrequencyAnalysis(b *testing.B) {
	ciphertext := Encrypt(
		"The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs.",
		7,
	)
	for i := 0; i < b.N; i++ {
		FrequencyAnalysis(ciphertext)
	}
}
