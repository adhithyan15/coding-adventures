# CodingAdventures.DartmouthBasicParser

A pure C# parser for the original 1964 Dartmouth BASIC language. It tokenizes
source with `DartmouthBasicLexer`, loads the shared `dartmouth_basic.grammar`
from an embedded resource, and produces the generic grammar-parser `ASTNode`.

```csharp
var ast = DartmouthBasicParser.ParseDartmouthBasic(
    "10 LET X = 5\n20 PRINT X\n30 END\n");
Console.WriteLine(ast.RuleName); // program
```

`CreateDartmouthBasicParser` provides the two-step API, while `ParseTokens`
accepts an existing lexer token stream. Parsing requires every non-EOF token to
be consumed, so incomplete or malformed statements are rejected rather than
treated as a shorter program. No filesystem or network access is required at
runtime.
