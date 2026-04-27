namespace CodingAdventures.StarlarkParser;

/// <summary>
/// Starlark parser - parses Starlark source text using the grammar-driven parser infrastructure.
/// </summary>
public sealed class StarlarkParser
{
    /// <summary>Returns the package identifier used by the parity placeholder packages.</summary>
    public string Ping() => "starlark-parser";
}