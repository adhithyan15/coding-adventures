using CodingAdventures.DocumentAst;
using CodingAdventures.DocumentAstToHtml;
using Parser = CodingAdventures.CommonmarkParser.CommonmarkParser;
using Renderer = CodingAdventures.DocumentAstToHtml.DocumentAstToHtml;

namespace CodingAdventures.Commonmark;

/// <summary>
/// The convenient front door for the two-stage CommonMark pipeline.  Keeping
/// this type thin is intentional: callers can inspect or transform the typed
/// tree between <see cref="Parse"/> and <see cref="ToHtml"/> without a second
/// parser or renderer implementation hiding inside the facade.
/// </summary>
public static class Commonmark
{
    public const string Version = "0.1.0";

    public const string CommonmarkVersion = "0.31.2";

    /// <summary>Parse CommonMark text into the shared document AST.</summary>
    public static DocumentNode Parse(string markdown) => Parser.Parse(markdown);

    /// <summary>Render a shared document AST to an HTML fragment.</summary>
    public static string ToHtml(DocumentNode document, RenderOptions? options = null)
        => Renderer.ToHtml(document, options);
}
