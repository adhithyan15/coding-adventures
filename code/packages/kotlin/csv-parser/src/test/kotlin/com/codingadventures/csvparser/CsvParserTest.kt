// ============================================================================
// CsvParserTest.kt — Unit Tests for CsvParser
// ============================================================================
//
// Test strategy mirrors the Java implementation with idiomatic Kotlin style.
// Tests cover: basic CSV, quoted fields, ragged rows, custom delimiter,
// newline variants, edge cases, error cases, and whitespace handling.
// ============================================================================

package com.codingadventures.csvparser

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class CsvParserTest {

    // =========================================================================
    // 1. Basic CSV
    // =========================================================================

    @Test
    fun singleRowWithHeader() {
        val rows = parseCSV("name,age\nAlice,30\n")
        assertEquals(1, rows.size)
        assertEquals("Alice", rows[0]["name"])
        assertEquals("30", rows[0]["age"])
    }

    @Test
    fun multipleDataRows() {
        val rows = parseCSV("name,age\nAlice,30\nBob,25\nCarol,35\n")
        assertEquals(3, rows.size)
        assertEquals("Alice", rows[0]["name"])
        assertEquals("Bob", rows[1]["name"])
        assertEquals("Carol", rows[2]["name"])
    }

    @Test
    fun headerOnlyReturnsEmptyList() {
        assertTrue(parseCSV("name,age\n").isEmpty())
    }

    @Test
    fun emptyInputReturnsEmptyList() {
        assertTrue(parseCSV("").isEmpty())
    }

    @Test
    fun singleColumnCSV() {
        val rows = parseCSV("name\nAlice\nBob\n")
        assertEquals(2, rows.size)
        assertEquals("Alice", rows[0]["name"])
        assertEquals("Bob", rows[1]["name"])
    }

    // =========================================================================
    // 2. Quoted Fields
    // =========================================================================

    @Test
    fun quotedFieldWithComma() {
        val rows = parseCSV("name,address\nAlice,\"123 Main St, Suite 4\"\n")
        assertEquals("123 Main St, Suite 4", rows[0]["address"])
    }

    @Test
    fun quotedFieldWithNewline() {
        val rows = parseCSV("name,bio\nAlice,\"Line1\nLine2\"\n")
        assertEquals("Line1\nLine2", rows[0]["bio"])
    }

    @Test
    fun escapedDoubleQuote() {
        // "say ""hello""" → say "hello"
        val rows = parseCSV("name,greeting\nAlice,\"say \"\"hello\"\"\"\n")
        assertEquals("say \"hello\"", rows[0]["greeting"])
    }

    @Test
    fun quotedEmptyField() {
        val rows = parseCSV("a,b\n\"\",x\n")
        assertEquals("", rows[0]["a"])
        assertEquals("x", rows[0]["b"])
    }

    @Test
    fun quotedFieldAtEndOfRow() {
        val rows = parseCSV("a,b\nfoo,\"bar baz\"\n")
        assertEquals("bar baz", rows[0]["b"])
    }

    // =========================================================================
    // 3. Ragged Rows
    // =========================================================================

    @Test
    fun shortRowPaddedWithEmptyStrings() {
        val rows = parseCSV("a,b,c\n1,2\n")
        assertEquals("1", rows[0]["a"])
        assertEquals("2", rows[0]["b"])
        assertEquals("", rows[0]["c"]) // padded
    }

    @Test
    fun longRowTruncated() {
        val rows = parseCSV("a,b\n1,2,3,4\n")
        assertEquals("1", rows[0]["a"])
        assertEquals("2", rows[0]["b"])
        assertFalse(rows[0].containsKey("c"))
    }

    // =========================================================================
    // 4. Custom Delimiter
    // =========================================================================

    @Test
    fun tabDelimitedValues() {
        val rows = parseCSVWithDelimiter("name\tage\nAlice\t30\nBob\t25\n", '\t')
        assertEquals(2, rows.size)
        assertEquals("Alice", rows[0]["name"])
        assertEquals("30", rows[0]["age"])
    }

    @Test
    fun pipeDelimitedValues() {
        val rows = parseCSVWithDelimiter("a|b|c\n1|2|3\n", '|')
        assertEquals("1", rows[0]["a"])
        assertEquals("2", rows[0]["b"])
        assertEquals("3", rows[0]["c"])
    }

    @Test
    fun semicolonDelimitedValues() {
        val rows = parseCSVWithDelimiter("city;country\nParis;France\nBerlin;Germany\n", ';')
        assertEquals("Paris", rows[0]["city"])
        assertEquals("France", rows[0]["country"])
    }

    // =========================================================================
    // 5. Newline Variants
    // =========================================================================

    @Test
    fun windowsCRLFNewlines() {
        val rows = parseCSV("name,age\r\nAlice,30\r\nBob,25\r\n")
        assertEquals(2, rows.size)
        assertEquals("Alice", rows[0]["name"])
        assertEquals("Bob", rows[1]["name"])
    }

    @Test
    fun oldMacCRNewlines() {
        val rows = parseCSV("name,age\rAlice,30\rBob,25\r")
        assertEquals(2, rows.size)
        assertEquals("Alice", rows[0]["name"])
    }

    @Test
    fun noTrailingNewline() {
        val rows = parseCSV("name,age\nAlice,30\nBob,25")
        assertEquals(2, rows.size)
        assertEquals("Alice", rows[0]["name"])
        assertEquals("Bob", rows[1]["name"])
    }

    // =========================================================================
    // 6. Edge Cases
    // =========================================================================

    @Test
    fun emptyFieldsInMiddleOfRow() {
        val rows = parseCSV("a,b,c\n1,,3\n")
        assertEquals("1", rows[0]["a"])
        assertEquals("", rows[0]["b"])
        assertEquals("3", rows[0]["c"])
    }

    @Test
    fun emptyFieldAtStartOfRow() {
        val rows = parseCSV("a,b,c\n,2,3\n")
        assertEquals("", rows[0]["a"])
        assertEquals("2", rows[0]["b"])
    }

    @Test
    fun emptyFieldAtEndOfRow() {
        val rows = parseCSV("a,b,c\n1,2,\n")
        assertEquals("1", rows[0]["a"])
        assertEquals("2", rows[0]["b"])
        assertEquals("", rows[0]["c"])
    }

    @Test
    fun whitespacePreserved() {
        val rows = parseCSV("name,value\n Alice , 42 \n")
        assertEquals(" Alice ", rows[0]["name"])
        assertEquals(" 42 ", rows[0]["value"])
    }

    @Test
    fun multipleBlankLinesIgnored() {
        val rows = parseCSV("name,age\n\nAlice,30\n\nBob,25\n\n")
        assertEquals(2, rows.size)
        assertEquals("Alice", rows[0]["name"])
        assertEquals("Bob", rows[1]["name"])
    }

    @Test
    fun mapKeysMatchHeaderOrder() {
        // LinkedHashMap should preserve column insertion order
        val rows = parseCSV("c,b,a\n3,2,1\n")
        val keys = rows[0].keys.toList()
        assertEquals(listOf("c", "b", "a"), keys)
    }

    // =========================================================================
    // 7. Error Case
    // =========================================================================

    @Test
    fun unclosedQuotedFieldThrows() {
        assertThrows<CsvParseException> {
            parseCSV("name,age\n\"Alice,30\n")
        }
    }

    @Test
    fun unclosedQuoteInMiddleOfFileThrows() {
        assertThrows<CsvParseException> {
            parseCSV("a,b\n\"unclosed,1\nok,2\n")
        }
    }

    // =========================================================================
    // 8. Numeric Values
    // =========================================================================

    @Test
    fun numericValuesReturnedAsStrings() {
        val rows = parseCSV("x,y,z\n1,3.14,-7\n")
        assertEquals("1", rows[0]["x"])
        assertEquals("3.14", rows[0]["y"])
        assertEquals("-7", rows[0]["z"])
    }
}
