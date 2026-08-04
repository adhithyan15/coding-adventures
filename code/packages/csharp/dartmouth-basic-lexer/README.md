# CodingAdventures.DartmouthBasicLexer

A pure C# lexer for the original 1964 Dartmouth BASIC language. It loads the
shared `dartmouth_basic.tokens` grammar from an embedded resource, then applies
the language's two context-sensitive rules: the first number on a physical line
is a `LINE_NUM`, and `REM` discards everything through the end of that line.

```csharp
var tokens = DartmouthBasicLexer.TokenizeDartmouthBasic(
    "10 LET X = 5\n20 PRINT X\n30 END\n");
```

Keywords are normalized to uppercase, names and function identifiers to
lowercase, strings retain their original case without surrounding quotes, and
every stream ends with `EOF`. The grammar is embedded, so tokenization performs
no filesystem or network access.
