package vigenerecipher

import (
	"testing"
)

// longEnglishText provides enough statistical signal (~300 chars) for the
// IC-based key length estimation and chi-squared key recovery to work.
const longEnglishText = "The quick brown fox jumps over the lazy dog near the riverbank where " +
	"the tall grass sways gently in the warm summer breeze and the birds " +
	"sing their melodious songs while the sun sets behind the distant " +
	"mountains casting long shadows across the peaceful valley below and " +
	"the farmers return from the golden fields carrying baskets of fresh " +
	"wheat and corn while their children play happily in the meadows " +
	"chasing butterflies and picking wildflowers that grow abundantly " +
	"along the winding country roads that lead through the ancient forest " +
	"where owls hoot softly in the towering oak trees above the mossy " +
	"ground covered with fallen leaves and acorns from the previous autumn"

// ---------------------------------------------------------------------------
// Encryption Tests
// ---------------------------------------------------------------------------

func TestEncryptParityATTACKATDAWN(t *testing.T) {
	got, err := Encrypt("ATTACKATDAWN", "LEMON")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got != "LXFOPVEFRNHR" {
		t.Errorf("Encrypt(ATTACKATDAWN, LEMON) = %q, want %q", got, "LXFOPVEFRNHR")
	}
}

func TestEncryptParityMixedCase(t *testing.T) {
	got, err := Encrypt("Hello, World!", "key")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got != "Rijvs, Uyvjn!" {
		t.Errorf("Encrypt(Hello, World!, key) = %q, want %q", got, "Rijvs, Uyvjn!")
	}
}

func TestEncryptEmpty(t *testing.T) {
	got, err := Encrypt("", "KEY")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got != "" {
		t.Errorf("Encrypt empty = %q, want empty", got)
	}
}

func TestEncryptSingleChar(t *testing.T) {
	tests := []struct {
		plain, key, want string
	}{
		{"A", "B", "B"},
		{"Z", "A", "Z"},
		{"Z", "B", "A"},
	}
	for _, tc := range tests {
		got, err := Encrypt(tc.plain, tc.key)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if got != tc.want {
			t.Errorf("Encrypt(%q, %q) = %q, want %q", tc.plain, tc.key, got, tc.want)
		}
	}
}

func TestEncryptPreservesNonAlpha(t *testing.T) {
	got, err := Encrypt("123!@#", "key")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got != "123!@#" {
		t.Errorf("non-alpha should pass through, got %q", got)
	}
}

func TestEncryptKeyDoesNotAdvanceOnNonAlpha(t *testing.T) {
	// Key "AB" (shifts 0, 1): 'A' +0 = 'A', space passes, 'A' +1 = 'B'
	got, err := Encrypt("A A", "AB")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got != "A B" {
		t.Errorf("key should not advance on space, got %q", got)
	}
}

func TestEncryptCaseInsensitiveKey(t *testing.T) {
	r1, _ := Encrypt("HELLO", "KEY")
	r2, _ := Encrypt("HELLO", "key")
	r3, _ := Encrypt("HELLO", "Key")
	if r1 != r2 || r2 != r3 {
		t.Errorf("key should be case-insensitive: %q, %q, %q", r1, r2, r3)
	}
}

func TestEncryptInvalidKeyEmpty(t *testing.T) {
	_, err := Encrypt("hello", "")
	if err == nil {
		t.Error("expected error for empty key")
	}
}

func TestEncryptInvalidKeyNonAlpha(t *testing.T) {
	_, err := Encrypt("hello", "key1")
	if err == nil {
		t.Error("expected error for non-alpha key")
	}
}

// ---------------------------------------------------------------------------
// Decryption Tests
// ---------------------------------------------------------------------------

func TestDecryptParityATTACKATDAWN(t *testing.T) {
	got, err := Decrypt("LXFOPVEFRNHR", "LEMON")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got != "ATTACKATDAWN" {
		t.Errorf("Decrypt = %q, want ATTACKATDAWN", got)
	}
}

func TestDecryptParityMixedCase(t *testing.T) {
	got, err := Decrypt("Rijvs, Uyvjn!", "key")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got != "Hello, World!" {
		t.Errorf("Decrypt = %q, want Hello, World!", got)
	}
}

func TestDecryptEmpty(t *testing.T) {
	got, err := Decrypt("", "KEY")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got != "" {
		t.Errorf("Decrypt empty = %q, want empty", got)
	}
}

func TestDecryptInvalidKey(t *testing.T) {
	_, err := Decrypt("hello", "")
	if err == nil {
		t.Error("expected error for empty key")
	}
	_, err = Decrypt("hello", "k3y")
	if err == nil {
		t.Error("expected error for non-alpha key")
	}
}

