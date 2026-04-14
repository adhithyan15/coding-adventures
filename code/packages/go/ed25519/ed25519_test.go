// Package ed25519 tests verify correctness using the official test vectors from
// RFC 8032 Section 7.1, plus additional edge-case tests for field arithmetic,
// point operations, encoding/decoding, and error handling.
package ed25519

import (
	"encoding/hex"
	"math/big"
	"testing"
)

// ═══════════════════════════════════════════════════════════════════════════════
// FIELD ARITHMETIC TESTS
// ═══════════════════════════════════════════════════════════════════════════════

func TestFieldInvBasic(t *testing.T) {
	// 3 * inv(3) ≡ 1 (mod p)
	three := big.NewInt(3)
	inv := fieldInv(three)
	product := new(big.Int).Mul(three, inv)
	product.Mod(product, p)
	if product.Cmp(big.NewInt(1)) != 0 {
		t.Errorf("3 * inv(3) mod p = %s, want 1", product)
	}
}

func TestFieldInvLarge(t *testing.T) {
	val := new(big.Int).Lsh(big.NewInt(1), 200)
	val.Add(val, big.NewInt(37))
	inv := fieldInv(val)
	product := new(big.Int).Mul(val, inv)
	product.Mod(product, p)
	if product.Cmp(big.NewInt(1)) != 0 {
		t.Errorf("val * inv(val) mod p = %s, want 1", product)
	}
}

func TestFieldInvOne(t *testing.T) {
	inv := fieldInv(big.NewInt(1))
	if inv.Cmp(big.NewInt(1)) != 0 {
		t.Errorf("inv(1) = %s, want 1", inv)
	}
}

func TestSqrtM1Squared(t *testing.T) {
	// sqrtM1² should equal -1 mod p
	sq := new(big.Int).Mul(sqrtM1, sqrtM1)
	sq.Mod(sq, p)
	negOne := new(big.Int).Sub(p, big.NewInt(1))
	if sq.Cmp(negOne) != 0 {
		t.Errorf("sqrtM1² mod p = %s, want %s", sq, negOne)
	}
}

func TestFieldSqrtPerfectSquare(t *testing.T) {
	val := big.NewInt(42)
	sq := new(big.Int).Mul(val, val)
	sq.Mod(sq, p)
	root := fieldSqrt(sq)
	if root == nil {
		t.Fatal("fieldSqrt returned nil for a perfect square")
	}
	check := new(big.Int).Mul(root, root)
	check.Mod(check, p)
	if check.Cmp(sq) != 0 {
		t.Errorf("root² mod p = %s, want %s", check, sq)
	}
}

func TestFieldSqrtNoRoot(t *testing.T) {
	// 2 is a quadratic non-residue mod p
	root := fieldSqrt(big.NewInt(2))
	if root != nil {
		t.Errorf("fieldSqrt(2) = %s, want nil (no square root)", root)
	}
}

func TestFieldSqrtZero(t *testing.T) {
	root := fieldSqrt(big.NewInt(0))
	if root == nil {
		t.Fatal("fieldSqrt(0) returned nil")
	}
	if root.Sign() != 0 {
		t.Errorf("fieldSqrt(0) = %s, want 0", root)
	}
}

// ═══════════════════════════════════════════════════════════════════════════════
// POINT OPERATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

func TestIdentityAdd(t *testing.T) {
	// identity + B = B
	result := pointAdd(identity(), basePoint())
	enc := pointEncode(result)
	expected := pointEncode(basePoint())
	if enc != expected {
		t.Errorf("identity + B != B")
	}
}

func TestDoubleEqualsAdd(t *testing.T) {
	bp := basePoint()
	doubled := pointDouble(bp)
	added := pointAdd(bp, bp)
	if pointEncode(doubled) != pointEncode(added) {
		t.Error("double(B) != B + B")
	}
}

func TestScalarMultZero(t *testing.T) {
	result := scalarMult(big.NewInt(0), basePoint())
	enc := pointEncode(result)
	expected := pointEncode(identity())
	if enc != expected {
		t.Error("0 * B != identity")
	}
}

func TestScalarMultOne(t *testing.T) {
	result := scalarMult(big.NewInt(1), basePoint())
	enc := pointEncode(result)
	expected := pointEncode(basePoint())
	if enc != expected {
		t.Error("1 * B != B")
	}
}

func TestScalarMultTwo(t *testing.T) {
	bp := basePoint()
	result := scalarMult(big.NewInt(2), bp)
	expected := pointAdd(bp, bp)
	if pointEncode(result) != pointEncode(expected) {
		t.Error("2 * B != B + B")
	}
}

