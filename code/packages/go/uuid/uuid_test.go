// Package ca_uuid tests — comprehensive coverage for all UUID versions.
//
// # Testing Philosophy
//
// We test three categories of behavior:
//
//  1. RFC compliance: the standard defines exact test vectors for v3 and v5
//     that every conforming implementation must produce. These tests MUST pass.
//
//  2. Structural correctness: version and variant bits must be set correctly,
//     strings must match the 8-4-4-4-12 format, nil/max sentinels must work.
//
//  3. Statistical properties: v4 UUIDs must be unique, v7 UUIDs must be
//     time-ordered. We test these with small samples since exhaustive testing
//     of 122-bit random space is impractical.
//
// # RFC Test Vectors
//
// RFC 4122 Appendix B and RFC 9562 specify:
//
//	V3(NamespaceDNS, "python.org") = "6fa459ea-ee8a-3ca4-894e-db77e160355e"
//	V5(NamespaceDNS, "python.org") = "886313e1-3b8a-5372-9b90-0c9aee199e5d"
//
// These vectors are the gold standard for testing UUID implementations.
package ca_uuid

import (
	"strings"
	"testing"
	"time"
)

// ─── Parse Tests ─────────────────────────────────────────────────────────────

// TestParseStandard verifies that the canonical lowercase hyphenated form parses
// correctly and round-trips back to the same string.
func TestParseStandard(t *testing.T) {
	input := "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
	u, err := Parse(input)
	if err != nil {
		t.Fatalf("Parse(%q) returned unexpected error: %v", input, err)
	}
	if got := u.String(); got != input {
		t.Errorf("String() = %q, want %q", got, input)
	}
}

// TestParseUppercase verifies that uppercase hex digits are accepted.
// RFC 4122 is case-insensitive on parse; the canonical output is lowercase.
func TestParseUppercase(t *testing.T) {
	input := "6BA7B810-9DAD-11D1-80B4-00C04FD430C8"
	u, err := Parse(input)
	if err != nil {
		t.Fatalf("Parse(%q) returned unexpected error: %v", input, err)
	}
	want := "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
	if got := u.String(); got != want {
		t.Errorf("String() = %q, want %q", got, want)
	}
}

// TestParseCompact verifies the 32-character hex string without hyphens.
func TestParseCompact(t *testing.T) {
	compact := "6ba7b8109dad11d180b400c04fd430c8"
	u, err := Parse(compact)
	if err != nil {
		t.Fatalf("Parse(%q) returned unexpected error: %v", compact, err)
	}
	want := "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
	if got := u.String(); got != want {
		t.Errorf("String() = %q, want %q", got, want)
	}
}

// TestParseBraced verifies the Microsoft GUID braced form {xxxxxxxx-...}.
func TestParseBraced(t *testing.T) {
	braced := "{6ba7b810-9dad-11d1-80b4-00c04fd430c8}"
	u, err := Parse(braced)
	if err != nil {
		t.Fatalf("Parse(%q) returned unexpected error: %v", braced, err)
	}
	want := "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
	if got := u.String(); got != want {
		t.Errorf("String() = %q, want %q", got, want)
	}
}

// TestParseURN verifies the URN form urn:uuid:xxxxxxxx-...
func TestParseURN(t *testing.T) {
	urn := "urn:uuid:6ba7b810-9dad-11d1-80b4-00c04fd430c8"
	u, err := Parse(urn)
	if err != nil {
		t.Fatalf("Parse(%q) returned unexpected error: %v", urn, err)
	}
	want := "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
	if got := u.String(); got != want {
		t.Errorf("String() = %q, want %q", got, want)
	}
}

// TestParseLeadingTrailingWhitespace verifies that surrounding whitespace is
// ignored, as the regex allows optional \s* at boundaries.
func TestParseLeadingTrailingWhitespace(t *testing.T) {
	input := "  6ba7b810-9dad-11d1-80b4-00c04fd430c8  "
	u, err := Parse(input)
	if err != nil {
		t.Fatalf("Parse(%q) returned unexpected error: %v", input, err)
	}
	want := "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
	if got := u.String(); got != want {
		t.Errorf("String() = %q, want %q", got, want)
	}
}

// TestParseInvalidEmpty verifies that an empty string is rejected.
func TestParseInvalidEmpty(t *testing.T) {
	_, err := Parse("")
	if err == nil {
		t.Error("Parse(\"\") should have returned an error, got nil")
	}
}

