package blake2b

import (
	"bytes"
	"encoding/hex"
	"testing"
)

// Test vectors across this file are cross-validated against Python's
// hashlib.blake2b, which wraps the reference BLAKE2 implementation. The same
// KATs are mirrored in every language implementation in this repo so that a
// regression in any one language is visible as a divergence from the shared
// numbers below.

// ----------------------------------------------------------------------
// Canonical vectors -- a handful of well-known inputs
// ----------------------------------------------------------------------

func TestEmptyMessage(t *testing.T) {
	want := "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"
	got, err := SumHex(nil, 64, nil, nil, nil)
	if err != nil {
		t.Fatal(err)
	}
	if got != want {
		t.Fatalf("got %s\nwant %s", got, want)
	}
}

func TestAbc(t *testing.T) {
	want := "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
	got, _ := SumHex([]byte("abc"), 64, nil, nil, nil)
	if got != want {
		t.Fatalf("got %s\nwant %s", got, want)
	}
}

func TestQuickBrownFox(t *testing.T) {
	want := "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918"
	got, _ := SumHex([]byte("The quick brown fox jumps over the lazy dog"), 64, nil, nil, nil)
	if got != want {
		t.Fatalf("got %s\nwant %s", got, want)
	}
}

func TestTruncatedDigest(t *testing.T) {
	want := "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8"
	got, _ := SumHex(nil, 32, nil, nil, nil)
	if got != want {
		t.Fatalf("got %s\nwant %s", got, want)
	}
}

func TestKeyedLongVector(t *testing.T) {
	key := make([]byte, 64)
	for i := range key {
		key[i] = byte(i + 1)
	}
	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i)
	}
	want := "402fa70e35f026c9bfc1202805e931b995647fe479e1701ad8b7203cddad5927ee7950b898a5a8229443d93963e4f6f27136b2b56f6845ab18f59bc130db8bf3"
	got, _ := SumHex(data, 64, key, nil, nil)
	if got != want {
		t.Fatalf("got %s\nwant %s", got, want)
	}
}

// ----------------------------------------------------------------------
// Block-boundary message-length KATs. BlockSize is 128 -- the values at
// 127, 128, and 129 catch the final-block-flag off-by-one.
// ----------------------------------------------------------------------

var sizeKAT = []struct {
	size int
	want string
}{
	{0, "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"},
	{1, "4fe4da61bcc756071b226843361d74944c72245d23e8245ea678c13fdcd7fe2ae529cf999ad99cc24f7a73416a18ba53e76c0afef83b16a568b12fbfc1a2674d"},
	{63, "70b2a0e6daecac22c7a2df82c06e3fc0b4c66bd5ef8098e4ed54e723b393d79ef3bceba079a01a14c6ef2ae2ed1171df1662cd14ef38e6f77b01c7f48144dd09"},
	{64, "3db7bb5c40745f0c975ac6bb8578f590e2cd2cc1fc6d13533ef725325c9fddff5cca24e7a591a0f6032a24fad0e09f6df873c4ff314628391f78df7f09cb7ed7"},
	{65, "149c114a3e8c6e06bafee27c9d0de0e39ef28294fa0d9f81876dcceb10bb41101e256593587e46b844819ed7ded90d56c0843df06c95d1695c3de635cd7a888e"},
	{127, "71546bbf9110ad184cc60f2eb120fcfd9b4dbbca7a7f1270045b8a23a6a4f4330f65c1f030dd2f5fabc6c57617242c37cf427bd90407fac5b9deffd3ae888c39"},
	{128, "2d9e329f42afa3601d646692b81c13e87fcaff5bf15972e9813d7373cb6d181f9599f4d513d4af4fd6ebd37497aceb29aba5ee23ed764d8510b552bd088814fb"},
	{129, "47889df9eb4d717afc5019df5c6a83df00a0b8677395e078cd5778ace0f338a618e68b7d9afb065d9e6a01ccd31d109447e7fae771c3ee3e105709194122ba2b"},
	{255, "1a5199ac66a00e8a87ad1c7fbad30b33137dd8312bf6d98602dacf8f40ea2cb623a7fbc63e5a6bfa434d337ae7da5ca1a52502a215a3fe0297a151be85d88789"},
	{256, "91019c558584980249ca43eceed27e19f1c3c24161b93eed1eee2a6a774f60bf8a81b43750870bee1698feac9c5336ae4d5c842e7ead159bf3916387e8ded9ae"},
	{257, "9f1975efca45e7b74b020975d4d2c22802906ed8bfefca51ac497bd23147fc8f303890d8e5471ab6caaa02362e831a9e8d3435279912ccd4842c7806b096c348"},
	{1024, "eddc3f3af9392eff065b359ce5f2b28f71e9f3a3a50e60ec27787b9fa623094d17b046c1dfce89bc5cdfc951b95a9a9c05fb8cc2361c905db01dd237fe56efb3"},
	{4096, "31404c9c7ed64c59112579f300f2afef181ee6283c3918bf026c4ed4bcde0697a7834f3a3410396622ef3d4f432602528a689498141c184cc2063554ba688dc7"},
	{9999, "b4a5808e65d7424b517bde11e04075a09b1343148e3ab2c8b13ff35c542e0a2beff6309ecc54b59ac046f6d65a9e3680c6372a033607709c95d5fd8070be6069"},
}