func TestScalarMultOrder(t *testing.T) {
	// L * B = identity
	result := scalarMult(curveL, basePoint())
	enc := pointEncode(result)
	expected := pointEncode(identity())
	if enc != expected {
		t.Error("L * B != identity")
	}
}

func TestBasePointOnCurve(t *testing.T) {
	// -x² + y² = 1 + d·x²·y²
	xSq := new(big.Int).Mul(baseX, baseX)
	xSq.Mod(xSq, p)
	ySq := new(big.Int).Mul(baseY, baseY)
	ySq.Mod(ySq, p)

	lhs := new(big.Int).Sub(ySq, xSq) // y² - x²  (because -x² + y²)
	lhs.Mod(lhs, p)

	rhs := new(big.Int).Mul(d, xSq)
	rhs.Mul(rhs, ySq)
	rhs.Add(rhs, big.NewInt(1))
	rhs.Mod(rhs, p)

	if lhs.Cmp(rhs) != 0 {
		t.Error("base point not on curve")
	}
}

// ═══════════════════════════════════════════════════════════════════════════════
// POINT ENCODING/DECODING TESTS
// ═══════════════════════════════════════════════════════════════════════════════

func TestEncodeDecodeBasePoint(t *testing.T) {
	bp := basePoint()
	encoded := pointEncode(bp)
	decoded, ok := pointDecode(encoded)
	if !ok {
		t.Fatal("failed to decode base point")
	}
	if pointEncode(decoded) != encoded {
		t.Error("encode-decode round trip failed for base point")
	}
}

func TestEncodeDecodeIdentity(t *testing.T) {
	id := identity()
	encoded := pointEncode(id)
	decoded, ok := pointDecode(encoded)
	if !ok {
		t.Fatal("failed to decode identity")
	}
	if pointEncode(decoded) != encoded {
		t.Error("encode-decode round trip failed for identity")
	}
}

func TestDecodeYOutOfRange(t *testing.T) {
	// Set y = p (too large). Encode p as little-endian 32 bytes.
	pBytes := bigIntToLE32(p)
	pBytes[31] &= 0x7F // clear sign bit
	_, ok := pointDecode(pBytes)
	if ok {
		t.Error("should reject y >= p")
	}
}

func TestEncodeDecodeDoubleBase(t *testing.T) {
	double := scalarMult(big.NewInt(2), basePoint())
	encoded := pointEncode(double)
	decoded, ok := pointDecode(encoded)
	if !ok {
		t.Fatal("failed to decode 2*B")
	}
	if pointEncode(decoded) != encoded {
		t.Error("encode-decode round trip failed for 2*B")
	}
}

// ═══════════════════════════════════════════════════════════════════════════════
// RFC 8032 TEST VECTORS
// ═══════════════════════════════════════════════════════════════════════════════

// hexToBytes decodes a hex string, panicking on error (test-only helper).
func hexToBytes(s string) []byte {
	b, err := hex.DecodeString(s)
	if err != nil {
		panic("bad hex: " + err.Error())
	}
	return b
}

// hexTo32 decodes a hex string into a [32]byte array.
func hexTo32(s string) [32]byte {
	b := hexToBytes(s)
	var arr [32]byte
	copy(arr[:], b)
	return arr
}