// TestParseInvalidTooShort verifies that a truncated UUID string is rejected.
func TestParseInvalidTooShort(t *testing.T) {
	_, err := Parse("6ba7b810-9dad-11d1-80b4")
	if err == nil {
		t.Error("Parse of truncated UUID should have returned an error")
	}
}

// TestParseInvalidBadChars verifies that non-hex characters are rejected.
func TestParseInvalidBadChars(t *testing.T) {
	_, err := Parse("6ba7b810-9dad-11d1-80b4-00c04fd430zz")
	if err == nil {
		t.Error("Parse with non-hex chars should have returned an error")
	}
}

// TestParseInvalidGarbage verifies that completely garbage input is rejected.
func TestParseInvalidGarbage(t *testing.T) {
	_, err := Parse("not-a-uuid-at-all")
	if err == nil {
		t.Error("Parse of garbage string should have returned an error")
	}
}

// ─── IsValid Tests ───────────────────────────────────────────────────────────

// TestIsValid checks that IsValid returns true for valid UUIDs and false for
// invalid ones.
func TestIsValid(t *testing.T) {
	tests := []struct {
		input string
		valid bool
	}{
		{"6ba7b810-9dad-11d1-80b4-00c04fd430c8", true},
		{"6BA7B810-9DAD-11D1-80B4-00C04FD430C8", true},
		{"6ba7b8109dad11d180b400c04fd430c8", true},
		{"{6ba7b810-9dad-11d1-80b4-00c04fd430c8}", true},
		{"urn:uuid:6ba7b810-9dad-11d1-80b4-00c04fd430c8", true},
		{"", false},
		{"not-valid", false},
		{"6ba7b810-9dad-11d1-80b4", false},
	}
	for _, tt := range tests {
		got := IsValid(tt.input)
		if got != tt.valid {
			t.Errorf("IsValid(%q) = %v, want %v", tt.input, got, tt.valid)
		}
	}
}

// ─── String Tests ────────────────────────────────────────────────────────────

// TestString verifies the canonical 8-4-4-4-12 lowercase output format.
func TestString(t *testing.T) {
	u := NamespaceDNS
	s := u.String()

	// Must be exactly 36 characters: 32 hex + 4 hyphens.
	if len(s) != 36 {
		t.Errorf("String() length = %d, want 36", len(s))
	}

	// Hyphens must be at positions 8, 13, 18, 23.
	for _, pos := range []int{8, 13, 18, 23} {
		if s[pos] != '-' {
			t.Errorf("String()[%d] = %q, want '-'", pos, s[pos])
		}
	}

	// Must be lowercase.
	if s != strings.ToLower(s) {
		t.Errorf("String() = %q is not all lowercase", s)
	}

	want := "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
	if s != want {
		t.Errorf("String() = %q, want %q", s, want)
	}
}

// ─── ToInt Tests ─────────────────────────────────────────────────────────────

// TestToInt verifies the big-endian (hi, lo) uint64 decomposition.
func TestToInt(t *testing.T) {
	// Nil UUID: both halves must be 0.
	hi, lo := Nil.ToInt()
	if hi != 0 || lo != 0 {
		t.Errorf("Nil.ToInt() = (%d, %d), want (0, 0)", hi, lo)
	}

	// Max UUID: both halves must be all-bits-set.
	hi, lo = Max.ToInt()
	const maxU64 = ^uint64(0)
	if hi != maxU64 || lo != maxU64 {
		t.Errorf("Max.ToInt() = (0x%x, 0x%x), want (0x%x, 0x%x)", hi, lo, maxU64, maxU64)
	}
}

// ─── Version Tests ───────────────────────────────────────────────────────────

// TestVersionV1 verifies that V1() UUIDs report version 1.
func TestVersionV1(t *testing.T) {
	u, err := V1()
	if err != nil {
		t.Fatalf("V1() error: %v", err)
	}
	if v := u.Version(); v != 1 {
		t.Errorf("V1().Version() = %d, want 1", v)
	}
}

// TestVersionV3 verifies that V3() UUIDs report version 3.
func TestVersionV3(t *testing.T) {
	u := V3(NamespaceDNS, "test")
	if v := u.Version(); v != 3 {
		t.Errorf("V3().Version() = %d, want 3", v)
	}
}

// TestVersionV4 verifies that V4() UUIDs report version 4.
func TestVersionV4(t *testing.T) {
	u, err := V4()
	if err != nil {
		t.Fatalf("V4() error: %v", err)
	}
	if v := u.Version(); v != 4 {
		t.Errorf("V4().Version() = %d, want 4", v)
	}
}

