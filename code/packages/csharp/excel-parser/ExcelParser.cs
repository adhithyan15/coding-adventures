namespace CodingAdventures.ExcelParser;

/// <summary>
/// Excel parser - parses Excel source text using the grammar-driven parser infrastructure.
/// </summary>
public sealed class ExcelParser
{
    /// <summary>Returns the package identifier used by the parity placeholder packages.</summary>
    public string Ping() => "excel-parser";
}