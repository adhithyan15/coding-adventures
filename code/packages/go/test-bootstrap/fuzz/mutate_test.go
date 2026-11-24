package fuzz

import (
	"bytes"
	"encoding/hex"
	"math/big"
	"slices"
	"testing"
)

func hx(t testing.TB, s string) []byte {
	t.Helper()
	b, err := hex.DecodeString(s)
	if err != nil {
		t.Fatal(err)
	}
	return b
}

func bg(t testing.TB, s string) *big.Int {
	t.Helper()
	v, ok := new(big.Int).SetString(s, 10)
	if !ok {
		t.Fatalf("bad integer %q", s)
	}
	return v
}

func endianText(s string) Endian {
	if s == "big" {
		return BigEndian
	}
	return LittleEndian
}

// TestMutatorsMatchTheReferenceOracle runs every vector (generated from the
// independent Python reference, see testdata/gen_reference_vectors.py) through
// BOTH the big-integer entry point (the one the corpus runner uses) and, where
// the arguments fit, the int64 entry point.  The corpus alone cannot pin these:
// its splice has equal splits and its mutate_sequence case does not pin every
// constant (VER06e section 9, item 14).
func TestMutatorsMatchTheReferenceOracle(t *testing.T) {
	counts := map[string]int{}
	for i, v := range referenceVectors {
		a, want := v.args, hx(t, v.out)
		counts[v.op]++
		var got, small []byte
		fits := func(idx ...int) bool {
			for _, j := range idx {
				if !bg(t, a[j]).IsInt64() {
					return false
				}
			}
			return true
		}
		switch v.op {
		case "bit_flip":
			got = BitFlipBig(hx(t, a[0]), bg(t, a[1]), bg(t, a[2]))
			if fits(1, 2) {
				small = BitFlip(hx(t, a[0]), bg(t, a[1]).Int64(), bg(t, a[2]).Int64())
			}
		case "byte_flip":
			got = ByteFlipBig(hx(t, a[0]), bg(t, a[1]), bg(t, a[2]))
			if fits(1, 2) {
				small = ByteFlip(hx(t, a[0]), bg(t, a[1]).Int64(), bg(t, a[2]).Int64())
			}
		case "arithmetic":
			w := int(bg(t, a[3]).Int64())
			got = ByteArithmeticBig(hx(t, a[0]), bg(t, a[1]), bg(t, a[2]), w, endianText(a[4]))
			if fits(1, 2) {
				small = ByteArithmetic(hx(t, a[0]), bg(t, a[1]).Int64(), bg(t, a[2]).Int64(), w, endianText(a[4]))
			}
		case "boundary":
			w := int(bg(t, a[2]).Int64())
			got = BoundaryInsertBig(hx(t, a[0]), bg(t, a[1]), w, bg(t, a[3]), endianText(a[4]))
			if fits(1) && bg(t, a[3]).Sign() >= 0 && bg(t, a[3]).IsUint64() {
				small = BoundaryInsert(hx(t, a[0]), bg(t, a[1]).Int64(), w, bg(t, a[3]).Uint64(), endianText(a[4]))
			}
		case "block_delete":
			got = BlockDeleteBig(hx(t, a[0]), bg(t, a[1]), bg(t, a[2]))
			if fits(1, 2) {
				small = BlockDelete(hx(t, a[0]), bg(t, a[1]).Int64(), bg(t, a[2]).Int64())
			}
		case "block_insert":
			got = BlockInsertBig(hx(t, a[0]), bg(t, a[1]), hx(t, a[2]))
			if fits(1) {
				small = BlockInsert(hx(t, a[0]), bg(t, a[1]).Int64(), hx(t, a[2]))
			}
		case "block_duplicate":
			got = BlockDuplicateBig(hx(t, a[0]), bg(t, a[1]), bg(t, a[2]), bg(t, a[3]))
			if fits(1, 2, 3) {
				small = BlockDuplicate(hx(t, a[0]), bg(t, a[1]).Int64(), bg(t, a[2]).Int64(), bg(t, a[3]).Int64())
			}
		case "splice":
			got = SpliceBig(hx(t, a[0]), hx(t, a[1]), bg(t, a[2]), bg(t, a[3]))
			if fits(2, 3) {
				small = Splice(hx(t, a[0]), hx(t, a[1]), bg(t, a[2]).Int64(), bg(t, a[3]).Int64())
			}
		case "sequence":
			var err error
			got, err = MutateSequence(hx(t, a[0]), bg(t, a[1]).Uint64(), int(bg(t, a[2]).Int64()), int(bg(t, a[3]).Int64()))
			if err != nil {
				t.Fatal(err)
			}
		default:
			t.Fatalf("unknown vector op %q", v.op)
		}
		if !bytes.Equal(got, want) {
			t.Fatalf("vector %d %s %v: got %x, want %x", i, v.op, a, got, want)
		}
		if small != nil && !bytes.Equal(small, want) {
			t.Fatalf("vector %d %s %v (int64 path): got %x, want %x", i, v.op, a, small, want)
		}
	}
	for _, op := range []string{"bit_flip", "byte_flip", "arithmetic", "boundary", "block_delete", "block_insert", "block_duplicate", "splice", "sequence"} {
		if counts[op] < 50 {
			t.Errorf("only %d reference vectors for %s", counts[op], op)
		}
	}
}