// TestVersionV5 verifies that V5() UUIDs report version 5.
func TestVersionV5(t *testing.T) {
	u := V5(NamespaceDNS, "test")
	if v := u.Version(); v != 5 {
		t.Errorf("V5().Version() = %d, want 5", v)
	}
}

// TestVersionV7 verifies that V7() UUIDs report version 7.
func TestVersionV7(t *testing.T) {
	u, err := V7()
	if err != nil {
		t.Fatalf("V7() error: %v", err)
	}
	if v := u.Version(); v != 7 {
		t.Errorf("V7().Version() = %d, want 7", v)
	}
}

// ─── Variant Tests ───────────────────────────────────────────────────────────

// TestVariantRFC4122 verifies that all UUIDs generated by this package report
// the RFC 4122 variant ("rfc4122"). This is the correct variant for all modern
// UUIDs (v1, v3, v4, v5, v7).
func TestVariantRFC4122(t *testing.T) {
	generators := []struct {
		name string
		fn   func() UUID
	}{
		{"V1", func() UUID { u, _ := V1(); return u }},
		{"V3", func() UUID { return V3(NamespaceDNS, "test") }},
		{"V4", func() UUID { u, _ := V4(); return u }},
		{"V5", func() UUID { return V5(NamespaceDNS, "test") }},
		{"V7", func() UUID { u, _ := V7(); return u }},
	}
	for _, g := range generators {
		u := g.fn()
		if v := u.Variant(); v != "rfc4122" {
			t.Errorf("%s Variant() = %q, want %q", g.name, v, "rfc4122")
		}
	}
}

// TestVariantNCS verifies detection of the NCS variant (top bit = 0).
func TestVariantNCS(t *testing.T) {
	var u UUID
	u[8] = 0x00 // top bit 0 → NCS
	if v := u.Variant(); v != "ncs" {
		t.Errorf("Variant() = %q, want \"ncs\"", v)
	}
}

// TestVariantMicrosoft verifies detection of the Microsoft variant (top 3 bits = 110).
func TestVariantMicrosoft(t *testing.T) {
	var u UUID
	u[8] = 0xC0 // 1100 0000 → Microsoft
	if v := u.Variant(); v != "microsoft" {
		t.Errorf("Variant() = %q, want \"microsoft\"", v)
	}
}

// TestVariantReserved verifies detection of the reserved variant (top 3 bits = 111).
func TestVariantReserved(t *testing.T) {
	var u UUID
	u[8] = 0xE0 // 1110 0000 → Reserved
	if v := u.Variant(); v != "reserved" {
		t.Errorf("Variant() = %q, want \"reserved\"", v)
	}
}

// ─── Nil and Max Tests ───────────────────────────────────────────────────────

// TestNil verifies the nil UUID sentinel value.
func TestNil(t *testing.T) {
	if !Nil.IsNil() {
		t.Error("Nil.IsNil() = false, want true")
	}
	if Nil.IsMax() {
		t.Error("Nil.IsMax() = true, want false")
	}
	want := "00000000-0000-0000-0000-000000000000"
	if got := Nil.String(); got != want {
		t.Errorf("Nil.String() = %q, want %q", got, want)
	}
}

// TestMax verifies the max UUID sentinel value.
func TestMax(t *testing.T) {
	if !Max.IsMax() {
		t.Error("Max.IsMax() = false, want true")
	}
	if Max.IsNil() {
		t.Error("Max.IsNil() = true, want false")
	}
	want := "ffffffff-ffff-ffff-ffff-ffffffffffff"
	if got := Max.String(); got != want {
		t.Errorf("Max.String() = %q, want %q", got, want)
	}
}

// TestIsNilFalseForNonNil verifies that a non-nil UUID correctly reports IsNil
// as false.
func TestIsNilFalseForNonNil(t *testing.T) {
	u, _ := V4()
	if u.IsNil() {
		t.Error("V4 UUID reported IsNil() = true; extremely unlikely for random UUID")
	}
}

// ─── Namespace Constant Tests ─────────────────────────────────────────────────

