namespace CodingAdventures.GfmParser.FSharp

open CodingAdventures.CommonmarkParser.FSharp

[<AbstractClass; Sealed>]
type GfmParser =
    static member Parse(markdown: string) = MarkdownParser.Parse(markdown, enableGfm = true)
    static member Version = "0.1.0"
    static member GfmVersion = "0.31.2"
