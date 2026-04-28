package hashfunctions

import "testing"

func TestFnv1aVectors(t *testing.T) {
	tests32 := map[string]uint32{
		"":       2166136261,
		"a":      3826002220,
		"abc":    440920331,
		"hello":  1335831723,
		"foobar": 3214735720,
	}
	for input, want := range tests32 {
		if got := Fnv1a32([]byte(input)); got != want {
			t.Fatalf("Fnv1a32(%q) = %d, want %d", input, got, want)
		}
	}

	tests64 := map[string]uint64{
		"":      14695981039346656037,
		"a":     12638187200555641996,
		"abc":   16654208175385433931,
		"hello": 11831194018420276491,
	}
	for input, want := range tests64 {
		if got := Fnv1a64([]byte(input)); got != want {
			t.Fatalf("Fnv1a64(%q) = %d, want %d", input, got, want)
		}
	}
}

func TestDjb2Vectors(t *testing.T) {
	tests := map[string]uint64{
		"":      5381,
		"a":     177670,
		"abc":   193485963,
		"hello": 210714636441,
	}
	for input, want := range tests {
		if got := Djb2([]byte(input)); got != want {
			t.Fatalf("Djb2(%q) = %d, want %d", input, got, want)
		}
	}
}

func TestPolynomialRolling(t *testing.T) {
	if got := PolynomialRolling([]byte("")); got != 0 {
		t.Fatalf("PolynomialRolling(empty) = %d, want 0", got)
	}
	if got := PolynomialRolling([]byte("a")); got != 97 {
		t.Fatalf("PolynomialRolling(a) = %d, want 97", got)
	}
	if got := PolynomialRolling([]byte("ab")); got != 3105 {
		t.Fatalf("PolynomialRolling(ab) = %d, want 3105", got)
	}
	if got := PolynomialRolling([]byte("abc")); got != 96354 {
		t.Fatalf("PolynomialRolling(abc) = %d, want 96354", got)
	}
	if got := PolynomialRollingWithParams([]byte("hello world"), 31, 100); got >= 100 {
		t.Fatalf("custom modulus was not respected: %d", got)
	}
}

func TestPolynomialRollingPanicsOnZeroModulus(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("expected panic")
		}
	}()
	PolynomialRollingWithParams([]byte("x"), 31, 0)
}

func TestMurmur3Vectors(t *testing.T) {
	if got := Murmur3_32WithSeed([]byte(""), 0); got != 0 {
		t.Fatalf("Murmur3 empty seed 0 = %d", got)
	}
	if got := Murmur3_32WithSeed([]byte(""), 1); got != 0x514e28b7 {
		t.Fatalf("Murmur3 empty seed 1 = %#x", got)
	}
	if got := Murmur3_32([]byte("a")); got != 0x3c2569b2 {
		t.Fatalf("Murmur3 a = %#x", got)
	}
	if got := Murmur3_32([]byte("abc")); got != 0xb3dd93fa {
		t.Fatalf("Murmur3 abc = %#x", got)
	}
}

func TestMurmur3TailPathsAndSeeds(t *testing.T) {
	inputs := []string{"abcd", "abcde", "abcdef", "abcdefg"}
	for _, input := range inputs {
		_ = Murmur3_32([]byte(input))
	}
	if Murmur3_32WithSeed([]byte("hello"), 0) == Murmur3_32WithSeed([]byte("hello"), 1) {
		t.Fatal("seed should change output")
	}
}

func TestAnalysisHelpers(t *testing.T) {
	score := AvalancheScore(func(data []byte) uint64 { return uint64(Fnv1a32(data)) }, 32, 8)
	if score < 0 || score > 1 {
		t.Fatalf("score out of range: %f", score)
	}

	chi2 := DistributionTest(func([]byte) uint64 { return 0 }, [][]byte{
		[]byte("a"), []byte("b"), []byte("c"), []byte("d"),
	}, 4)
	if chi2 != 12 {
		t.Fatalf("chi2 = %f, want 12", chi2)
	}
}
