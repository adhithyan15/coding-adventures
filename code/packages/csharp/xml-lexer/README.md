# CodingAdventures.XmlLexer.CSharp

A context-sensitive XML tokenizer that emits the shared XML grammar's token
names using the existing C# lexer token model.

## Usage

```csharp
using CodingAdventures.XmlLexer;

var tokens = XmlTokenizer.TokenizeXml("<p>Hello &amp; world</p>");
```

The scanner switches among content, tag, comment, CDATA, and processing
instruction modes. Whitespace is skipped only when it begins a match in the
content or tag modes; whitespace inside comments, CDATA, and processing
instructions is preserved.

The result always ends with an `EOF` token. `CreateXmlLexer` is available when
an explicitly configured lexer object is preferred.

## Testing

```sh
dotnet test tests/CodingAdventures.XmlLexer.Tests/CodingAdventures.XmlLexer.Tests.csproj
```