// TestNamespaceConstants verifies that the four RFC 4122 namespace UUIDs match
// the values specified in RFC 4122 Appendix C.
func TestNamespaceConstants(t *testing.T) {
	tests := []struct {
		name string
		uuid UUID
		want string
	}{
		{"DNS", NamespaceDNS, "6ba7b810-9dad-11d1-80b4-00c04fd430c8"},
		{"URL", NamespaceURL, "6ba7b811-9dad-11d1-80b4-00c04fd430c8"},
		{"OID", NamespaceOID, "6ba7b812-9dad-11d1-80b4-00c04fd430c8"},
		{"X500", NamespaceX500, "6ba7b814-9dad-11d1-80b4-00c04fd430c8"},
	}
	for _, tt := range tests {
		if got := tt.uuid.String(); got != tt.want {
			t.Errorf("Namespace%s = %q, want %q", tt.name, got, tt.want)
		}
	}
}

// ─── UUID v4 Tests ───────────────────────────────────────────────────────────

// TestV4Version verifies the version field is 4.
func TestV4Version(t *testing.T) {
	for i := 0; i < 10; i++ {
		u, err := V4()
		if err != nil {
			t.Fatalf("V4() error: %v", err)
		}
		if v := u.Version(); v != 4 {
			t.Errorf("V4 UUID has version %d, want 4", v)
		}
	}
}

// TestV4Uniqueness generates 1000 v4 UUIDs and verifies they are all distinct.
// The probability of a collision in 1000 random 122-bit values is
// approximately 1000^2 / 2^123 ≈ 10^-31, effectively impossible.
func TestV4Uniqueness(t *testing.T) {
	seen := make(map[UUID]struct{}, 1000)
	for i := 0; i < 1000; i++ {
		u, err := V4()
		if err != nil {
			t.Fatalf("V4() error on iteration %d: %v", i, err)
		}
		if _, exists := seen[u]; exists {
			t.Fatalf("V4() collision at iteration %d: %s", i, u)
		}
		seen[u] = struct{}{}
	}
}

// TestV4NotNil verifies that V4 does not produce the nil UUID.
// (Astronomically unlikely but we check the logic is correct.)
func TestV4NotNil(t *testing.T) {
	u, err := V4()
	if err != nil {
		t.Fatalf("V4() error: %v", err)
	}
	if u.IsNil() {
		t.Error("V4() produced nil UUID")
	}
}

// ─── UUID v5 RFC Test Vector ─────────────────────────────────────────────────

// TestV5RFC is the critical RFC compliance test. Every conforming UUID v5
// implementation must produce this exact output for these inputs.
//
// Source: RFC 4122 Appendix B (confirmed in RFC 9562 Appendix B.3):
//
//	namespace = NamespaceDNS = 6ba7b810-9dad-11d1-80b4-00c04fd430c8
//	name      = "python.org"
//	expected  = 886313e1-3b8a-5372-9b90-0c9aee199e5d
func TestV5RFC(t *testing.T) {
	want := "886313e1-3b8a-5372-9b90-0c9aee199e5d"
	u := V5(NamespaceDNS, "python.org")
	if got := u.String(); got != want {
		t.Errorf("V5(NamespaceDNS, \"python.org\") = %q, want %q", got, want)
	}
}

// TestV5Determinism verifies that V5 is deterministic: same inputs → same output.
func TestV5Determinism(t *testing.T) {
	a := V5(NamespaceDNS, "example.com")
	b := V5(NamespaceDNS, "example.com")
	if a != b {
		t.Errorf("V5 is not deterministic: %s != %s", a, b)
	}
}

// TestV5NamespaceSeparation verifies that different namespaces produce different
// UUIDs for the same name.
func TestV5NamespaceSeparation(t *testing.T) {
	uDNS := V5(NamespaceDNS, "example.com")
	uURL := V5(NamespaceURL, "example.com")
	if uDNS == uURL {
		t.Error("V5 with different namespaces produced the same UUID")
	}
}

// TestV5NameSeparation verifies that different names produce different UUIDs
// within the same namespace.
func TestV5NameSeparation(t *testing.T) {
	u1 := V5(NamespaceDNS, "example.com")
	u2 := V5(NamespaceDNS, "other.com")
	if u1 == u2 {
		t.Error("V5 with different names produced the same UUID")
	}
}

// ─── UUID v3 RFC Test Vector ─────────────────────────────────────────────────

// TestV3RFC is the critical RFC compliance test for v3. Every conforming UUID v3
// implementation must produce this exact output.
//
// Source: RFC 4122 Appendix B (confirmed in RFC 9562 Appendix B.1):
//
//	namespace = NamespaceDNS = 6ba7b810-9dad-11d1-80b4-00c04fd430c8
//	name      = "python.org"
//	expected  = 6fa459ea-ee8a-3ca4-894e-db77e160355e
func TestV3RFC(t *testing.T) {
	want := "6fa459ea-ee8a-3ca4-894e-db77e160355e"
	u := V3(NamespaceDNS, "python.org")
	if got := u.String(); got != want {
		t.Errorf("V3(NamespaceDNS, \"python.org\") = %q, want %q", got, want)
	}
}

