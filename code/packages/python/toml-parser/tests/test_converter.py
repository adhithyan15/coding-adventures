"""Tests for the TOML converter (semantic phase).

These tests verify that the converter correctly:

1. Builds nested dictionaries from table headers and dotted keys.
2. Converts all value types to their Python equivalents.
3. Enforces semantic constraints (key uniqueness, table consistency,
   inline table immutability, array-of-tables rules).
4. Handles string escape sequences and multi-line string processing.
5. Handles number formats (hex, octal, binary, underscores, special floats).
6. Handles date/time parsing.
"""

from __future__ import annotations

import datetime
import math

import pytest

from toml_parser import TOMLConversionError, TOMLDocument, parse_toml

# =============================================================================
# String Conversion Tests
# =============================================================================


class TestStringConversion:
    """Test conversion of all four TOML string types."""

    def test_basic_string(self) -> None:
        """Basic string: double-quoted, escapes processed."""
        doc = parse_toml('key = "hello world"')
        assert doc["key"] == "hello world"

    def test_basic_string_escape_newline(self) -> None:
        """Basic string with \\n escape."""
        doc = parse_toml('key = "hello\\nworld"')
        assert doc["key"] == "hello\nworld"

    def test_basic_string_escape_tab(self) -> None:
        """Basic string with \\t escape."""
        doc = parse_toml('key = "hello\\tworld"')
        assert doc["key"] == "hello\tworld"

    def test_basic_string_escape_backslash(self) -> None:
        """Basic string with \\\\ escape."""
        doc = parse_toml('key = "hello\\\\world"')
        assert doc["key"] == "hello\\world"

    def test_basic_string_escape_quote(self) -> None:
        """Basic string with \\" escape."""
        doc = parse_toml('key = "hello\\"world"')
        assert doc["key"] == 'hello"world'

    def test_basic_string_unicode_4digit(self) -> None:
        """Basic string with \\u escape (4 hex digits)."""
        doc = parse_toml('key = "\\u0041"')
        assert doc["key"] == "A"

    def test_basic_string_unicode_8digit(self) -> None:
        """Basic string with \\U escape (8 hex digits)."""
        doc = parse_toml('key = "\\U0001F600"')
        assert doc["key"] == "\U0001F600"  # grinning face emoji

    def test_basic_string_multiple_escapes(self) -> None:
        """Basic string with multiple escape sequences."""
        doc = parse_toml('key = "a\\tb\\nc"')
        assert doc["key"] == "a\tb\nc"

    def test_literal_string(self) -> None:
        """Literal string: single-quoted, no escape processing."""
        doc = parse_toml("key = 'hello world'")
        assert doc["key"] == "hello world"

    def test_literal_string_no_escapes(self) -> None:
        """Literal strings preserve backslashes verbatim."""
        doc = parse_toml("key = 'hello\\nworld'")
        assert doc["key"] == "hello\\nworld"

    def test_ml_basic_string(self) -> None:
        """Multi-line basic string."""
        doc = parse_toml('key = """hello\nworld"""')
        assert doc["key"] == "hello\nworld"

    def test_ml_basic_string_opening_newline_trimmed(self) -> None:
        """Multi-line basic string trims newline after opening quotes."""
        doc = parse_toml('key = """\nhello"""')
        assert doc["key"] == "hello"

    def test_ml_basic_string_line_ending_backslash(self) -> None:
        """Multi-line basic string with line-ending backslash."""
        doc = parse_toml('key = """\\\n  hello"""')
        assert doc["key"] == "hello"

    def test_ml_basic_string_escapes(self) -> None:
        """Multi-line basic string processes escapes."""
        doc = parse_toml('key = """hello\\tworld"""')
        assert doc["key"] == "hello\tworld"

    def test_ml_literal_string(self) -> None:
        """Multi-line literal string."""
        doc = parse_toml("key = '''hello\nworld'''")
        assert doc["key"] == "hello\nworld"

    def test_ml_literal_string_opening_newline_trimmed(self) -> None:
        """Multi-line literal string trims newline after opening quotes."""
        doc = parse_toml("key = '''\nhello'''")
        assert doc["key"] == "hello"

    def test_ml_literal_string_no_escapes(self) -> None:
        """Multi-line literal strings preserve backslashes."""
        doc = parse_toml("key = '''hello\\nworld'''")
        assert doc["key"] == "hello\\nworld"

    def test_empty_basic_string(self) -> None:
        """Empty basic string."""
        doc = parse_toml('key = ""')
        assert doc["key"] == ""

    def test_empty_literal_string(self) -> None:
        """Empty literal string."""
        doc = parse_toml("key = ''")
        assert doc["key"] == ""


