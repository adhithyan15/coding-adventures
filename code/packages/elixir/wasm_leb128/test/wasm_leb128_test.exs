defmodule CodingAdventures.WasmLeb128Test do
  use ExUnit.Case
  doctest CodingAdventures.WasmLeb128

  alias CodingAdventures.WasmLeb128

  # ── Module Loads ───────────────────────────────────────────────────────────

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.WasmLeb128)
  end

  # ── Unsigned Decoding ──────────────────────────────────────────────────────

  describe "decode_unsigned/2" do
    # Test case 1: Zero
    test "decodes zero from <<0x00>>" do
      assert WasmLeb128.decode_unsigned(<<0x00>>, 0) == {:ok, {0, 1}}
    end

    # Test case 2: One-byte unsigned value
    test "decodes 3 from <<0x03>>" do
      assert WasmLeb128.decode_unsigned(<<0x03>>, 0) == {:ok, {3, 1}}
    end

    # Test case 4: Multi-byte value 624485
    # Note: the correct LEB128 encoding of 624485 is <<0xE5, 0x8E, 0x26>>.
    # Some references erroneously list <<0xE5, 0x88, 0x26>>, which decodes
    # to 623717 not 624485. The correct second byte is 0x8E.
    test "decodes 624485 from <<0xE5, 0x8E, 0x26>>" do
      assert WasmLeb128.decode_unsigned(<<0xE5, 0x8E, 0x26>>, 0) == {:ok, {624485, 3}}
    end

    # Test case 5: Max u32 = 4294967295 = 0xFFFFFFFF
    test "decodes max u32 (4294967295)" do
      data = <<0xFF, 0xFF, 0xFF, 0xFF, 0x0F>>
      assert WasmLeb128.decode_unsigned(data, 0) == {:ok, {4294967295, 5}}
    end

    # Test case 10: Non-zero offset
    test "decodes at non-zero offset" do
      # Two garbage bytes then 624485
      buf = <<0x00, 0x00, 0xE5, 0x8E, 0x26>>
      assert WasmLeb128.decode_unsigned(buf, 2) == {:ok, {624485, 3}}
    end

    # Test case 9: Unterminated sequence
    test "returns error for unterminated sequence <<0x80, 0x80>>" do
      assert {:error, msg} = WasmLeb128.decode_unsigned(<<0x80, 0x80>>, 0)
      assert String.contains?(msg, "unterminated")
    end

    test "returns error when offset is out of bounds" do
      assert {:error, msg} = WasmLeb128.decode_unsigned(<<0x01>>, 5)
      assert String.contains?(msg, "out of bounds")
    end

    test "default offset is 0" do
      assert WasmLeb128.decode_unsigned(<<0x03>>) == {:ok, {3, 1}}
    end
  end

  # ── Signed Decoding ────────────────────────────────────────────────────────

  describe "decode_signed/2" do
    # Test case 1 (signed): Zero
    test "decodes zero from <<0x00>>" do
      assert WasmLeb128.decode_signed(<<0x00>>, 0) == {:ok, {0, 1}}
    end

    # Test case 3: One-byte signed negative
    # 0x7E = 0b0111_1110, data bits = 0b111_1110, bit 6 set → sign extend → -2
    test "decodes -2 from <<0x7E>>" do
      assert WasmLeb128.decode_signed(<<0x7E>>, 0) == {:ok, {-2, 1}}
    end

    # Test case 6: Max i32 = 2147483647
    test "decodes max i32 (2147483647)" do
      data = <<0xFF, 0xFF, 0xFF, 0xFF, 0x07>>
      assert WasmLeb128.decode_signed(data, 0) == {:ok, {2147483647, 5}}
    end

    # Test case 7: Min i32 = -2147483648
    test "decodes min i32 (-2147483648)" do
      data = <<0x80, 0x80, 0x80, 0x80, 0x78>>
      assert WasmLeb128.decode_signed(data, 0) == {:ok, {-2147483648, 5}}
    end

    # Test case 9 (signed): Unterminated
    test "returns error for unterminated sequence" do
      assert {:error, msg} = WasmLeb128.decode_signed(<<0x80, 0x80>>, 0)
      assert String.contains?(msg, "unterminated")
    end

    # Test case 10 (signed): Non-zero offset
    test "decodes at non-zero offset" do
      # Place -2 (0x7E) at offset 3
      buf = <<0x00, 0x00, 0x00, 0x7E>>
      assert WasmLeb128.decode_signed(buf, 3) == {:ok, {-2, 1}}
    end

    test "returns error when offset is out of bounds" do
      assert {:error, msg} = WasmLeb128.decode_signed(<<0x01>>, 5)
      assert String.contains?(msg, "out of bounds")
    end

    test "default offset is 0" do
      assert WasmLeb128.decode_signed(<<0x7E>>) == {:ok, {-2, 1}}
    end
  end

  # ── Unsigned Encoding ──────────────────────────────────────────────────────

  describe "encode_unsigned/1" do
    test "encodes 0 as <<0x00>>" do
      assert WasmLeb128.encode_unsigned(0) == <<0x00>>
    end

    test "encodes 3 as <<0x03>>" do
      assert WasmLeb128.encode_unsigned(3) == <<0x03>>
    end

    test "encodes 624485 as <<0xE5, 0x8E, 0x26>>" do
      assert WasmLeb128.encode_unsigned(624485) == <<0xE5, 0x8E, 0x26>>
    end

    test "encodes max u32 (4294967295)" do
      assert WasmLeb128.encode_unsigned(4294967295) == <<0xFF, 0xFF, 0xFF, 0xFF, 0x0F>>
    end
  end

  # ── Signed Encoding ────────────────────────────────────────────────────────

  describe "encode_signed/1" do
    test "encodes 0 as <<0x00>>" do
      assert WasmLeb128.encode_signed(0) == <<0x00>>
    end

    test "encodes -2 as <<0x7E>>" do
      assert WasmLeb128.encode_signed(-2) == <<0x7E>>
    end

    test "encodes -2147483648 (min i32)" do
      assert WasmLeb128.encode_signed(-2147483648) == <<0x80, 0x80, 0x80, 0x80, 0x78>>
    end

    test "encodes 2147483647 (max i32)" do
      assert WasmLeb128.encode_signed(2147483647) == <<0xFF, 0xFF, 0xFF, 0xFF, 0x07>>
    end
  end

  # ── Round-Trips ────────────────────────────────────────────────────────────

  describe "unsigned round-trips" do
    # Test case 8: encode → decode must recover the original value.
    for value <- [0, 1, 127, 128, 255, 624_485, 4_294_967_295] do
      @value value
      test "round-trips unsigned #{@value}" do
        encoded = WasmLeb128.encode_unsigned(@value)
        assert {:ok, {@value, consumed}} = WasmLeb128.decode_unsigned(encoded, 0)
        assert consumed == byte_size(encoded)
      end
    end
  end

  describe "signed round-trips" do
    # Test case 11: signed negative round-trips.
    for value <- [0, 1, -1, -2, 63, -64, 127, -128, 2_147_483_647, -2_147_483_648] do
      @value value
      test "round-trips signed #{@value}" do
        encoded = WasmLeb128.encode_signed(@value)
        assert {:ok, {@value, consumed}} = WasmLeb128.decode_signed(encoded, 0)
        assert consumed == byte_size(encoded)
      end
    end
  end
end
