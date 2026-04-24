# frozen_string_literal: true

# test_zip.rb — CMP09 ZIP package tests (TC-1 through TC-12).

require "minitest/autorun"
require "coding_adventures_zip"

class TestCRC32 < Minitest::Test
  def test_empty_input
    assert_equal 0x00000000, CodingAdventures::Zip.crc32("".b)
  end

  def test_hello_world
    assert_equal 0x0D4A1185, CodingAdventures::Zip.crc32("hello world")
  end

  def test_incremental_matches_single_call
    data = "hello world"
    half = data.bytesize / 2
    a = CodingAdventures::Zip.crc32(data[0, half])
    b = CodingAdventures::Zip.crc32(data[half..], initial: a)
    assert_equal CodingAdventures::Zip.crc32(data), b
  end
end

class TestDosDatetime < Minitest::Test
  def test_dos_epoch
    assert_equal 0x00210000, CodingAdventures::Zip::DOS_EPOCH
  end

  def test_time_field_zero_for_midnight
    assert_equal 0, CodingAdventures::Zip.dos_datetime(1980, 1, 1) & 0xFFFF
  end

  def test_date_field_for_1980_01_01
    assert_equal 33, (CodingAdventures::Zip.dos_datetime(1980, 1, 1) >> 16) & 0xFFFF
  end
end

# TC-1: Single file, Stored (no compression)
class TestTC1SingleFileStored < Minitest::Test
  def test_round_trip_without_compression
    data    = "Hello, ZIP!".b
    archive = CodingAdventures::Zip.zip([["hello.txt", data]], compress: false)
    files   = CodingAdventures::Zip.unzip(archive)
    assert_equal "Hello, ZIP!", files["hello.txt"]
  end

  def test_stored_entry_has_method_0
    archive = CodingAdventures::Zip.zip([["a.txt", "abc".b]], compress: false)
    reader  = CodingAdventures::Zip::ZipReader.new(archive)
    assert_equal 0, reader.entries.first.method
  end
end

# TC-2: Single file, DEFLATE
class TestTC2SingleFileDeflate < Minitest::Test
  def test_round_trip_repetitive_text_via_deflate
    data    = ("ABCABCABC" * 100).b
    archive = CodingAdventures::Zip.zip([["rep.txt", data]], compress: true)
    files   = CodingAdventures::Zip.unzip(archive)
    assert_equal data, files["rep.txt"]
  end

  def test_deflate_shrinks_repetitive_data
    data    = ("x" * 1000).b
    archive = CodingAdventures::Zip.zip([["x.txt", data]], compress: true)
    reader  = CodingAdventures::Zip::ZipReader.new(archive)
    entry   = reader.entries.first
    assert_operator entry.compressed_size, :<, entry.size
    assert_equal 8, entry.method
  end
end

# TC-3: Multiple files
class TestTC3MultipleFiles < Minitest::Test
  def test_packs_and_unpacks_three_files
    entries = [["a.txt", "alpha".b], ["b.txt", "beta".b], ["c.txt", "gamma".b]]
    archive = CodingAdventures::Zip.zip(entries)
    files   = CodingAdventures::Zip.unzip(archive)
    assert_equal "alpha", files["a.txt"]
    assert_equal "beta",  files["b.txt"]
    assert_equal "gamma", files["c.txt"]
  end

  def test_entry_list_has_correct_count
    archive = CodingAdventures::Zip.zip([["one.txt", "1".b], ["two.txt", "2".b]])
    assert_equal 2, CodingAdventures::Zip::ZipReader.new(archive).entries.length
  end
end

# TC-4: Directory entry
class TestTC4DirectoryEntry < Minitest::Test
  def test_directory_entry_is_marked
    w = CodingAdventures::Zip::ZipWriter.new
    w.add_directory("mydir/")
    archive = w.finish
    reader  = CodingAdventures::Zip::ZipReader.new(archive)
    dir     = reader.entries.find { |e| e.name == "mydir/" }
    assert dir&.directory?
  end

  def test_reading_directory_returns_empty_string
    w = CodingAdventures::Zip::ZipWriter.new
    w.add_directory("dir/")
    archive = w.finish
    reader  = CodingAdventures::Zip::ZipReader.new(archive)
    dir     = reader.entries.find { |e| e.name == "dir/" }
    assert_equal "".b, reader.read(dir)
  end
end

# TC-5: CRC-32 mismatch
class TestTC5CRC32Mismatch < Minitest::Test
  def test_raises_on_corrupted_data
    data    = "important data".b
    archive = CodingAdventures::Zip.zip([["file.txt", data]], compress: false).b

    # Corrupt a byte in the file data section (after 30-byte local header + name)
    lh_name_len = archive.byteslice(26, 2).unpack1("v")
    data_start  = 30 + lh_name_len
    archive.setbyte(data_start, archive.getbyte(data_start) ^ 0xFF)

    reader = CodingAdventures::Zip::ZipReader.new(archive)
    entry  = reader.entries.first
    assert_raises(RuntimeError) { reader.read(entry) }
  end
end

# TC-6: Random-access read
class TestTC6RandomAccess < Minitest::Test
  def test_reads_specific_file_from_ten_file_archive
    entries = 10.times.map { |i| ["f#{i}.txt", "content of f#{i}".b] }
    archive = CodingAdventures::Zip.zip(entries)
    content = CodingAdventures::Zip::ZipReader.new(archive).read_by_name("f5.txt")
    assert_equal "content of f5", content
  end
end

