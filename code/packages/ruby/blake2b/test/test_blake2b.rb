# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_blake2b"

# All expected values in this file are pre-computed from Python's
# hashlib.blake2b, which wraps the reference implementation.  The same KATs
# are mirrored in every language's BLAKE2b test suite for cross-language
# consistency.
class TestBlake2b < Minitest::Test
  Blake2b = CodingAdventures::Blake2b

  def bytes_from_range(start, stop)
    (start...stop).map { |i| (i & 0xff).chr }.join.b
  end

  def hex(bytes)
    bytes.unpack1("H*")
  end

  # --- Canonical vectors ---

  def test_empty_message_default_digest
    assert_equal(
      "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce",
      Blake2b.blake2b_hex("".b)
    )
  end

  def test_abc
    assert_equal(
      "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923",
      Blake2b.blake2b_hex("abc".b)
    )
  end

  def test_fox
    assert_equal(
      "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918",
      Blake2b.blake2b_hex("The quick brown fox jumps over the lazy dog".b)
    )
  end

  def test_truncated_digest_size_32
    assert_equal(
      "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8",
      Blake2b.blake2b_hex("".b, digest_size: 32)
    )
  end

  def test_keyed_long_vector
    key = bytes_from_range(1, 65)
    data = bytes_from_range(0, 256)
    assert_equal(
      "402fa70e35f026c9bfc1202805e931b995647fe479e1701ad8b7203cddad5927ee7950b898a5a8229443d93963e4f6f27136b2b56f6845ab18f59bc130db8bf3",
      Blake2b.blake2b_hex(data, key: key)
    )
  end

  # --- Block-boundary sizes ---

  BLOCK_KATS = [
    [0, "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"],
    [1, "4fe4da61bcc756071b226843361d74944c72245d23e8245ea678c13fdcd7fe2ae529cf999ad99cc24f7a73416a18ba53e76c0afef83b16a568b12fbfc1a2674d"],
    [63, "70b2a0e6daecac22c7a2df82c06e3fc0b4c66bd5ef8098e4ed54e723b393d79ef3bceba079a01a14c6ef2ae2ed1171df1662cd14ef38e6f77b01c7f48144dd09"],
    [64, "3db7bb5c40745f0c975ac6bb8578f590e2cd2cc1fc6d13533ef725325c9fddff5cca24e7a591a0f6032a24fad0e09f6df873c4ff314628391f78df7f09cb7ed7"],
    [65, "149c114a3e8c6e06bafee27c9d0de0e39ef28294fa0d9f81876dcceb10bb41101e256593587e46b844819ed7ded90d56c0843df06c95d1695c3de635cd7a888e"],
    [127, "71546bbf9110ad184cc60f2eb120fcfd9b4dbbca7a7f1270045b8a23a6a4f4330f65c1f030dd2f5fabc6c57617242c37cf427bd90407fac5b9deffd3ae888c39"],
    [128, "2d9e329f42afa3601d646692b81c13e87fcaff5bf15972e9813d7373cb6d181f9599f4d513d4af4fd6ebd37497aceb29aba5ee23ed764d8510b552bd088814fb"],
    [129, "47889df9eb4d717afc5019df5c6a83df00a0b8677395e078cd5778ace0f338a618e68b7d9afb065d9e6a01ccd31d109447e7fae771c3ee3e105709194122ba2b"],
    [255, "1a5199ac66a00e8a87ad1c7fbad30b33137dd8312bf6d98602dacf8f40ea2cb623a7fbc63e5a6bfa434d337ae7da5ca1a52502a215a3fe0297a151be85d88789"],
    [256, "91019c558584980249ca43eceed27e19f1c3c24161b93eed1eee2a6a774f60bf8a81b43750870bee1698feac9c5336ae4d5c842e7ead159bf3916387e8ded9ae"],
    [257, "9f1975efca45e7b74b020975d4d2c22802906ed8bfefca51ac497bd23147fc8f303890d8e5471ab6caaa02362e831a9e8d3435279912ccd4842c7806b096c348"],
    [1024, "eddc3f3af9392eff065b359ce5f2b28f71e9f3a3a50e60ec27787b9fa623094d17b046c1dfce89bc5cdfc951b95a9a9c05fb8cc2361c905db01dd237fe56efb3"],
    [4096, "31404c9c7ed64c59112579f300f2afef181ee6283c3918bf026c4ed4bcde0697a7834f3a3410396622ef3d4f432602528a689498141c184cc2063554ba688dc7"],
    [9999, "b4a5808e65d7424b517bde11e04075a09b1343148e3ab2c8b13ff35c542e0a2beff6309ecc54b59ac046f6d65a9e3680c6372a033607709c95d5fd8070be6069"]
  ].freeze

  def test_block_boundary_sizes
    BLOCK_KATS.each do |size, want|
      data = Array.new(size) { |i| (i * 7 + 3) & 0xff }.pack("C*")
      assert_equal(want, Blake2b.blake2b_hex(data), "size #{size}")
    end
  end

  # --- Variable digest sizes ---

  DIGEST_SIZE_KATS = [
    [1, "b5"],
    [16, "249df9a49f517ddcd37f5c897620ec73"],
    [20, "3c523ed102ab45a37d54f5610d5a983162fde84f"],
    [32, "01718cec35cd3d796dd00020e0bfecb473ad23457d063b75eff29c0ffa2e58a9"],
    [48, "b7c81b228b6bd912930e8f0b5387989691c1cee1e65aade4da3b86a3c9f678fc8018f6ed9e2906720c8d2a3aeda9c03d"],
    [64, "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918"]
  ].freeze

  def test_variable_digest_sizes
    data = "The quick brown fox jumps over the lazy dog".b
    DIGEST_SIZE_KATS.each do |ds, want|
      out = Blake2b.blake2b(data, digest_size: ds)
      assert_equal(ds, out.bytesize, "digest_size #{ds}")
      assert_equal(want, hex(out), "digest_size #{ds}")
    end
  end

  # --- Keyed ---

  KEYED_KATS = [
    [1, "affd4e429aa2fb18da276f6ecff16f7d048769cacefe1a7ac75184448e082422"],
    [16, "5f8510d05dac42e8b6fc542af93f349d41ae4ebaf5cecae4af43fae54c7ca618"],
    [32, "88a78036d5890e91b5e3d70ba4738d2be302b76e0857d8ee029dc56dfa04fe67"],
    [64, "df7eab2ec9135ab8c58f48c288cdc873bac245a7fa46ca9f047cab672bd1eabb"]
  ].freeze

  def test_keyed_variants
    data = "secret message body".b
    KEYED_KATS.each do |klen, want|
      key = bytes_from_range(1, klen + 1)
      assert_equal(want, Blake2b.blake2b_hex(data, key: key, digest_size: 32), "keyLen #{klen}")
    end
  end

  def test_salt_and_personal
    salt = bytes_from_range(0, 16)
    personal = bytes_from_range(16, 32)
    assert_equal(
      "a2185d648fc63f3d363871a76360330c9b238af5466a20f94bb64d363289b95da0453438eea300cd6f31521274ec001011fa29e91a603fabf00f2b454e30bf3d",
      Blake2b.blake2b_hex("parameterized hash".b, salt: salt, personal: personal)
    )
  end

  # --- Streaming ---

  def test_streaming_single_chunk_matches_one_shot
    h = Blake2b::Hasher.new
    h.update("hello world".b)
    assert_equal hex(Blake2b.blake2b("hello world".b)), hex(h.digest)
  end

  def test_streaming_byte_by_byte
    data = bytes_from_range(0, 200)
    h = Blake2b::Hasher.new(digest_size: 32)
    data.bytes.each { |b| h.update([b].pack("C")) }
    assert_equal hex(Blake2b.blake2b(data, digest_size: 32)), hex(h.digest)
  end

  def test_streaming_across_block_boundary
    data = bytes_from_range(0, 129)
    h = Blake2b::Hasher.new
    h.update(data.byteslice(0, 127))
    h.update(data.byteslice(127, data.bytesize - 127))
    assert_equal hex(Blake2b.blake2b(data)), hex(h.digest)
  end

  def test_streaming_exact_block_then_more
    # 128 bytes exact, then 4 more.  The 128-byte block must NOT be flagged
    # final while more data is still coming.
    data = Array.new(132) { |i| i & 0xff }.pack("C*")
    h = Blake2b::Hasher.new
    h.update(data.byteslice(0, 128))
    h.update(data.byteslice(128, 4))
    assert_equal hex(Blake2b.blake2b(data)), hex(h.digest)
  end

  def test_digest_is_idempotent
    h = Blake2b::Hasher.new
    h.update("hello".b)
    assert_equal h.hex_digest, h.hex_digest
  end

  def test_update_after_digest_continues_stream
    h = Blake2b::Hasher.new(digest_size: 32)
    h.update("hello ".b)
    _ = h.digest
    h.update("world".b)
    assert_equal(
      Blake2b.blake2b_hex("hello world".b, digest_size: 32),
      h.hex_digest
    )
  end

  def test_copy_is_independent
    h = Blake2b::Hasher.new
    h.update("prefix ".b)
    c = h.copy
    h.update("path A".b)
    c.update("path B".b)
    assert_equal hex(Blake2b.blake2b("prefix path A".b)), hex(h.digest)
    assert_equal hex(Blake2b.blake2b("prefix path B".b)), hex(c.digest)
  end

  # --- Validation ---

  def test_rejects_digest_size_zero
    assert_raises(ArgumentError) { Blake2b.blake2b("".b, digest_size: 0) }
  end

  def test_rejects_digest_size_65
    assert_raises(ArgumentError) { Blake2b.blake2b("".b, digest_size: 65) }
  end

  def test_rejects_digest_size_non_integer
    assert_raises(ArgumentError) { Blake2b.blake2b("".b, digest_size: 1.5) }
  end

  def test_rejects_key_too_long
    assert_raises(ArgumentError) { Blake2b.blake2b("".b, key: ("a" * 65).b) }
  end

  def test_rejects_wrong_salt_length
    assert_raises(ArgumentError) { Blake2b.blake2b("".b, salt: ("a" * 8).b) }
  end

  def test_rejects_wrong_personal_length
    assert_raises(ArgumentError) { Blake2b.blake2b("".b, personal: ("a" * 20).b) }
  end

  def test_accepts_max_64_byte_key
    Blake2b.blake2b("x".b, key: ("\x41".b * 64))
  end

  def test_hex_digest_matches_bytes_of_digest
    h = Blake2b::Hasher.new(digest_size: 32)
    h.update("hex check".b)
    assert_equal hex(h.digest), h.hex_digest
  end
end
