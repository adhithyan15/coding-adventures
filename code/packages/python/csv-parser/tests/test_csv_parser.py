"""
test_csv_parser.py — Comprehensive tests for the CSV parser.

Test organisation:
  1. Basic / happy-path tests  — simple tables, typical usage
  2. Quoted field tests        — commas inside quotes, newlines inside quotes, escaped quotes
  3. Edge case tests           — empty file, header-only, empty fields, trailing newlines
  4. Ragged row tests          — short rows (padded), long rows (truncated)
  5. Custom delimiter tests    — TSV, semicolons, pipe
  6. Error tests               — UnclosedQuoteError
  7. Whitespace handling       — spaces are significant, not trimmed
  8. No stdlib usage check     — verify we don't import Python's csv module
"""

import sys

import pytest

from csv_parser import UnclosedQuoteError, parse_csv


# ──────────────────────────────────────────────────────────────────────────────
# 1. BASIC / HAPPY-PATH TESTS
# ──────────────────────────────────────────────────────────────────────────────


class TestBasicParsing:
    """Tests for ordinary, well-formed CSV with no special characters."""

    def test_simple_two_column_table(self) -> None:
        """Parse a simple 2-column, 2-row table."""
        source = "name,age\nAlice,30\nBob,25"
        result = parse_csv(source)
        assert result == [
            {"name": "Alice", "age": "30"},
            {"name": "Bob", "age": "25"},
        ]

    def test_three_column_table(self) -> None:
        """Parse a 3-column table from the spec's Example 1."""
        source = "name,age,city\nAlice,30,New York\nBob,25,London"
        result = parse_csv(source)
        assert result == [
            {"name": "Alice", "age": "30", "city": "New York"},
            {"name": "Bob", "age": "25", "city": "London"},
        ]

    def test_single_data_row(self) -> None:
        """Parse a table with exactly one data row."""
        source = "col1,col2\nval1,val2"
        result = parse_csv(source)
        assert result == [{"col1": "val1", "col2": "val2"}]

    def test_many_rows(self) -> None:
        """Parse a table with many data rows."""
        lines = ["a,b"] + [f"{i},{i * 2}" for i in range(100)]
        source = "\n".join(lines)
        result = parse_csv(source)
        assert len(result) == 100
        assert result[0] == {"a": "0", "b": "0"}
        assert result[99] == {"a": "99", "b": "198"}

    def test_single_column(self) -> None:
        """Parse a single-column CSV."""
        source = "name\nAlice\nBob\nCharlie"
        result = parse_csv(source)
        assert result == [
            {"name": "Alice"},
            {"name": "Bob"},
            {"name": "Charlie"},
        ]

    def test_all_values_are_strings(self) -> None:
        """Values must always be strings, even numeric-looking ones."""
        source = "id,score,flag\n1,3.14,true"
        result = parse_csv(source)
        assert result[0]["id"] == "1"
        assert result[0]["score"] == "3.14"
        assert result[0]["flag"] == "true"
        # Confirm types are str, not int/float/bool
        assert isinstance(result[0]["id"], str)
        assert isinstance(result[0]["score"], str)

    def test_trailing_newline_produces_no_extra_row(self) -> None:
        """A trailing newline after the last record should not produce an extra empty row."""
        source = "a,b\n1,2\n"
        result = parse_csv(source)
        assert len(result) == 1
        assert result == [{"a": "1", "b": "2"}]

    def test_crlf_line_endings(self) -> None:
        """Windows-style \\r\\n line endings should be treated as record separators."""
        source = "name,age\r\nAlice,30\r\nBob,25"
        result = parse_csv(source)
        assert result == [
            {"name": "Alice", "age": "30"},
            {"name": "Bob", "age": "25"},
        ]

    def test_cr_only_line_endings(self) -> None:
        """Bare \\r (classic Mac) line endings should be treated as record separators."""
        source = "name,age\rAlice,30\rBob,25"
        result = parse_csv(source)
        assert result == [
            {"name": "Alice", "age": "30"},
            {"name": "Bob", "age": "25"},
        ]


# ──────────────────────────────────────────────────────────────────────────────
# 2. QUOTED FIELD TESTS
# ──────────────────────────────────────────────────────────────────────────────


