namespace CodingAdventures.StarlarkLexer;

/// <summary>
/// Starlark lexer - tokenizes Starlark source text using the grammar-driven lexer infrastructure.
/// </summary>
public sealed class StarlarkLexer
{
    /// <summary>Returns the package identifier used by the parity placeholder packages.</summary>
    public string Ping() => "starlark-lexer";
}