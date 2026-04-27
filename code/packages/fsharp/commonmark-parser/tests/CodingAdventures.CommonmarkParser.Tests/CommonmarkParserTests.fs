namespace CodingAdventures.CommonmarkParser.FSharp.Tests

open System
open CodingAdventures.CommonmarkParser.FSharp
open CodingAdventures.DocumentAst.FSharp
open Xunit

module CommonmarkParserTests =
    [<Fact>]
    let ``parse returns empty document for empty input`` () =
        let doc = CommonmarkParser.Parse ""
        Assert.Empty(doc.Children)

    [<Fact>]
    let ``parse parses atx and setext headings`` () =
        let doc = CommonmarkParser.Parse "# Hello\n\nWorld\n---\n"

        match doc.Children with
        | [ HeadingNode (1, _); HeadingNode (2, _) ] -> ()
        | _ -> Assert.Fail("Expected two heading nodes")

    [<Fact>]
    let ``parse parses inline formatting links images and entities`` () =
        let doc =
            CommonmarkParser.Parse "A *small* **test** with [link](https://example.com \"Title\") ![cat](cat.png) &amp; `code`."

        match doc.Children with
        | [ ParagraphNode children ] ->
            Assert.Contains(children, function EmphasisNode _ -> true | _ -> false)
            Assert.Contains(children, function StrongNode _ -> true | _ -> false)
            Assert.Contains(children, function LinkNode (destination, _, _) -> destination = "https://example.com" | _ -> false)
            Assert.Contains(children, function ImageNode (_, _, alt) -> alt = "cat" | _ -> false)
            Assert.Contains(children, function CodeSpanNode value -> value = "code" | _ -> false)
            Assert.Contains(children, function TextNode value -> value.Contains("&") | _ -> false)
        | _ -> Assert.Fail("Expected paragraph")

    [<Fact>]
    let ``parse parses autolinks and breaks`` () =
        let doc = CommonmarkParser.Parse "<https://example.com>\nline  \nnext"

        match doc.Children with
        | [ ParagraphNode children ] ->
            Assert.Contains(children, function AutolinkNode _ -> true | _ -> false)
            Assert.Contains(children, function SoftBreakNode -> true | _ -> false)
            Assert.Contains(children, function HardBreakNode -> true | _ -> false)
        | _ -> Assert.Fail("Expected paragraph")

    [<Fact>]
    let ``parse parses lists and blockquotes`` () =
        let doc = CommonmarkParser.Parse "> quoted\n>\n> again\n\n- one\n- two"

        match doc.Children with
        | [ BlockquoteNode quoteChildren; ListNode (false, None, _, children) ] ->
            Assert.Equal(2, List.length quoteChildren)
            Assert.Equal(2, List.length children)
        | _ -> Assert.Fail("Expected blockquote plus list")

    [<Fact>]
    let ``parse parses ordered list fence and thematic break`` () =
        let doc = CommonmarkParser.Parse "1. one\n2. two\n\n---\n\n```fsharp\nprintfn \"hi\"\n```"

        match doc.Children with
        | [ ListNode (true, Some 1, _, _); ThematicBreakNode; CodeBlockNode (Some "fsharp", value) ] ->
            Assert.Contains("printfn", value)
        | _ -> Assert.Fail("Expected ordered list, thematic break, and fenced code block")

    [<Fact>]
    let ``parse parses simple raw html block`` () =
        let doc = CommonmarkParser.Parse "<aside>hello</aside>"

        match doc.Children with
        | [ RawBlockNode ("html", value) ] -> Assert.Equal("<aside>hello</aside>", value)
        | _ -> Assert.Fail("Expected raw html block")

    [<Fact>]
    let ``parse rejects excessive block nesting`` () =
        let deeplyNested = String.replicate 65 "> " + "boom"
        Assert.Throws<InvalidOperationException>(fun () -> CommonmarkParser.Parse deeplyNested |> ignore)

    [<Fact>]
    let ``parse treats oversized ordered marker as paragraph`` () =
        let doc = CommonmarkParser.Parse "999999999999999999999. item"

        match doc.Children with
        | [ ParagraphNode [ TextNode value ] ] -> Assert.Contains("999999999999999999999. item", value)
        | _ -> Assert.Fail("Expected paragraph")

    [<Fact>]
    let ``parse handles delimiter heavy malformed input without failure`` () =
        let malformed = String.replicate 5000 "["
        let doc = CommonmarkParser.Parse malformed

        match doc.Children with
        | [ ParagraphNode _ ] -> ()
        | _ -> Assert.Fail("Expected paragraph")