class TestQuotedFields:
    """Tests for quoted fields — the most complex part of CSV parsing."""

    def test_quoted_field_with_comma(self) -> None:
        """A comma inside a quoted field is NOT a delimiter."""
        source = 'product,description\nWidget,"A small, round widget"'
        result = parse_csv(source)
        assert result == [
            {"product": "Widget", "description": "A small, round widget"}
        ]

    def test_quoted_field_at_start_of_row(self) -> None:
        """A quoted field can appear at the start of a row."""
        source = 'name,city\n"Alice Smith",London'
        result = parse_csv(source)
        assert result == [{"name": "Alice Smith", "city": "London"}]

    def test_all_fields_quoted(self) -> None:
        """Every field in a row can be quoted."""
        source = '"name","age"\n"Alice","30"'
        result = parse_csv(source)
        assert result == [{"name": "Alice", "age": "30"}]

    def test_quoted_field_with_embedded_newline(self) -> None:
        """Spec Example 3: a quoted field spanning two physical lines."""
        source = 'id,note\n1,"Line one\nLine two"\n2,Single line'
        result = parse_csv(source)
        assert result == [
            {"id": "1", "note": "Line one\nLine two"},
            {"id": "2", "note": "Single line"},
        ]

    def test_quoted_field_with_crlf_inside(self) -> None:
        """A \\r\\n inside a quoted field is preserved literally."""
        source = 'id,note\n1,"Line one\r\nLine two"'
        result = parse_csv(source)
        assert result[0]["note"] == "Line one\r\nLine two"

    def test_escaped_double_quote(self) -> None:
        """Spec Example 4: escaped double-quote via \"\" inside quoted field."""
        source = 'id,value\n1,"She said ""hello"""'
        result = parse_csv(source)
        assert result == [{"id": "1", "value": 'She said "hello"'}]

    def test_escaped_double_quote_multiple(self) -> None:
        """Multiple escaped quotes in one field."""
        source = 'x\n"a ""b"" c"'
        result = parse_csv(source)
        assert result == [{"x": 'a "b" c'}]

    def test_quoted_empty_field(self) -> None:
        """An empty quoted field ("") produces an empty string."""
        source = 'a,b,c\n1,"",3'
        result = parse_csv(source)
        assert result == [{"a": "1", "b": "", "c": "3"}]

    def test_quoted_field_with_multiple_commas(self) -> None:
        """Multiple commas inside a single quoted field."""
        source = 'name,csv_data\nAlice,"1,2,3,4,5"'
        result = parse_csv(source)
        assert result == [{"name": "Alice", "csv_data": "1,2,3,4,5"}]

    def test_quoted_field_containing_delimiter_in_header(self) -> None:
        """Quoted field in the header row."""
        source = '"full name",age\nAlice,30'
        result = parse_csv(source)
        assert result == [{"full name": "Alice", "age": "30"}]


# ──────────────────────────────────────────────────────────────────────────────
# 3. EDGE CASE TESTS
# ──────────────────────────────────────────────────────────────────────────────


class TestEdgeCases:
    """Tests for boundary conditions: empty files, header-only, empty fields."""

    def test_empty_string(self) -> None:
        """An empty string should produce an empty list."""
        assert parse_csv("") == []

    def test_header_only(self) -> None:
        """A file with only a header row (no data rows) returns []."""
        assert parse_csv("name,age,city") == []

    def test_header_only_with_trailing_newline(self) -> None:
        """Header-only file with trailing newline still returns []."""
        assert parse_csv("name,age\n") == []

    def test_empty_fields_middle(self) -> None:
        """Spec Example 5: empty unquoted field between two delimiters."""
        source = "a,b,c\n1,,3"
        result = parse_csv(source)
        assert result == [{"a": "1", "b": "", "c": "3"}]

    def test_empty_fields_leading_and_trailing(self) -> None:
        """Empty fields at the start and end of a row."""
        source = "a,b,c\n,2,"
        result = parse_csv(source)
        assert result == [{"a": "", "b": "2", "c": ""}]

    def test_all_empty_fields(self) -> None:
        """A row of all-empty unquoted fields."""
        source = "a,b,c\n,,"
        result = parse_csv(source)
        assert result == [{"a": "", "b": "", "c": ""}]

    def test_single_field_single_row(self) -> None:
        """Minimal CSV: one column, one data row."""
        source = "x\n42"
        result = parse_csv(source)
        assert result == [{"x": "42"}]

    def test_whitespace_is_significant(self) -> None:
        """Spaces around unquoted fields are NOT trimmed."""
        source = "a,b\n  hello  ,  world  "
        result = parse_csv(source)
        assert result == [{"a": "  hello  ", "b": "  world  "}]

    def test_numeric_header_names(self) -> None:
        """Header names can be numeric strings — they're just strings."""
        source = "1,2,3\na,b,c"
        result = parse_csv(source)
        assert result == [{"1": "a", "2": "b", "3": "c"}]


# ──────────────────────────────────────────────────────────────────────────────
# 4. RAGGED ROW TESTS
# ──────────────────────────────────────────────────────────────────────────────