# TC-7: Incompressible data → Stored
class TestTC7IncompressibleData < Minitest::Test
  def test_incompressible_data_stored_as_method_0
    # 256 distinct bytes — DEFLATE will expand, so ZIP falls back to Stored
    data    = (0..255).map(&:chr).join.b
    archive = CodingAdventures::Zip.zip([["rand.bin", data]], compress: true)
    reader  = CodingAdventures::Zip::ZipReader.new(archive)
    entry   = reader.entries.first
    assert_equal 0,    entry.method
    assert_equal data, reader.read(entry)
  end
end

# TC-8: Empty file
class TestTC8EmptyFile < Minitest::Test
  def test_empty_file_round_trips_correctly
    archive = CodingAdventures::Zip.zip([["empty.txt", "".b]])
    files   = CodingAdventures::Zip.unzip(archive)
    assert_equal "".b, files["empty.txt"]
  end

  def test_empty_file_has_size_0_in_entries
    archive = CodingAdventures::Zip.zip([["e.txt", "".b]])
    entry   = CodingAdventures::Zip::ZipReader.new(archive).entries.first
    assert_equal 0, entry.size
    assert_equal 0, entry.compressed_size
  end
end

# TC-9: Large file
class TestTC9LargeFile < Minitest::Test
  def test_compresses_and_decompresses_100kb
    data    = (("A".."Z").to_a * 4000).join[0, 100_000].b
    archive = CodingAdventures::Zip.zip([["big.bin", data]], compress: true)
    files   = CodingAdventures::Zip.unzip(archive)
    assert_equal data, files["big.bin"]
  end

  def test_10kb_all_same_byte_compresses_significantly
    data    = ("A" * 10_000).b
    archive = CodingAdventures::Zip.zip([["aaaa.bin", data]], compress: true)
    entry   = CodingAdventures::Zip::ZipReader.new(archive).entries.first
    assert_operator entry.compressed_size, :<, 200
  end
end

# TC-10: Unicode filename
class TestTC10UnicodeFilename < Minitest::Test
  def test_preserves_unicode_filename
    name    = "日本語/résumé.txt"
    archive = CodingAdventures::Zip.zip([[name, "hello".b]])
    files   = CodingAdventures::Zip.unzip(archive)
    assert files.key?(name), "Expected key #{name.inspect} in #{files.keys.inspect}"
    assert_equal "hello", files[name]
  end
end

# TC-11: Nested paths
class TestTC11NestedPaths < Minitest::Test
  def test_preserves_deep_nested_filename
    name    = "a/b/c/deep.txt"
    archive = CodingAdventures::Zip.zip([[name, "deep".b]])
    files   = CodingAdventures::Zip.unzip(archive)
    assert_equal "deep", files[name]
  end

  def test_mixed_nested_and_flat_files
    entries = [
      ["root.txt",          "root".b],
      ["sub/file.txt",      "sub".b],
      ["sub/deep/file.txt", "deep".b]
    ]
    archive = CodingAdventures::Zip.zip(entries)
    files   = CodingAdventures::Zip.unzip(archive)
    assert_equal "root", files["root.txt"]
    assert_equal "sub",  files["sub/file.txt"]
    assert_equal "deep", files["sub/deep/file.txt"]
  end
end

# TC-12: Empty archive
class TestTC12EmptyArchive < Minitest::Test
  def test_empty_writer_produces_valid_archive
    archive = CodingAdventures::Zip::ZipWriter.new.finish
    reader  = CodingAdventures::Zip::ZipReader.new(archive)
    assert_equal [], reader.entries
  end

  def test_unzip_of_empty_archive_returns_empty_hash
    archive = CodingAdventures::Zip::ZipWriter.new.finish
    assert_equal({}, CodingAdventures::Zip.unzip(archive))
  end
end

# Error paths
class TestZipReaderErrors < Minitest::Test
  def test_raises_on_no_eocd
    assert_raises(RuntimeError) { CodingAdventures::Zip::ZipReader.new("not a zip") }
  end

  def test_raises_on_no_eocd_large_buffer
    assert_raises(RuntimeError) { CodingAdventures::Zip::ZipReader.new("A" * 30) }
  end

  def test_read_by_name_raises_for_missing_entry
    archive = CodingAdventures::Zip.zip([["f.txt", "x".b]])
    assert_raises(RuntimeError) do
      CodingAdventures::Zip::ZipReader.new(archive).read_by_name("missing.txt")
    end
  end

  def test_raises_on_unsupported_compression_method
    archive = CodingAdventures::Zip.zip([["f.txt", "x".b]], compress: false).b
    # Find CD header (sig 0x02014B50) and patch method to 99
    4.upto(archive.bytesize - 4) do |i|
      if archive.byteslice(i, 4).unpack1("V") == 0x02014B50
        archive.setbyte(i + 10, 99)
        archive.setbyte(i + 11, 0)
        break
      end
    end
    reader = CodingAdventures::Zip::ZipReader.new(archive)
    assert_raises(RuntimeError) { reader.read(reader.entries.first) }
  end
end

class TestZipWriterAPI < Minitest::Test
  def test_add_file_and_directory_combined
    w = CodingAdventures::Zip::ZipWriter.new
    w.add_directory("docs/")
    w.add_file("docs/readme.txt", "Read me".b, compress: false)
    archive = w.finish
    reader  = CodingAdventures::Zip::ZipReader.new(archive)
    entries = reader.entries
    assert_equal 2,     entries.length
    assert entries[0].directory?
    assert_equal "Read me", reader.read_by_name("docs/readme.txt")
  end
end