// ---------------------------------------------------------------------------
// Round-Trip Tests
// ---------------------------------------------------------------------------

func TestRoundTrip(t *testing.T) {
	cases := []struct {
		text, key string
	}{
		{"ATTACKATDAWN", "LEMON"},
		{"Hello, World!", "key"},
		{"The quick brown fox!", "SECRET"},
		{"abcdefghijklmnopqrstuvwxyz", "Z"},
		{"AAAAAA", "ABCDEF"},
		{"12345 numbers 67890", "test"},
		{"MiXeD CaSe TeXt!!!", "MiXeD"},
		{"", "anykey"},
	}

	for _, tc := range cases {
		encrypted, err := Encrypt(tc.text, tc.key)
		if err != nil {
			t.Fatalf("Encrypt(%q, %q): %v", tc.text, tc.key, err)
		}
		decrypted, err := Decrypt(encrypted, tc.key)
		if err != nil {
			t.Fatalf("Decrypt(%q, %q): %v", encrypted, tc.key, err)
		}
		if decrypted != tc.text {
			t.Errorf("round-trip failed: %q -> encrypt -> %q -> decrypt -> %q",
				tc.text, encrypted, decrypted)
		}
	}
}

// ---------------------------------------------------------------------------
// Cryptanalysis Tests
// ---------------------------------------------------------------------------

func TestFindKeyLengthSecret(t *testing.T) {
	encrypted, _ := Encrypt(longEnglishText, "SECRET")
	got := FindKeyLength(encrypted, 20)
	if got != 6 {
		t.Errorf("FindKeyLength = %d, want 6", got)
	}
}

func TestFindKeyLengthLemon(t *testing.T) {
	encrypted, _ := Encrypt(longEnglishText, "LEMON")
	got := FindKeyLength(encrypted, 20)
	if got != 5 {
		t.Errorf("FindKeyLength = %d, want 5", got)
	}
}

func TestFindKeyLengthShort(t *testing.T) {
	encrypted, _ := Encrypt(longEnglishText, "DAWN")
	got := FindKeyLength(encrypted, 20)
	if got != 4 {
		t.Errorf("FindKeyLength = %d, want 4", got)
	}
}

func TestFindKeySecret(t *testing.T) {
	encrypted, _ := Encrypt(longEnglishText, "SECRET")
	got := FindKey(encrypted, 6)
	if got != "SECRET" {
		t.Errorf("FindKey = %q, want SECRET", got)
	}
}

func TestFindKeyLemon(t *testing.T) {
	encrypted, _ := Encrypt(longEnglishText, "LEMON")
	got := FindKey(encrypted, 5)
	if got != "LEMON" {
		t.Errorf("FindKey = %q, want LEMON", got)
	}
}

func TestBreakCipher(t *testing.T) {
	encrypted, _ := Encrypt(longEnglishText, "SECRET")
	key, plaintext, err := BreakCipher(encrypted)
	if err != nil {
		t.Fatalf("BreakCipher error: %v", err)
	}
	if key != "SECRET" {
		t.Errorf("BreakCipher key = %q, want SECRET", key)
	}
	if plaintext != longEnglishText {
		t.Errorf("BreakCipher plaintext mismatch")
	}
}

func TestBreakCipherLemon(t *testing.T) {
	encrypted, _ := Encrypt(longEnglishText, "LEMON")
	key, plaintext, err := BreakCipher(encrypted)
	if err != nil {
		t.Fatalf("BreakCipher error: %v", err)
	}
	if key != "LEMON" {
		t.Errorf("BreakCipher key = %q, want LEMON", key)
	}
	if plaintext != longEnglishText {
		t.Errorf("BreakCipher plaintext mismatch")
	}
}

// ---------------------------------------------------------------------------
// Edge Cases
// ---------------------------------------------------------------------------

func TestKeyAIsIdentity(t *testing.T) {
	text := "Hello, World!"
	got, _ := Encrypt(text, "A")
	if got != text {
		t.Errorf("key A should be identity, got %q", got)
	}
}

func TestKeyZWraps(t *testing.T) {
	got1, _ := Encrypt("A", "Z")
	if got1 != "Z" {
		t.Errorf("A + Z = %q, want Z", got1)
	}
	got2, _ := Encrypt("B", "Z")
	if got2 != "A" {
		t.Errorf("B + Z = %q, want A", got2)
	}
}

func TestOnlyNonAlpha(t *testing.T) {
	got, _ := Encrypt("123 !@# $%^", "key")
	if got != "123 !@# $%^" {
		t.Errorf("non-alpha only = %q, want unchanged", got)
	}
}
