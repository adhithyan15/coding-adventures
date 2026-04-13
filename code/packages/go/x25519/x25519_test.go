package x25519

import (
	"encoding/hex"
	"math/big"
	"testing"
)

// ============================================================================
// Helper: decode hex to [32]byte
// ============================================================================

func hexTo32(t *testing.T, s string) [32]byte {
	t.Helper()
	b, err := hex.DecodeString(s)
	if err != nil {
		t.Fatalf("bad hex: %v", err)
	}
	if len(b) != 32 {
		t.Fatalf("expected 32 bytes, got %d", len(b))
	}
	var out [32]byte
	copy(out[:], b)
	return out
}

// ============================================================================
// Field Arithmetic Tests
// ============================================================================

func TestFieldAddBasic(t *testing.T) {
	result := fieldAdd(big.NewInt(3), big.NewInt(5))
	if result.Cmp(big.NewInt(8)) != 0 {
		t.Errorf("expected 8, got %s", result)
	}
}

func TestFieldAddWraps(t *testing.T) {
	pMinus1 := new(big.Int).Sub(p, big.NewInt(1))
	result := fieldAdd(pMinus1, big.NewInt(1))
	if result.Cmp(big.NewInt(0)) != 0 {
		t.Errorf("expected 0, got %s", result)
	}
}

func TestFieldSubBasic(t *testing.T) {
	result := fieldSub(big.NewInt(10), big.NewInt(3))
	if result.Cmp(big.NewInt(7)) != 0 {
		t.Errorf("expected 7, got %s", result)
	}
}

func TestFieldSubWraps(t *testing.T) {
	result := fieldSub(big.NewInt(3), big.NewInt(5))
	expected := new(big.Int).Sub(p, big.NewInt(2))
	if result.Cmp(expected) != 0 {
		t.Errorf("expected p-2, got %s", result)
	}
}

func TestFieldMulBasic(t *testing.T) {
	result := fieldMul(big.NewInt(3), big.NewInt(7))
	if result.Cmp(big.NewInt(21)) != 0 {
		t.Errorf("expected 21, got %s", result)
	}
}

func TestFieldSquareConsistency(t *testing.T) {
	vals := []int64{7, 42, 121666}
	for _, v := range vals {
		bv := big.NewInt(v)
		sq := fieldSquare(bv)
		mul := fieldMul(bv, bv)
		if sq.Cmp(mul) != 0 {
			t.Errorf("square(%d) != mul(%d, %d)", v, v, v)
		}
	}
}

func TestFieldInvert(t *testing.T) {
	vals := []int64{1, 2, 3, 7, 42, 121666}
	for _, v := range vals {
		bv := big.NewInt(v)
		inv := fieldInvert(bv)
		product := fieldMul(bv, inv)
		if product.Cmp(big.NewInt(1)) != 0 {
			t.Errorf("invert(%d) failed: %d * inv = %s", v, v, product)
		}
	}
}

func TestA24Value(t *testing.T) {
	if a24.Cmp(big.NewInt(121666)) != 0 {
		t.Errorf("expected a24 = 121666, got %s", a24)
	}
	// 4 * a24 - 2 = 486662 (the curve parameter A)
	curveA := new(big.Int).Mul(a24, big.NewInt(4))
	curveA.Sub(curveA, big.NewInt(2))
	if curveA.Cmp(big.NewInt(486662)) != 0 {
		t.Errorf("expected 4*a24-2 = 486662, got %s", curveA)
	}
}

// ============================================================================
// Cswap Tests
// ============================================================================

func TestCswapNoSwap(t *testing.T) {
	a, b := cswap(0, big.NewInt(10), big.NewInt(20))
	if a.Cmp(big.NewInt(10)) != 0 || b.Cmp(big.NewInt(20)) != 0 {
		t.Errorf("no-swap failed: got %s, %s", a, b)
	}
}