func TestFlipsApplyByParityForEnormousCounts(t *testing.T) {
	data := []byte{0x00, 0x00}
	// 17 over 16 bits: every bit once plus bit 0 again -> bit 0 unchanged, rest flipped.
	if got := BitFlip(data, 0, 17); !bytes.Equal(got, []byte{0xFE, 0xFF}) {
		t.Fatalf("got %x", got)
	}
	// Must return instantly for 10^18 (a loop would never finish).
	if got := BitFlip(data, 3, 1_000_000_000_000_000_000); len(got) != 2 {
		t.Fatal("length changed")
	}
	if got := ByteFlip([]byte{1, 2, 3}, 0, 1<<62); len(got) != 3 {
		t.Fatal("length changed")
	}
	// count = k*n is the identity for even k, a full inversion for odd k.
	if got := ByteFlip([]byte{1, 2, 3}, 5, 6); !bytes.Equal(got, []byte{1, 2, 3}) {
		t.Fatalf("even wraps: %x", got)
	}
	if got := ByteFlip([]byte{1, 2, 3}, 5, 3); !bytes.Equal(got, []byte{0xFE, 0xFD, 0xFC}) {
		t.Fatalf("odd wrap: %x", got)
	}
	// Big and int64 agree on the extremes.
	if !bytes.Equal(BitFlipBig(data, big.NewInt(-1), big.NewInt(1<<62)), BitFlip(data, -1, 1<<62)) {
		t.Fatal("Big and int64 disagree")
	}
}

func TestFlipsAreInvolutionsAndKeepLength(t *testing.T) {
	data := []byte{9, 8, 7, 6, 5}
	for _, n := range []int64{1, 2, 7, 40, 41, 1000} {
		for _, off := range []int64{-9, 0, 3, 99} {
			if got := BitFlip(BitFlip(data, off, n), off, n); !bytes.Equal(got, data) {
				t.Fatalf("bit flip off=%d n=%d not an involution", off, n)
			}
			if got := ByteFlip(ByteFlip(data, off, n), off, n); !bytes.Equal(got, data) {
				t.Fatalf("byte flip off=%d n=%d not an involution", off, n)
			}
		}
	}
}

func TestFlipEdgeCases(t *testing.T) {
	if got := BitFlip(nil, 3, 5); len(got) != 0 {
		t.Fatal("empty input must stay empty")
	}
	if got := ByteFlip([]byte{1}, 0, 0); !bytes.Equal(got, []byte{1}) {
		t.Fatal("count 0 is a no-op")
	}
	if got := ByteFlip([]byte{1}, 0, -5); !bytes.Equal(got, []byte{1}) {
		t.Fatal("negative count is a no-op")
	}
	if got := BitFlipBig([]byte{1}, big.NewInt(0), big.NewInt(-5)); !bytes.Equal(got, []byte{1}) {
		t.Fatal("negative big count is a no-op")
	}
	if got := BitFlip([]byte{0, 0}, -7, 1); !bytes.Equal(got, []byte{0, 0x02}) {
		t.Fatalf("-7 mod 16 = 9: got %x", got)
	}
}

