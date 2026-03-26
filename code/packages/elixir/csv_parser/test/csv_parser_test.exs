defmodule CsvParserTest do
  use ExUnit.Case, async: true

  # We alias the module under test for brevity.
  alias CsvParser

  # ===========================================================================
  # 1. EMPTY AND MINIMAL INPUTS
  # ===========================================================================
  #
  # The edge cases at the extremes of "small input" trip up many CSV parsers.
  # Verify these before testing any actual data.

  describe "empty and minimal inputs" do
    test "empty string returns empty list" do
      # An empty file has no header and no data.
      assert {:ok, []} = CsvParser.parse_csv("")
    end

    test "single newline returns empty list" do
      # A file containing only a newline has no real content.
      assert {:ok, []} = CsvParser.parse_csv("\n")
    end

    test "header-only file (one row, no data rows) returns empty list" do
      # The header row defines columns but there are no data rows to return.
      assert {:ok, []} = CsvParser.parse_csv("name,age,city")
    end

    test "header-only file with trailing newline returns empty list" do
      assert {:ok, []} = CsvParser.parse_csv("name,age,city\n")
    end

    test "single-column header only" do
      assert {:ok, []} = CsvParser.parse_csv("id")
    end

    test "single-column with one data row" do
      assert {:ok, rows} = CsvParser.parse_csv("id\n42")
      assert rows == [%{"id" => "42"}]
    end

    test "single-column with one data row and trailing newline" do
      assert {:ok, rows} = CsvParser.parse_csv("id\n42\n")
      assert rows == [%{"id" => "42"}]
    end
  end

  # ===========================================================================
  # 2. SIMPLE MULTI-COLUMN TABLES
  # ===========================================================================
  #
  # The "happy path": well-formed CSV with quoted fields, unquoted fields,
  # multiple rows, trailing newline or not.

  describe "simple multi-column table" do
    test "three columns, two data rows" do
      # This is the canonical CSV example. All fields are plain unquoted strings.
      input = "name,age,city\nAlice,30,New York\nBob,25,London\n"

      assert {:ok, rows} = CsvParser.parse_csv(input)

      assert rows == [
               %{"name" => "Alice", "age" => "30", "city" => "New York"},
               %{"name" => "Bob", "age" => "25", "city" => "London"}
             ]
    end

    test "three columns, two data rows, no trailing newline" do
      # Trailing newline is optional per spec.
      input = "name,age,city\nAlice,30,New York\nBob,25,London"

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert length(rows) == 2
      assert hd(rows)["name"] == "Alice"
      assert List.last(rows)["name"] == "Bob"
    end

    test "all values are returned as strings" do
      # No type coercion: numbers, booleans, nulls — all strings.
      input = "int,float,bool,empty\n42,3.14,true,\n"

      assert {:ok, [row]} = CsvParser.parse_csv(input)

      # The integer 42 is returned as the STRING "42", not the integer 42.
      assert row["int"] == "42"
      assert row["float"] == "3.14"
      assert row["bool"] == "true"
      # The empty field is the empty string, not nil.
      assert row["empty"] == ""
    end

    test "whitespace in values is preserved" do
      # Spec says: whitespace is significant. Leading/trailing spaces are
      # part of the field value. Trimming is the caller's responsibility.
      input = "a,b\n  hello  ,  world  \n"

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["a"] == "  hello  "
      assert row["b"] == "  world  "
    end
  end

  # ===========================================================================
  # 3. QUOTED FIELDS
  # ===========================================================================

  describe "quoted fields" do
    test "quoted field containing the delimiter" do
      # The comma inside "A small, round widget" is literal, not a delimiter.
      input = ~s(product,price,description\nWidget,9.99,"A small, round widget"\n)

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["description"] == "A small, round widget"
    end

    test "quoted field containing a newline" do
      # A quoted field can span multiple physical lines. The embedded newline
      # is preserved literally in the output.
      input = "id,note\n1,\"Line one\nLine two\"\n2,Single line\n"

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert rows |> hd() |> Map.get("note") == "Line one\nLine two"
      assert rows |> List.last() |> Map.get("note") == "Single line"
    end

    test "quoted field with escaped double-quote (\"\")" do
      # Inside a quoted field, two consecutive double-quotes ("") represent
      # a single double-quote character.
      #
      # Input:   1,"She said ""hello"""
      # Decoded: 1, She said "hello"
      input = ~s(id,value\n1,"She said ""hello"""\n2,plain\n)

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert hd(rows)["value"] == ~s(She said "hello")
      assert List.last(rows)["value"] == "plain"
    end

    test "quoted field with multiple escaped double-quotes" do
      # Multiple "" escapes in one field.
      # "a""b""c" → a"b"c
      input = ~s(x\n"a""b""c"\n)

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["x"] == ~s(a"b"c)
    end

    test "entire field is an escaped double-quote" do
      # The field value is just a single double-quote character.
      # Input: """"  (four characters: open, escaped-quote, close)
      input = ~s(x\n""\n)

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["x"] == ""
    end

    test "quoted field containing only double-quotes escaped" do
      input = ~s(x\n""""\n)

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["x"] == ~s(")
    end

    test "quoted field is first field in row" do
      input = ~s(a,b\n"quoted first",second\n)

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["a"] == "quoted first"
      assert row["b"] == "second"
    end

    test "quoted field is last field in row" do
      input = ~s(a,b\nfirst,"quoted last"\n)

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["a"] == "first"
      assert row["b"] == "quoted last"
    end

    test "all fields quoted" do
      input = ~s("name","age"\n"Alice","30"\n)

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["name"] == "Alice"
      assert row["age"] == "30"
    end

    test "quoted empty field" do
      # An empty quoted field: just "" (two double-quote chars)
      input = ~s(a,b,c\n1,"",3\n)

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["b"] == ""
    end
  end

  # ===========================================================================
  # 4. EMPTY FIELDS
  # ===========================================================================
  #
  # Adjacent delimiters produce empty string fields.

  describe "empty fields" do
    test "middle field is empty" do
      # a,,b → three fields: "a", "", "b"
      input = "a,b,c\n1,,3\n"

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["a"] == "1"
      assert row["b"] == ""
      assert row["c"] == "3"
    end

    test "first field is empty" do
      # ,2,3 → "", "2", "3"
      input = "a,b,c\n,2,\n"

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["a"] == ""
      assert row["b"] == "2"
      assert row["c"] == ""
    end

    test "all fields are empty" do
      # ,, → three empty fields
      input = "a,b,c\n,,\n"

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["a"] == ""
      assert row["b"] == ""
      assert row["c"] == ""
    end

    test "multiple rows with empty fields" do
      input = "a,b,c\n1,,3\n,2,\n"

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert length(rows) == 2
      assert hd(rows) == %{"a" => "1", "b" => "", "c" => "3"}
      assert List.last(rows) == %{"a" => "", "b" => "2", "c" => ""}
    end
  end

  # ===========================================================================
  # 5. CUSTOM DELIMITER
  # ===========================================================================

  describe "custom delimiter" do
    test "tab delimiter (TSV)" do
      input = "name\tage\nAlice\t30\n"

      assert {:ok, rows} = CsvParser.parse_csv(input, "\t")
      assert rows == [%{"name" => "Alice", "age" => "30"}]
    end

    test "semicolon delimiter" do
      input = "a;b;c\n1;2;3\n"

      assert {:ok, rows} = CsvParser.parse_csv(input, ";")
      assert rows == [%{"a" => "1", "b" => "2", "c" => "3"}]
    end

    test "pipe delimiter" do
      input = "x|y\nhello|world\n"

      assert {:ok, rows} = CsvParser.parse_csv(input, "|")
      assert rows == [%{"x" => "hello", "y" => "world"}]
    end

    test "tab delimiter with quoted field containing comma" do
      # With tab as delimiter, commas inside fields are literal.
      input = "name\tnote\nAlice\t\"a,b,c\"\n"

      assert {:ok, [row]} = CsvParser.parse_csv(input, "\t")
      assert row["note"] == "a,b,c"
    end

    test "invalid delimiter (multiple characters) raises ArgumentError" do
      assert_raise ArgumentError, fn ->
        CsvParser.parse_csv("a,b\n1,2\n", ",,")
      end
    end
  end

  # ===========================================================================
  # 6. RAGGED ROWS (MISMATCHED COLUMN COUNTS)
  # ===========================================================================
  #
  # The spec says: pad short rows with "", truncate long rows to header length.

  describe "ragged rows" do
    test "row with fewer fields than header gets padded with empty strings" do
      # Header has 3 columns. Data row only has 2 fields.
      # Missing "c" column should be filled with "".
      input = "a,b,c\n1,2\n"

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["a"] == "1"
      assert row["b"] == "2"
      assert row["c"] == ""
    end

    test "row with more fields than header gets truncated" do
      # Header has 2 columns. Data row has 4 fields.
      # Extra "3" and "4" should be discarded.
      input = "a,b\n1,2,3,4\n"

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert Map.keys(row) |> Enum.sort() == ["a", "b"]
      assert row["a"] == "1"
      assert row["b"] == "2"
    end

    test "row with only one field when header has three" do
      input = "a,b,c\nonlyone\n"

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["a"] == "onlyone"
      assert row["b"] == ""
      assert row["c"] == ""
    end

    test "mixed ragged rows" do
      input = "a,b,c\n1,2\n1,2,3\n1,2,3,4,5\n"

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert length(rows) == 3

      # Short row → padded
      assert Enum.at(rows, 0) == %{"a" => "1", "b" => "2", "c" => ""}

      # Correct length → unchanged
      assert Enum.at(rows, 1) == %{"a" => "1", "b" => "2", "c" => "3"}

      # Long row → truncated
      assert Enum.at(rows, 2) == %{"a" => "1", "b" => "2", "c" => "3"}
    end
  end

  # ===========================================================================
  # 7. LINE ENDINGS
  # ===========================================================================
  #
  # CSV files come from many operating systems. We must handle all three
  # common newline conventions:
  #   - \n   (Unix / Linux / macOS since 10.0)
  #   - \r\n (Windows)
  #   - \r   (old Mac OS 9)

  describe "line endings" do
    test "Unix line endings (LF only)" do
      input = "a,b\n1,2\n3,4\n"

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert length(rows) == 2
    end

    test "Windows line endings (CRLF)" do
      input = "a,b\r\n1,2\r\n3,4\r\n"

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert length(rows) == 2
      assert hd(rows) == %{"a" => "1", "b" => "2"}
    end

    test "old Mac line endings (CR only)" do
      input = "a,b\r1,2\r3,4\r"

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert length(rows) == 2
    end

    test "embedded newline in quoted field with CRLF file" do
      # Even in a CRLF file, an embedded LF inside a quoted field is literal.
      input = "id,note\r\n1,\"Line1\nLine2\"\r\n"

      assert {:ok, [row]} = CsvParser.parse_csv(input)
      assert row["note"] == "Line1\nLine2"
    end
  end

  # ===========================================================================
  # 8. ERROR CASES
  # ===========================================================================

  describe "error cases" do
    test "unclosed quoted field at end of input" do
      # The input ends while still inside a quoted field — this is an error.
      input = ~s(id,value\n1,"unclosed)

      assert {:error, reason} = CsvParser.parse_csv(input)
      assert reason =~ "Unclosed quoted field"
    end

    test "unclosed quoted field with embedded content" do
      input = ~s(a,b\n1,"this has a comma, and no end)

      assert {:error, _reason} = CsvParser.parse_csv(input)
    end

    test "unclosed quoted field spanning multiple lines" do
      input = "a,b\n1,\"line one\nline two (still no closing quote)"

      assert {:error, _reason} = CsvParser.parse_csv(input)
    end
  end

  # ===========================================================================
  # 9. MULTI-ROW DATA
  # ===========================================================================

  describe "multiple data rows" do
    test "three data rows" do
      input = "name,score\nAlice,95\nBob,87\nCharlie,72\n"

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert length(rows) == 3
      scores = Enum.map(rows, & &1["score"])
      assert scores == ["95", "87", "72"]
    end

    test "preserves row order" do
      # Row order must match the order in the input file.
      input = "n\n1\n2\n3\n4\n5\n"

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert Enum.map(rows, & &1["n"]) == ["1", "2", "3", "4", "5"]
    end
  end

  # ===========================================================================
  # 10. PARSE_CSV/1 AND PARSE_CSV/2 EQUIVALENCE
  # ===========================================================================

  describe "parse_csv/1 and parse_csv/2" do
    test "parse_csv/1 and parse_csv/2 with comma give same result" do
      input = "a,b\n1,2\n"

      assert CsvParser.parse_csv(input) == CsvParser.parse_csv(input, ",")
    end
  end

  # ===========================================================================
  # 11. COMPLEX / INTEGRATION TESTS
  # ===========================================================================

  describe "complex integration cases" do
    test "mixed quoted and unquoted fields in same row" do
      input = ~s(product,price,description\nWidget,9.99,"A small, round widget"\nGadget,19.99,Electronic device\n)

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert length(rows) == 2
      assert hd(rows)["description"] == "A small, round widget"
      assert List.last(rows)["description"] == "Electronic device"
    end

    test "quoted field with embedded newline in multi-row file" do
      # Row 1: id=1, note spans two physical lines
      # Row 2: id=2, note is single line
      input = "id,note\n1,\"Line one\nLine two\"\n2,Single line\n"

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert length(rows) == 2
      assert hd(rows)["note"] == "Line one\nLine two"
      assert List.last(rows)["note"] == "Single line"
    end

    test "product catalog CSV" do
      input = """
      sku,name,price,in_stock
      WIDGET-001,Widget Pro,9.99,true
      GADGET-002,"Super Gadget, v2",49.99,false
      THNG-003,"The ""Thing""",0.99,true
      """

      assert {:ok, rows} = CsvParser.parse_csv(input)
      assert length(rows) == 3

      [widget, gadget, thing] = rows
      assert widget["sku"] == "WIDGET-001"
      assert widget["name"] == "Widget Pro"

      # Quoted field with embedded comma
      assert gadget["name"] == "Super Gadget, v2"

      # Quoted field with escaped double-quotes
      assert thing["name"] == ~s(The "Thing")
    end
  end
end