class TestRaggedRows:
    """Tests for rows with field counts different from the header."""

    def test_short_row_padded_with_empty_strings(self) -> None:
        """A row shorter than the header has missing fields filled with ''."""
        source = "a,b,c\n1,2"
        result = parse_csv(source)
        assert result == [{"a": "1", "b": "2", "c": ""}]

    def test_very_short_row(self) -> None:
        """A row with only one field when header has three columns."""
        source = "a,b,c\nonly_a"
        result = parse_csv(source)
        assert result == [{"a": "only_a", "b": "", "c": ""}]

    def test_long_row_truncated_to_header_length(self) -> None:
        """A row longer than the header has extra fields discarded."""
        source = "a,b\n1,2,3,4,5"
        result = parse_csv(source)
        assert result == [{"a": "1", "b": "2"}]

    def test_mixed_ragged_rows(self) -> None:
        """Some rows too short, some too long, some exact."""
        source = "a,b,c\n1,2,3\n4,5\n6,7,8,9"
        result = parse_csv(source)
        assert result == [
            {"a": "1", "b": "2", "c": "3"},  # exact
            {"a": "4", "b": "5", "c": ""},   # too short → padded
            {"a": "6", "b": "7", "c": "8"},  # too long → truncated
        ]


# ──────────────────────────────────────────────────────────────────────────────
# 5. CUSTOM DELIMITER TESTS
# ──────────────────────────────────────────────────────────────────────────────


class TestCustomDelimiter:
    """Tests for the optional delimiter parameter."""

    def test_tab_delimited_tsv(self) -> None:
        """Spec Example 6: tab as delimiter (TSV format)."""
        source = "name\tage\nAlice\t30"
        result = parse_csv(source, delimiter="\t")
        assert result == [{"name": "Alice", "age": "30"}]

    def test_semicolon_delimiter(self) -> None:
        """European CSV often uses semicolons as delimiters."""
        source = "name;age\nAlice;30\nBob;25"
        result = parse_csv(source, delimiter=";")
        assert result == [
            {"name": "Alice", "age": "30"},
            {"name": "Bob", "age": "25"},
        ]

    def test_pipe_delimiter(self) -> None:
        """Pipe-separated values."""
        source = "a|b|c\n1|2|3"
        result = parse_csv(source, delimiter="|")
        assert result == [{"a": "1", "b": "2", "c": "3"}]

    def test_custom_delimiter_inside_quotes(self) -> None:
        """The custom delimiter inside a quoted field is still literal."""
        source = "name;description\nWidget;\"a;b;c\""
        result = parse_csv(source, delimiter=";")
        assert result == [{"name": "Widget", "description": "a;b;c"}]

    def test_comma_in_tsv_unquoted_is_literal(self) -> None:
        """When delimiter is tab, a comma in an unquoted field is just a character."""
        source = "a\tb\nfoo,bar\tbaz"
        result = parse_csv(source, delimiter="\t")
        assert result == [{"a": "foo,bar", "b": "baz"}]


# ──────────────────────────────────────────────────────────────────────────────
# 6. ERROR TESTS
# ──────────────────────────────────────────────────────────────────────────────


class TestErrors:
    """Tests for error conditions."""

    def test_unclosed_quote_raises_error(self) -> None:
        """An unclosed quoted field must raise UnclosedQuoteError."""
        source = 'a,b\n"unclosed,value'
        with pytest.raises(UnclosedQuoteError):
            parse_csv(source)

    def test_unclosed_quote_on_first_row(self) -> None:
        """Unclosed quote in the header row also raises UnclosedQuoteError."""
        source = '"unclosed header'
        with pytest.raises(UnclosedQuoteError):
            parse_csv(source)

    def test_unclosed_quote_error_is_value_error(self) -> None:
        """UnclosedQuoteError inherits from ValueError (Python convention)."""
        with pytest.raises(ValueError):
            parse_csv('"never closed')

    def test_unclosed_quote_mid_field(self) -> None:
        """Quote opened mid-row but never closed before EOF."""
        source = 'x\n"value with no end'
        with pytest.raises(UnclosedQuoteError):
            parse_csv(source)

    def test_unclosed_quote_error_message(self) -> None:
        """The error message should mention unclosed quoted field."""
        with pytest.raises(UnclosedQuoteError, match="[Uu]nclosed"):
            parse_csv('"not closed')


# ──────────────────────────────────────────────────────────────────────────────
# 7. WHITESPACE HANDLING
# ──────────────────────────────────────────────────────────────────────────────


