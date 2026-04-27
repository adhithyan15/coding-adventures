// ============================================================================
// CsvParserTest.java — Unit Tests for CsvParser
// ============================================================================
//
// Test strategy:
//   1. Basic CSV — single row, multi-row, header only
//   2. Quoted fields — commas, newlines, escaped quotes
//   3. Ragged rows — short rows (padded), long rows (truncated)
//   4. Custom delimiter — TSV and pipe-separated
//   5. Newline variants — \n, \r\n, \r
//   6. Edge cases — empty input, empty fields, header-only, trailing newline
//   7. Error case — unclosed quoted field
//   8. Whitespace handling — spaces preserved (no trimming)
// ============================================================================

package com.codingadventures.csvparser;

import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class CsvParserTest {

    // =========================================================================
    // 1. Basic CSV
    // =========================================================================

    @Test
    void singleRowWithHeader() throws Exception {
        List<Map<String, String>> rows = CsvParser.parseCSV("name,age\nAlice,30\n");
        assertEquals(1, rows.size());
        assertEquals("Alice", rows.get(0).get("name"));
        assertEquals("30", rows.get(0).get("age"));
    }

    @Test
    void multipleDataRows() throws Exception {
        String csv = "name,age\nAlice,30\nBob,25\nCarol,35\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals(3, rows.size());
        assertEquals("Alice", rows.get(0).get("name"));
        assertEquals("Bob", rows.get(1).get("name"));
        assertEquals("Carol", rows.get(2).get("name"));
    }

    @Test
    void headerOnlyReturnsEmptyList() throws Exception {
        List<Map<String, String>> rows = CsvParser.parseCSV("name,age\n");
        assertTrue(rows.isEmpty());
    }

    @Test
    void emptyInputReturnsEmptyList() throws Exception {
        assertTrue(CsvParser.parseCSV("").isEmpty());
    }

    @Test
    void singleColumnCSV() throws Exception {
        List<Map<String, String>> rows = CsvParser.parseCSV("name\nAlice\nBob\n");
        assertEquals(2, rows.size());
        assertEquals("Alice", rows.get(0).get("name"));
        assertEquals("Bob", rows.get(1).get("name"));
    }

    // =========================================================================
    // 2. Quoted Fields
    // =========================================================================

    @Test
    void quotedFieldWithComma() throws Exception {
        // Field value contains a comma — must be quoted
        String csv = "name,address\nAlice,\"123 Main St, Suite 4\"\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals("123 Main St, Suite 4", rows.get(0).get("address"));
    }

    @Test
    void quotedFieldWithNewline() throws Exception {
        // Field value contains an embedded newline
        String csv = "name,bio\nAlice,\"Line1\nLine2\"\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals("Line1\nLine2", rows.get(0).get("bio"));
    }

    @Test
    void escapedDoubleQuote() throws Exception {
        // "" inside a quoted field is an escaped double-quote
        // "say ""hello""" → say "hello"
        String csv = "name,greeting\nAlice,\"say \"\"hello\"\"\"\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals("say \"hello\"", rows.get(0).get("greeting"));
    }

    @Test
    void quotedEmptyField() throws Exception {
        // A quoted field with no content = empty string
        String csv = "a,b\n\"\",x\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals("", rows.get(0).get("a"));
        assertEquals("x", rows.get(0).get("b"));
    }

    @Test
    void quotedFieldAtEndOfRow() throws Exception {
        String csv = "a,b\nfoo,\"bar baz\"\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals("bar baz", rows.get(0).get("b"));
    }

    // =========================================================================
    // 3. Ragged Rows
    // =========================================================================

    @Test
    void shortRowPaddedWithEmptyStrings() throws Exception {
        // Row has fewer fields than the header
        String csv = "a,b,c\n1,2\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals("1", rows.get(0).get("a"));
        assertEquals("2", rows.get(0).get("b"));
        assertEquals("", rows.get(0).get("c")); // padded with ""
    }

    @Test
    void longRowTruncated() throws Exception {
        // Row has more fields than the header — extras are silently discarded
        String csv = "a,b\n1,2,3,4\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals("1", rows.get(0).get("a"));
        assertEquals("2", rows.get(0).get("b"));
        assertFalse(rows.get(0).containsKey("c")); // extra fields not present
    }

    // =========================================================================
    // 4. Custom Delimiter
    // =========================================================================

    @Test
    void tabDelimitedValues() throws Exception {
        String tsv = "name\tage\nAlice\t30\nBob\t25\n";
        List<Map<String, String>> rows = CsvParser.parseCSVWithDelimiter(tsv, '\t');
        assertEquals(2, rows.size());
        assertEquals("Alice", rows.get(0).get("name"));
        assertEquals("30", rows.get(0).get("age"));
    }

    @Test
    void pipeDelimitedValues() throws Exception {
        String csv = "a|b|c\n1|2|3\n";
        List<Map<String, String>> rows = CsvParser.parseCSVWithDelimiter(csv, '|');
        assertEquals("1", rows.get(0).get("a"));
        assertEquals("2", rows.get(0).get("b"));
        assertEquals("3", rows.get(0).get("c"));
    }

    @Test
    void semicolonDelimitedValues() throws Exception {
        // Common in European CSV files (comma is the decimal separator there)
        String csv = "city;country\nParis;France\nBerlin;Germany\n";
        List<Map<String, String>> rows = CsvParser.parseCSVWithDelimiter(csv, ';');
        assertEquals("Paris", rows.get(0).get("city"));
        assertEquals("France", rows.get(0).get("country"));
    }

    // =========================================================================
    // 5. Newline Variants
    // =========================================================================

    @Test
    void windowsCRLFNewlines() throws Exception {
        // Windows-style \r\n line endings
        String csv = "name,age\r\nAlice,30\r\nBob,25\r\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals(2, rows.size());
        assertEquals("Alice", rows.get(0).get("name"));
        assertEquals("Bob", rows.get(1).get("name"));
    }

    @Test
    void oldMacCRNewlines() throws Exception {
        // Old Mac-style \r line endings (pre-OS X)
        String csv = "name,age\rAlice,30\rBob,25\r";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals(2, rows.size());
        assertEquals("Alice", rows.get(0).get("name"));
    }

    @Test
    void noTrailingNewline() throws Exception {
        // Last row has no trailing newline — still parsed correctly
        String csv = "name,age\nAlice,30\nBob,25";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals(2, rows.size());
        assertEquals("Alice", rows.get(0).get("name"));
        assertEquals("Bob", rows.get(1).get("name"));
    }

    // =========================================================================
    // 6. Edge Cases
    // =========================================================================

    @Test
    void emptyFieldsInMiddleOfRow() throws Exception {
        // Two consecutive delimiters produce an empty field
        String csv = "a,b,c\n1,,3\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals("1", rows.get(0).get("a"));
        assertEquals("", rows.get(0).get("b")); // empty middle field
        assertEquals("3", rows.get(0).get("c"));
    }

    @Test
    void emptyFieldAtStartOfRow() throws Exception {
        String csv = "a,b,c\n,2,3\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals("", rows.get(0).get("a"));
        assertEquals("2", rows.get(0).get("b"));
    }

    @Test
    void emptyFieldAtEndOfRow() throws Exception {
        // Trailing delimiter produces empty last field
        String csv = "a,b,c\n1,2,\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals("1", rows.get(0).get("a"));
        assertEquals("2", rows.get(0).get("b"));
        assertEquals("", rows.get(0).get("c"));
    }

    @Test
    void whitespacePreserved() throws Exception {
        // The parser does NOT trim whitespace — that's application logic
        String csv = "name,value\n Alice , 42 \n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals(" Alice ", rows.get(0).get("name"));
        assertEquals(" 42 ", rows.get(0).get("value"));
    }

    @Test
    void multipleBlankLinesIgnored() throws Exception {
        // Blank lines between data rows are skipped
        String csv = "name,age\n\nAlice,30\n\nBob,25\n\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals(2, rows.size());
        assertEquals("Alice", rows.get(0).get("name"));
        assertEquals("Bob", rows.get(1).get("name"));
    }

    @Test
    void mapKeysMatchHeaderOrder() throws Exception {
        // The returned maps preserve the header column order (LinkedHashMap)
        String csv = "c,b,a\n3,2,1\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        List<String> keys = new java.util.ArrayList<>(rows.get(0).keySet());
        assertEquals(List.of("c", "b", "a"), keys);
    }

    // =========================================================================
    // 7. Error Case
    // =========================================================================

    @Test
    void unclosedQuotedFieldThrows() {
        // A quoted field with no closing '"' is malformed
        String csv = "name,age\n\"Alice,30\n";
        assertThrows(CsvParser.CsvParseException.class,
            () -> CsvParser.parseCSV(csv));
    }

    @Test
    void unclosedQuoteInMiddleOfFileThrows() {
        // Unclosed quote anywhere in the file is an error
        String csv = "a,b\n\"unclosed,1\nok,2\n";
        assertThrows(CsvParser.CsvParseException.class,
            () -> CsvParser.parseCSV(csv));
    }

    // =========================================================================
    // 8. Numeric Values
    // =========================================================================

    @Test
    void numericValuesReturnedAsStrings() throws Exception {
        // All values are strings — the caller does type conversion
        String csv = "x,y,z\n1,3.14,-7\n";
        List<Map<String, String>> rows = CsvParser.parseCSV(csv);
        assertEquals("1", rows.get(0).get("x"));
        assertEquals("3.14", rows.get(0).get("y"));
        assertEquals("-7", rows.get(0).get("z"));
    }
}
