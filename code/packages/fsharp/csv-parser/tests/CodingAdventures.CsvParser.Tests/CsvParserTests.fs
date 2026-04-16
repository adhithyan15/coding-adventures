namespace CodingAdventures.CsvParser.Tests

open System
open System.Collections.Generic
open Xunit
open CodingAdventures.CsvParser

type CsvParserTests() =
    let assertRow (row: IReadOnlyDictionary<string, string>) (expected: (string * string) array) =
        Assert.Equal(expected.Length, row.Count)

        for key, value in expected do
            Assert.Equal(value, row[key])

    [<Fact>]
    member _.``parses simple tables and keeps values as strings``() =
        let rows = CsvParser.parseCsv "name,age,city\nAlice,30,New York\nBob,25,London\n"

        Assert.Equal(2, rows.Length)
        assertRow rows[0] [| ("name", "Alice"); ("age", "30"); ("city", "New York") |]
        assertRow rows[1] [| ("name", "Bob"); ("age", "25"); ("city", "London") |]

    [<Fact>]
    member _.``handles empty input header-only files and missing trailing newlines``() =
        Assert.Empty(CsvParser.parseCsv String.Empty)
        Assert.Empty(CsvParser.parseCsv "name,age\n")

        let rows = CsvParser.parseCsv "name,value\nhello,world"
        Assert.Single rows |> ignore
        assertRow rows[0] [| ("name", "hello"); ("value", "world") |]

    [<Fact>]
    member _.``quoted fields handle commas embedded newlines and escaped quotes``() =
        let commaRows = CsvParser.parseCsv "product,description\nWidget,\"A small, round widget\"\n"
        Assert.Single commaRows |> ignore
        assertRow commaRows[0] [| ("product", "Widget"); ("description", "A small, round widget") |]

        let newlineRows = CsvParser.parseCsv "note,text\n1,\"Line one\nLine two\"\n"
        Assert.Single newlineRows |> ignore
        assertRow newlineRows[0] [| ("note", "1"); ("text", "Line one\nLine two") |]

        let escapedQuoteRows = CsvParser.parseCsv "quote,value\n2,\"She said \"\"hello\"\"\"\n"
        Assert.Single escapedQuoteRows |> ignore
        assertRow escapedQuoteRows[0] [| ("quote", "2"); ("value", "She said \"hello\"") |]

    [<Fact>]
    member _.``empty fields and blank lines become empty strings``() =
        let rows = CsvParser.parseCsv "a,b,c\n,2,\n,,\n"
        Assert.Equal(2, rows.Length)
        assertRow rows[0] [| ("a", String.Empty); ("b", "2"); ("c", String.Empty) |]
        assertRow rows[1] [| ("a", String.Empty); ("b", String.Empty); ("c", String.Empty) |]

        let blankLineRows = CsvParser.parseCsv "a\n\n"
        Assert.Single blankLineRows |> ignore
        assertRow blankLineRows[0] [| ("a", String.Empty) |]

    [<Fact>]
    member _.``ragged rows are padded and truncated to header length``() =
        let rows =
            CsvParser.parseCsv "a,b,c\n1\n2,two\n3,three,THREE\n4,four,FOUR,extra\n"

        Assert.Equal(4, rows.Length)
        assertRow rows[0] [| ("a", "1"); ("b", String.Empty); ("c", String.Empty) |]
        assertRow rows[1] [| ("a", "2"); ("b", "two"); ("c", String.Empty) |]
        assertRow rows[2] [| ("a", "3"); ("b", "three"); ("c", "THREE") |]
        assertRow rows[3] [| ("a", "4"); ("b", "four"); ("c", "FOUR") |]

    [<Fact>]
    member _.``supports lf crlf and cr line endings``() =
        let lf = CsvParser.parseCsv "a,b\n1,2\n"
        let crlf = CsvParser.parseCsv "a,b\r\n1,2\r\n"
        let cr = CsvParser.parseCsv "a,b\r1,2\r"

        assertRow lf[0] [| ("a", "1"); ("b", "2") |]
        assertRow crlf[0] [| ("a", "1"); ("b", "2") |]
        assertRow cr[0] [| ("a", "1"); ("b", "2") |]

    [<Fact>]
    member _.``supports custom delimiters``() =
        let rows = CsvParser.parseCsvWithDelimiter "name\tage\nAlice\t30\n" '\t'
        Assert.Single rows |> ignore
        assertRow rows[0] [| ("name", "Alice"); ("age", "30") |]

        let pipeRows = CsvParser.parseCsvWithDelimiterString "name|age\nBob|25\n" "|"
        Assert.Single pipeRows |> ignore
        assertRow pipeRows[0] [| ("name", "Bob"); ("age", "25") |]

    [<Fact>]
    member _.``preserves whitespace outside quotes``() =
        let rows = CsvParser.parseCsv "a,b\n  spaced  ,\"trim? no\"\n"
        Assert.Single rows |> ignore
        assertRow rows[0] [| ("a", "  spaced  "); ("b", "trim? no") |]

    [<Fact>]
    member _.``throws when quoted field is not closed``() =
        Assert.Throws<UnclosedQuoteError>(fun () -> CsvParser.parseCsv "a,b\n1,\"oops" |> ignore)
        |> ignore

    [<Fact>]
    member _.``validates delimiter inputs``() =
        Assert.Throws<ArgumentException>(fun () -> CsvParser.parseCsvWithDelimiter "a,b\n" '"' |> ignore)
        |> ignore
        Assert.Throws<ArgumentException>(fun () -> CsvParser.parseCsvWithDelimiterString "a,b\n" "||" |> ignore)
        |> ignore