func TestCswapSwap(t *testing.T) {
	a, b := cswap(1, big.NewInt(10), big.NewInt(20))
	if a.Cmp(big.NewInt(20)) != 0 || b.Cmp(big.NewInt(10)) != 0 {
		t.Errorf("swap failed: got %s, %s", a, b)
	}
}

// ============================================================================
// Encoding Tests
// ============================================================================

func TestDecodeUCoordinateBasePoint(t *testing.T) {
	val := decodeUCoordinate(BasePoint)
	if val.Cmp(big.NewInt(9)) != 0 {
		t.Errorf("expected 9, got %s", val)
	}
}

func TestDecodeUCoordinateMasksHighBit(t *testing.T) {
	var uBytes [32]byte
	uBytes[31] = 0xFF
	val := decodeUCoordinate(uBytes)
	// 0xFF with high bit masked = 0x7F = 127, at byte position 31
	expected := new(big.Int).Lsh(big.NewInt(127), 31*8)
	if val.Cmp(expected) != 0 {
		t.Errorf("high bit masking failed")
	}
}

func TestEncodeDecodeRoundtrip(t *testing.T) {
	vals := []int64{0, 1, 9, 42}
	for _, v := range vals {
		bv := big.NewInt(v)
		encoded := encodeUCoordinate(bv)
		decoded := decodeUCoordinate(encoded)
		if decoded.Cmp(bv) != 0 {
			t.Errorf("roundtrip failed for %d: got %s", v, decoded)
		}
	}
}

func TestDecodeScalarClamps(t *testing.T) {
	var k [32]byte
	for i := range k {
		k[i] = 0xFF
	}
	val := decodeScalar(k)

	// Low 3 bits must be 0
	lowBits := new(big.Int).And(val, big.NewInt(7))
	if lowBits.Cmp(big.NewInt(0)) != 0 {
		t.Errorf("low 3 bits not cleared")
	}

	// Bit 255 must be 0
	if val.Bit(255) != 0 {
		t.Errorf("bit 255 not cleared")
	}

	// Bit 254 must be 1
	if val.Bit(254) != 1 {
		t.Errorf("bit 254 not set")
	}
}

// ============================================================================
// RFC 7748 Test Vectors
// ============================================================================

func TestVector1(t *testing.T) {
	scalar := hexTo32(t, "a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4")
	u := hexTo32(t, "e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c")
	expected := hexTo32(t, "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552")

	result, err := X25519(scalar, u)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != expected {
		t.Errorf("test vector 1 failed\ngot:      %x\nexpected: %x", result, expected)
	}
}

func TestVector2(t *testing.T) {
	scalar := hexTo32(t, "4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d")
	u := hexTo32(t, "e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493")
	expected := hexTo32(t, "95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957")

	result, err := X25519(scalar, u)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != expected {
		t.Errorf("test vector 2 failed\ngot:      %x\nexpected: %x", result, expected)
	}
}

// ============================================================================
// Diffie-Hellman Tests
// ============================================================================