# =============================================================================
# Integer Conversion Tests
# =============================================================================


class TestIntegerConversion:
    """Test conversion of all integer formats."""

    def test_positive_integer(self) -> None:
        """Simple positive integer."""
        doc = parse_toml("key = 42")
        assert doc["key"] == 42
        assert isinstance(doc["key"], int)

    def test_negative_integer(self) -> None:
        """Negative integer."""
        doc = parse_toml("key = -17")
        assert doc["key"] == -17

    def test_explicit_positive(self) -> None:
        """Explicitly positive integer."""
        doc = parse_toml("key = +99")
        assert doc["key"] == 99

    def test_zero(self) -> None:
        """Zero."""
        doc = parse_toml("key = 0")
        assert doc["key"] == 0

    def test_underscore_separator(self) -> None:
        """Underscore as visual separator."""
        doc = parse_toml("key = 1_000_000")
        assert doc["key"] == 1_000_000

    def test_hex_integer(self) -> None:
        """Hexadecimal integer."""
        doc = parse_toml("key = 0xFF")
        assert doc["key"] == 255

    def test_octal_integer(self) -> None:
        """Octal integer."""
        doc = parse_toml("key = 0o77")
        assert doc["key"] == 63

    def test_binary_integer(self) -> None:
        """Binary integer."""
        doc = parse_toml("key = 0b1010")
        assert doc["key"] == 10

    def test_hex_with_underscores(self) -> None:
        """Hex integer with underscore separators."""
        doc = parse_toml("key = 0xFF_FF")
        assert doc["key"] == 65535


# =============================================================================
# Float Conversion Tests
# =============================================================================


class TestFloatConversion:
    """Test conversion of all float formats."""

    def test_simple_float(self) -> None:
        """Simple decimal float."""
        doc = parse_toml("key = 3.14")
        assert doc["key"] == pytest.approx(3.14)
        assert isinstance(doc["key"], float)

    def test_negative_float(self) -> None:
        """Negative float."""
        doc = parse_toml("key = -0.5")
        assert doc["key"] == pytest.approx(-0.5)

    def test_scientific_notation(self) -> None:
        """Scientific notation."""
        doc = parse_toml("key = 1e10")
        assert doc["key"] == pytest.approx(1e10)

    def test_scientific_with_decimal(self) -> None:
        """Scientific notation with decimal."""
        doc = parse_toml("key = 6.022e23")
        assert doc["key"] == pytest.approx(6.022e23)

    def test_positive_inf(self) -> None:
        """Positive infinity."""
        doc = parse_toml("key = inf")
        assert doc["key"] == float("inf")

    def test_negative_inf(self) -> None:
        """Negative infinity."""
        doc = parse_toml("key = -inf")
        assert doc["key"] == float("-inf")

    def test_nan(self) -> None:
        """Not a number."""
        doc = parse_toml("key = nan")
        assert math.isnan(doc["key"])

    def test_positive_nan(self) -> None:
        """Explicit positive nan."""
        doc = parse_toml("key = +nan")
        assert math.isnan(doc["key"])

    def test_float_with_underscores(self) -> None:
        """Float with underscore separators."""
        doc = parse_toml("key = 1_000.000_1")
        assert doc["key"] == pytest.approx(1000.0001)


# =============================================================================
# Boolean Conversion Tests
# =============================================================================


class TestBooleanConversion:
    """Test conversion of boolean values."""

    def test_true(self) -> None:
        """Boolean true."""
        doc = parse_toml("key = true")
        assert doc["key"] is True

    def test_false(self) -> None:
        """Boolean false."""
        doc = parse_toml("key = false")
        assert doc["key"] is False


# =============================================================================
# Date/Time Conversion Tests
# =============================================================================


