namespace CodingAdventures.SqlLexer;

/// <summary>
/// SQL lexer - tokenizes SQL source text using the grammar-driven lexer infrastructure.
/// </summary>
public sealed class SqlLexer
{
    /// <summary>Returns the package identifier used by the parity placeholder packages.</summary>
    public string Ping() => "sql-lexer";
}