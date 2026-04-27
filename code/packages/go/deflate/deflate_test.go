package deflate_test

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/deflate"
)

// roundtrip is a helper that compresses and then decompresses data,
// asserting the result equals the original.
func roundtrip(t *testing.T, data []byte) {
	t.Helper()
	compressed, err := deflate.Compress(data)
	if err != nil {
		t.Fatalf("Compress(%q) error: %v", data, err)
	}
	decompressed, err := deflate.Decompress(compressed)
	if err != nil {
		t.Fatalf("Decompress error: %v", err)
	}
	if string(decompressed) != string(data) {
		t.Errorf("roundtrip mismatch:\n  want: %q\n   got: %q", data, decompressed)
	}
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

func TestEmpty(t *testing.T) {
	compressed, err := deflate.Compress(nil)
	if err != nil {
		t.Fatal(err)
	}
	decompressed, err := deflate.Decompress(compressed)
	if err != nil {
		t.Fatal(err)
	}
	if len(decompressed) != 0 {
		t.Errorf("expected empty, got %q", decompressed)
	}
}

func TestSingleByte(t *testing.T) {
	roundtrip(t, []byte{0x00})
	roundtrip(t, []byte{0xFF})
	roundtrip(t, []byte("A"))
}

func TestSingleByteRepeated(t *testing.T) {
	roundtrip(t, []byte("AAAAAAAAAAAAAAAAAAA"))
	data := make([]byte, 100)
	for i := range data {
		data[i] = 0x00
	}
	roundtrip(t, data)
}

// ---------------------------------------------------------------------------
// Spec examples
// ---------------------------------------------------------------------------

func TestSpecExampleAABBC(t *testing.T) {
	// "AAABBC" — all literals, no matches.
	data := []byte("AAABBC")
	roundtrip(t, data)

	compressed, err := deflate.Compress(data)
	if err != nil {
		t.Fatal(err)
	}
	// Verify dist_entry_count = 0 (no matches).
	distCount := int(compressed[6])<<8 | int(compressed[7])
	if distCount != 0 {
		t.Errorf("expected dist_entry_count=0 for all-literals, got %d", distCount)
	}
}

func TestSpecExampleAABCBBABC(t *testing.T) {
	// "AABCBBABC" — one LZSS match (offset=5, length=3).
	data := []byte("AABCBBABC")
	roundtrip(t, data)

	compressed, err := deflate.Compress(data)
	if err != nil {
		t.Fatal(err)
	}
	// Verify original_length stored correctly.
	origLen := int(compressed[0])<<24 | int(compressed[1])<<16 |
		int(compressed[2])<<8 | int(compressed[3])
	if origLen != 9 {
		t.Errorf("expected original_length=9, got %d", origLen)
	}
	// Verify dist_entry_count > 0.
	distCount := int(compressed[6])<<8 | int(compressed[7])
	if distCount == 0 {
		t.Errorf("expected dist_entry_count>0 for input with matches")
	}
}

// ---------------------------------------------------------------------------
// Match and overlap tests
// ---------------------------------------------------------------------------

func TestOverlappingMatch(t *testing.T) {
	// offset < length encodes a run.
	roundtrip(t, []byte("AAAAAAA"))
	roundtrip(t, []byte("ABABABABABAB"))
}

func TestMultipleMatches(t *testing.T) {
	roundtrip(t, []byte("ABCABCABCABC"))
	roundtrip(t, []byte("hello hello hello world"))
}

// ---------------------------------------------------------------------------
// Data variety tests
// ---------------------------------------------------------------------------

func TestAllBytes(t *testing.T) {
	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i)
	}
	roundtrip(t, data)
}

func TestBinaryData(t *testing.T) {
	data := make([]byte, 1000)
	for i := range data {
		data[i] = byte(i % 256)
	}
	roundtrip(t, data)
}

func TestLongerText(t *testing.T) {
	base := []byte("the quick brown fox jumps over the lazy dog ")
	data := make([]byte, 0, len(base)*10)
	for i := 0; i < 10; i++ {
		data = append(data, base...)
	}
	roundtrip(t, data)
}

func TestCompressionRatio(t *testing.T) {
	// Highly repetitive data should compress to < 50% of original.
	base := []byte("ABCABC")
	data := make([]byte, 0, len(base)*100)
	for i := 0; i < 100; i++ {
		data = append(data, base...)
	}
	compressed, err := deflate.Compress(data)
	if err != nil {
		t.Fatal(err)
	}
	if len(compressed) >= len(data)/2 {
		t.Errorf("expected significant compression: %d >= %d/2=%d",
			len(compressed), len(data), len(data)/2)
	}
}

// ---------------------------------------------------------------------------
// Diverse round-trip tests
// ---------------------------------------------------------------------------

func TestDiverse(t *testing.T) {
	tests := []struct {
		name string
		data []byte
	}{
		{"zeros100", make([]byte, 100)},
		{"ff100", func() []byte { d := make([]byte, 100); for i := range d { d[i] = 0xff }; return d }()},
		{"alphabet", []byte("abcdefghijklmnopqrstuvwxyz")},
		{"repeated_long", func() []byte {
			s := []byte("The quick brown fox ")
			d := make([]byte, 0, len(s)*20)
			for i := 0; i < 20; i++ {
				d = append(d, s...)
			}
			return d
		}()},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			roundtrip(t, tt.data)
		})
	}
}

func TestMaxMatchLength(t *testing.T) {
	// 300 identical bytes will produce a match near max_match=255.
	data := make([]byte, 300)
	for i := range data {
		data[i] = 'A'
	}
	roundtrip(t, data)
}

func TestVariousLengths(t *testing.T) {
	// Test various match lengths to exercise the length code table.
	for _, length := range []int{3, 4, 10, 11, 13, 19, 35, 67, 131, 227, 255} {
		prefix := make([]byte, length)
		for i := range prefix {
			prefix[i] = 'A'
		}
		data := append(prefix, []byte("BBB")...)
		data = append(data, prefix...)
		roundtrip(t, data)
	}
}