func TestNoMutatorModifiesItsArgument(t *testing.T) {
	orig := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	mk := func() []byte { return slices.Clone(orig) }
	for name, f := range map[string]func(d []byte) []byte{
		"bit":      func(d []byte) []byte { return BitFlip(d, 3, 5) },
		"byte":     func(d []byte) []byte { return ByteFlip(d, 3, 5) },
		"arith":    func(d []byte) []byte { return ByteArithmetic(d, 1, 9, 2, BigEndian) },
		"boundary": func(d []byte) []byte { return BoundaryInsert(d, 1, 4, 7, LittleEndian) },
		"delete":   func(d []byte) []byte { return BlockDelete(d, 1, 3) },
		"insert":   func(d []byte) []byte { return BlockInsert(d, 1, []byte{9}) },
		"dup":      func(d []byte) []byte { return BlockDuplicate(d, 1, 3, 2) },
		"splice":   func(d []byte) []byte { return Splice(d, d, 2, 3) },
		"noop":     func(d []byte) []byte { return ByteArithmetic(d, 0, 1, 3, LittleEndian) },
		"seq":      func(d []byte) []byte { r, _ := MutateSequence(d, 5, 20, 100); return r },
	} {
		in := mk()
		out := f(in)
		if !bytes.Equal(in, orig) {
			t.Errorf("%s modified its input", name)
		}
		if len(out) > 0 && len(in) > 0 && &out[0] == &in[0] {
			t.Errorf("%s returned an alias of its input", name)
		}
	}
}

func TestArithmeticWrapsAndIgnoresBadWidths(t *testing.T) {
	if got := ByteArithmetic([]byte{0xFF, 0x00}, 0, 1, 2, LittleEndian); !bytes.Equal(got, []byte{0x00, 0x01}) {
		t.Fatalf("u16 le carry: %x", got)
	}
	if got := ByteArithmetic([]byte{0xFF, 0xFF}, 0, 1, 2, BigEndian); !bytes.Equal(got, []byte{0, 0}) {
		t.Fatalf("u16 wrap: %x", got)
	}
	if got := ByteArithmetic([]byte{0, 0, 0, 0, 0, 0, 0, 0}, 0, -1, 8, BigEndian); !bytes.Equal(got, bytes.Repeat([]byte{0xFF}, 8)) {
		t.Fatalf("u64 -1: %x", got)
	}
	for _, w := range []int{0, 3, 5, 16, -1} {
		if got := ByteArithmetic([]byte{1, 2, 3, 4, 5, 6, 7, 8}, 0, 1, w, LittleEndian); !bytes.Equal(got, []byte{1, 2, 3, 4, 5, 6, 7, 8}) {
			t.Fatalf("width %d must be ignored", w)
		}
	}
	if got := ByteArithmetic([]byte{1}, 0, 1, 2, LittleEndian); !bytes.Equal(got, []byte{1}) {
		t.Fatal("a buffer shorter than the width is unchanged")
	}
	if got := ByteArithmetic([]byte{1, 2}, 0, 1, 1, Endian(7)); !bytes.Equal(got, []byte{2, 2}) {
		t.Fatalf("unknown endian is little: %x", got)
	}
}

func TestBoundaryTruncatesToTheWidth(t *testing.T) {
	if got := BoundaryInsert([]byte{0, 0, 0, 0}, 0, 4, 0x80000000, LittleEndian); !bytes.Equal(got, []byte{0, 0, 0, 0x80}) {
		t.Fatalf("%x", got)
	}
	if got := BoundaryInsert([]byte{0, 0}, 0, 1, 0x1FF, BigEndian); !bytes.Equal(got, []byte{0xFF, 0}) {
		t.Fatalf("val is reduced modulo 2^(8w): %x", got)
	}
	if got := BoundaryInsertBig([]byte{0, 0}, big.NewInt(0), 2, big.NewInt(-1), BigEndian); !bytes.Equal(got, []byte{0xFF, 0xFF}) {
		t.Fatalf("a negative val is its two's complement: %x", got)
	}
}

