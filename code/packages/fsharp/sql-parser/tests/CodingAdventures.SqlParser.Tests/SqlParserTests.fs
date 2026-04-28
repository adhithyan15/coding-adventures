namespace CodingAdventures.SqlParser.Tests

open CodingAdventures.SqlParser.FSharp
open Xunit

module SqlParserTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("sql-parser", SqlParser().Ping())