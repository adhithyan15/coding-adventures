using System;
using System.Collections.Generic;
using Parser = CodingAdventures.CsvParser.CsvParser;

namespace CodingAdventures.CsvParser.Tests;

public sealed class CsvParserTests
{
    [Fact]
    public void ParsesSimpleTablesAndKeepsValuesAsStrings()
    {
        var rows = Parser.ParseCsv("name,age,city\nAlice,30,New York\nBob,25,London\n");

        Assert.Equal(2, rows.Count);
        AssertRow(rows[0], ("name", "Alice"), ("age", "30"), ("city", "New York"));
        AssertRow(rows[1], ("name", "Bob"), ("age", "25"), ("city", "London"));
    }

    [Fact]
    public void HandlesEmptyInputHeaderOnlyFilesAndMissingTrailingNewlines()
    {
        Assert.Empty(Parser.ParseCsv(string.Empty));
        Assert.Empty(Parser.ParseCsv("name,age\n"));

        var rows = Parser.ParseCsv("name,value\nhello,world");
        Assert.Single(rows);
        AssertRow(rows[0], ("name", "hello"), ("value", "world"));
    }

    [Fact]
    public void QuotedFieldsHandleCommasEmbeddedNewlinesAndEscapedQuotes()
    {
        var commaRows = Parser.ParseCsv("product,description\nWidget,\"A small, round widget\"\n");
        Assert.Single(commaRows);
        AssertRow(commaRows[0], ("product", "Widget"), ("description", "A small, round widget"));

        var newlineRows = Parser.ParseCsv("note,text\n1,\"Line one\nLine two\"\n");
        Assert.Single(newlineRows);
        AssertRow(newlineRows[0], ("note", "1"), ("text", "Line one\nLine two"));

        var escapedQuoteRows = Parser.ParseCsv("quote,value\n2,\"She said \"\"hello\"\"\"\n");
        Assert.Single(escapedQuoteRows);
        AssertRow(escapedQuoteRows[0], ("quote", "2"), ("value", "She said \"hello\""));
    }

    [Fact]
    public void EmptyFieldsAndBlankLinesBecomeEmptyStrings()
    {
        var rows = Parser.ParseCsv("a,b,c\n,2,\n,,\n");
        Assert.Equal(2, rows.Count);
        AssertRow(rows[0], ("a", string.Empty), ("b", "2"), ("c", string.Empty));
        AssertRow(rows[1], ("a", string.Empty), ("b", string.Empty), ("c", string.Empty));

        var blankLineRows = Parser.ParseCsv("a\n\n");
        Assert.Single(blankLineRows);
        AssertRow(blankLineRows[0], ("a", string.Empty));
    }

    [Fact]
    public void RaggedRowsArePaddedAndTruncatedToHeaderLength()
    {
        var rows = Parser.ParseCsv(
            "a,b,c\n1\n2,two\n3,three,THREE\n4,four,FOUR,extra\n");

        Assert.Equal(4, rows.Count);
        AssertRow(rows[0], ("a", "1"), ("b", string.Empty), ("c", string.Empty));
        AssertRow(rows[1], ("a", "2"), ("b", "two"), ("c", string.Empty));
        AssertRow(rows[2], ("a", "3"), ("b", "three"), ("c", "THREE"));
        AssertRow(rows[3], ("a", "4"), ("b", "four"), ("c", "FOUR"));
    }

    [Fact]
    public void SupportsLfCrLfAndCrLineEndings()
    {
        var lf = Parser.ParseCsv("a,b\n1,2\n");
        var crlf = Parser.ParseCsv("a,b\r\n1,2\r\n");
        var cr = Parser.ParseCsv("a,b\r1,2\r");

        AssertRow(lf[0], ("a", "1"), ("b", "2"));
        AssertRow(crlf[0], ("a", "1"), ("b", "2"));
        AssertRow(cr[0], ("a", "1"), ("b", "2"));
    }

    [Fact]
    public void SupportsCustomDelimiters()
    {
        var rows = Parser.ParseCsvWithDelimiter("name\tage\nAlice\t30\n", '\t');
        Assert.Single(rows);
        AssertRow(rows[0], ("name", "Alice"), ("age", "30"));

        var pipeRows = Parser.ParseCsvWithDelimiter("name|age\nBob|25\n", "|");
        Assert.Single(pipeRows);
        AssertRow(pipeRows[0], ("name", "Bob"), ("age", "25"));
    }

    [Fact]
    public void PreservesWhitespaceOutsideQuotes()
    {
        var rows = Parser.ParseCsv("a,b\n  spaced  ,\"trim? no\"\n");
        Assert.Single(rows);
        AssertRow(rows[0], ("a", "  spaced  "), ("b", "trim? no"));
    }

    [Fact]
    public void ThrowsWhenQuotedFieldIsNotClosed()
    {
        Assert.Throws<UnclosedQuoteError>(() => Parser.ParseCsv("a,b\n1,\"oops"));
    }

    [Fact]
    public void ValidatesDelimiterInputs()
    {
        Assert.Throws<ArgumentException>(() => Parser.ParseCsvWithDelimiter("a,b\n", "\""));
        Assert.Throws<ArgumentException>(() => Parser.ParseCsvWithDelimiter("a,b\n", "||"));
    }

    private static void AssertRow(
        IReadOnlyDictionary<string, string> row,
        params (string Key, string Value)[] expected)
    {
        Assert.Equal(expected.Length, row.Count);

        foreach (var (key, value) in expected)
        {
            Assert.Equal(value, row[key]);
        }
    }
}