func TestVector1EmptyMessage(t *testing.T) {
	seed := hexTo32("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
	expectedPub := hexTo32("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
	expectedSigBytes := hexToBytes(
		"e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155" +
			"5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b")
	var expectedSig [64]byte
	copy(expectedSig[:], expectedSigBytes)
	message := []byte{}

	pub, sec := GenerateKeypair(seed)
	if pub != expectedPub {
		t.Errorf("public key mismatch:\n  got  %x\n  want %x", pub, expectedPub)
	}

	sig := Sign(message, sec)
	if sig != expectedSig {
		t.Errorf("signature mismatch:\n  got  %x\n  want %x", sig, expectedSig)
	}

	if !Verify(message, sig, pub) {
		t.Error("valid signature rejected")
	}
}

func TestVector2OneByte(t *testing.T) {
	seed := hexTo32("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb")
	expectedPub := hexTo32("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c")
	expectedSigBytes := hexToBytes(
		"92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da" +
			"085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00")
	var expectedSig [64]byte
	copy(expectedSig[:], expectedSigBytes)
	message := hexToBytes("72")

	pub, sec := GenerateKeypair(seed)
	if pub != expectedPub {
		t.Errorf("public key mismatch:\n  got  %x\n  want %x", pub, expectedPub)
	}

	sig := Sign(message, sec)
	if sig != expectedSig {
		t.Errorf("signature mismatch:\n  got  %x\n  want %x", sig, expectedSig)
	}

	if !Verify(message, sig, pub) {
		t.Error("valid signature rejected")
	}
}

func TestVector3TwoBytes(t *testing.T) {
	seed := hexTo32("c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7")
	expectedPub := hexTo32("fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025")
	expectedSigBytes := hexToBytes(
		"6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac" +
			"18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a")
	var expectedSig [64]byte
	copy(expectedSig[:], expectedSigBytes)
	message := hexToBytes("af82")

	pub, sec := GenerateKeypair(seed)
	if pub != expectedPub {
		t.Errorf("public key mismatch:\n  got  %x\n  want %x", pub, expectedPub)
	}

	sig := Sign(message, sec)
	if sig != expectedSig {
		t.Errorf("signature mismatch:\n  got  %x\n  want %x", sig, expectedSig)
	}

	if !Verify(message, sig, pub) {
		t.Error("valid signature rejected")
	}
}

func TestVector4_1023Bytes(t *testing.T) {
	seed := hexTo32("f5e5767cf153319517630f226876b86c8160cc583bc013744c6bf255f5cc0ee5")
	expectedPub := hexTo32("278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e")
	expectedSigBytes := hexToBytes(
		"d686294b743c6760c6a78a2c4c2fc76115c2600b8f083acde59e7cee32578c0f" +
			"59ea4219ab9b5896795e4e2b87a30270aa0e3099eee944e9e67a1b22df41ff07")
	var expectedSig [64]byte
	copy(expectedSig[:], expectedSigBytes)
	message := hexToBytes(
		"08b8b2b733424243760fe426a4b54908632110a66c2f6591eabd3345e3e4eb98" +
			"fa6e264bf09efe12ee50f8f54e9f77b1e355f6c50544e23fb1433ddf73be84d8" +
			"79de7c0046dc4996d9e773f4bc9efe5738829adb26c81b37c93a1b270b20329d" +
			"658675fc6ea534e0810a4432826bf58c941efb65d57a338bbd2e26640f89ffbc" +
			"1a858efcb8550ee3a5e1998bd177e93a7363c344fe6b199ee5d02e82d522c4fe" +
			"ba15452f80288a821a579116ec6dad2b3b310da903401aa62100ab5d1a36553e" +
			"06203b33890cc9b832f79ef80560ccb9a39ce767967ed628c6ad573cb116dbef" +
			"fefd75499da96bd68a8a97b928a8bbc103b6621fcde2beca1231d206be6cd9ec" +
			"7aff6f6c94fcd7204ed3455c68c83f4a41da4af2b74ef5c53f1d8ac70bdcb7ed" +
			"185ce81bd84359d44254d95629e9855a94a7c1958d1f8ada5d0532ed8a5aa3fb" +
			"2d17ba70eb6248e594e1a2297acbbb39d502f1a8c6eb6f1ce22b3de1a1f40cc2" +
			"4554119a831a9aad6079cad88425de6bde1a9187ebb6092cf67bf2b13fd65f27" +
			"088d78b7e883c8759d2c4f5c65adb7553878ad575f9fad878e80a0c9ba63bcbc" +
			"c2732e69485bbc9c90bfbd62481d9089beccf80cfe2df16a2cf65bd92dd597b0" +
			"7e0917af48bbb75fed413d238f5555a7a569d80c3414a8d0859dc65a46128bab" +
			"27af87a71314f318c782b23ebfe808b82b0ce26401d2e22f04d83d1255dc51ad" +
			"dd3b75a2b1ae0784504df543af8969be3ea7082ff7fc9888c144da2af58429ec" +
			"96031dbcad3dad9af0dcbaaaf268cb8fcffead94f3c7ca495e056a9b47acdb75" +
			"1fb73e666c6c655ade8297297d07ad1ba5e43f1bca32301651339e22904cc8c4" +
			"2f58c30c04aafdb038dda0847dd988dcda6f3bfd15c4b4c4525004aa06eeff8c" +
			"a61783aacec57fb3d1f92b0fe2fd1a85f6724517b65e614ad6808d6f6ee34dff" +
			"7310fdc82aebfd904b01e1dc54b2927094b2db68d6f903b68401adebf5a7e08d" +
			"78ff4ef5d63653a65040cf9bfd4aca7984a74d37145986780fc0b16ac451649d" +
			"e6188a7dbdf191f64b5fc5e2ab47b57f7f7276cd419c17a3ca8e1b939ae49e48" +
			"8acba6b965610b5480109c8b17b80e1b7b750dfc7598d5d5011fd2dcc5600a32" +
			"ef5b52a1ecc820e308aa342721aac0943bf6686b64b2579376504ccc493d97e6" +
			"aed3fb0f9cd71a43dd497f01f17c0e2cb3797aa2a2f256656168e6c496afc5fb" +
			"93246f6b1116398a346f1a641f3b041e989f7914f90cc2c7fff357876e506b50" +
			"d334ba77c225bc307ba537152f3f1610e4eafe595f6d9d90d11faa933a15ef13" +
			"69546868a7f3a45a96768d40fd9d03412c091c6315cf4fde7cb68606937380db" +
			"2eaaa707b4c4185c32eddcdd306705e4dc1ffc872eeee475a64dfac86aba41c0" +
			"618983f8741c5ef68d3a101e8a3b8cac60c905c15fc910840b94c00a0b9d00")

	pub, sec := GenerateKeypair(seed)
	if pub != expectedPub {
		t.Errorf("public key mismatch:\n  got  %x\n  want %x", pub, expectedPub)
	}

	sig := Sign(message, sec)
	if sig != expectedSig {
		t.Errorf("signature mismatch:\n  got  %x\n  want %x", sig, expectedSig)
	}

	if !Verify(message, sig, pub) {
		t.Error("valid signature rejected")
	}
}

// ═══════════════════════════════════════════════════════════════════════════════
// VERIFICATION EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════════

func testKeypair() ([32]byte, [64]byte) {
	seed := hexTo32("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
	return GenerateKeypair(seed)
}

func TestWrongMessage(t *testing.T) {
	pub, sec := testKeypair()
	sig := Sign([]byte("hello"), sec)
	if Verify([]byte("world"), sig, pub) {
		t.Error("should reject wrong message")
	}
}

func TestWrongPublicKey(t *testing.T) {
	_, sec1 := testKeypair()
	seed2 := hexTo32("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb")
	pub2, _ := GenerateKeypair(seed2)
	sig := Sign([]byte("hello"), sec1)
	if Verify([]byte("hello"), sig, pub2) {
		t.Error("should reject wrong public key")
	}
}

func TestTamperedSignatureR(t *testing.T) {
	pub, sec := testKeypair()
	sig := Sign([]byte("hello"), sec)
	sig[0] ^= 1 // flip one bit in R
	if Verify([]byte("hello"), sig, pub) {
		t.Error("should reject tampered R")
	}
}

func TestTamperedSignatureS(t *testing.T) {
	pub, sec := testKeypair()
	sig := Sign([]byte("hello"), sec)
	sig[32] ^= 1 // flip one bit in S
	if Verify([]byte("hello"), sig, pub) {
		t.Error("should reject tampered S")
	}
}

func TestSOutOfRange(t *testing.T) {
	pub, sec := testKeypair()
	sig := Sign([]byte("hello"), sec)
	// Replace S with L (which is >= L, so rejected)
	sBytes := bigIntToLE32(curveL)
	copy(sig[32:], sBytes[:])
	if Verify([]byte("hello"), sig, pub) {
		t.Error("should reject S >= L")
	}
}

// ═══════════════════════════════════════════════════════════════════════════════
// KEY GENERATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

func TestDeterministic(t *testing.T) {
	var seed [32]byte
	for i := range seed {
		seed[i] = byte(i)
	}
	pub1, sec1 := GenerateKeypair(seed)
	pub2, sec2 := GenerateKeypair(seed)
	if pub1 != pub2 {
		t.Error("same seed should produce same public key")
	}
	if sec1 != sec2 {
		t.Error("same seed should produce same secret key")
	}
}

func TestSignDeterministic(t *testing.T) {
	var seed [32]byte
	for i := range seed {
		seed[i] = byte(i)
	}
	_, sec := GenerateKeypair(seed)
	sig1 := Sign([]byte("hello"), sec)
	sig2 := Sign([]byte("hello"), sec)
	if sig1 != sig2 {
		t.Error("same message+key should produce same signature")
	}
}

func TestSecretKeyFormat(t *testing.T) {
	var seed [32]byte
	for i := range seed {
		seed[i] = byte(i)
	}
	pub, sec := GenerateKeypair(seed)
	if [32]byte(sec[:32]) != seed {
		t.Error("first 32 bytes of secret key should be seed")
	}
	if [32]byte(sec[32:]) != pub {
		t.Error("last 32 bytes of secret key should be public key")
	}
}