class TestDateTimeConversion:
    """Test conversion of date/time values."""

    def test_offset_datetime_z(self) -> None:
        """Offset datetime with Z suffix."""
        doc = parse_toml("key = 1979-05-27T07:32:00Z")
        expected = datetime.datetime(1979, 5, 27, 7, 32, 0, tzinfo=datetime.UTC)
        assert doc["key"] == expected

    def test_offset_datetime_offset(self) -> None:
        """Offset datetime with numeric offset."""
        doc = parse_toml("key = 1979-05-27T07:32:00+09:00")
        assert doc["key"].tzinfo is not None
        assert doc["key"].utcoffset() == datetime.timedelta(hours=9)

    def test_offset_datetime_with_fractional(self) -> None:
        """Offset datetime with fractional seconds."""
        doc = parse_toml("key = 1979-05-27T07:32:00.999Z")
        assert doc["key"].microsecond == 999000

    def test_local_datetime(self) -> None:
        """Local datetime (no timezone)."""
        doc = parse_toml("key = 1979-05-27T07:32:00")
        expected = datetime.datetime(1979, 5, 27, 7, 32, 0)
        assert doc["key"] == expected
        assert doc["key"].tzinfo is None

    def test_local_datetime_space_separator(self) -> None:
        """Local datetime with space instead of T."""
        doc = parse_toml("key = 1979-05-27 07:32:00")
        expected = datetime.datetime(1979, 5, 27, 7, 32, 0)
        assert doc["key"] == expected

    def test_local_date(self) -> None:
        """Local date."""
        doc = parse_toml("key = 1979-05-27")
        expected = datetime.date(1979, 5, 27)
        assert doc["key"] == expected
        assert isinstance(doc["key"], datetime.date)

    def test_local_time(self) -> None:
        """Local time."""
        doc = parse_toml("key = 07:32:00")
        expected = datetime.time(7, 32, 0)
        assert doc["key"] == expected

    def test_local_time_with_fractional(self) -> None:
        """Local time with fractional seconds."""
        doc = parse_toml("key = 07:32:00.999")
        assert doc["key"].microsecond == 999000


# =============================================================================
# Array Conversion Tests
# =============================================================================


class TestArrayConversion:
    """Test conversion of TOML arrays."""

    def test_empty_array(self) -> None:
        """Empty array."""
        doc = parse_toml("key = []")
        assert doc["key"] == []

    def test_integer_array(self) -> None:
        """Array of integers."""
        doc = parse_toml("key = [1, 2, 3]")
        assert doc["key"] == [1, 2, 3]

    def test_string_array(self) -> None:
        """Array of strings."""
        doc = parse_toml('key = ["a", "b", "c"]')
        assert doc["key"] == ["a", "b", "c"]

    def test_mixed_array(self) -> None:
        """TOML v1.0 allows mixed-type arrays."""
        doc = parse_toml('key = [1, "two", true]')
        assert doc["key"] == [1, "two", True]

    def test_nested_array(self) -> None:
        """Nested arrays."""
        doc = parse_toml("key = [[1, 2], [3, 4]]")
        assert doc["key"] == [[1, 2], [3, 4]]

    def test_multiline_array(self) -> None:
        """Multi-line array."""
        source = "key = [\n  1,\n  2,\n  3,\n]"
        doc = parse_toml(source)
        assert doc["key"] == [1, 2, 3]

    def test_array_trailing_comma(self) -> None:
        """Array with trailing comma."""
        doc = parse_toml("key = [1, 2, 3,]")
        assert doc["key"] == [1, 2, 3]


# =============================================================================
# Inline Table Tests
# =============================================================================


class TestInlineTableConversion:
    """Test conversion of inline tables."""

    def test_empty_inline_table(self) -> None:
        """Empty inline table."""
        doc = parse_toml("key = {}")
        assert doc["key"] == {}
        assert isinstance(doc["key"], TOMLDocument)

    def test_simple_inline_table(self) -> None:
        """Inline table with key-value pairs."""
        doc = parse_toml("point = { x = 1, y = 2 }")
        assert doc["point"]["x"] == 1
        assert doc["point"]["y"] == 2

    def test_nested_inline_table(self) -> None:
        """Nested inline tables."""
        doc = parse_toml("key = { inner = { a = 1 } }")
        assert doc["key"]["inner"]["a"] == 1

    def test_inline_table_string_values(self) -> None:
        """Inline table with string values."""
        doc = parse_toml('name = { first = "Tom", last = "Preston-Werner" }')
        assert doc["name"]["first"] == "Tom"
        assert doc["name"]["last"] == "Preston-Werner"


# =============================================================================
# Table Tests
# =============================================================================


