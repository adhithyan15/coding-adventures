namespace CodingAdventures.SqlLexer.Tests

open CodingAdventures.SqlLexer.FSharp
open Xunit

module SqlLexerTests =
    [<Fact>]
    let pingReturnsPackageName () =
        Assert.Equal("sql-lexer", SqlLexer().Ping())