func TestInterestListsAreTheSpecOnesAndFitTheirWidth(t *testing.T) {
	want := map[int][]uint64{
		1: {0, 1, 127, 128, 255},
		2: {0, 1, 255, 256, 32767, 32768, 65535},
		4: {0, 1, 65535, 65536, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF},
		8: {0, 1, 0xFFFFFFFF, 0x100000000, 0x7FFFFFFFFFFFFFFF, 0x8000000000000000, 0xFFFFFFFFFFFFFFFF},
	}
	for w, list := range want {
		got := InterestValues(w)
		if !slices.Equal(got, list) {
			t.Errorf("width %d: %v", w, got)
		}
		for _, v := range got {
			if w < 8 && v >= 1<<(8*w) {
				t.Errorf("%d does not fit %d bytes", v, w)
			}
		}
		got[0] = 99 // a copy: the table is not exposed
		if InterestValues(w)[0] != 0 {
			t.Error("InterestValues must return a copy")
		}
	}
	if InterestValues(3) != nil {
		t.Fatal("unsupported width has no list")
	}
}

func TestBlockOperationsEdgeCases(t *testing.T) {
	if got := BlockDelete([]byte{1, 2, 3}, 2, 10); !bytes.Equal(got, []byte{1, 2}) {
		t.Fatalf("delete clamps at the end: %x", got)
	}
	if got := BlockDelete(nil, 0, 3); len(got) != 0 {
		t.Fatal("delete from empty")
	}
	if got := BlockDelete([]byte{1, 2, 3}, -1, 1); !bytes.Equal(got, []byte{1, 2}) {
		t.Fatalf("negative offsets wrap: %x", got)
	}
	if got := BlockInsert(nil, 5, []byte{7}); !bytes.Equal(got, []byte{7}) {
		t.Fatal("an empty input becomes the insert")
	}
	if got := BlockInsert([]byte{1}, 1, []byte{7}); !bytes.Equal(got, []byte{1, 7}) {
		t.Fatalf("the end is a valid insertion point: %x", got)
	}
	if got := BlockDuplicate([]byte{1, 2, 3}, 2, 9, 3); !bytes.Equal(got, []byte{1, 2, 3, 3}) {
		t.Fatalf("duplicate clamps at the end: %x", got)
	}
	if got := BlockDuplicate(nil, 0, 1, 0); len(got) != 0 {
		t.Fatal("duplicate of empty")
	}
	if got := Splice([]byte{1, 2}, []byte{3, 4}, -5, 99); !bytes.Equal(got, nil) {
		t.Fatalf("splits clamp: %x", got)
	}
	// Unequal splits: the corpus case has equal ones, which cannot tell the two apart.
	if got := Splice([]byte{1, 2, 3, 4}, []byte{5, 6, 7, 8}, 1, 3); !bytes.Equal(got, []byte{1, 8}) {
		t.Fatalf("splice(1,3): %x", got)
	}
	if got := Splice([]byte{1, 2, 3, 4}, []byte{5, 6, 7, 8}, 3, 1); !bytes.Equal(got, []byte{1, 2, 3, 6, 7, 8}) {
		t.Fatalf("splice(3,1): %x", got)
	}
}

func TestMutateSequenceBoundsGrowthAndSteps(t *testing.T) {
	cur := []byte{1, 2, 3, 4}
	for seed := uint64(0); seed < 50; seed++ {
		got, err := MutateSequence(cur, seed, 40, 16)
		if err != nil || len(got) > 16 {
			t.Fatalf("seed %d: len %d err %v", seed, len(got), err)
		}
	}
	if _, err := MutateSequence(cur, 1, MaxIterations+1, 16); err != ErrSteps {
		t.Fatalf("err = %v", err)
	}
	if got, err := MutateSequence(cur, 1, -3, 16); err != nil || !bytes.Equal(got, cur) {
		t.Fatalf("negative steps run none: %x %v", got, err)
	}
	if got, _ := MutateSequence(nil, 1, 0, 16); len(got) != 0 {
		t.Fatal("zero steps leave an empty buffer empty")
	}
	if got, _ := MutateSequence(nil, 1, 1, 16); len(got) == 0 {
		t.Fatal("an empty buffer is refilled to 00 before a step")
	}
}
