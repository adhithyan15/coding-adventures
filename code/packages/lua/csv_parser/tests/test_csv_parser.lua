-- Tests for csv_parser
--
-- Comprehensive test suite covering the four-state RFC 4180 automaton:
-- basic parsing, quoted fields, escaped quotes, all line-ending conventions,
-- custom delimiters, empty fields, edge cases.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local m = require("coding_adventures.csv_parser")

describe("csv_parser", function()

    -- -----------------------------------------------------------------------
    -- Meta / version
    -- -----------------------------------------------------------------------

    it("has VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    it("exposes a parse function", function()
        assert.is_function(m.parse)
    end)

    -- -----------------------------------------------------------------------
    -- Empty / trivial inputs
    -- -----------------------------------------------------------------------

    it("returns empty table for empty string", function()
        local rows = m.parse("")
        assert.equals(0, #rows)
    end)

    it("parses a single field with no newline", function()
        local rows = m.parse("hello")
        assert.equals(1, #rows)
        assert.equals(1, #rows[1])
        assert.equals("hello", rows[1][1])
    end)

    it("parses a single row with no trailing newline", function()
        local rows = m.parse("a,b,c")
        assert.equals(1, #rows)
        assert.same({"a", "b", "c"}, rows[1])
    end)

    -- -----------------------------------------------------------------------
    -- Basic multi-row parsing
    -- -----------------------------------------------------------------------

    it("parses two rows separated by LF", function()
        local rows = m.parse("a,b,c\n1,2,3")
        assert.equals(2, #rows)
        assert.same({"a", "b", "c"}, rows[1])
        assert.same({"1", "2", "3"}, rows[2])
    end)

    it("parses two rows separated by CRLF", function()
        local rows = m.parse("a,b,c\r\n1,2,3")
        assert.equals(2, #rows)
        assert.same({"a", "b", "c"}, rows[1])
        assert.same({"1", "2", "3"}, rows[2])
    end)

    it("parses two rows separated by bare CR", function()
        local rows = m.parse("a,b,c\r1,2,3")
        assert.equals(2, #rows)
        assert.same({"a", "b", "c"}, rows[1])
        assert.same({"1", "2", "3"}, rows[2])
    end)

    it("trailing LF does not produce extra empty row", function()
        local rows = m.parse("a,b\n1,2\n")
        assert.equals(2, #rows)
        assert.same({"a", "b"}, rows[1])
        assert.same({"1", "2"}, rows[2])
    end)

    it("trailing CRLF does not produce extra empty row", function()
        local rows = m.parse("a,b\r\n1,2\r\n")
        assert.equals(2, #rows)
    end)

    -- -----------------------------------------------------------------------
    -- Empty fields
    -- -----------------------------------------------------------------------

    it("handles leading empty field", function()
        local rows = m.parse(",b,c")
        assert.equals(1, #rows)
        assert.same({"", "b", "c"}, rows[1])
    end)

    it("handles trailing empty field (no trailing newline)", function()
        local rows = m.parse("a,b,")
        assert.equals(1, #rows)
        assert.same({"a", "b", ""}, rows[1])
    end)

    it("handles consecutive empty fields", function()
        local rows = m.parse("a,,c")
        assert.equals(1, #rows)
        assert.same({"a", "", "c"}, rows[1])
    end)

    it("handles a row of all empty fields", function()
        local rows = m.parse(",,")
        assert.equals(1, #rows)
        assert.same({"", "", ""}, rows[1])
    end)

    -- -----------------------------------------------------------------------
    -- Quoted fields
    -- -----------------------------------------------------------------------

    it("parses a quoted field", function()
        local rows = m.parse('"hello"')
        assert.equals(1, #rows)
        assert.equals("hello", rows[1][1])
    end)

    it("quoted field with embedded comma is a single field", function()
        local rows = m.parse('"a,b",c')
        assert.equals(1, #rows)
        assert.same({"a,b", "c"}, rows[1])
    end)

    it("quoted field with embedded newline spans within one field", function()
        local rows = m.parse('"line1\nline2",end')
        assert.equals(1, #rows)
        assert.equals("line1\nline2", rows[1][1])
        assert.equals("end", rows[1][2])
    end)

    it("escaped quote inside quoted field", function()
        -- RFC 4180: a literal " is represented as "" inside a quoted field
        local rows = m.parse('"say ""hi"""')
        assert.equals(1, #rows)
        assert.equals('say "hi"', rows[1][1])
    end)

    it("empty quoted field", function()
        local rows = m.parse('""')
        assert.equals(1, #rows)
        assert.equals("", rows[1][1])
    end)

    it("quoted field followed by delimiter then unquoted", function()
        local rows = m.parse('"foo",bar')
        assert.equals(1, #rows)
        assert.same({"foo", "bar"}, rows[1])
    end)

    it("multiple quoted fields in one row", function()
        local rows = m.parse('"a","b","c"')
        assert.equals(1, #rows)
        assert.same({"a", "b", "c"}, rows[1])
    end)

    -- -----------------------------------------------------------------------
    -- Custom delimiter
    -- -----------------------------------------------------------------------

    it("parses with semicolon delimiter", function()
        local rows = m.parse("a;b;c\n1;2;3", {delimiter = ";"})
        assert.equals(2, #rows)
        assert.same({"a", "b", "c"}, rows[1])
        assert.same({"1", "2", "3"}, rows[2])
    end)

    it("parses with tab delimiter", function()
        local rows = m.parse("a\tb\tc", {delimiter = "\t"})
        assert.equals(1, #rows)
        assert.same({"a", "b", "c"}, rows[1])
    end)

    it("commas are literal when delimiter is semicolon", function()
        local rows = m.parse("a,b;c", {delimiter = ";"})
        assert.equals(1, #rows)
        assert.same({"a,b", "c"}, rows[1])
    end)

    -- -----------------------------------------------------------------------
    -- Multi-row realistic data
    -- -----------------------------------------------------------------------

    it("parses a realistic CSV with header and data rows", function()
        local csv = "name,age,city\nAlice,30,\"New York\"\nBob,25,London"
        local rows = m.parse(csv)
        assert.equals(3, #rows)
        assert.same({"name", "age", "city"}, rows[1])
        assert.same({"Alice", "30", "New York"}, rows[2])
        assert.same({"Bob", "25", "London"}, rows[3])
    end)

    it("handles numeric fields as strings", function()
        local rows = m.parse("1,2.5,-3,1e10")
        assert.same({"1", "2.5", "-3", "1e10"}, rows[1])
    end)

    -- -----------------------------------------------------------------------
    -- Error cases
    -- -----------------------------------------------------------------------

    it("errors on invalid (multi-char) delimiter", function()
        assert.has_error(function()
            m.parse("a,b", {delimiter = ",,"})
        end)
    end)

    it("errors on non-string delimiter", function()
        assert.has_error(function()
            m.parse("a,b", {delimiter = 44})
        end)
    end)

end)
