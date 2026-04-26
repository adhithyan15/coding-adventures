namespace CodingAdventures.ExcelLexer;

/// <summary>
/// Excel lexer - tokenizes Excel source text using the grammar-driven lexer infrastructure.
/// </summary>
public sealed class ExcelLexer
{
    /// <summary>Returns the package identifier used by the parity placeholder packages.</summary>
    public string Ping() => "excel-lexer";
}