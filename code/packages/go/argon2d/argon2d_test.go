package argon2d

import (
	"bytes"
	"encoding/hex"
	"testing"
)

func filled(n int, b byte) []byte {
	out := make([]byte, n)
	for i := range out {
		out[i] = b
	}
	return out
}

func mustHex(s string) []byte {
	b, err := hex.DecodeString(s)
	if err != nil {
		panic(err)
	}
	return b
}

var (
	rfcPassword = filled(32, 0x01)
	rfcSalt     = filled(16, 0x02)
	rfcKey      = filled(8, 0x03)
	rfcAD       = filled(12, 0x04)
	rfcExpected = mustHex("512b391b6f1162975371d30919734294f868e3be3984f3c1a13a4db9fabe4acb")
)

// TestRFC9106Vector -- the canonical §5.1 gold-standard vector.
func TestRFC9106Vector(t *testing.T) {
	tag, err := Sum(rfcPassword, rfcSalt, 3, 32, 4, 32, &Options{Key: rfcKey, AssociatedData: rfcAD})
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(tag, rfcExpected) {
		t.Fatalf("mismatch\n got %x\nwant %x", tag, rfcExpected)
	}
}

func TestHexMatchesBytes(t *testing.T) {
	tag, _ := Sum(rfcPassword, rfcSalt, 3, 32, 4, 32, nil)
	h, _ := SumHex(rfcPassword, rfcSalt, 3, 32, 4, 32, nil)
	if h != hex.EncodeToString(tag) {
		t.Fatalf("hex != bytes.hex: %s vs %x", h, tag)
	}
}

func TestShortSaltRejected(t *testing.T) {
	_, err := Sum([]byte("pw"), []byte("short"), 1, 8, 1, 32, nil)
	if err == nil {
		t.Fatal("expected error for short salt")
	}
}

func TestTagLengthTooSmallRejected(t *testing.T) {
	_, err := Sum([]byte("pw"), []byte("saltsalt"), 1, 8, 1, 3, nil)
	if err == nil {
		t.Fatal("expected error for tagLength < 4")
	}
}

func TestMemoryBelowMinimumRejected(t *testing.T) {
	_, err := Sum([]byte("pw"), []byte("saltsalt"), 1, 1, 1, 32, nil)
	if err == nil {
		t.Fatal("expected error for memoryCost < 8*p")
	}
}

func TestZeroTimeCostRejected(t *testing.T) {
	_, err := Sum([]byte("pw"), []byte("saltsalt"), 0, 8, 1, 32, nil)
	if err == nil {
		t.Fatal("expected error for timeCost = 0")
	}
}

func TestZeroParallelismRejected(t *testing.T) {
	_, err := Sum([]byte("pw"), []byte("saltsalt"), 1, 8, 0, 32, nil)
	if err == nil {
		t.Fatal("expected error for parallelism = 0")
	}
}

func TestUnsupportedVersionRejected(t *testing.T) {
	_, err := Sum([]byte("pw"), []byte("saltsalt"), 1, 8, 1, 32, &Options{Version: 0x10})
	if err == nil {
		t.Fatal("expected error for version != 0x13")
	}
}

func TestDeterministic(t *testing.T) {
	a, _ := Sum([]byte("password"), []byte("somesalt"), 1, 8, 1, 32, nil)
	b, _ := Sum([]byte("password"), []byte("somesalt"), 1, 8, 1, 32, nil)
	if !bytes.Equal(a, b) {
		t.Fatal("non-deterministic output")
	}
	if len(a) != 32 {
		t.Fatalf("length = %d, want 32", len(a))
	}
}

func TestDifferentPasswordsDiffer(t *testing.T) {
	a, _ := Sum([]byte("password1"), []byte("somesalt"), 1, 8, 1, 32, nil)
	b, _ := Sum([]byte("password2"), []byte("somesalt"), 1, 8, 1, 32, nil)
	if bytes.Equal(a, b) {
		t.Fatal("tags should differ on password change")
	}
}

func TestDifferentSaltsDiffer(t *testing.T) {
	a, _ := Sum([]byte("password"), []byte("saltsalt"), 1, 8, 1, 32, nil)
	b, _ := Sum([]byte("password"), []byte("saltsal2"), 1, 8, 1, 32, nil)
	if bytes.Equal(a, b) {
		t.Fatal("tags should differ on salt change")
	}
}

func TestKeyBinds(t *testing.T) {
	a, _ := Sum([]byte("password"), []byte("saltsalt"), 1, 8, 1, 32, nil)
	b, _ := Sum([]byte("password"), []byte("saltsalt"), 1, 8, 1, 32, &Options{Key: []byte("secret!!")})
	if bytes.Equal(a, b) {
		t.Fatal("tag should be bound to key")
	}
}

func TestAssociatedDataBinds(t *testing.T) {
	a, _ := Sum([]byte("password"), []byte("saltsalt"), 1, 8, 1, 32, nil)
	b, _ := Sum([]byte("password"), []byte("saltsalt"), 1, 8, 1, 32, &Options{AssociatedData: []byte("ad")})
	if bytes.Equal(a, b) {
		t.Fatal("tag should be bound to associated data")
	}
}

func TestTagLengthVariants(t *testing.T) {
	for _, T := range []int{4, 16, 32, 64, 65, 128} {
		tag, err := Sum([]byte("password"), []byte("saltsalt"), 1, 8, 1, T, nil)
		if err != nil {
			t.Fatalf("T=%d: %v", T, err)
		}
		if len(tag) != T {
			t.Fatalf("T=%d: got %d bytes", T, len(tag))
		}
	}
}

func TestMultiLaneParameters(t *testing.T) {
	tag, err := Sum(rfcPassword, rfcSalt, 3, 32, 4, 32, nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(tag) != 32 {
		t.Fatalf("length = %d", len(tag))
	}
}

func TestMultiplePasses(t *testing.T) {
	t1, _ := Sum([]byte("password"), []byte("saltsalt"), 1, 8, 1, 32, nil)
	t2, _ := Sum([]byte("password"), []byte("saltsalt"), 2, 8, 1, 32, nil)
	t3, _ := Sum([]byte("password"), []byte("saltsalt"), 3, 8, 1, 32, nil)
	if bytes.Equal(t1, t2) || bytes.Equal(t2, t3) || bytes.Equal(t1, t3) {
		t.Fatal("tags should differ across t=1,2,3")
	}
}
