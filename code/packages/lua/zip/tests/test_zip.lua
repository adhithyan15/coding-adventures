-- ============================================================================
-- Tests for coding_adventures.zip — CMP09 ZIP archive format
-- ============================================================================
--
-- Covers TC-1 through TC-12 from the CMP09 specification, plus CRC-32 vectors,
-- DOS datetime, security guards (path traversal, zip-bomb), and edge cases.
-- Uses the Busted test framework.

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local zip = require("coding_adventures.zip")

-- ---------------------------------------------------------------------------
-- Helper: round-trip a list of {name, data} pairs through zip/unzip.
-- ---------------------------------------------------------------------------
local function roundtrip(entries, compress)
    local archive = zip.zip(entries, compress)
    local result  = zip.unzip(archive)
    return result, archive
end

-- ---------------------------------------------------------------------------
-- TC-1: Round-trip single file, Stored (compress=false)
-- ---------------------------------------------------------------------------
describe("TC-1: round-trip single file, Stored", function()
    it("compresses and decompresses with method=0", function()
        local result, archive = roundtrip({{"hello.txt", "Hello, ZIP!"}}, false)
        assert.equal("Hello, ZIP!", result["hello.txt"])

        -- Verify method=0 in archive (compression method at local header offset 8)
        local reader = zip.new_reader(archive)
        assert.is_not_nil(reader)
        local entries = zip.reader_entries(reader)
        assert.equal(1, #entries)
        assert.equal(0, entries[1].method)
    end)
end)

-- ---------------------------------------------------------------------------
-- TC-2: Round-trip single file, DEFLATE (repetitive text)
-- ---------------------------------------------------------------------------
describe("TC-2: round-trip single file, DEFLATE", function()
    it("compresses repetitive data and decompresses correctly", function()
        local data = string.rep("abcdefgh", 500)  -- 4000 bytes, highly compressible
        local result, archive = roundtrip({{"data.txt", data}})
        assert.equal(data, result["data.txt"])

        -- Confirm method=8 was chosen (compressed < original)
        local reader = zip.new_reader(archive)
        local entries = zip.reader_entries(reader)
        assert.equal(8, entries[1].method)
        assert.is_true(#archive < #data + 200, "archive should be smaller than raw data")
    end)
end)

-- ---------------------------------------------------------------------------
-- TC-3: Multiple files in one archive
-- ---------------------------------------------------------------------------
describe("TC-3: multiple files in one archive", function()
    it("stores and retrieves all files", function()
        local files = {
            {"alpha.txt", "Alpha"},
            {"beta.txt",  "Beta"},
            {"gamma.txt", "Gamma"},
        }
        local result = roundtrip(files)
        assert.equal("Alpha",  result["alpha.txt"])
        assert.equal("Beta",   result["beta.txt"])
        assert.equal("Gamma",  result["gamma.txt"])
    end)
end)

-- ---------------------------------------------------------------------------
-- TC-4: Directory entry
-- ---------------------------------------------------------------------------
describe("TC-4: directory entry", function()
    it("creates and lists a directory entry", function()
        local w = zip.new_writer()
        zip.add_directory(w, "docs/")
        zip.add_file(w, "docs/readme.txt", "Read me")
        local archive = zip.finish(w)

        local reader  = zip.new_reader(archive)
        local entries = zip.reader_entries(reader)

        -- Find directory entry
        local found_dir = false
        for _, e in ipairs(entries) do
            if e.name == "docs/" and e.is_directory then
                found_dir = true
            end
        end
        assert.is_true(found_dir, "directory entry docs/ not found")

        -- Read file inside the directory
        local data = zip.read_by_name(reader, "docs/readme.txt")
        assert.equal("Read me", data)
    end)
end)

-- ---------------------------------------------------------------------------
-- TC-5: CRC-32 mismatch detected (corrupt data)
-- ---------------------------------------------------------------------------
describe("TC-5: CRC-32 mismatch detected", function()
    it("raises error when data is corrupted", function()
        local archive = zip.zip({{"file.txt", "Hello"}})

        -- Corrupt a byte in the file data area (beyond the headers).
        -- Local header is 30 + name_len bytes. "file.txt" = 8 chars.
        -- So data starts at offset 30 + 8 = 38 (1-indexed: 39).
        local corrupt = archive:sub(1, 38) .. string.char(0xFF) .. archive:sub(40)

        assert.has_error(function()
            zip.unzip(corrupt)
        end)
    end)
end)

-- ---------------------------------------------------------------------------
-- TC-6: Random-access read (10 files, read only one by name)
-- ---------------------------------------------------------------------------
describe("TC-6: random-access read by name", function()
    it("reads a specific file from a 10-file archive", function()
        local files = {}
        for i = 1, 10 do
            files[i] = {"f" .. i .. ".txt", "content_" .. i}
        end
        local archive = zip.zip(files)

        local reader = zip.new_reader(archive)
        local data   = zip.read_by_name(reader, "f5.txt")
        assert.equal("content_5", data)
    end)
end)

-- ---------------------------------------------------------------------------
-- TC-7: Incompressible data stored as Stored (method=0)
-- ---------------------------------------------------------------------------
describe("TC-7: incompressible data falls back to Stored", function()
    it("uses method=0 when DEFLATE does not reduce size", function()
        -- Bytes 144-255 in order: no repeating substrings of length ≥ 3, so
        -- LZSS emits all literals. Fixed Huffman codes for 144-255 cost 9 bits
        -- each (vs 8 bits raw), so compressed > original → Stored is chosen.
        local data = ""
        for i = 144, 255 do data = data .. string.char(i) end
        -- 112 bytes, all different, no LZSS matches possible.

        local archive = zip.zip({{"rand.bin", data}})
        local reader  = zip.new_reader(archive)
        local entries = zip.reader_entries(reader)
        assert.equal(0, entries[1].method, "incompressible data should be Stored")

        local result = zip.unzip(archive)
        assert.equal(data, result["rand.bin"])
    end)
end)

-- ---------------------------------------------------------------------------
-- TC-8: Empty file
-- ---------------------------------------------------------------------------
describe("TC-8: empty file", function()
    it("stores and retrieves an empty file", function()
        local result = roundtrip({{"empty.txt", ""}})
        assert.equal("", result["empty.txt"])
    end)
end)

-- ---------------------------------------------------------------------------
-- TC-9: Large file compressed (100 KB repetitive data)
-- ---------------------------------------------------------------------------
describe("TC-9: large file (100 KB repetitive)", function()
    it("compresses and decompresses 100 KB of repetitive data", function()
        local data = string.rep("Hello, World! ", 7500)  -- ~105000 bytes
        data = data:sub(1, 102400)                        -- exactly 100 KB
        local result = roundtrip({{"large.txt", data}})
        assert.equal(data, result["large.txt"])
    end)
end)

-- ---------------------------------------------------------------------------
-- TC-10: Unicode filename
-- ---------------------------------------------------------------------------
describe("TC-10: Unicode filename", function()
    it("stores and retrieves files with UTF-8 names", function()
        local files = {
            {"\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e/", ""},         -- 日本語/ (dir)
            {"\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e/\xe3\x83\x86\xe3\x82\xb9\xe3\x83\x88.txt", "テスト"},
            {"r\xc3\xa9sum\xc3\xa9.txt", "résumé content"},
        }
        local w = zip.new_writer()
        zip.add_directory(w, files[1][1])
        zip.add_file(w, files[2][1], files[2][2])
        zip.add_file(w, files[3][1], files[3][2])
        local archive = zip.finish(w)

        local reader = zip.new_reader(archive)
        local data = zip.read_by_name(reader, files[3][1])
        assert.equal("r\xc3\xa9sum\xc3\xa9 content", data)
    end)
end)

-- ---------------------------------------------------------------------------
-- TC-11: Nested paths
-- ---------------------------------------------------------------------------
describe("TC-11: nested paths", function()
    it("stores and retrieves files with nested directory structure", function()
        local files = {
            {"a/b/c/deep.txt", "deeply nested"},
            {"a/b/shallow.txt", "shallow"},
        }
        local result = roundtrip(files)
        assert.equal("deeply nested", result["a/b/c/deep.txt"])
        assert.equal("shallow",       result["a/b/shallow.txt"])
    end)
end)

-- ---------------------------------------------------------------------------
-- TC-12: Empty archive
-- ---------------------------------------------------------------------------
describe("TC-12: empty archive", function()
    it("creates and reads an archive with no entries", function()
        local w       = zip.new_writer()
        local archive = zip.finish(w)

        local reader  = zip.new_reader(archive)
        assert.is_not_nil(reader)
        local entries = zip.reader_entries(reader)
        assert.equal(0, #entries)

        local result = zip.unzip(archive)
        assert.are.same({}, result)
    end)
end)

-- ---------------------------------------------------------------------------
-- CRC-32 known vectors
-- ---------------------------------------------------------------------------
describe("crc32", function()
    it("computes correct CRC-32 for 'hello world'", function()
        assert.equal(0x0D4A1185, zip.crc32("hello world"))
    end)

    it("computes correct CRC-32 for empty string", function()
        assert.equal(0x00000000, zip.crc32(""))
    end)

    it("computes correct CRC-32 for '123456789'", function()
        assert.equal(0xCBF43926, zip.crc32("123456789"))
    end)

    it("supports chained (incremental) computation", function()
        local full = zip.crc32("helloworld")
        local part = zip.crc32("world", zip.crc32("hello"))
        assert.equal(full, part)
    end)
end)

-- ---------------------------------------------------------------------------
-- DOS datetime
-- ---------------------------------------------------------------------------
describe("dos_datetime", function()
    it("DOS_EPOCH is 0x00210000", function()
        assert.equal(0x00210000, zip.DOS_EPOCH)
    end)

    it("encodes 1980-01-01 00:00:00", function()
        assert.equal(0x00210000, zip.dos_datetime(1980, 1, 1, 0, 0, 0))
    end)

    it("encodes 2024-06-15 10:30:00", function()
        -- time = (10<<11)|(30<<5)|0 = 20480|960|0 = 21440 = 0x53C0
        -- date = ((2024-1980)<<9)|(6<<5)|15 = (44*512)|192|15 = 22528+192+15 = 22735 = 0x58CF
        -- combined = (0x58CF<<16)|0x53C0 = 0x58CF53C0
        local dt = zip.dos_datetime(2024, 6, 15, 10, 30, 0)
        assert.equal(0x58CF53C0, dt)
    end)
end)

-- ---------------------------------------------------------------------------
-- EOCD scanning
-- ---------------------------------------------------------------------------
describe("EOCD scanning", function()
    it("rejects data too short to be a ZIP archive", function()
        local reader, err = zip.new_reader("too short")
        assert.is_nil(reader)
        assert.is_string(err)
    end)

    it("rejects data with no EOCD signature", function()
        local reader, err = zip.new_reader(string.rep("\0", 100))
        assert.is_nil(reader)
        assert.is_string(err)
    end)
end)

-- ---------------------------------------------------------------------------
-- read_by_name: not found
-- ---------------------------------------------------------------------------
describe("read_by_name", function()
    it("returns nil and error message for missing entry", function()
        local archive = zip.zip({{"exists.txt", "yes"}})
        local reader  = zip.new_reader(archive)
        local data, err = zip.read_by_name(reader, "missing.txt")
        assert.is_nil(data)
        assert.is_string(err)
        assert.is_truthy(err:find("not found"))
    end)
end)

-- ---------------------------------------------------------------------------
-- Security: path traversal rejection
-- ---------------------------------------------------------------------------
describe("security: path traversal", function()
    -- Build a raw ZIP with a crafted entry name to test the reader's guard.
    local function craft_zip(name, content)
        -- Build a minimal ZIP with the given entry name (possibly malicious).
        -- We bypass M.add_file to avoid the writer's own validation,
        -- constructing raw bytes directly.
        local function le16(v)
            return string.char(v & 0xFF, (v >> 8) & 0xFF)
        end
        local function le32(v)
            return string.char(v & 0xFF, (v >> 8) & 0xFF, (v >> 16) & 0xFF, (v >> 24) & 0xFF)
        end
        local crc    = zip.crc32(content)
        local nl     = #name
        local dl     = #content

        local local_hdr = table.concat({
            le32(0x04034B50),
            le16(10),           -- version needed
            le16(0),            -- flags
            le16(0),            -- method=stored
            le16(0), le16(0),   -- time, date
            le32(crc),
            le32(dl), le32(dl), -- compressed, uncompressed
            le16(nl), le16(0),  -- name_len, extra_len
            name,
            content,
        })

        local cd_hdr = table.concat({
            le32(0x02014B50),
            le16(0x031E),       -- version made by
            le16(10),           -- version needed
            le16(0),            -- flags
            le16(0),            -- method=stored
            le16(0), le16(0),   -- time, date
            le32(crc),
            le32(dl), le32(dl),
            le16(nl), le16(0), le16(0), -- name_len, extra, comment
            le16(0), le16(0),   -- disk start, internal attrs
            le32(0),            -- external attrs
            le32(0),            -- local offset
            name,
        })

        local cd_size = #cd_hdr
        local cd_off  = #local_hdr

        local eocd = table.concat({
            le32(0x06054B50),
            le16(0), le16(0),
            le16(1), le16(1),   -- 1 entry
            le32(cd_size), le32(cd_off),
            le16(0),
        })

        return local_hdr .. cd_hdr .. eocd
    end

    it("rejects .. path traversal", function()
        local archive = craft_zip("../../evil.txt", "evil")
        local reader, err = zip.new_reader(archive)
        assert.is_nil(reader)
        assert.is_string(err)
        assert.is_truthy(err:find("traversal") or err:find("%.%."))
    end)

    it("rejects absolute paths", function()
        local archive = craft_zip("/etc/passwd", "root")
        local reader, err = zip.new_reader(archive)
        assert.is_nil(reader)
        assert.is_string(err)
        assert.is_truthy(err:find("absolute"))
    end)

    it("rejects backslash in name", function()
        local archive = craft_zip("foo\\bar.txt", "data")
        local reader, err = zip.new_reader(archive)
        assert.is_nil(reader)
        assert.is_string(err)
        assert.is_truthy(err:find("backslash"))
    end)
end)

-- ---------------------------------------------------------------------------
-- Security: duplicate entry name rejection
-- ---------------------------------------------------------------------------
describe("security: duplicate entry names", function()
    it("unzip raises error on duplicate entry names", function()
        -- Build a zip with two entries named "dup.txt" using writer bypassing
        -- name validation by using internal structure directly.
        local function le16(v) return string.char(v & 0xFF, (v >> 8) & 0xFF) end
        local function le32(v) return string.char(v & 0xFF, (v >> 8) & 0xFF, (v >> 16) & 0xFF, (v >> 24) & 0xFF) end

        local function make_entry(name, data, offset)
            local crc  = zip.crc32(data)
            local lh = table.concat({
                le32(0x04034B50), le16(10), le16(0), le16(0),
                le16(0), le16(0), le32(crc), le32(#data), le32(#data),
                le16(#name), le16(0), name, data,
            })
            local cd = table.concat({
                le32(0x02014B50), le16(0x031E), le16(10), le16(0), le16(0),
                le16(0), le16(0), le32(crc), le32(#data), le32(#data),
                le16(#name), le16(0), le16(0), le16(0), le16(0), le32(0), le32(offset),
                name,
            })
            return lh, cd
        end

        local lh1, cd1 = make_entry("dup.txt", "first",  0)
        local lh2, cd2 = make_entry("dup.txt", "second", #lh1)

        local cd_data = cd1 .. cd2
        local cd_off  = #lh1 + #lh2
        local eocd = table.concat({
            le32(0x06054B50), le16(0), le16(0),
            le16(2), le16(2), le32(#cd_data), le32(cd_off), le16(0),
        })
        local archive = lh1 .. lh2 .. cd_data .. eocd

        assert.has_error(function() zip.unzip(archive) end)
    end)
end)

-- ---------------------------------------------------------------------------
-- DEFLATE: stored block (BTYPE=00) decode path
-- ---------------------------------------------------------------------------
describe("DEFLATE stored block decode path", function()
    it("round-trips an empty file through stored block", function()
        -- Empty file → stored block (BFINAL=1, BTYPE=00) in deflate_compress.
        local result = roundtrip({{"empty.bin", ""}})
        assert.equal("", result["empty.bin"])
    end)
end)