func TestAcrossSizes(t *testing.T) {
	for _, tc := range sizeKAT {
		data := make([]byte, tc.size)
		for i := range data {
			data[i] = byte((i*7 + 3) & 0xff)
		}
		got, _ := SumHex(data, 64, nil, nil, nil)
		if got != tc.want {
			t.Errorf("size=%d\n  got  = %s\n  want = %s", tc.size, got, tc.want)
		}
	}
}

// ----------------------------------------------------------------------
// Variable digest sizes
// ----------------------------------------------------------------------

func TestVariousDigestSizes(t *testing.T) {
	data := []byte("The quick brown fox jumps over the lazy dog")
	cases := []struct {
		ds   int
		want string
	}{
		{1, "b5"},
		{16, "249df9a49f517ddcd37f5c897620ec73"},
		{20, "3c523ed102ab45a37d54f5610d5a983162fde84f"},
		{32, "01718cec35cd3d796dd00020e0bfecb473ad23457d063b75eff29c0ffa2e58a9"},
		{48, "b7c81b228b6bd912930e8f0b5387989691c1cee1e65aade4da3b86a3c9f678fc8018f6ed9e2906720c8d2a3aeda9c03d"},
		{64, "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918"},
	}
	for _, tc := range cases {
		got, err := SumHex(data, tc.ds, nil, nil, nil)
		if err != nil {
			t.Fatalf("ds=%d: %v", tc.ds, err)
		}
		if got != tc.want {
			t.Errorf("ds=%d\n  got  = %s\n  want = %s", tc.ds, got, tc.want)
		}
		if len(got) != 2*tc.ds {
			t.Errorf("ds=%d but hex digest length=%d", tc.ds, len(got))
		}
	}
}

// ----------------------------------------------------------------------
// Keyed vectors across several key lengths
// ----------------------------------------------------------------------

func TestKeyedAcrossKeyLengths(t *testing.T) {
	data := []byte("secret message body")
	cases := []struct {
		keyLen int
		want   string
	}{
		{1, "affd4e429aa2fb18da276f6ecff16f7d048769cacefe1a7ac75184448e082422"},
		{16, "5f8510d05dac42e8b6fc542af93f349d41ae4ebaf5cecae4af43fae54c7ca618"},
		{32, "88a78036d5890e91b5e3d70ba4738d2be302b76e0857d8ee029dc56dfa04fe67"},
		{64, "df7eab2ec9135ab8c58f48c288cdc873bac245a7fa46ca9f047cab672bd1eabb"},
	}
	for _, tc := range cases {
		key := make([]byte, tc.keyLen)
		for i := range key {
			key[i] = byte(i + 1)
		}
		got, err := SumHex(data, 32, key, nil, nil)
		if err != nil {
			t.Fatalf("keyLen=%d: %v", tc.keyLen, err)
		}
		if got != tc.want {
			t.Errorf("keyLen=%d\n  got  = %s\n  want = %s", tc.keyLen, got, tc.want)
		}
	}
}

func TestSaltAndPersonal(t *testing.T) {
	salt := make([]byte, 16)
	personal := make([]byte, 16)
	for i := range salt {
		salt[i] = byte(i)
		personal[i] = byte(i + 16)
	}
	want := "a2185d648fc63f3d363871a76360330c9b238af5466a20f94bb64d363289b95da0453438eea300cd6f31521274ec001011fa29e91a603fabf00f2b454e30bf3d"
	got, err := SumHex([]byte("parameterized hash"), 64, nil, salt, personal)
	if err != nil {
		t.Fatal(err)
	}
	if got != want {
		t.Fatalf("salt+personal mismatch\n  got  = %s\n  want = %s", got, want)
	}
}

// ----------------------------------------------------------------------
// Streaming behavior
// ----------------------------------------------------------------------