// TestV3Determinism verifies that V3 is deterministic.
func TestV3Determinism(t *testing.T) {
	a := V3(NamespaceDNS, "example.com")
	b := V3(NamespaceDNS, "example.com")
	if a != b {
		t.Errorf("V3 is not deterministic: %s != %s", a, b)
	}
}

// TestV3DifferentFromV5 verifies that V3 and V5 produce different UUIDs for the
// same inputs, as they use different hash algorithms.
func TestV3DifferentFromV5(t *testing.T) {
	v3 := V3(NamespaceDNS, "python.org")
	v5 := V5(NamespaceDNS, "python.org")
	if v3 == v5 {
		t.Error("V3 and V5 produced the same UUID (different algorithms should differ)")
	}
}

// ─── UUID v1 Tests ───────────────────────────────────────────────────────────

// TestV1Variant verifies the RFC 4122 variant.
func TestV1Variant(t *testing.T) {
	u, err := V1()
	if err != nil {
		t.Fatalf("V1() error: %v", err)
	}
	if v := u.Variant(); v != "rfc4122" {
		t.Errorf("V1 Variant() = %q, want \"rfc4122\"", v)
	}
}

// TestV1TimestampReasonable verifies that the V1 timestamp is in a plausible
// range: not zero and not impossibly far in the future. The 60-bit timestamp
// encodes 100ns ticks since 1582-10-15. We compute expected bounds.
func TestV1TimestampReasonable(t *testing.T) {
	u, err := V1()
	if err != nil {
		t.Fatalf("V1() error: %v", err)
	}

	// The version nibble occupies the top 4 bits of bytes 6-7. To extract the
	// raw 12-bit time_hi we mask off the version nibble.
	timeHi := uint64(u[6]&0x0F)<<8 | uint64(u[7])
	timeMid := uint64(u[4])<<8 | uint64(u[5])
	timeLow := uint64(u[0])<<24 | uint64(u[1])<<16 | uint64(u[2])<<8 | uint64(u[3])

	// Reconstruct the 60-bit Gregorian timestamp.
	ts := (timeHi << 48) | (timeMid << 32) | timeLow

	// Subtract the Gregorian offset to get Unix time in 100ns ticks.
	unixTicks := ts - gregorianOffset
	unixNano := int64(unixTicks) * 100

	// The timestamp should be within ±1 second of now.
	nowNano := time.Now().UnixNano()
	diff := unixNano - nowNano
	if diff < 0 {
		diff = -diff
	}
	if diff > int64(time.Second) {
		t.Errorf("V1 timestamp differs from now by %v (expected < 1s)", time.Duration(diff))
	}
}

// TestV1NodeMulticastBit verifies that the multicast bit is set in the node
// field, indicating a randomly generated node ID (not a real MAC address).
func TestV1NodeMulticastBit(t *testing.T) {
	u, err := V1()
	if err != nil {
		t.Fatalf("V1() error: %v", err)
	}
	if u[10]&0x01 == 0 {
		t.Error("V1 node[0] multicast bit (LSB) is not set; should be 1 for random node")
	}
}

// TestV1Uniqueness generates 100 V1 UUIDs and verifies they are distinct.
func TestV1Uniqueness(t *testing.T) {
	seen := make(map[UUID]struct{}, 100)
	for i := 0; i < 100; i++ {
		u, err := V1()
		if err != nil {
			t.Fatalf("V1() error on iteration %d: %v", i, err)
		}
		if _, exists := seen[u]; exists {
			t.Fatalf("V1() collision at iteration %d: %s", i, u)
		}
		seen[u] = struct{}{}
	}
}

// ─── UUID v7 Tests ───────────────────────────────────────────────────────────