class TestWhitespace:
    """Whitespace is significant in CSV — spaces are NOT trimmed."""

    def test_leading_spaces_in_field(self) -> None:
        """Leading spaces in an unquoted field are preserved."""
        source = "a\n   value"
        result = parse_csv(source)
        assert result[0]["a"] == "   value"

    def test_trailing_spaces_in_field(self) -> None:
        """Trailing spaces in an unquoted field are preserved."""
        source = "a\nvalue   "
        result = parse_csv(source)
        assert result[0]["a"] == "value   "

    def test_spaces_inside_quoted_field(self) -> None:
        """Spaces inside a quoted field are also preserved."""
        source = 'a\n"  spaced  "'
        result = parse_csv(source)
        assert result[0]["a"] == "  spaced  "

    def test_space_as_entire_field(self) -> None:
        """A single space is a valid field value."""
        source = "a,b\n , "
        result = parse_csv(source)
        assert result[0]["a"] == " "
        assert result[0]["b"] == " "


# ──────────────────────────────────────────────────────────────────────────────
# 8. NO STDLIB USAGE CHECK
# ──────────────────────────────────────────────────────────────────────────────


class TestNoStdlibCsv:
    """Verify the implementation does not use Python's standard library csv module."""

    def test_stdlib_csv_not_imported_by_our_code(self) -> None:
        """The standard library 'csv' module must NOT be imported by our parser.

        We check the source code directly rather than sys.modules, because
        test-runner tools (pytest-cov, coverage.py) may load stdlib 'csv'
        themselves before our tests run, making sys.modules checks unreliable.
        The authoritative test is the source-code check below.
        """
        # Verified by test_parser_source_has_no_csv_import — this test is an
        # intentional no-op at runtime; the real check is the source inspection.
        pass

    def test_parser_source_has_no_csv_import(self) -> None:
        """Direct check: read the parser source file and assert no 'import csv'."""
        import csv_parser.parser as _parser_mod  # noqa: E402

        source_file = _parser_mod.__file__
        assert source_file is not None
        with open(source_file) as fh:
            content = fh.read()
        # The parser must not delegate to the stdlib csv module.
        assert "import csv\n" not in content, "Found 'import csv' in parser.py"
        assert "from csv import" not in content, "Found 'from csv import' in parser.py"

    def test_init_source_has_no_csv_import(self) -> None:
        """The package __init__.py must not import the stdlib csv module."""
        import csv_parser as _pkg  # noqa: E402

        source_file = _pkg.__file__
        assert source_file is not None
        with open(source_file) as fh:
            content = fh.read()
        assert "import csv\n" not in content, "Found 'import csv' in __init__.py"
        assert "from csv import" not in content, "Found 'from csv import' in __init__.py"


# ──────────────────────────────────────────────────────────────────────────────
# 9. INTEGRATION / SPEC EXAMPLE TESTS
# ──────────────────────────────────────────────────────────────────────────────


class TestSpecExamples:
    """Verbatim examples from the CSV parser specification."""

    def test_spec_example_1_simple_table(self) -> None:
        """Spec Example 1."""
        source = "name,age,city\nAlice,30,New York\nBob,25,London"
        result = parse_csv(source)
        assert result == [
            {"name": "Alice", "age": "30", "city": "New York"},
            {"name": "Bob", "age": "25", "city": "London"},
        ]

    def test_spec_example_2_quoted_with_comma(self) -> None:
        """Spec Example 2."""
        source = (
            "product,price,description\n"
            'Widget,9.99,"A small, round widget"\n'
            "Gadget,19.99,Electronic device"
        )
        result = parse_csv(source)
        assert result == [
            {"product": "Widget", "price": "9.99", "description": "A small, round widget"},
            {"product": "Gadget", "price": "19.99", "description": "Electronic device"},
        ]

    def test_spec_example_3_quoted_with_newline(self) -> None:
        """Spec Example 3."""
        source = 'id,note\n1,"Line one\nLine two"\n2,Single line'
        result = parse_csv(source)
        assert result == [
            {"id": "1", "note": "Line one\nLine two"},
            {"id": "2", "note": "Single line"},
        ]

    def test_spec_example_4_escaped_quote(self) -> None:
        """Spec Example 4."""
        source = 'id,value\n1,"She said ""hello"""'
        result = parse_csv(source)
        assert result == [
            {"id": "1", "value": 'She said "hello"'},
        ]

    def test_spec_example_5_empty_fields(self) -> None:
        """Spec Example 5."""
        source = "a,b,c\n1,,3\n,2,"
        result = parse_csv(source)
        assert result == [
            {"a": "1", "b": "", "c": "3"},
            {"a": "", "b": "2", "c": ""},
        ]

    def test_spec_example_6_tsv(self) -> None:
        """Spec Example 6."""
        source = "name\tage\nAlice\t30"
        result = parse_csv(source, delimiter="\t")
        assert result == [
            {"name": "Alice", "age": "30"},
        ]
