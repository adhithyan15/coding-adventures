defmodule CodingAdventures.Zip.PortableConformanceTest do
  use ExUnit.Case, async: true
  import Bitwise

  alias CodingAdventures.Zip
  alias CodingAdventures.Zip.RawInflateError

  @fixture_path Path.expand(
                  "../../../../specs/fixtures/zip-raw-rfc1951-v1/cases.json",
                  __DIR__
                )
  @fixture @fixture_path |> File.read!() |> Jason.decode!()

  defp from_hex(value), do: Base.decode16!(value, case: :lower)

  defp expected_bytes(test_case) do
    output = test_case["expected"]["output"]

    if Map.has_key?(output, "hex") do
      from_hex(output["hex"])
    else
      :binary.copy(from_hex(output["repeat_hex"]), output["count"])
    end
  end

  defp inflate_limit(test_case) do
    Map.get(test_case, "max_output", Zip.raw_inflate_max_output())
  end

  defp zlib_raw_inflate(data) do
    zstream = :zlib.open()
    :ok = :zlib.inflateInit(zstream, -15)
    output = zstream |> :zlib.inflate(data) |> IO.iodata_to_binary()
    :ok = :zlib.inflateEnd(zstream)
    :ok = :zlib.close(zstream)
    output
  end

  defp zlib_raw_deflate(data) do
    zstream = :zlib.open()
    :ok = :zlib.deflateInit(zstream, :best_compression, :deflated, -15, 8, :default)
    output = zstream |> :zlib.deflate(data, :finish) |> IO.iodata_to_binary()
    :ok = :zlib.deflateEnd(zstream)
    :ok = :zlib.close(zstream)
    output
  end

  test "closed fixture metadata matches the public profile" do
    assert length(@fixture["cases"]) == 34

    assert @fixture["limits"] == %{
             "default_max_output" => Zip.raw_inflate_max_output(),
             "hard_max_output" => Zip.raw_inflate_max_output()
           }

    assert @fixture["error_ids"] == Zip.raw_inflate_error_codes()
  end

  for test_case <- @fixture["cases"] do
    @test_case test_case

    test test_case["id"] do
      test_case = @test_case
      expected = test_case["expected"]

      case test_case["operation"] do
        "inflate" ->
          input = from_hex(test_case["input_hex"])
          result = Zip.raw_inflate_counted(input, inflate_limit(test_case))
          assert result.output == expected_bytes(test_case)
          assert result.bytes_consumed == expected["bytes_consumed"]
          assert Zip.raw_inflate(input, inflate_limit(test_case)) == expected_bytes(test_case)

        "inflate-error" ->
          error =
            assert_raise RawInflateError, fn ->
              Zip.raw_inflate_counted(
                from_hex(test_case["input_hex"]),
                inflate_limit(test_case)
              )
            end

          assert error.code == expected["error_id"]
          assert Exception.message(error) == expected["error_id"]

        "deflate-interoperability" ->
          compressed = Zip.raw_deflate(from_hex(test_case["input_hex"]))
          assert zlib_raw_inflate(compressed) == expected_bytes(test_case)

        "crc32" ->
          initial_hex = test_case["initial_crc32_hex"]
          initial_hex = if is_binary(initial_hex), do: initial_hex, else: "00000000"
          checksum = String.to_integer(initial_hex, 16)

          checksum =
            Enum.reduce(test_case["chunks_hex"], checksum, fn chunk, previous ->
              Zip.crc32(from_hex(chunk), previous)
            end)

          assert checksum
                 |> Integer.to_string(16)
                 |> String.downcase()
                 |> String.pad_leading(8, "0") ==
                   expected["crc32_hex"]
      end
    end
  end

  defp raw_zip(name, compressed, uncompressed, declared_size \\ nil) do
    declared_size = declared_size || byte_size(uncompressed)
    checksum = Zip.crc32(uncompressed)
    name_size = byte_size(name)
    compressed_size = byte_size(compressed)

    local =
      <<0x04034B50::little-32, 20::little-16, 0x0800::little-16, 8::little-16, 0::little-16,
        0::little-16, checksum::little-32, compressed_size::little-32, declared_size::little-32,
        name_size::little-16, 0::little-16, name::binary, compressed::binary>>

    central_offset = byte_size(local)

    central =
      <<0x02014B50::little-32, 0x031E::little-16, 20::little-16, 0x0800::little-16, 8::little-16,
        0::little-16, 0::little-16, checksum::little-32, compressed_size::little-32,
        declared_size::little-32, name_size::little-16, 0::little-16, 0::little-16, 0::little-16,
        0::little-16, 0::little-32, 0::little-32, name::binary>>

    eocd =
      <<0x06054B50::little-32, 0::little-16, 0::little-16, 1::little-16, 1::little-16,
        byte_size(central)::little-32, central_offset::little-32, 0::little-16>>

    local <> central <> eocd
  end

  @dynamic Base.decode16!(
             "0dc28911c0200c03b0d8f97028ec3f6ed129cab7dd96a0c2445bdb93809663a5d303f6b265e20c2b79ea03379d227e",
             case: :lower
           )
  @dynamic_output Base.decode16!(
                    "0406030b000e070909010906010a04070007000000000501010908030108050302030401000401000207090009020a0a020605020d060c01020b020302090201",
                    case: :lower
                  )

  test "ZIP reader accepts a dynamic raw payload" do
    reader = Zip.new_reader(raw_zip("dynamic.bin", @dynamic, @dynamic_output))
    assert Zip.reader_read(reader, hd(Zip.reader_entries(reader))) == @dynamic_output
  end

  test "ZIP reader rejects a compressed-payload suffix cavity" do
    reader = Zip.new_reader(raw_zip("cavity.bin", @dynamic <> <<0xDE, 0xAD>>, @dynamic_output))

    assert_raise RuntimeError, "zip: compressed payload contains trailing bytes", fn ->
      Zip.reader_read(reader, hd(Zip.reader_entries(reader)))
    end
  end

  test "ZIP reader rejects a declared-size mismatch without truncation" do
    archive = raw_zip("size.bin", @dynamic, @dynamic_output, byte_size(@dynamic_output) + 1)
    reader = Zip.new_reader(archive)

    assert_raise RuntimeError, "zip: uncompressed size does not match the directory", fn ->
      Zip.reader_read(reader, hd(Zip.reader_entries(reader)))
    end
  end

  test "caller output limits reject invalid values before decoding" do
    for limit <- [-1, Zip.raw_inflate_max_output() + 1, 1.5] do
      error =
        assert_raise RawInflateError, fn ->
          Zip.raw_inflate_counted(<<1, 0, 0, 255, 255>>, limit)
        end

      assert error.code == "invalid-output-limit"
    end
  end

  @tag timeout: 60_000
  test "foreign stream exercises the full 32 KiB distance window" do
    prefix = for i <- 0..32_767, into: <<>>, do: <<band(i * 73 + div(i, 251), 0xFF)>>
    expected = prefix <> prefix
    compressed = zlib_raw_deflate(expected)
    assert Zip.raw_inflate(compressed, byte_size(expected)) == expected
  end
end
