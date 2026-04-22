defmodule CodingAdventures.Argon2idTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Argon2id

  @rfc_password :binary.copy(<<0x01>>, 32)
  @rfc_salt :binary.copy(<<0x02>>, 16)
  @rfc_key :binary.copy(<<0x03>>, 8)
  @rfc_ad :binary.copy(<<0x04>>, 12)
  @rfc_expected_hex "0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659"

  test "RFC 9106 §5.3 gold-standard vector" do
    assert Argon2id.argon2id_hex(@rfc_password, @rfc_salt, 3, 32, 4, 32,
             key: @rfc_key, associated_data: @rfc_ad) == @rfc_expected_hex
  end

  test "hex matches binary" do
    tag = Argon2id.argon2id(@rfc_password, @rfc_salt, 3, 32, 4, 32,
                            key: @rfc_key, associated_data: @rfc_ad)
    assert Base.encode16(tag, case: :lower) == @rfc_expected_hex
  end

  test "rejects short salt" do
    assert_raise ArgumentError, fn -> Argon2id.argon2id("pw", "short", 1, 8, 1, 32) end
  end

  test "rejects zero time_cost" do
    assert_raise ArgumentError, fn ->
      Argon2id.argon2id("pw", String.duplicate("a", 8), 0, 8, 1, 32)
    end
  end

  test "rejects tag_length under 4" do
    assert_raise ArgumentError, fn ->
      Argon2id.argon2id("pw", String.duplicate("a", 8), 1, 8, 1, 3)
    end
  end

  test "rejects memory below floor" do
    assert_raise ArgumentError, fn ->
      Argon2id.argon2id("pw", String.duplicate("a", 8), 1, 7, 1, 32)
    end
  end

  test "rejects zero parallelism" do
    assert_raise ArgumentError, fn ->
      Argon2id.argon2id("pw", String.duplicate("a", 8), 1, 8, 0, 32)
    end
  end

  test "rejects unsupported version" do
    assert_raise ArgumentError, fn ->
      Argon2id.argon2id("pw", String.duplicate("a", 8), 1, 8, 1, 32, version: 0x10)
    end
  end

  test "deterministic" do
    a = Argon2id.argon2id_hex("pw", String.duplicate("a", 8), 1, 8, 1, 32)
    b = Argon2id.argon2id_hex("pw", String.duplicate("a", 8), 1, 8, 1, 32)
    assert a == b
  end

  test "differs on password" do
    refute Argon2id.argon2id_hex("pw1", String.duplicate("a", 8), 1, 8, 1, 32) ==
             Argon2id.argon2id_hex("pw2", String.duplicate("a", 8), 1, 8, 1, 32)
  end

  test "differs on salt" do
    refute Argon2id.argon2id_hex("pw", String.duplicate("a", 8), 1, 8, 1, 32) ==
             Argon2id.argon2id_hex("pw", String.duplicate("b", 8), 1, 8, 1, 32)
  end

  test "key binds" do
    a = Argon2id.argon2id_hex("pw", String.duplicate("a", 8), 1, 8, 1, 32)
    b = Argon2id.argon2id_hex("pw", String.duplicate("a", 8), 1, 8, 1, 32, key: "k1")
    c = Argon2id.argon2id_hex("pw", String.duplicate("a", 8), 1, 8, 1, 32, key: "k2")
    refute a == b
    refute b == c
  end

  test "associated_data binds" do
    a = Argon2id.argon2id_hex("pw", String.duplicate("a", 8), 1, 8, 1, 32)
    b = Argon2id.argon2id_hex("pw", String.duplicate("a", 8), 1, 8, 1, 32, associated_data: "x")
    c = Argon2id.argon2id_hex("pw", String.duplicate("a", 8), 1, 8, 1, 32, associated_data: "y")
    refute a == b
    refute b == c
  end

  test "tag_length 4" do
    assert byte_size(Argon2id.argon2id("pw", String.duplicate("a", 8), 1, 8, 1, 4)) == 4
  end

  test "tag_length 16" do
    assert byte_size(Argon2id.argon2id("pw", String.duplicate("a", 8), 1, 8, 1, 16)) == 16
  end

  test "tag_length 65 crosses H' boundary" do
    assert byte_size(Argon2id.argon2id("pw", String.duplicate("a", 8), 1, 8, 1, 65)) == 65
  end

  test "tag_length 128" do
    assert byte_size(Argon2id.argon2id("pw", String.duplicate("a", 8), 1, 8, 1, 128)) == 128
  end

  test "multi-lane" do
    assert byte_size(Argon2id.argon2id("pw", String.duplicate("a", 8), 1, 16, 2, 32)) == 32
  end

  test "multi-pass" do
    refute Argon2id.argon2id_hex("pw", String.duplicate("a", 8), 1, 8, 1, 32) ==
             Argon2id.argon2id_hex("pw", String.duplicate("a", 8), 2, 8, 1, 32)
  end
end