class TestTableConversion:
    """Test conversion of table structures."""

    def test_simple_table(self) -> None:
        """Simple table with key-value pairs."""
        source = '[server]\nhost = "localhost"\nport = 8080'
        doc = parse_toml(source)
        assert doc["server"]["host"] == "localhost"
        assert doc["server"]["port"] == 8080

    def test_multiple_tables(self) -> None:
        """Multiple tables."""
        source = '[a]\nx = 1\n[b]\ny = 2'
        doc = parse_toml(source)
        assert doc["a"]["x"] == 1
        assert doc["b"]["y"] == 2

    def test_dotted_table(self) -> None:
        """Dotted table header."""
        source = '[a.b]\nc = 1'
        doc = parse_toml(source)
        assert doc["a"]["b"]["c"] == 1

    def test_implicit_table(self) -> None:
        """Implicit tables from dotted keys."""
        source = "a.b.c = 1"
        doc = parse_toml(source)
        assert doc["a"]["b"]["c"] == 1

    def test_implicit_then_explicit(self) -> None:
        """Implicit table later explicitly defined."""
        source = "a.b = 1\n[a]\nc = 2"
        doc = parse_toml(source)
        assert doc["a"]["b"] == 1
        assert doc["a"]["c"] == 2

    def test_super_table_implicit(self) -> None:
        """Super-table created implicitly by sub-table."""
        source = '[a.b]\nc = 1\n[a.d]\ne = 2'
        doc = parse_toml(source)
        assert doc["a"]["b"]["c"] == 1
        assert doc["a"]["d"]["e"] == 2


# =============================================================================
# Array-of-Tables Tests
# =============================================================================


class TestArrayOfTablesConversion:
    """Test conversion of array-of-tables."""

    def test_simple_array_table(self) -> None:
        """Simple array of tables."""
        source = '[[products]]\nname = "Hammer"\n[[products]]\nname = "Nail"'
        doc = parse_toml(source)
        assert len(doc["products"]) == 2
        assert doc["products"][0]["name"] == "Hammer"
        assert doc["products"][1]["name"] == "Nail"

    def test_array_table_with_sub_tables(self) -> None:
        """Array of tables with sub-tables."""
        source = '[[fruit]]\nname = "apple"\n[fruit.physical]\ncolor = "red"'
        doc = parse_toml(source)
        assert doc["fruit"][0]["name"] == "apple"
        assert doc["fruit"][0]["physical"]["color"] == "red"

    def test_nested_array_tables(self) -> None:
        """Nested array of tables."""
        source = (
            '[[fruit]]\nname = "apple"\n'
            '[[fruit.variety]]\nname = "red delicious"\n'
            '[[fruit.variety]]\nname = "granny smith"'
        )
        doc = parse_toml(source)
        assert doc["fruit"][0]["name"] == "apple"
        assert len(doc["fruit"][0]["variety"]) == 2
        assert doc["fruit"][0]["variety"][0]["name"] == "red delicious"
        assert doc["fruit"][0]["variety"][1]["name"] == "granny smith"


# =============================================================================
# Key Type Tests
# =============================================================================


class TestKeyTypes:
    """Test that various token types work as keys."""

    def test_bare_key(self) -> None:
        """Standard bare key."""
        doc = parse_toml("name = 1")
        assert doc["name"] == 1

    def test_bare_key_with_dashes(self) -> None:
        """Bare key with dashes."""
        doc = parse_toml("my-key = 1")
        assert doc["my-key"] == 1

    def test_bare_key_with_underscores(self) -> None:
        """Bare key with underscores."""
        doc = parse_toml("my_key = 1")
        assert doc["my_key"] == 1

    def test_basic_string_key(self) -> None:
        """Quoted basic string key."""
        doc = parse_toml('"key with spaces" = 1')
        assert doc["key with spaces"] == 1

    def test_literal_string_key(self) -> None:
        """Quoted literal string key."""
        doc = parse_toml("'key' = 1")
        assert doc["key"] == 1

    def test_integer_as_key(self) -> None:
        """Integer token used as a key (valid in TOML)."""
        doc = parse_toml("42 = 1")
        assert doc["42"] == 1

    def test_true_as_key(self) -> None:
        """Boolean true token used as a key (valid in TOML)."""
        doc = parse_toml("true = 1")
        assert doc["true"] == 1

    def test_false_as_key(self) -> None:
        """Boolean false token used as a key."""
        doc = parse_toml("false = 1")
        assert doc["false"] == 1


