namespace CodingAdventures.TomlLexer;

/// <summary>
/// TOML lexer - tokenizes TOML source text using the grammar-driven lexer infrastructure.
/// </summary>
public sealed class TomlLexer
{
    /// <summary>Returns the package identifier used by the parity placeholder packages.</summary>
    public string Ping() => "toml-lexer";
}