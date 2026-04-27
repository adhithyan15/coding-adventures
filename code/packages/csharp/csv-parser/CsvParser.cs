using System;
using System.Collections.Generic;
using System.Text;

namespace CodingAdventures.CsvParser;

// CsvParser.cs -- CSV is simple until quotes turn commas back into ordinary text
// =============================================================================
//
// CSV looks regular until one column contains:
//
//   "New York, NY"
//
// The comma inside quotes is data, not a separator. That means parsing CSV is
// not just "split on commas" -- the parser must remember whether it is inside
// a quoted field. A small state machine is the cleanest way to do that.

/// <summary>
/// Raised when the input ends while the parser is still inside a quoted field.
/// </summary>
public sealed class UnclosedQuoteError : Exception
{
    /// <summary>
    /// Create the standard unclosed-quote error.
    /// </summary>
    public UnclosedQuoteError()
        : base("unclosed quoted field: EOF reached inside a quoted field")
    {
    }
}

/// <summary>
/// State-machine CSV parser following RFC 4180 style quoting rules.
/// </summary>
public static class CsvParser
{
    private enum ParseState
    {
        FieldStart,
        InUnquotedField,
        InQuotedField,
        InQuotedMaybeEnd,
    }

    /// <summary>
    /// Parse CSV text using a comma as the delimiter.
    /// </summary>
    public static List<Dictionary<string, string>> ParseCsv(string source) =>
        ParseCsvWithDelimiter(source, ',');

    /// <summary>
    /// Parse CSV text using a single-character delimiter string.
    /// </summary>
    public static List<Dictionary<string, string>> ParseCsvWithDelimiter(
        string source,
        string delimiter)
    {
        ArgumentNullException.ThrowIfNull(delimiter);

        if (delimiter.Length != 1)
        {
            throw new ArgumentException("Delimiter must be a single character.", nameof(delimiter));
        }

        return ParseCsvWithDelimiter(source, delimiter[0]);
    }

    /// <summary>
    /// Parse CSV text using a custom delimiter.
    /// </summary>
    public static List<Dictionary<string, string>> ParseCsvWithDelimiter(
        string source,
        char delimiter)
    {
        ArgumentNullException.ThrowIfNull(source);

        if (delimiter == '"')
        {
            throw new ArgumentException("Delimiter cannot be a double quote.", nameof(delimiter));
        }

        var rawRows = TokenizeRows(source, delimiter);
        if (rawRows.Count == 0)
        {
            return [];
        }

        var header = rawRows[0];
        if (rawRows.Count == 1)
        {
            return [];
        }

        var rows = new List<Dictionary<string, string>>(rawRows.Count - 1);
        for (var rowIndex = 1; rowIndex < rawRows.Count; rowIndex++)
        {
            rows.Add(BuildRowMap(header, rawRows[rowIndex]));
        }

        return rows;
    }

    private static List<List<string>> TokenizeRows(string source, char delimiter)
    {
        var rows = new List<List<string>>();
        var currentRow = new List<string>();
        var fieldBuffer = new StringBuilder();
        var state = ParseState.FieldStart;
        var index = 0;

        while (index < source.Length)
        {
            var ch = source[index];

            switch (state)
            {
                case ParseState.FieldStart:
                    if (ch == '"')
                    {
                        state = ParseState.InQuotedField;
                        index++;
                    }
                    else if (ch == delimiter)
                    {
                        currentRow.Add(string.Empty);
                        index++;
                    }
                    else if (IsNewlineStart(ch))
                    {
                        if (currentRow.Count > 0)
                        {
                            currentRow.Add(string.Empty);
                        }

                        rows.Add(currentRow);
                        currentRow = [];
                        index = ConsumeNewline(source, index);
                    }
                    else
                    {
                        fieldBuffer.Append(ch);
                        state = ParseState.InUnquotedField;
                        index++;
                    }

                    break;

                case ParseState.InUnquotedField:
                    if (ch == delimiter)
                    {
                        currentRow.Add(fieldBuffer.ToString());
                        fieldBuffer.Clear();
                        state = ParseState.FieldStart;
                        index++;
                    }
                    else if (IsNewlineStart(ch))
                    {
                        currentRow.Add(fieldBuffer.ToString());
                        fieldBuffer.Clear();
                        rows.Add(currentRow);
                        currentRow = [];
                        state = ParseState.FieldStart;
                        index = ConsumeNewline(source, index);
                    }
                    else
                    {
                        fieldBuffer.Append(ch);
                        index++;
                    }

                    break;

                case ParseState.InQuotedField:
                    if (ch == '"')
                    {
                        state = ParseState.InQuotedMaybeEnd;
                    }
                    else
                    {
                        fieldBuffer.Append(ch);
                    }

                    index++;
                    break;

                case ParseState.InQuotedMaybeEnd:
                    if (ch == '"')
                    {
                        fieldBuffer.Append('"');
                        state = ParseState.InQuotedField;
                        index++;
                    }
                    else if (ch == delimiter)
                    {
                        currentRow.Add(fieldBuffer.ToString());
                        fieldBuffer.Clear();
                        state = ParseState.FieldStart;
                        index++;
                    }
                    else if (IsNewlineStart(ch))
                    {
                        currentRow.Add(fieldBuffer.ToString());
                        fieldBuffer.Clear();
                        rows.Add(currentRow);
                        currentRow = [];
                        state = ParseState.FieldStart;
                        index = ConsumeNewline(source, index);
                    }
                    else
                    {
                        // Lenient mode: treat unexpected text after a closing
                        // quote as plain characters in the same field.
                        fieldBuffer.Append(ch);
                        state = ParseState.InUnquotedField;
                        index++;
                    }

                    break;
            }
        }

        switch (state)
        {
            case ParseState.FieldStart:
                if (currentRow.Count > 0)
                {
                    currentRow.Add(string.Empty);
                    rows.Add(currentRow);
                }

                break;

            case ParseState.InUnquotedField:
                currentRow.Add(fieldBuffer.ToString());
                rows.Add(currentRow);
                break;

            case ParseState.InQuotedField:
                throw new UnclosedQuoteError();

            case ParseState.InQuotedMaybeEnd:
                currentRow.Add(fieldBuffer.ToString());
                rows.Add(currentRow);
                break;
        }

        return rows;
    }

    private static Dictionary<string, string> BuildRowMap(
        IReadOnlyList<string> header,
        IReadOnlyList<string> row)
    {
        var map = new Dictionary<string, string>(header.Count, StringComparer.Ordinal);

        for (var index = 0; index < header.Count; index++)
        {
            map[header[index]] = index < row.Count ? row[index] : string.Empty;
        }

        return map;
    }

    private static bool IsNewlineStart(char ch) => ch is '\n' or '\r';

    private static int ConsumeNewline(string source, int index)
    {
        if (source[index] == '\r' && index + 1 < source.Length && source[index + 1] == '\n')
        {
            return index + 2;
        }

        return index + 1;
    }
}