// TestV7TimeOrdering verifies that UUIDs generated later compare greater than
// UUIDs generated earlier. V7's design goal is chronological sortability.
//
// We generate pairs with a small sleep between them to ensure the millisecond
// timestamp actually advances. (Go's time.Now() typically has sub-ms precision
// but the sleep ensures the test is not flaky on slow machines.)
func TestV7TimeOrdering(t *testing.T) {
	u1, err := V7()
	if err != nil {
		t.Fatalf("V7() first call error: %v", err)
	}
	time.Sleep(2 * time.Millisecond)
	u2, err := V7()
	if err != nil {
		t.Fatalf("V7() second call error: %v", err)
	}

	// Compare as byte arrays: the 48-bit timestamp occupies the most significant
	// bytes, so lexicographic comparison equals chronological comparison.
	if Compare(u1, u2) >= 0 {
		t.Errorf("V7 time ordering violated: %s >= %s", u1, u2)
	}
}

// TestV7Uniqueness generates 1000 v7 UUIDs and verifies they are all distinct.
func TestV7Uniqueness(t *testing.T) {
	seen := make(map[UUID]struct{}, 1000)
	for i := 0; i < 1000; i++ {
		u, err := V7()
		if err != nil {
			t.Fatalf("V7() error on iteration %d: %v", i, err)
		}
		if _, exists := seen[u]; exists {
			t.Fatalf("V7() collision at iteration %d: %s", i, u)
		}
		seen[u] = struct{}{}
	}
}

// TestV7TimestampReasonable verifies that the V7 timestamp in bytes 0-5 is
// within ±1 second of the current Unix time in milliseconds.
func TestV7TimestampReasonable(t *testing.T) {
	u, err := V7()
	if err != nil {
		t.Fatalf("V7() error: %v", err)
	}

	// Extract the 48-bit millisecond timestamp from the first 6 bytes.
	tsMs := int64(u[0])<<40 | int64(u[1])<<32 | int64(u[2])<<24 |
		int64(u[3])<<16 | int64(u[4])<<8 | int64(u[5])

	nowMs := time.Now().UnixNano() / 1_000_000
	diff := tsMs - nowMs
	if diff < 0 {
		diff = -diff
	}
	if diff > 1000 { // more than 1 second off
		t.Errorf("V7 timestamp %d ms differs from now %d ms by %d ms (expected < 1000)", tsMs, nowMs, diff)
	}
}

// ─── Comparison Tests ────────────────────────────────────────────────────────

// TestComparison verifies the Compare function's ordering semantics.
func TestComparison(t *testing.T) {
	// Nil < Max since all bytes of Nil are 0 and all bytes of Max are 0xFF.
	if c := Compare(Nil, Max); c >= 0 {
		t.Errorf("Compare(Nil, Max) = %d, want negative", c)
	}
	// Max > Nil.
	if c := Compare(Max, Nil); c <= 0 {
		t.Errorf("Compare(Max, Nil) = %d, want positive", c)
	}
	// Equal UUIDs compare as 0.
	if c := Compare(Nil, Nil); c != 0 {
		t.Errorf("Compare(Nil, Nil) = %d, want 0", c)
	}
}

// TestV7SortOrder generates a batch of V7 UUIDs and verifies they are already
// in sorted order (since they are generated sequentially with advancing time).
func TestV7SortOrder(t *testing.T) {
	const n = 10
	uuids := make([]UUID, n)
	for i := 0; i < n; i++ {
		u, err := V7()
		if err != nil {
			t.Fatalf("V7() error: %v", err)
		}
		uuids[i] = u
		if i > 0 {
			// Allow equal (same millisecond) or greater — due to the random bits,
			// two UUIDs in the same millisecond could be in any order. We only
			// check that none come strictly before the previous one's timestamp.
			// Extract timestamps for comparison.
			prev := uuids[i-1]
			prevMs := int64(prev[0])<<40 | int64(prev[1])<<32 | int64(prev[2])<<24 |
				int64(prev[3])<<16 | int64(prev[4])<<8 | int64(prev[5])
			currMs := int64(u[0])<<40 | int64(u[1])<<32 | int64(u[2])<<24 |
				int64(u[3])<<16 | int64(u[4])<<8 | int64(u[5])
			if currMs < prevMs {
				t.Errorf("V7 UUID[%d] timestamp %d < UUID[%d] timestamp %d", i, currMs, i-1, prevMs)
			}
		}
	}
}

// ─── Bytes Tests ─────────────────────────────────────────────────────────────

// TestBytes verifies that Bytes() returns a 16-byte slice matching the UUID.
func TestBytes(t *testing.T) {
	u := NamespaceDNS
	b := u.Bytes()
	if len(b) != 16 {
		t.Errorf("Bytes() length = %d, want 16", len(b))
	}
	for i, v := range b {
		if v != u[i] {
			t.Errorf("Bytes()[%d] = %d, want %d", i, v, u[i])
		}
	}
}
