using CodingAdventures.CommonmarkParser;
using CodingAdventures.DocumentAst;

namespace CodingAdventures.GfmParser;

public static class GfmParser
{
    public const string VERSION = "0.1.0";
    public const string GFM_VERSION = "0.31.2";

    public static DocumentNode Parse(string markdown) => MarkdownParser.Parse(markdown, enableGfm: true);
}