# =============================================================================
# Semantic Error Tests
# =============================================================================


class TestSemanticErrors:
    """Test that semantic constraints are enforced."""

    def test_duplicate_key(self) -> None:
        """Duplicate key in the same table."""
        with pytest.raises(TOMLConversionError, match="Duplicate key"):
            parse_toml("a = 1\na = 2")

    def test_duplicate_key_in_table(self) -> None:
        """Duplicate key within a table section."""
        with pytest.raises(TOMLConversionError, match="Duplicate key"):
            parse_toml("[t]\na = 1\na = 2")

    def test_duplicate_table(self) -> None:
        """Defining the same table twice."""
        with pytest.raises(TOMLConversionError, match="already defined"):
            parse_toml("[a]\nx = 1\n[a]\ny = 2")

    def test_key_overwrites_table(self) -> None:
        """A key that would overwrite a table."""
        with pytest.raises(TOMLConversionError, match="non-table"):
            parse_toml("a = 1\n[a]\nb = 2")

    def test_table_overwrites_key(self) -> None:
        """A table that would overwrite a scalar key."""
        with pytest.raises(TOMLConversionError):
            parse_toml("[a]\nb = 1\n[a.b]\nc = 2")

    def test_inline_table_immutable(self) -> None:
        """Cannot extend an inline table with a table header."""
        with pytest.raises(TOMLConversionError, match="inline table"):
            parse_toml("a = { b = 1 }\n[a]\nc = 2")

    def test_inline_table_immutable_dotted(self) -> None:
        """Cannot extend an inline table with a dotted key."""
        with pytest.raises(TOMLConversionError, match="inline table"):
            parse_toml("a = { b = 1 }\na.c = 2")

    def test_array_table_conflicts_with_table(self) -> None:
        """Array of tables conflicts with regular table."""
        with pytest.raises(TOMLConversionError):
            parse_toml("[a]\nb = 1\n[[a]]\nc = 2")

    def test_table_conflicts_with_array_table(self) -> None:
        """Regular table conflicts with array of tables."""
        with pytest.raises(TOMLConversionError):
            parse_toml("[[a]]\nb = 1\n[a]\nc = 2")


# =============================================================================
# TOMLDocument Type Tests
# =============================================================================


class TestTOMLDocument:
    """Test the TOMLDocument dict subclass."""

    def test_is_dict(self) -> None:
        """TOMLDocument is a dict subclass."""
        doc = TOMLDocument()
        assert isinstance(doc, dict)

    def test_repr(self) -> None:
        """TOMLDocument has a custom repr."""
        doc = TOMLDocument({"a": 1})
        assert "TOMLDocument" in repr(doc)

    def test_parse_returns_toml_document(self) -> None:
        """parse_toml returns a TOMLDocument instance."""
        doc = parse_toml("key = 1")
        assert isinstance(doc, TOMLDocument)

    def test_nested_tables_are_toml_document(self) -> None:
        """Nested tables are also TOMLDocument instances."""
        doc = parse_toml("[server]\nport = 8080")
        assert isinstance(doc["server"], TOMLDocument)


# =============================================================================
# Edge Cases
# =============================================================================


class TestEdgeCases:
    """Test edge cases and unusual but valid TOML."""

    def test_empty_document(self) -> None:
        """Empty string produces empty document."""
        doc = parse_toml("")
        assert doc == {}

    def test_only_comments(self) -> None:
        """Document with only comments produces empty document."""
        doc = parse_toml("# just a comment\n# another one")
        assert doc == {}

    def test_only_newlines(self) -> None:
        """Document with only blank lines produces empty document."""
        doc = parse_toml("\n\n\n")
        assert doc == {}

    def test_deeply_nested_dotted_key(self) -> None:
        """Deeply nested dotted key."""
        doc = parse_toml("a.b.c.d.e = 1")
        assert doc["a"]["b"]["c"]["d"]["e"] == 1

    def test_array_of_inline_tables(self) -> None:
        """Array containing inline tables."""
        doc = parse_toml("key = [{ a = 1 }, { a = 2 }]")
        assert doc["key"] == [{"a": 1}, {"a": 2}]

    def test_root_level_keys_before_tables(self) -> None:
        """Root-level keys before any table headers."""
        source = 'title = "TOML"\n[server]\nport = 8080'
        doc = parse_toml(source)
        assert doc["title"] == "TOML"
        assert doc["server"]["port"] == 8080
