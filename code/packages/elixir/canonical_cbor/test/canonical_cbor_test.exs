defmodule CodingAdventures.CanonicalCborTest do
  use ExUnit.Case, async: true
  import Bitwise

  alias CodingAdventures.CanonicalCbor

  alias CodingAdventures.CanonicalCbor.{
    CborError,
    Constants,
    DecodeMapSpansResult,
    Messages,
    Span,
    SpannedMapEntry,
    Types
  }

  alias CodingAdventures.CanonicalCbor.Types.{
    CborArray,
    CborBool,
    CborBytes,
    CborMap,
    CborMapEntry,
    CborNegative,
    CborNull,
    CborTag,
    CborText,
    CborUnsigned
  }

  # ===========================================================================
  # Simple PRNG for Property & Mutation Tests
  # ===========================================================================

  defmodule PRNG do
    def new(seed) do
      rem(seed, 0xFFFFFFFF)
    end

    def next_u32(state) do
      new_state = rem(state * 1_664_525 + 1_013_904_223, 0x100000000)
      {new_state, new_state}
    end

    def next_int(state, max) when max > 0 do
      {val, next_state} = next_u32(state)
      {rem(val, max), next_state}
    end

    def next_bytes(state, len) do
      Enum.reduce(1..len, {<<>>, state}, fn _, {acc, s} ->
        {b, s2} = next_int(s, 256)
        {acc <> <<b::8>>, s2}
      end)
    end
  end

  # ===========================================================================
  # 1. 7-Class BEAM/Elixir CVE Resilience Showcase
  # ===========================================================================

  describe "CVE-1: 64-bit integer wraparound and float precision trap" do
    test "rejects float numbers in integer constructors" do
      assert_raise CborError, fn -> CanonicalCbor.cbor_unsigned(1.5) end
      assert_raise CborError, fn -> CanonicalCbor.cbor_negative(2.5) end
      assert_raise CborError, fn -> CanonicalCbor.cbor_tag(1.5, CanonicalCbor.cbor_null()) end
    end

    test "rejects negative numbers in unsigned and tag constructors" do
      assert_raise CborError, fn -> CanonicalCbor.cbor_unsigned(-1) end
      assert_raise CborError, fn -> CanonicalCbor.cbor_tag(-1, CanonicalCbor.cbor_null()) end
    end

    test "rejects integers above 2^64 - 1" do
      too_big = 18_446_744_073_709_551_616
      assert_raise CborError, fn -> CanonicalCbor.cbor_unsigned(too_big) end
      assert_raise CborError, fn -> CanonicalCbor.cbor_negative(too_big) end
      assert_raise CborError, fn -> CanonicalCbor.cbor_tag(too_big, CanonicalCbor.cbor_null()) end
    end

    test "exact 64-bit boundaries roundtrip identically" do
      boundaries = [
        0,
        23,
        24,
        255,
        256,
        65535,
        65536,
        4_294_967_295,
        4_294_967_296,
        18_446_744_073_709_551_615
      ]

      for b <- boundaries do
        u = CanonicalCbor.cbor_unsigned(b)
        assert {:ok, enc} = CanonicalCbor.encode_checked(u)
        assert {:ok, dec} = CanonicalCbor.decode(enc)
        assert dec == u
        assert dec.value == b

        n = CanonicalCbor.cbor_negative(b)
        assert {:ok, enc_n} = CanonicalCbor.encode_checked(n)
        assert {:ok, dec_n} = CanonicalCbor.decode(enc_n)
        assert dec_n == n
        assert dec_n.value == b
      end
    end
  end

  describe "CVE-2: Pre-allocation OOM expansion bomb defense" do
    test "rejects hostile byte lengths before allocation" do
      hostile_bytes = <<0x5B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF>>
      assert {:error, %CborError{id: :length_too_large}} = CanonicalCbor.decode(hostile_bytes)
    end

    test "rejects hostile text lengths before allocation" do
      hostile_text = <<0x7B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF>>
      assert {:error, %CborError{id: :length_too_large}} = CanonicalCbor.decode(hostile_text)
    end

    test "rejects hostile array declared length exceeding capacity" do
      hostile_array = <<0x98, 100, 1>>
      assert {:error, %CborError{id: :length_too_large}} = CanonicalCbor.decode(hostile_array)
    end

    test "rejects hostile map declared length exceeding capacity" do
      hostile_map = <<0xB8, 100, 0, 1>>
      assert {:error, %CborError{id: :length_too_large}} = CanonicalCbor.decode(hostile_map)
    end
  end

  describe "CVE-3: Deep nesting recursion stack exhaustion (depth 128 vs 129)" do
    test "depth 128 arrays encode and decode cleanly" do
      cur =
        Enum.reduce(1..128, CanonicalCbor.cbor_array([]), fn _, acc ->
          CanonicalCbor.cbor_array([acc])
        end)

      assert {:ok, enc} = CanonicalCbor.encode_checked(cur)
      assert {:ok, dec} = CanonicalCbor.decode(enc)
      assert dec == cur
    end

    test "depth 129 arrays fail on encode and decode" do
      deep_128 =
        Enum.reduce(1..128, CanonicalCbor.cbor_array([]), fn _, acc ->
          CanonicalCbor.cbor_array([acc])
        end)

      deep_129 = CanonicalCbor.cbor_array([deep_128])
      assert {:error, %CborError{id: :encode_too_deep}} = CanonicalCbor.encode_checked(deep_129)

      wire_129 = String.duplicate(<<0x81>>, 129) <> <<0x80>>
      assert {:error, %CborError{id: :too_deep}} = CanonicalCbor.decode(wire_129)
    end

    test "nested tags: 128 tags succeed, 129 tags fail" do
      cur_tag =
        Enum.reduce(1..128, CanonicalCbor.cbor_unsigned(1), fn _, acc ->
          CanonicalCbor.cbor_tag(0, acc)
        end)

      assert {:ok, enc_tag} = CanonicalCbor.encode_checked(cur_tag)
      assert {:ok, dec_tag} = CanonicalCbor.decode(enc_tag)
      assert dec_tag == cur_tag

      tag_129 = CanonicalCbor.cbor_tag(0, cur_tag)
      assert {:error, %CborError{id: :encode_too_deep}} = CanonicalCbor.encode_checked(tag_129)
    end

    test "nested map keys: 128 map keys succeed, 129 map keys fail symmetrically" do
      cur_map =
        Enum.reduce(1..128, CanonicalCbor.cbor_unsigned(1), fn _, acc ->
          CanonicalCbor.cbor_map([
            CanonicalCbor.cbor_map_entry(acc, CanonicalCbor.cbor_unsigned(0))
          ])
        end)

      assert {:ok, enc_mk} = CanonicalCbor.encode_checked(cur_map)
      assert {:ok, dec_mk} = CanonicalCbor.decode(enc_mk)
      assert dec_mk == cur_map

      map_129 =
        CanonicalCbor.cbor_map([
          CanonicalCbor.cbor_map_entry(cur_map, CanonicalCbor.cbor_unsigned(0))
        ])

      assert {:error, %CborError{id: :encode_too_deep}} = CanonicalCbor.encode_checked(map_129)

      bad_mk_wire = String.duplicate(<<0xA1>>, 129) <> <<0x01>> <> String.duplicate(<<0x00>>, 129)
      assert {:error, %CborError{id: :too_deep}} = CanonicalCbor.decode(bad_mk_wire)
    end
  end

  describe "CVE-4: Canonical length-first map key ordering & duplicate key defense" do
    test "reorders map keys length-first and lex-tiebroken" do
      k1 = CanonicalCbor.cbor_unsigned(1)
      k2 = CanonicalCbor.cbor_unsigned(24)
      k3 = CanonicalCbor.cbor_text("a")
      k4 = CanonicalCbor.cbor_text("b")
      k5 = CanonicalCbor.cbor_bytes(<<1, 2, 3, 4, 5>>)

      map_val =
        CanonicalCbor.cbor_map([
          CanonicalCbor.cbor_map_entry(k5, CanonicalCbor.cbor_null()),
          CanonicalCbor.cbor_map_entry(k4, CanonicalCbor.cbor_bool(false)),
          CanonicalCbor.cbor_map_entry(k1, CanonicalCbor.cbor_unsigned(100)),
          CanonicalCbor.cbor_map_entry(k3, CanonicalCbor.cbor_bool(true)),
          CanonicalCbor.cbor_map_entry(k2, CanonicalCbor.cbor_text("val"))
        ])

      assert {:ok, enc} = CanonicalCbor.encode_checked(map_val)
      assert {:ok, dec} = CanonicalCbor.decode(enc)

      expected_canonical =
        CanonicalCbor.cbor_map([
          CanonicalCbor.cbor_map_entry(k1, CanonicalCbor.cbor_unsigned(100)),
          CanonicalCbor.cbor_map_entry(k2, CanonicalCbor.cbor_text("val")),
          CanonicalCbor.cbor_map_entry(k3, CanonicalCbor.cbor_bool(true)),
          CanonicalCbor.cbor_map_entry(k4, CanonicalCbor.cbor_bool(false)),
          CanonicalCbor.cbor_map_entry(k5, CanonicalCbor.cbor_null())
        ])

      assert dec == expected_canonical
      assert CanonicalCbor.encode_checked!(dec) == enc
    end

    test "duplicate keys in CborMap raise duplicate-map-key on encode" do
      dup_map =
        CanonicalCbor.cbor_map([
          CanonicalCbor.cbor_map_entry(
            CanonicalCbor.cbor_text("key"),
            CanonicalCbor.cbor_unsigned(1)
          ),
          CanonicalCbor.cbor_map_entry(
            CanonicalCbor.cbor_text("key"),
            CanonicalCbor.cbor_unsigned(2)
          )
        ])

      assert {:error, %CborError{id: :duplicate_map_key}} = CanonicalCbor.encode_checked(dup_map)
    end

    test "out of order map keys on wire raise non-canonical-map-order on decode" do
      bad_order_wire = <<0xA2, 0x61, ?b, 0x01, 0x61, ?a, 0x02>>

      assert {:error, %CborError{id: :non_canonical_map_order}} =
               CanonicalCbor.decode(bad_order_wire)
    end

    test "duplicate map keys on wire raise non-canonical-map-order on decode" do
      dup_wire = <<0xA2, 0x61, ?a, 0x01, 0x61, ?a, 0x02>>
      assert {:error, %CborError{id: :non_canonical_map_order}} = CanonicalCbor.decode(dup_wire)
    end
  end

  describe "CVE-5: Strict UTF-8 vs surrogate smuggling & unaligned bitstring confusion" do
    test "surrogates and overlong sequences are rejected" do
      assert_raise CborError, fn -> CanonicalCbor.cbor_text(<<0xED, 0xA0, 0x80>>) end
      assert_raise CborError, fn -> CanonicalCbor.cbor_text(<<0xC0, 0x80>>) end
      assert_raise CborError, fn -> CanonicalCbor.cbor_text(<<0xF4, 0x90, 0x80, 0x80>>) end
    end

    test "unaligned bitstrings are rejected" do
      unaligned = <<1::3>>
      assert_raise CborError, fn -> CanonicalCbor.cbor_bytes(unaligned) end
      assert_raise CborError, fn -> CanonicalCbor.decode!(unaligned) end
      assert_raise CborError, fn -> CanonicalCbor.decode_map_spans!(unaligned) end

      val = CanonicalCbor.cbor_unsigned(1)

      assert {:error, %CborError{id: :unsupported_simple}} =
               CanonicalCbor.encode_into_checked(val, unaligned)
    end
  end

  describe "CVE-6: Trailing bytes and forged struct tampering defense" do
    test "trailing bytes are rejected" do
      assert {:error, %CborError{id: :trailing_bytes}} = CanonicalCbor.decode(<<0x00, 0x00>>)
      assert {:error, %CborError{id: :trailing_bytes}} = CanonicalCbor.decode(<<0xA0, 0xFF>>)
    end

    test "forged struct with invalid fields is rejected at encode time" do
      forged = %CborUnsigned{value: "evil"}
      assert {:error, %CborError{id: :unsupported_simple}} = CanonicalCbor.encode_checked(forged)

      forged_neg = %CborNegative{value: -1}

      assert {:error, %CborError{id: :unsupported_simple}} =
               CanonicalCbor.encode_checked(forged_neg)

      forged_bytes = %CborBytes{bytes: 123}

      assert {:error, %CborError{id: :unsupported_simple}} =
               CanonicalCbor.encode_checked(forged_bytes)

      forged_text = %CborText{text: <<0xFF>>}
      assert {:error, %CborError{id: :invalid_utf8}} = CanonicalCbor.encode_checked(forged_text)

      forged_arr = %CborArray{items: "not list"}

      assert {:error, %CborError{id: :unsupported_simple}} =
               CanonicalCbor.encode_checked(forged_arr)

      forged_tag = %CborTag{tag: -1, value: CanonicalCbor.cbor_null()}

      assert {:error, %CborError{id: :unsupported_simple}} =
               CanonicalCbor.encode_checked(forged_tag)

      tag_invalid_inner = %CborTag{tag: 1, value: %CborText{text: <<0xFF>>}}

      assert {:error, %CborError{id: :invalid_utf8}} =
               CanonicalCbor.encode_checked(tag_invalid_inner)

      forged_bool = %CborBool{value: "not bool"}

      assert {:error, %CborError{id: :unsupported_simple}} =
               CanonicalCbor.encode_checked(forged_bool)
    end

    test "non-cbor values rejected by encode_checked" do
      assert {:error, %CborError{id: :unsupported_simple}} =
               CanonicalCbor.encode_checked("raw string")

      assert {:error, %CborError{id: :unsupported_simple}} = CanonicalCbor.encode_checked(123)
      assert {:error, %CborError{id: :unsupported_simple}} = CanonicalCbor.encode_checked(nil)
    end
  end

  describe "CVE-7: Payload-blind errors and SpannedMapEntry redaction" do
    test "SpannedMapEntry inspect redacts secret keys and values" do
      secret_k = CanonicalCbor.cbor_text("API_SECRET_KEY_12345")
      secret_v = CanonicalCbor.cbor_bytes("super_confidential_token")
      span = Span.new(10, 42)
      sme = SpannedMapEntry.new(secret_k, secret_v, span)

      str = inspect(sme)
      assert not String.contains?(str, "API_SECRET_KEY_12345")
      assert not String.contains?(str, "super_confidential_token")
      assert String.contains?(str, "<redacted>")
      assert String.contains?(str, "[10, 42)")
    end

    test "all 14 error messages are fixed strings with zero user reflection" do
      for id <- Messages.all_error_ids() do
        msg = Messages.message_for(id)
        assert String.starts_with?(msg, "canonical-cbor: ")
      end

      unknown_msg = Messages.message_for("SUPER_SECRET_PAYLOAD")
      assert unknown_msg == Messages.unknown_error_id_message()
      assert not String.contains?(unknown_msg, "SUPER_SECRET_PAYLOAD")
    end
  end

  # ===========================================================================
  # 2. Non-Minimal Integer Header Widening Across Major Types 0..6
  # ===========================================================================

  describe "Non-minimal integer header widening across major types 0..6" do
    test "rejects widened headers for majors 0, 1, 2, 3, 4, 6" do
      majors = [0, 1, 2, 3, 4, 6]

      for major <- majors do
        lead = major <<< 5

        # 1-byte header with argument <= 23 (e.g. arg = 0)
        w1 = <<lead ||| 24, 0>>
        w1 = if major == 6, do: w1 <> <<0>>, else: w1
        assert {:error, %CborError{id: :non_minimal_integer}} = CanonicalCbor.decode(w1)

        # 2-byte header with argument <= 255 (e.g. arg = 1)
        w2 = <<lead ||| 25, 1::16-big>>
        w2 = if major == 6, do: w2 <> <<0>>, else: w2
        assert {:error, %CborError{id: :non_minimal_integer}} = CanonicalCbor.decode(w2)

        # 4-byte header with argument <= 65535 (e.g. arg = 1)
        w4 = <<lead ||| 26, 1::32-big>>
        w4 = if major == 6, do: w4 <> <<0>>, else: w4
        assert {:error, %CborError{id: :non_minimal_integer}} = CanonicalCbor.decode(w4)

        # 8-byte header with argument <= 2^32 - 1 (e.g. arg = 1)
        w8 = <<lead ||| 27, 1::64-big>>
        w8 = if major == 6, do: w8 <> <<0>>, else: w8
        assert {:error, %CborError{id: :non_minimal_integer}} = CanonicalCbor.decode(w8)
      end
    end

    test "rejects widened simple values" do
      assert {:error, %CborError{id: :non_minimal_integer}} =
               CanonicalCbor.decode(<<0xF8, 20>>)
    end
  end

  # ===========================================================================
  # 3. DecodeMapSpans Exact 0-Based Wire Spans and Slice Invariant
  # ===========================================================================

  describe "DecodeMapSpans exact 0-based wire spans and slice invariant" do
    test "returns exact wire spans matching value slices" do
      k1 = CanonicalCbor.cbor_unsigned(1)
      v1 = CanonicalCbor.cbor_text("alpha")
      k2 = CanonicalCbor.cbor_text("beta")

      v2 =
        CanonicalCbor.cbor_array([CanonicalCbor.cbor_unsigned(10), CanonicalCbor.cbor_bool(true)])

      map_val =
        CanonicalCbor.cbor_map([
          CanonicalCbor.cbor_map_entry(k1, v1),
          CanonicalCbor.cbor_map_entry(k2, v2)
        ])

      wire = CanonicalCbor.encode_checked!(map_val)

      assert {:ok, %DecodeMapSpansResult{is_map: true, entries: entries}} =
               CanonicalCbor.decode_map_spans(wire)

      assert length(entries) == 2

      for entry <- entries do
        span = entry.value_span
        expected_bytes = CanonicalCbor.encode_checked!(entry.value)
        actual_slice = binary_part(wire, span.start, span.finish - span.start)
        assert actual_slice == expected_bytes
        assert Span.length(span) == byte_size(actual_slice)
      end
    end

    test "non-map wire returns DecodeMapSpansResult.non_map()" do
      u_wire = CanonicalCbor.encode_checked!(CanonicalCbor.cbor_unsigned(42))

      assert {:ok, %DecodeMapSpansResult{is_map: false, entries: nil} = res} =
               CanonicalCbor.decode_map_spans(u_wire)

      assert inspect(res) == "DecodeMapSpansResult.non_map"
    end

    test "map or non-map with trailing bytes raises trailing-bytes" do
      bad_map_wire = CanonicalCbor.encode_checked!(CanonicalCbor.cbor_map([])) <> <<0xFF>>

      assert {:error, %CborError{id: :trailing_bytes}} =
               CanonicalCbor.decode_map_spans(bad_map_wire)

      bad_u_wire = CanonicalCbor.encode_checked!(CanonicalCbor.cbor_unsigned(1)) <> <<0xFF>>

      assert {:error, %CborError{id: :trailing_bytes}} =
               CanonicalCbor.decode_map_spans(bad_u_wire)
    end

    test "decode_map_spans rejects invalid or non-map payloads" do
      # <<>> (:unexpected_eof)
      assert {:error, %CborError{id: :unexpected_eof}} = CanonicalCbor.decode_map_spans(<<>>)

      # <<0xBF, 0xFF>> (:indefinite)
      assert {:error, %CborError{id: :indefinite}} =
               CanonicalCbor.decode_map_spans(<<0xBF, 0xFF>>)

      # <<0xB8, 0x00>> (:non_minimal_integer)
      assert {:error, %CborError{id: :non_minimal_integer}} =
               CanonicalCbor.decode_map_spans(<<0xB8, 0x00>>)

      # <<0xBC>> (:reserved)
      assert {:error, %CborError{id: :reserved}} = CanonicalCbor.decode_map_spans(<<0xBC>>)
    end
  end

  # ===========================================================================
  # 4. Atomic Rollback in encode_into_checked
  # ===========================================================================

  describe "encode_into_checked atomic rollback" do
    test "successful append extends buffer" do
      orig = <<1, 2, 3>>
      val = CanonicalCbor.cbor_unsigned(42)

      assert {:ok, combined} = CanonicalCbor.encode_into_checked(val, orig)
      assert combined == orig <> CanonicalCbor.encode_checked!(val)
    end

    test "failed append preserves original buffer without corruption" do
      orig = <<1, 2, 3>>
      bad_val = %CborUnsigned{value: "evil"}

      assert {:error, %CborError{id: :unsupported_simple}} =
               CanonicalCbor.encode_into_checked(bad_val, orig)
    end

    test "fails fast with :encode_too_large under structural sharing bomb" do
      # BEAM structural sharing: 1,000 references to a ~1 MiB binary chunk
      chunk = :binary.copy(<<0>>, 1_048_570)
      large_bytes = CanonicalCbor.cbor_bytes(chunk)
      array = CanonicalCbor.cbor_array(List.duplicate(large_bytes, 1_000))

      assert {:error, %CborError{id: :encode_too_large}} = CanonicalCbor.encode_checked(array)

      assert {:error, %CborError{id: :encode_too_large}} =
               CanonicalCbor.encode_into_checked(array, <<0>>)
    end

    test "fails fast with :encode_too_large when buffer exceeds cap" do
      overflow_buffer = :binary.copy(<<0>>, Constants.max_encoded_bytes() + 1)
      val = CanonicalCbor.cbor_null()

      assert {:error, %CborError{id: :encode_too_large}} =
               CanonicalCbor.encode_into_checked(val, overflow_buffer)
    end
  end

  # ===========================================================================
  # 5. Deterministic PRNG Property Roundtrip & Permutations
  # ===========================================================================

  describe "Deterministic PRNG Property Tests" do
    test "round-trip property across random shapes" do
      seed = 20_261_114
      state = PRNG.new(seed)

      Enum.reduce(1..500, state, fn _iter, s ->
        {val, s2} = generate_random_cbor(s, 0)
        enc = CanonicalCbor.encode_checked!(val)
        dec = CanonicalCbor.decode!(enc)
        assert dec == val
        assert CanonicalCbor.encode_checked!(dec) == enc
        s2
      end)
    end

    test "map permutation invariance: order of map entries produces identical wire" do
      k1 = CanonicalCbor.cbor_unsigned(1)
      v1 = CanonicalCbor.cbor_text("one")
      k2 = CanonicalCbor.cbor_text("two")
      v2 = CanonicalCbor.cbor_unsigned(2)
      k3 = CanonicalCbor.cbor_bytes(<<3, 4>>)
      v3 = CanonicalCbor.cbor_bool(true)

      e1 = CanonicalCbor.cbor_map_entry(k1, v1)
      e2 = CanonicalCbor.cbor_map_entry(k2, v2)
      e3 = CanonicalCbor.cbor_map_entry(k3, v3)

      m_123 = CanonicalCbor.cbor_map([e1, e2, e3])
      m_321 = CanonicalCbor.cbor_map([e3, e2, e1])
      m_231 = CanonicalCbor.cbor_map([e2, e3, e1])

      w1 = CanonicalCbor.encode_checked!(m_123)
      w2 = CanonicalCbor.encode_checked!(m_321)
      w3 = CanonicalCbor.encode_checked!(m_231)

      assert w1 == w2
      assert w2 == w3
    end
  end

  defp generate_random_cbor(state, depth) do
    if depth >= 3 do
      # Leaf items at deeper recursion
      {kind_idx, s2} = PRNG.next_int(state, 5)

      case kind_idx do
        0 ->
          {u, s3} = PRNG.next_u32(s2)
          {CanonicalCbor.cbor_unsigned(u), s3}

        1 ->
          {n, s3} = PRNG.next_u32(s2)
          {CanonicalCbor.cbor_negative(n), s3}

        2 ->
          {b, s3} = PRNG.next_bytes(s2, 4)
          {CanonicalCbor.cbor_bytes(b), s3}

        3 ->
          {CanonicalCbor.cbor_bool(rem(s2, 2) == 0), s2}

        _ ->
          {CanonicalCbor.cbor_null(), s2}
      end
    else
      {choice, s2} = PRNG.next_int(state, 9)

      case choice do
        0 ->
          {u, s3} = PRNG.next_u32(s2)
          {CanonicalCbor.cbor_unsigned(u), s3}

        1 ->
          {n, s3} = PRNG.next_u32(s2)
          {CanonicalCbor.cbor_negative(n), s3}

        2 ->
          {b, s3} = PRNG.next_bytes(s2, 3)
          {CanonicalCbor.cbor_bytes(b), s3}

        3 ->
          {CanonicalCbor.cbor_text("str_#{rem(s2, 100)}"), s2}

        4 ->
          {len, s3} = PRNG.next_int(s2, 4)

          {items, s4} =
            Enum.reduce(1..len//1, {[], s3}, fn _, {acc, st} ->
              {it, st2} = generate_random_cbor(st, depth + 1)
              {[it | acc], st2}
            end)

          {CanonicalCbor.cbor_array(Enum.reverse(items)), s4}

        5 ->
          {tag_num, s3} = PRNG.next_int(s2, 1000)
          {inner, s4} = generate_random_cbor(s3, depth + 1)
          {CanonicalCbor.cbor_tag(tag_num, inner), s4}

        6 ->
          {len, s3} = PRNG.next_int(s2, 3)

          {entries, s4} =
            Enum.reduce(0..len//1, {[], s3}, fn idx, {acc, st} ->
              key = CanonicalCbor.cbor_unsigned(idx)
              {val, st2} = generate_random_cbor(st, depth + 1)
              {[CanonicalCbor.cbor_map_entry(key, val) | acc], st2}
            end)

          {CanonicalCbor.cbor_map(Enum.reverse(entries)), s4}

        7 ->
          {CanonicalCbor.cbor_bool(rem(s2, 2) == 0), s2}

        _ ->
          {CanonicalCbor.cbor_null(), s2}
      end
    end
  end

  # ===========================================================================
  # 6. 10,000-Iteration Adversarial Byte-Mutation Fuzz Loop
  # ===========================================================================

  describe "10,000-iteration adversarial byte-mutation fuzz loop" do
    test "never crashes or panics on arbitrary mutated inputs" do
      # Seed corpora
      seed_items = [
        CanonicalCbor.cbor_unsigned(42),
        CanonicalCbor.cbor_negative(10),
        CanonicalCbor.cbor_bytes(<<1, 2, 3, 4>>),
        CanonicalCbor.cbor_text("hello world"),
        CanonicalCbor.cbor_array([CanonicalCbor.cbor_unsigned(1), CanonicalCbor.cbor_null()]),
        CanonicalCbor.cbor_map([
          CanonicalCbor.cbor_map_entry(
            CanonicalCbor.cbor_text("k"),
            CanonicalCbor.cbor_unsigned(100)
          )
        ]),
        CanonicalCbor.cbor_tag(5, CanonicalCbor.cbor_bool(true)),
        CanonicalCbor.cbor_bool(false),
        CanonicalCbor.cbor_null()
      ]

      seed_wires = Enum.map(seed_items, &CanonicalCbor.encode_checked!/1)
      num_seeds = length(seed_wires)

      state = PRNG.new(20_261_114)

      Enum.reduce(1..10_000, state, fn _iter, s ->
        {seed_idx, s2} = PRNG.next_int(s, num_seeds)
        base_wire = Enum.at(seed_wires, seed_idx)

        {mutated, s3} = mutate_bytes(base_wire, s2)

        # Decoder MUST return {:ok, _} or {:error, %CborError{}} — NEVER raise
        case CanonicalCbor.decode(mutated) do
          {:ok, val} ->
            assert Types.is_cbor_value?(val)
            assert {:ok, ^mutated} = CanonicalCbor.encode_checked(val)

          {:error, %CborError{id: id, message: msg}} ->
            assert is_atom(id) or is_binary(id)
            assert is_binary(msg)
            assert id in Messages.all_error_ids()
        end

        s3
      end)
    end
  end

  defp mutate_bytes(bin, state) do
    len = byte_size(bin)

    if len == 0 do
      {<<0>>, state}
    else
      {action, s2} = PRNG.next_int(state, 5)
      {pos, s3} = PRNG.next_int(s2, len)

      case action do
        0 ->
          # Bit flip
          {bit, s4} = PRNG.next_int(s3, 8)
          <<head::binary-size(pos), byte::8, tail::binary>> = bin
          flipped = bxor(byte, 1 <<< bit)
          {head <> <<flipped::8>> <> tail, s4}

        1 ->
          # Replace byte with random
          {r_byte, s4} = PRNG.next_int(s3, 256)
          <<head::binary-size(pos), _old::8, tail::binary>> = bin
          {head <> <<r_byte::8>> <> tail, s4}

        2 ->
          # Truncate
          {binary_part(bin, 0, pos), s3}

        3 ->
          # Insert byte
          {ins_byte, s4} = PRNG.next_int(s3, 256)
          <<head::binary-size(pos), tail::binary>> = bin
          {head <> <<ins_byte::8>> <> tail, s4}

        _ ->
          # Delete byte
          <<head::binary-size(pos), _del::8, tail::binary>> = bin
          {head <> tail, s3}
      end
    end
  end

  # ===========================================================================
  # 7. Exhaustive Coverage of Inspect & Predicates
  # ===========================================================================

  describe "Inspect implementations and predicates" do
    test "inspect produces clean representations" do
      assert inspect(CanonicalCbor.cbor_unsigned(42)) == "CborUnsigned(42)"
      assert inspect(CanonicalCbor.cbor_negative(10)) == "CborNegative(10)"
      assert inspect(CanonicalCbor.cbor_bytes(<<1, 2, 3>>)) == "CborBytes(#3)"
      assert inspect(CanonicalCbor.cbor_text("hello")) == ~s[CborText("hello")]
      assert inspect(CanonicalCbor.cbor_array([CanonicalCbor.cbor_null()])) == "CborArray(#1)"
      assert inspect(CanonicalCbor.cbor_bool(true)) == "CborBool(true)"
      assert inspect(CanonicalCbor.cbor_bool(false)) == "CborBool(false)"
      assert inspect(CanonicalCbor.cbor_null()) == "CborNull"

      tag = CanonicalCbor.cbor_tag(1, CanonicalCbor.cbor_null())
      assert inspect(tag) == "CborTag(1, CborNull)"

      entry =
        CanonicalCbor.cbor_map_entry(CanonicalCbor.cbor_unsigned(1), CanonicalCbor.cbor_text("a"))

      assert inspect(entry) == ~s[CborMapEntry(CborUnsigned(1) => CborText("a"))]

      map = CanonicalCbor.cbor_map([entry])
      assert inspect(map) == "CborMap(#1)"

      span = Span.new(1, 5)
      assert inspect(span) == "[1, 5)"

      dm_res =
        DecodeMapSpansResult.map([
          SpannedMapEntry.new(CanonicalCbor.cbor_null(), CanonicalCbor.cbor_null(), span)
        ])

      assert inspect(dm_res) == "DecodeMapSpansResult.map(#1)"

      err = %CborError{id: :unexpected_eof, message: "canonical-cbor: unexpected end of input"}
      assert String.contains?(inspect(err), "unexpected_eof")
    end

    test "predicates correctly classify genuine instances" do
      u = CanonicalCbor.cbor_unsigned(1)
      assert Types.is_cbor_value?(u)
      assert not Types.is_cbor_map_entry?(u)
      assert not Types.is_span?(u)
      assert not Types.is_spanned_map_entry?(u)
      assert not Types.is_decode_map_spans_result?(u)
      assert not Types.is_cbor_error?(u)

      entry = CanonicalCbor.cbor_map_entry(u, u)
      assert Types.is_cbor_map_entry?(entry)
      assert not Types.is_cbor_value?(entry)

      span = Span.new(0, 1)
      assert Types.is_span?(span)

      sme = SpannedMapEntry.new(u, u, span)
      assert Types.is_spanned_map_entry?(sme)

      res = DecodeMapSpansResult.non_map()
      assert Types.is_decode_map_spans_result?(res)

      err = %CborError{id: :unexpected_eof, message: "err"}
      assert Types.is_cbor_error?(err)
    end

    test "constructor error validation branches" do
      assert_raise CborError, fn -> Span.new("not int", 5) end
      assert_raise CborError, fn -> Span.new(5, 2) end

      assert_raise CborError, fn ->
        SpannedMapEntry.new("not val", CanonicalCbor.cbor_null(), Span.new(0, 1))
      end

      assert_raise CborError, fn ->
        SpannedMapEntry.new(CanonicalCbor.cbor_null(), "not val", Span.new(0, 1))
      end

      assert_raise CborError, fn ->
        SpannedMapEntry.new(CanonicalCbor.cbor_null(), CanonicalCbor.cbor_null(), "not span")
      end

      assert_raise CborError, fn -> DecodeMapSpansResult.map("not list") end
      assert_raise CborError, fn -> DecodeMapSpansResult.map(["not entry"]) end
      assert_raise CborError, fn -> CanonicalCbor.cbor_array("not list") end
      assert_raise CborError, fn -> CanonicalCbor.cbor_array(["not cbor"]) end
      assert_raise CborError, fn -> CanonicalCbor.cbor_map("not list") end
      assert_raise CborError, fn -> CanonicalCbor.cbor_map(["not entry"]) end

      assert_raise CborError, fn ->
        CanonicalCbor.cbor_map_entry("not cbor", CanonicalCbor.cbor_null())
      end

      assert_raise CborError, fn ->
        CanonicalCbor.cbor_map_entry(CanonicalCbor.cbor_null(), "not cbor")
      end

      assert_raise CborError, fn -> CanonicalCbor.cbor_bool("not bool") end
      assert_raise CborError, fn -> CanonicalCbor.cbor_text(123) end
    end

    test "constants accessor functions" do
      assert Constants.max_nesting_depth() == 128
      assert Constants.max_encoded_bytes() == 1_048_576
      assert Constants.root_depth() == 0
      assert Constants.min_bytes_per_raw_byte() == 1
      assert Constants.min_bytes_per_array_item() == 1
      assert Constants.min_bytes_per_map_entry() == 2
      assert Constants.major_unsigned() == 0
      assert Constants.major_negative() == 1
      assert Constants.major_bytes() == 2
      assert Constants.major_text() == 3
      assert Constants.major_array() == 4
      assert Constants.major_map() == 5
      assert Constants.major_tag() == 6
      assert Constants.major_simple_or_float() == 7
      assert Constants.major_type_shift() == 5
      assert Constants.major_type_mask() == 0x07
      assert Constants.additional_info_mask() == 0x1F
      assert Constants.info_inline_max() == 23
      assert Constants.info_1_byte() == 24
      assert Constants.info_2_bytes() == 25
      assert Constants.info_4_bytes() == 26
      assert Constants.info_8_bytes() == 27
      assert Constants.info_reserved_28() == 28
      assert Constants.info_reserved_29() == 29
      assert Constants.info_reserved_30() == 30
      assert Constants.info_indefinite() == 31
      assert Constants.arg_width_1_byte() == 1
      assert Constants.arg_width_2_bytes() == 2
      assert Constants.arg_width_4_bytes() == 4
      assert Constants.arg_width_8_bytes() == 8
      assert Constants.max_uint8() == 0xFF
      assert Constants.max_uint16() == 0xFFFF
      assert Constants.max_uint32() == 0xFFFFFFFF
      assert Constants.max_uint64() == 18_446_744_073_709_551_615
      assert Constants.wire_byte_false() == 0xF4
      assert Constants.wire_byte_true() == 0xF5
      assert Constants.wire_byte_null() == 0xF6
      assert Constants.simple_false() == 20
      assert Constants.simple_true() == 21
      assert Constants.simple_null() == 22
      assert Constants.min_1_byte_simple_value() == 32
      assert Constants.simple_float_16() == 25
      assert Constants.simple_float_32() == 26
      assert Constants.simple_float_64() == 27
    end

    test "message_for covers all atoms and strings" do
      ids = [
        {:unexpected_eof, "unexpected-eof"},
        {:trailing_bytes, "trailing-bytes"},
        {:reserved, "reserved"},
        {:indefinite, "indefinite"},
        {:non_minimal_integer, "non-minimal-integer"},
        {:invalid_utf8, "invalid-utf8"},
        {:non_canonical_map_order, "non-canonical-map-order"},
        {:unsupported_simple, "unsupported-simple"},
        {:float_not_supported, "float-not-supported"},
        {:too_deep, "too-deep"},
        {:length_too_large, "length-too-large"},
        {:duplicate_map_key, "duplicate-map-key"},
        {:encode_too_deep, "encode-too-deep"},
        {:encode_too_large, "encode-too-large"}
      ]

      for {atom_id, str_id} <- ids do
        msg_atom = Messages.message_for(atom_id)
        msg_str = Messages.message_for(str_id)
        assert msg_atom == msg_str
        assert String.starts_with?(msg_atom, "canonical-cbor: ")
      end

      assert Messages.message_for(:unknown_id) == Messages.unknown_error_id_message()
      assert Messages.redacted_placeholder() == "<redacted>"
    end

    test "CborError exception construction variants" do
      e1 = CborError.exception(:unexpected_eof)
      assert e1.id == :unexpected_eof
      assert e1.message == Messages.message_for(:unexpected_eof)

      e2 = CborError.exception(id: :trailing_bytes)
      assert e2.id == :trailing_bytes
      assert e2.message == Messages.message_for(:trailing_bytes)

      e3 = CborError.exception([])
      assert e3.id == :unknown
      assert e3.message == Messages.unknown_error_id_message()

      # Custom injected :message is ignored in favor of Messages single source of truth
      e4 = CborError.exception(id: :unexpected_eof, message: "ATTACKER_INJECTED_STRING")
      refute e4.message =~ "ATTACKER_INJECTED_STRING"
      assert e4.message == Messages.message_for(:unexpected_eof)

      e5 = CborError.exception(:non_minimal_integer)
      assert e5.message == Messages.message_for(:non_minimal_integer)

      e6 = CborError.exception(:completely_unknown_id)
      assert e6.id == :completely_unknown_id
      assert e6.message == Messages.unknown_error_id_message()

      e7 = CborError.exception(12345)
      assert e7.id == :unknown
      assert e7.message == Messages.unknown_error_id_message()
    end

    test "predicates and smart constructors reject forged structs with invalid fields" do
      refute Types.is_cbor_value?(%CborUnsigned{value: "evil"})
      refute Types.is_cbor_value?(%CborNegative{value: -1})
      refute Types.is_cbor_value?(%CborBytes{bytes: 123})
      refute Types.is_cbor_value?(%CborText{text: <<0xFF>>})
      refute Types.is_cbor_value?(%CborArray{items: [%CborUnsigned{value: "evil"}]})

      refute Types.is_cbor_value?(%CborMap{
               entries: [%CborMapEntry{key: %CborUnsigned{value: "evil"}, value: %CborNull{}}]
             })

      refute Types.is_cbor_value?(%CborTag{tag: 1, value: %CborUnsigned{value: "evil"}})
      refute Types.is_cbor_value?(%CborBool{value: :not_a_bool})

      refute Types.is_span?(%Span{start: 50, finish: 1})
      refute Types.is_span?(%Span{start: -1, finish: 10})
      refute Types.is_span?(%Span{start: :bad, finish: 10})

      refute Types.is_spanned_map_entry?(%SpannedMapEntry{
               key: %CborText{text: <<0xFF>>},
               value: %CborNull{},
               value_span: %Span{start: 0, finish: 1}
             })

      refute Types.is_spanned_map_entry?(%SpannedMapEntry{
               key: %CborNull{},
               value: %CborNull{},
               value_span: %Span{start: 50, finish: 1}
             })

      assert_raise CborError, fn ->
        SpannedMapEntry.new(%CborNull{}, %CborNull{}, %Span{start: 50, finish: 1})
      end

      assert_raise CborError, fn ->
        SpannedMapEntry.new(%CborText{text: <<0xFF>>}, %CborNull{}, %Span{start: 0, finish: 1})
      end

      assert_raise CborError, fn ->
        CborArray.new([%CborUnsigned{value: "evil"}])
      end

      assert_raise CborError, fn ->
        CborMapEntry.new(%CborUnsigned{value: "evil"}, %CborNull{})
      end

      assert_raise CborError, fn ->
        CborMap.new([:not_an_entry])
      end
    end

    test "Inspect never raises or leaks secret payload on malformed SpannedMapEntry or other structs" do
      malformed_entry = %SpannedMapEntry{
        key: "SECRET_KEY",
        value: "SECRET_VALUE",
        value_span: :bad
      }

      inspected = inspect(malformed_entry)
      refute inspected =~ "SECRET_KEY"
      refute inspected =~ "SECRET_VALUE"
      assert inspected =~ "<redacted>"

      # Malformed DecodeMapSpansResult, CborArray, CborMap, CborMapEntry, CborBytes, CborText, CborUnsigned, CborNegative, CborTag, CborBool, Span
      assert inspect(%DecodeMapSpansResult{is_map: true, entries: :invalid}) =~ "<invalid>"
      assert inspect(%CborArray{items: :invalid}) =~ "<invalid>"
      assert inspect(%CborMap{entries: :invalid}) =~ "<invalid>"
      assert inspect(%CborMapEntry{key: :bad, value: :bad}) =~ "CborMapEntry"
      assert inspect(%CborBytes{bytes: :invalid}) =~ "<invalid>"
      assert inspect(%CborText{text: <<0xFF>>}) =~ "<invalid>"
      assert inspect(%CborText{text: 123}) =~ "<invalid>"
      assert inspect(%CborUnsigned{value: :bad}) =~ "<invalid>"
      assert inspect(%CborNegative{value: :bad}) =~ "<invalid>"
      assert inspect(%CborTag{tag: :bad, value: :bad}) =~ "<invalid>"
      assert inspect(%CborBool{value: :bad}) =~ "<invalid>"
      assert inspect(%Span{start: :bad, finish: :bad}) =~ "<invalid>"
    end

    test "CanonicalCbor facade delegates all constructors, codecs, and predicates" do
      u = CanonicalCbor.cbor_unsigned(10)
      n = CanonicalCbor.cbor_negative(10)
      b = CanonicalCbor.cbor_bytes(<<1, 2>>)
      t = CanonicalCbor.cbor_text("text")
      a = CanonicalCbor.cbor_array([u])
      me = CanonicalCbor.cbor_map_entry(u, t)
      m = CanonicalCbor.cbor_map([me])
      tag = CanonicalCbor.cbor_tag(1, u)
      bl = CanonicalCbor.cbor_bool(true)
      nl = CanonicalCbor.cbor_null()

      assert CanonicalCbor.is_cbor_value?(u)
      assert CanonicalCbor.is_cbor_value?(n)
      assert CanonicalCbor.is_cbor_value?(b)
      assert CanonicalCbor.is_cbor_value?(t)
      assert CanonicalCbor.is_cbor_value?(a)
      assert CanonicalCbor.is_cbor_value?(m)
      assert CanonicalCbor.is_cbor_value?(tag)
      assert CanonicalCbor.is_cbor_value?(bl)
      assert CanonicalCbor.is_cbor_value?(nl)

      assert CanonicalCbor.is_cbor_map_entry?(me)

      sp = CanonicalCbor.span(0, 5)
      assert CanonicalCbor.is_span?(sp)

      sme = CanonicalCbor.spanned_map_entry(u, t, sp)
      assert CanonicalCbor.is_spanned_map_entry?(sme)

      dres = DecodeMapSpansResult.non_map()
      assert CanonicalCbor.is_decode_map_spans_result?(dres)

      err = CborError.exception(:reserved)
      assert CanonicalCbor.is_cbor_error?(err)

      enc = CanonicalCbor.encode_checked!(u)
      assert CanonicalCbor.decode!(enc) == u
    end
  end
end
