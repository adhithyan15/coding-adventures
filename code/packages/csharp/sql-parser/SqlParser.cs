namespace CodingAdventures.SqlParser;

/// <summary>
/// SQL parser - parses SQL source text using the grammar-driven parser infrastructure.
/// </summary>
public sealed class SqlParser
{
    /// <summary>Returns the package identifier used by the parity placeholder packages.</summary>
    public string Ping() => "sql-parser";
}