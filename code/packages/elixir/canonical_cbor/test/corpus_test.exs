defmodule CodingAdventures.CanonicalCbor.CorpusTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CanonicalCbor
  alias CodingAdventures.CanonicalCbor.CborError

  @corpus_path Path.expand("../../../../specs/fixtures/canonical-cbor-v1/cases.json", __DIR__)

  test "verifies all 55 cases in canonical-cbor-v1 cases.json corpus" do
    assert File.exists?(@corpus_path), "Missing corpus fixture at #{@corpus_path}"

    json_text = File.read!(@corpus_path)
    data = JSON.decode!(json_text)
    cases = data["cases"]
    assert length(cases) == 55

    for case_data <- cases do
      id = case_data["id"]
      op = case_data["operation"]

      case op do
        "round-trip" ->
          in_bytes = Base.decode16!(case_data["input"], case: :mixed)
          expected_bytes = Base.decode16!(case_data["expected"], case: :mixed)

          assert {:ok, val} = CanonicalCbor.decode(in_bytes), "#{id}: decode failed"
          assert {:ok, enc} = CanonicalCbor.encode_checked(val), "#{id}: encode failed"
          assert enc == expected_bytes, "#{id}: re-encoded bytes do not match expected"
          assert {:ok, val2} = CanonicalCbor.decode(enc), "#{id}: re-decode failed"
          assert val2 == val, "#{id}: deep equality failed across roundtrip"

        "encode-map" ->
          # input is pairs of hex: 6162=f6;1818=f5;60=f4;00=01;6161=02
          pairs = String.split(case_data["input"], ";")

          entries =
            Enum.map(pairs, fn pair_str ->
              [k_hex, v_hex] = String.split(pair_str, "=")
              k_bytes = Base.decode16!(k_hex, case: :mixed)
              v_bytes = Base.decode16!(v_hex, case: :mixed)

              k_val = CanonicalCbor.decode!(k_bytes)
              v_val = CanonicalCbor.decode!(v_bytes)
              CanonicalCbor.cbor_map_entry(k_val, v_val)
            end)

          map_val = CanonicalCbor.cbor_map(entries)
          expected_bytes = Base.decode16!(case_data["expected"], case: :mixed)

          assert {:ok, enc} = CanonicalCbor.encode_checked(map_val), "#{id}: encode map failed"
          assert enc == expected_bytes, "#{id}: encoded map order did not match expected"

        "generated-round-trip" ->
          input = case_data["input"]

          cond do
            input == "nested-array:128" ->
              val =
                Enum.reduce(1..128, CanonicalCbor.cbor_array([]), fn _, acc ->
                  CanonicalCbor.cbor_array([acc])
                end)

              assert {:ok, enc} = CanonicalCbor.encode_checked(val)
              expected_wire = String.duplicate(<<0x81>>, 128) <> <<0x80>>
              assert enc == expected_wire
              assert {:ok, dec} = CanonicalCbor.decode(enc)
              assert dec == val

            String.starts_with?(input, "bytes-repeat:") ->
              [_, count_str, byte_hex] = String.split(input, ":")
              count = String.to_integer(count_str)
              byte_val = Base.decode16!(byte_hex, case: :mixed)
              raw = String.duplicate(byte_val, count)

              val = CanonicalCbor.cbor_bytes(raw)
              assert {:ok, enc} = CanonicalCbor.encode_checked(val)
              assert {:ok, dec} = CanonicalCbor.decode(enc)
              assert dec == val
          end

        "decode-error" ->
          input = case_data["input"]
          expected_id = case_data["expected"]

          in_bytes =
            if input == "nested-array-wire:129" do
              String.duplicate(<<0x81>>, 129) <> <<0x80>>
            else
              Base.decode16!(input, case: :mixed)
            end

          assert {:error, %CborError{id: id}} = CanonicalCbor.decode(in_bytes),
                 "#{id}: expected decode error #{expected_id}, but decode succeeded"

          expected_atom = String.to_existing_atom(String.replace(expected_id, "-", "_"))

          assert id == expected_atom or id == expected_id,
                 "#{id}: expected error #{expected_id}, got #{inspect(id)}"

        "encode-error" ->
          input = case_data["input"]
          expected_id = case_data["expected"]

          val =
            cond do
              input == "nested-array:129" ->
                Enum.reduce(1..129, CanonicalCbor.cbor_array([]), fn _, acc ->
                  CanonicalCbor.cbor_array([acc])
                end)

              String.starts_with?(input, "bytes-repeat:") ->
                [_, count_str, byte_hex] = String.split(input, ":")
                count = String.to_integer(count_str)
                byte_val = Base.decode16!(byte_hex, case: :mixed)
                raw = String.duplicate(byte_val, count)
                CanonicalCbor.cbor_bytes(raw)

              input == "duplicate-map-key" ->
                k = CanonicalCbor.cbor_unsigned(1)

                CanonicalCbor.cbor_map([
                  CanonicalCbor.cbor_map_entry(k, CanonicalCbor.cbor_unsigned(10)),
                  CanonicalCbor.cbor_map_entry(k, CanonicalCbor.cbor_unsigned(20))
                ])
            end

          assert {:error, %CborError{id: id}} = CanonicalCbor.encode_checked(val),
                 "#{id}: expected encode error #{expected_id}, but encode succeeded"

          expected_atom = String.to_existing_atom(String.replace(expected_id, "-", "_"))

          assert id == expected_atom or id == expected_id,
                 "#{id}: expected error #{expected_id}, got #{inspect(id)}"
      end
    end
  end
end