func TestAlicePublicKey(t *testing.T) {
	alicePrivate := hexTo32(t, "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
	expected := hexTo32(t, "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a")

	result, err := X25519Base(alicePrivate)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != expected {
		t.Errorf("Alice public key failed\ngot:      %x\nexpected: %x", result, expected)
	}
}

func TestBobPublicKey(t *testing.T) {
	bobPrivate := hexTo32(t, "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")
	expected := hexTo32(t, "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f")

	result, err := X25519Base(bobPrivate)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result != expected {
		t.Errorf("Bob public key failed\ngot:      %x\nexpected: %x", result, expected)
	}
}

func TestSharedSecret(t *testing.T) {
	alicePrivate := hexTo32(t, "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
	bobPrivate := hexTo32(t, "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")

	alicePublic, err := X25519Base(alicePrivate)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	bobPublic, err := X25519Base(bobPrivate)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	sharedAB, err := X25519(alicePrivate, bobPublic)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	sharedBA, err := X25519(bobPrivate, alicePublic)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	expected := hexTo32(t, "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742")

	if sharedAB != expected {
		t.Errorf("shared secret AB failed\ngot:      %x\nexpected: %x", sharedAB, expected)
	}
	if sharedBA != expected {
		t.Errorf("shared secret BA failed\ngot:      %x\nexpected: %x", sharedBA, expected)
	}
}

func TestGenerateKeypairIsX25519Base(t *testing.T) {
	private := hexTo32(t, "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
	fromBase, _ := X25519Base(private)
	fromKeypair, _ := GenerateKeypair(private)
	if fromBase != fromKeypair {
		t.Errorf("GenerateKeypair != X25519Base")
	}
}

// ============================================================================
// Iterated Tests
// ============================================================================

func TestOneIteration(t *testing.T) {
	var k, u [32]byte
	k[0] = 9
	u[0] = 9

	newK, err := X25519(k, u)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	expected := hexTo32(t, "422c8e7a6227d7bca1350b3e2bb7279f7897b87bb6854b783c60e80311ae3079")
	if newK != expected {
		t.Errorf("1 iteration failed\ngot:      %x\nexpected: %x", newK, expected)
	}
}

func TestThousandIterations(t *testing.T) {
	var k, u [32]byte
	k[0] = 9
	u[0] = 9

	for i := 0; i < 1000; i++ {
		newK, err := X25519(k, u)
		if err != nil {
			t.Fatalf("iteration %d: unexpected error: %v", i, err)
		}
		u = k
		k = newK
	}

	expected := hexTo32(t, "684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51")
	if k != expected {
		t.Errorf("1000 iterations failed\ngot:      %x\nexpected: %x", k, expected)
	}
}

// func TestMillionIterations(t *testing.T) {
//     // WARNING: This takes a very long time. Uncomment only for thorough verification.
//     var k, u [32]byte
//     k[0] = 9
//     u[0] = 9
//     for i := 0; i < 1_000_000; i++ {
//         newK, err := X25519(k, u)
//         if err != nil {
//             t.Fatalf("iteration %d: %v", i, err)
//         }
//         u = k
//         k = newK
//     }
//     expected := hexTo32(t, "7c3911e0ab2586fd864497297e575e6f3bc601c0883c30df5f4dd2d24f665424")
//     if k != expected {
//         t.Errorf("1M iterations failed")
//     }
// }

// ============================================================================
// Edge Cases
// ============================================================================

func TestBasePointIsNine(t *testing.T) {
	var expected [32]byte
	expected[0] = 9
	if BasePoint != expected {
		t.Errorf("base point is not 9")
	}
}

func TestPValue(t *testing.T) {
	// p = 2^255 - 19
	expected := new(big.Int).Sub(
		new(big.Int).Exp(big.NewInt(2), big.NewInt(255), nil),
		big.NewInt(19),
	)
	if p.Cmp(expected) != 0 {
		t.Errorf("p value incorrect")
	}
}

func TestFieldIdentityElements(t *testing.T) {
	val := big.NewInt(42)
	if fieldAdd(val, big.NewInt(0)).Cmp(val) != 0 {
		t.Errorf("additive identity failed")
	}
	if fieldMul(val, big.NewInt(1)).Cmp(val) != 0 {
		t.Errorf("multiplicative identity failed")
	}
}

func TestFieldNegation(t *testing.T) {
	a := big.NewInt(12345)
	negA := fieldSub(big.NewInt(0), a)
	result := fieldAdd(a, negA)
	if result.Cmp(big.NewInt(0)) != 0 {
		t.Errorf("a + (-a) != 0")
	}
}

func TestFieldDistributive(t *testing.T) {
	a, b, c := big.NewInt(123), big.NewInt(456), big.NewInt(789)
	lhs := fieldMul(a, fieldAdd(b, c))
	rhs := fieldAdd(fieldMul(a, b), fieldMul(a, c))
	if lhs.Cmp(rhs) != 0 {
		t.Errorf("distributive law failed")
	}
}
