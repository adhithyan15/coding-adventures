defmodule CodingAdventures.ZipTest do
  use ExUnit.Case, async: true
  import Bitwise

  alias CodingAdventures.Zip

  # ─── Helpers ─────────────────────────────────────────────────────────────────

  defp roundtrip(name, data, opts \\ []) do
    archive = Zip.zip([{name, data}], opts)
    %{^name => result} = Zip.unzip(archive)
    result
  end

  # ─── TC-1: Round-trip single file, Stored ─────────────────────────────────

  test "TC-1: stored round-trip" do
    data = "Hello, ZIP!"
    assert roundtrip("hello.txt", data, compress: false) == data
  end

  # ─── TC-2: Round-trip single file, DEFLATE ────────────────────────────────

  test "TC-2: deflate round-trip with repetitive text" do
    data = String.duplicate("abcdefgh", 200)
    archive = Zip.zip([{"rep.txt", data}])
    %{"rep.txt" => result} = Zip.unzip(archive)
    assert result == data
    # Verify DEFLATE was actually used (archive smaller than naive stored size)
    assert byte_size(archive) < byte_size(data)
  end

  # ─── TC-3: Multiple files ─────────────────────────────────────────────────

  test "TC-3: multiple files round-trip" do
    entries = [
      {"a.txt", "Alice"},
      {"b.txt", "Bob"},
      {"c.txt", "Charlie"}
    ]
    archive = Zip.zip(entries)
    result = Zip.unzip(archive)
    assert result["a.txt"] == "Alice"
    assert result["b.txt"] == "Bob"
    assert result["c.txt"] == "Charlie"
  end

  # ─── TC-4: Directory entry ────────────────────────────────────────────────

  test "TC-4: directory entry" do
    w = Zip.new_writer()
    w = Zip.add_directory(w, "docs/")
    w = Zip.add_file(w, "docs/readme.txt", "Read me")
    archive = Zip.finish(w)
    result = Zip.unzip(archive)
    assert result["docs/"] == ""
    assert result["docs/readme.txt"] == "Read me"
  end

  # ─── TC-5: CRC-32 mismatch ───────────────────────────────────────────────

  test "TC-5: CRC-32 mismatch raises error" do
    archive = Zip.zip([{"data.bin", "hello world"}])
    # Local header = 30 bytes + 8 bytes name ("data.bin") = 38 bytes;
    # file data starts at byte 38. Corrupt the first data byte.
    data_offset = 38
    <<pre::binary-size(data_offset), byte, rest::binary>> = archive
    corrupted = pre <> <<bxor(byte, 0xFF)>> <> rest
    assert_raise RuntimeError, ~r/CRC-32 mismatch/, fn ->
      Zip.unzip(corrupted)
    end
  end

  # ─── TC-6: Random-access read ────────────────────────────────────────────

  test "TC-6: random-access read by name" do
    entries = Enum.map(0..9, fn i -> {"f#{i}.txt", "content #{i}"} end)
    archive = Zip.zip(entries)
    reader = Zip.new_reader(archive)
    result = Zip.read_by_name(reader, "f5.txt")
    assert result == "content 5"
  end

  # ─── TC-7: Incompressible stored ─────────────────────────────────────────

  test "TC-7: incompressible data stored as method 0" do
    # Random-ish bytes that deflate can't compress smaller than raw
    data = :crypto.strong_rand_bytes(256)
    archive = Zip.zip([{"rand.bin", data}])
    reader = Zip.new_reader(archive)
    [entry] = Zip.reader_entries(reader)
    assert entry.method == 0
    result = Zip.reader_read(reader, entry)
    assert result == data
  end

  # ─── TC-8: Empty file ────────────────────────────────────────────────────

  test "TC-8: empty file" do
    assert roundtrip("empty.txt", "") == ""
  end

  # ─── TC-9: Large file ────────────────────────────────────────────────────

  test "TC-9: large repetitive file compresses well" do
    data = String.duplicate("A", 10_000)
    archive = Zip.zip([{"big.txt", data}])
    %{"big.txt" => result} = Zip.unzip(archive)
    assert result == data
    assert byte_size(archive) < div(byte_size(data), 2)
  end

  # ─── TC-10: Unicode filename ─────────────────────────────────────────────

  test "TC-10: unicode filename" do
    name = "日本語/résumé.txt"
    data = "Unicode filename test"
    archive = Zip.zip([{name, data}])
    %{^name => result} = Zip.unzip(archive)
    assert result == data
  end

  # ─── TC-11: Nested paths ─────────────────────────────────────────────────

  test "TC-11: nested paths" do
    entries = [
      {"a/b/c/d.txt", "deep"},
      {"a/b/e.txt",   "mid"}
    ]
    archive = Zip.zip(entries)
    result = Zip.unzip(archive)
    assert result["a/b/c/d.txt"] == "deep"
    assert result["a/b/e.txt"]   == "mid"
  end

  # ─── TC-12: Empty archive ────────────────────────────────────────────────

  test "TC-12: empty archive" do
    archive = Zip.finish(Zip.new_writer())
    result = Zip.unzip(archive)
    assert result == %{}
  end

  # ─── CRC-32 known vectors ─────────────────────────────────────────────────

  test "CRC-32: empty string" do
    assert Zip.crc32("") == 0x00000000
  end

  test "CRC-32: 'hello world'" do
    assert Zip.crc32("hello world") == 0x0D4A1185
  end

  test "CRC-32: '123456789'" do
    assert Zip.crc32("123456789") == 0xCBF43926
  end

  # ─── DOS datetime ────────────────────────────────────────────────────────

  test "dos_epoch constant is 1980-01-01" do
    assert Zip.dos_epoch() == 0x00210000
    assert Zip.dos_datetime(1980, 1, 1) == 0x00210000
  end

  test "dos_datetime encodes correctly" do
    # 2024-06-15 14:30:00
    dt = Zip.dos_datetime(2024, 6, 15, 14, 30, 0)
    assert is_integer(dt)
    assert dt > 0
  end

  # ─── read_by_name error path ─────────────────────────────────────────────

  test "read_by_name raises for missing entry" do
    archive = Zip.zip([{"a.txt", "hello"}])
    reader = Zip.new_reader(archive)
    assert_raise RuntimeError, ~r/not found/, fn ->
      Zip.read_by_name(reader, "missing.txt")
    end
  end

  # ─── Bad EOCD signature ──────────────────────────────────────────────────

  test "bad EOCD raises error" do
    assert_raise RuntimeError, ~r/EOCD/, fn ->
      Zip.unzip("not a zip file")
    end
  end

  # ─── Bad local header signature ──────────────────────────────────────────

  test "bad local header signature raises error" do
    archive = Zip.zip([{"f.txt", "data"}])
    # Corrupt the local header signature (first 4 bytes)
    <<_sig::4-bytes, rest::binary>> = archive
    corrupted = <<0xFF, 0xFF, 0xFF, 0xFF>> <> rest
    reader = Zip.new_reader(corrupted)
    [entry] = Zip.reader_entries(reader)
    assert_raise RuntimeError, ~r/local header/, fn ->
      Zip.reader_read(reader, entry)
    end
  end

  # ─── add_file with compress: false explicit ───────────────────────────────

  test "add_file with compress: false stores verbatim" do
    w = Zip.new_writer()
    w = Zip.add_file(w, "raw.bin", "AAAAAAAAAA", compress: false)
    archive = Zip.finish(w)
    reader = Zip.new_reader(archive)
    [entry] = Zip.reader_entries(reader)
    assert entry.method == 0
  end

  # ─── writer entries list ─────────────────────────────────────────────────

  test "reader entries list returns correct count" do
    archive = Zip.zip([{"a.txt", "a"}, {"b.txt", "b"}, {"c.txt", "c"}])
    reader = Zip.new_reader(archive)
    assert length(Zip.reader_entries(reader)) == 3
  end

  # ─── EOCD not at end-22 (junk bytes after archive) ───────────────────────
  # Tests the find_eocd_loop recursive scan path.

  test "EOCD scan finds signature when extra bytes trail the archive" do
    archive = Zip.zip([{"note.txt", "hello"}])
    # Append some bytes — EOCD is now not at (size - 22)
    padded = archive <> <<0x00, 0x00, 0x00, 0x00>>
    %{"note.txt" => result} = Zip.unzip(padded)
    assert result == "hello"
  end

  # ─── DEFLATE BTYPE=00 stored block decode ────────────────────────────────
  # Craft a minimal ZIP with method=8 containing a BTYPE=00 DEFLATE stream
  # to exercise the decompress_stored code path.

  defp make_btype00_zip(name, data) do
    import Bitwise
    crc  = Zip.crc32(data)
    len  = byte_size(data)
    nlen = bxor(len, 0xFFFF)
    # DEFLATE BTYPE=00 stored block: BFINAL=1, BTYPE=00, align byte, LEN(2), NLEN(2), data
    compressed = <<0x01, len :: little-16, nlen :: little-16>> <> data
    comp_size  = byte_size(compressed)
    name_bin   = name
    name_len   = byte_size(name_bin)
    local = <<
      0x50, 0x4B, 0x03, 0x04,
      20    :: little-16,
      0x00, 0x08,
      8     :: little-16,
      0x00, 0x00, 0x21, 0x00,
      crc       :: little-32,
      comp_size :: little-32,
      len       :: little-32,
      name_len  :: little-16,
      0         :: little-16
    >> <> name_bin <> compressed

    cd = <<
      0x50, 0x4B, 0x01, 0x02,
      0x1E, 0x03,
      20    :: little-16,
      0x00, 0x08,
      8     :: little-16,
      0x00, 0x00, 0x21, 0x00,
      crc       :: little-32,
      comp_size :: little-32,
      len       :: little-32,
      name_len  :: little-16,
      0 :: little-16,
      0 :: little-16,
      0 :: little-16,
      0 :: little-16,
      0 :: little-32,
      0 :: little-32
    >> <> name_bin

    cd_offset = byte_size(local)
    cd_size   = byte_size(cd)
    eocd = <<
      0x50, 0x4B, 0x05, 0x06,
      0 :: little-16,
      0 :: little-16,
      1 :: little-16,
      1 :: little-16,
      cd_size   :: little-32,
      cd_offset :: little-32,
      0 :: little-16
    >>

    local <> cd <> eocd
  end

  test "DEFLATE BTYPE=00 stored block round-trips correctly" do
    data    = "hello world"
    archive = make_btype00_zip("btype00.txt", data)
    %{"btype00.txt" => result} = Zip.unzip(archive)
    assert result == data
  end

  # ─── NLEN complement check ───────────────────────────────────────────────

  test "DEFLATE stored block with wrong NLEN raises" do
    # Build a stored-block ZIP with NLEN that does NOT complement LEN
    name = "x.txt"
    data = "hello"
    crc  = Zip.crc32(data)
    len  = byte_size(data)
    # Use wrong NLEN (should be bxor(len, 0xFFFF) but we use 0x0000)
    compressed = <<0x01, len :: little-16, 0x00, 0x00>> <> data
    comp_size  = byte_size(compressed)
    name_len   = byte_size(name)
    local = <<
      0x50, 0x4B, 0x03, 0x04,
      20    :: little-16, 0x00, 0x08, 8 :: little-16,
      0x00, 0x00, 0x21, 0x00,
      crc       :: little-32,
      comp_size :: little-32,
      len       :: little-32,
      name_len  :: little-16, 0 :: little-16
    >> <> name <> compressed
    cd = <<
      0x50, 0x4B, 0x01, 0x02, 0x1E, 0x03,
      20    :: little-16, 0x00, 0x08, 8 :: little-16,
      0x00, 0x00, 0x21, 0x00,
      crc :: little-32, comp_size :: little-32, len :: little-32,
      name_len :: little-16, 0 :: little-16, 0 :: little-16,
      0 :: little-16, 0 :: little-16, 0 :: little-32, 0 :: little-32
    >> <> name
    cd_offset = byte_size(local)
    cd_size   = byte_size(cd)
    eocd = <<
      0x50, 0x4B, 0x05, 0x06,
      0 :: little-16, 0 :: little-16,
      1 :: little-16, 1 :: little-16,
      cd_size :: little-32, cd_offset :: little-32, 0 :: little-16
    >>
    archive = local <> cd <> eocd
    reader = Zip.new_reader(archive)
    [entry] = Zip.reader_entries(reader)
    assert_raise RuntimeError, ~r/NLEN complement/, fn ->
      Zip.reader_read(reader, entry)
    end
  end

  # ─── Duplicate entry names ────────────────────────────────────────────────

  test "unzip raises on duplicate entry names" do
    # Build a raw ZIP with two entries sharing the same name
    name  = "dup.txt"
    data1 = "first"
    data2 = "second"
    name_len = byte_size(name)

    make_local = fn data ->
      crc = Zip.crc32(data)
      len = byte_size(data)
      <<
        0x50, 0x4B, 0x03, 0x04,
        10 :: little-16, 0x00, 0x08, 0 :: little-16,
        0x00, 0x00, 0x21, 0x00,
        crc :: little-32, len :: little-32, len :: little-32,
        name_len :: little-16, 0 :: little-16
      >> <> name <> data
    end

    make_cd = fn data, offset ->
      crc = Zip.crc32(data)
      len = byte_size(data)
      <<
        0x50, 0x4B, 0x01, 0x02, 0x1E, 0x03,
        10 :: little-16, 0x00, 0x08, 0 :: little-16,
        0x00, 0x00, 0x21, 0x00,
        crc :: little-32, len :: little-32, len :: little-32,
        name_len :: little-16, 0 :: little-16, 0 :: little-16,
        0 :: little-16, 0 :: little-16, 0 :: little-32,
        offset :: little-32
      >> <> name
    end

    local1 = make_local.(data1)
    local2 = make_local.(data2)
    offset2 = byte_size(local1)
    cd1  = make_cd.(data1, 0)
    cd2  = make_cd.(data2, offset2)
    locals = local1 <> local2
    cds    = cd1 <> cd2
    cd_offset = byte_size(locals)
    cd_size   = byte_size(cds)
    eocd = <<
      0x50, 0x4B, 0x05, 0x06,
      0 :: little-16, 0 :: little-16,
      2 :: little-16, 2 :: little-16,
      cd_size :: little-32, cd_offset :: little-32, 0 :: little-16
    >>
    archive = locals <> cds <> eocd
    assert_raise RuntimeError, ~r/duplicate entry/, fn ->
      Zip.unzip(archive)
    end
  end

  # ─── CRC-32 chained initial value ────────────────────────────────────────

  test "CRC-32 chaining with non-zero initial matches single-call result" do
    full = "hello world"
    half1 = "hello "
    half2 = "world"
    c1 = Zip.crc32(half1)
    c2 = Zip.crc32(half2, c1)
    assert c2 == Zip.crc32(full)
  end

  # ─── CD offset overlapping EOCD ──────────────────────────────────────────

  test "CD offset past EOCD raises error" do
    # EOCD with cd_offset pointing past itself
    eocd = <<
      0x50, 0x4B, 0x05, 0x06,
      0 :: little-16, 0 :: little-16,
      0 :: little-16, 0 :: little-16,
      0 :: little-32,
      22 :: little-32,   # cd_offset = 22 > eocd_offset = 0
      0 :: little-16
    >>
    assert_raise RuntimeError, ~r/CD offset/, fn ->
      Zip.unzip(eocd)
    end
  end

  # ─── unsupported compression method ─────────────────────────────────────

  test "unsupported compression method raises" do
    # Build a ZIP claiming method=12 (bzip2 — not supported)
    name = "x.txt"
    name_len = byte_size(name)
    local = <<
      0x50, 0x4B, 0x03, 0x04,
      20       :: little-16,
      0x00, 0x08,
      12       :: little-16,
      0x00, 0x00, 0x21, 0x00,
      0        :: little-32,
      0        :: little-32,
      0        :: little-32,
      name_len :: little-16,
      0        :: little-16
    >> <> name
    cd = <<
      0x50, 0x4B, 0x01, 0x02,
      0x1E, 0x03,
      20       :: little-16,
      0x00, 0x08,
      12       :: little-16,
      0x00, 0x00, 0x21, 0x00,
      0        :: little-32,
      0        :: little-32,
      0        :: little-32,
      name_len :: little-16,
      0 :: little-16,
      0 :: little-16,
      0 :: little-16,
      0 :: little-16,
      0 :: little-32,
      0 :: little-32
    >> <> name
    cd_offset = byte_size(local)
    cd_size   = byte_size(cd)
    eocd = <<
      0x50, 0x4B, 0x05, 0x06,
      0 :: little-16, 0 :: little-16,
      1 :: little-16, 1 :: little-16,
      cd_size :: little-32, cd_offset :: little-32,
      0 :: little-16
    >>
    archive = local <> cd <> eocd
    reader = Zip.new_reader(archive)
    [entry] = Zip.reader_entries(reader)
    assert_raise RuntimeError, ~r/unsupported compression method/, fn ->
      Zip.reader_read(reader, entry)
    end
  end
end