func TestStreamingSingleChunk(t *testing.T) {
	h, _ := New(64, nil, nil, nil)
	h.Update([]byte("hello world"))
	want, _ := Sum([]byte("hello world"), 64, nil, nil, nil)
	if !bytes.Equal(h.Digest(), want) {
		t.Fatal("streaming single chunk mismatch")
	}
}

func TestStreamingByteByByte(t *testing.T) {
	data := make([]byte, 200)
	for i := range data {
		data[i] = byte(i)
	}
	h, _ := New(32, nil, nil, nil)
	for _, b := range data {
		h.Update([]byte{b})
	}
	want, _ := Sum(data, 32, nil, nil, nil)
	if !bytes.Equal(h.Digest(), want) {
		t.Fatal("byte-by-byte streaming mismatch")
	}
}

func TestStreamingAcrossBlockBoundary(t *testing.T) {
	data := make([]byte, 129)
	for i := range data {
		data[i] = byte(i)
	}
	h, _ := New(64, nil, nil, nil)
	h.Update(data[:127])
	h.Update(data[127:])
	want, _ := Sum(data, 64, nil, nil, nil)
	if !bytes.Equal(h.Digest(), want) {
		t.Fatal("streaming across block boundary mismatch")
	}
}

func TestStreamingExactBlockThenMore(t *testing.T) {
	// Canonical BLAKE2 off-by-one: 128 bytes exact, then more.  The hasher
	// must not flag the first 128-byte block as final while more data is
	// still coming.
	data := make([]byte, 128+4)
	for i := range data {
		data[i] = byte(i)
	}
	h, _ := New(64, nil, nil, nil)
	h.Update(data[:128])
	h.Update(data[128:])
	want, _ := Sum(data, 64, nil, nil, nil)
	if !bytes.Equal(h.Digest(), want) {
		t.Fatal("streaming exact-block-then-more mismatch")
	}
}

func TestDigestIdempotent(t *testing.T) {
	h, _ := New(64, nil, nil, nil)
	h.Update([]byte("hello"))
	first := h.Digest()
	second := h.Digest()
	if !bytes.Equal(first, second) {
		t.Fatal("digest should be idempotent")
	}
}

func TestUpdateAfterDigest(t *testing.T) {
	h, _ := New(32, nil, nil, nil)
	h.Update([]byte("hello "))
	_ = h.Digest()
	h.Update([]byte("world"))
	want, _ := Sum([]byte("hello world"), 32, nil, nil, nil)
	if !bytes.Equal(h.Digest(), want) {
		t.Fatal("update after digest should continue the stream")
	}
}

func TestCopyIsIndependent(t *testing.T) {
	h, _ := New(64, nil, nil, nil)
	h.Update([]byte("prefix "))
	clone := h.Copy()
	h.Update([]byte("path A"))
	clone.Update([]byte("path B"))

	wantA, _ := Sum([]byte("prefix path A"), 64, nil, nil, nil)
	wantB, _ := Sum([]byte("prefix path B"), 64, nil, nil, nil)
	if !bytes.Equal(h.Digest(), wantA) {
		t.Fatal("original hasher diverged after copy")
	}
	if !bytes.Equal(clone.Digest(), wantB) {
		t.Fatal("clone did not evolve independently")
	}
}

// ----------------------------------------------------------------------
// Argument validation
// ----------------------------------------------------------------------

func TestInvalidDigestSize(t *testing.T) {
	for _, bad := range []int{0, -1, 65, 100} {
		if _, err := New(bad, nil, nil, nil); err == nil {
			t.Errorf("expected error for digest size %d", bad)
		}
	}
}

func TestKeyTooLong(t *testing.T) {
	if _, err := New(32, make([]byte, 65), nil, nil); err == nil {
		t.Error("expected error for key length 65")
	}
}

func TestSaltWrongLength(t *testing.T) {
	if _, err := New(32, nil, make([]byte, 8), nil); err == nil {
		t.Error("expected error for salt length 8")
	}
}

func TestPersonalWrongLength(t *testing.T) {
	if _, err := New(32, nil, nil, make([]byte, 20)); err == nil {
		t.Error("expected error for personal length 20")
	}
}

func TestMaxLengthKeyAccepted(t *testing.T) {
	key := make([]byte, 64)
	for i := range key {
		key[i] = byte(i + 1)
	}
	if _, err := Sum([]byte("x"), 64, key, nil, nil); err != nil {
		t.Fatalf("64-byte key should be accepted: %v", err)
	}
}

func TestHexDigestMatchesDigestHex(t *testing.T) {
	h, _ := New(32, nil, nil, nil)
	h.Update([]byte("hex check"))
	if h.HexDigest() != hex.EncodeToString(h.Digest()) {
		t.Fatal("hex digest should match digest.hex()")
	}
}
