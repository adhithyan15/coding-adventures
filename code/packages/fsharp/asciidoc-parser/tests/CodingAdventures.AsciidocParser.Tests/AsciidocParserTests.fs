namespace CodingAdventures.AsciidocParser.FSharp.Tests

open System
open Xunit
open CodingAdventures.AsciidocParser
open CodingAdventures.DocumentAst.FSharp

module AsciidocParserTests =
    let private unexpected value = failwithf "unexpected AST node: %A" value

    [<Fact>]
    let ``parse rejects null`` () =
        Assert.Throws<ArgumentNullException>(fun () -> AsciidocParser.parse Unchecked.defaultof<string> |> ignore)
        |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> AsciidocParser.parseInline Unchecked.defaultof<string> |> ignore)
        |> ignore

    [<Fact>]
    let ``empty blank and comment-only sources produce empty documents`` () =
        Assert.Empty((AsciidocParser.parse String.Empty).Children)
        Assert.Empty((AsciidocParser.parse "  \n\t\n").Children)
        Assert.Empty((AsciidocParser.parse "// hidden\n// also hidden").Children)

    [<Fact>]
    let ``headings clamp levels and parse inline markup`` () =
        match (AsciidocParser.parse "= One\n== Two\n======= *Deep*").Children with
        | [ HeadingNode(1, _); HeadingNode(2, _); HeadingNode(6, [ StrongNode _ ]) ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``paragraphs preserve soft breaks and blank-line boundaries`` () =
        match (AsciidocParser.parse "first line\nsecond line\n\nnext").Children with
        | [ ParagraphNode [ TextNode "first line"; SoftBreakNode; TextNode "second line" ]
            ParagraphNode [ TextNode "next" ] ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``thematic breaks and headings interrupt paragraphs`` () =
        match (AsciidocParser.parse "paragraph\n'''\n== Heading").Children with
        | [ ParagraphNode _; ThematicBreakNode; HeadingNode _ ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``source and literal blocks preserve content`` () =
        match (AsciidocParser.parse "[source, fsharp]\n----\nlet x = 1\n----\n....\nliteral\n....").Children with
        | [ CodeBlockNode(Some "fsharp", "let x = 1\n"); CodeBlockNode(None, "literal\n") ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``unterminated code blocks are emitted leniently`` () =
        match (AsciidocParser.parse "----\nopen\n").Children with
        | [ CodeBlockNode(None, "open\n") ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``passthrough and quote blocks produce raw and recursive nodes`` () =
        match (AsciidocParser.parse "++++\n<b>raw</b>\n++++\n____\n= Nested\n\ntext\n____").Children with
        | [ RawBlockNode("html", "<b>raw</b>"); BlockquoteNode [ HeadingNode _; ParagraphNode _ ] ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``ordered unordered and nested lists are built`` () =
        match (AsciidocParser.parse "* one\n** nested\n* two\n\n. first\n. second").Children with
        | [ ListNode(false, None, true, [ ListItemNode [ ParagraphNode _; ListNode _ ]; ListItemNode _ ])
            ListNode(true, Some 1, true, [ ListItemNode _; ListItemNode _ ]) ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``lists terminate when another block starts`` () =
        match (AsciidocParser.parse "* item\nparagraph\n\n. ordered\n== Heading").Children with
        | [ ListNode _; ParagraphNode _; ListNode _; HeadingNode _ ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``inline parser handles strong emphasis code and literal delimiters`` () =
        let nodes = AsciidocParser.parseInline "a *bold* **wide** _ital_ __free__ `*code*` and *open"
        let strongCount = nodes |> List.filter (function StrongNode _ -> true | _ -> false) |> List.length
        let emphasisCount = nodes |> List.filter (function EmphasisNode _ -> true | _ -> false) |> List.length
        Assert.Equal(2, strongCount)
        Assert.Equal(2, emphasisCount)
        Assert.Contains(CodeSpanNode "*code*", nodes)
        match List.last nodes with
        | TextNode value -> Assert.EndsWith("*open", value)
        | node -> unexpected node

    [<Fact>]
    let ``inline parser handles links images cross-references and URLs`` () =
        let nodes =
            AsciidocParser.parseInline
                "link:https://a.test[A] image:cat.png[Cat] <<intro,Intro>> <<plain>> https://b.test[B] http://c.test"
        let links = nodes |> List.choose (function LinkNode(destination, _, _) -> Some destination | _ -> None)
        Assert.Equal<string list>([ "https://a.test"; "#intro"; "#plain"; "https://b.test" ], links)
        Assert.Contains(ImageNode("cat.png", None, "Cat"), nodes)
        Assert.Contains(AutolinkNode("http://c.test", false), nodes)

    [<Fact>]
    let ``empty link labels use their destinations`` () =
        match AsciidocParser.parseInline "link:https://a.test[]", AsciidocParser.parseInline "https://b.test[]" with
        | [ LinkNode("https://a.test", None, [ TextNode "https://a.test" ]) ],
          [ LinkNode("https://b.test", None, [ TextNode "https://b.test" ]) ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``inline parser classifies hard and soft breaks`` () =
        let nodes = AsciidocParser.parseInline "a  \nb\\\nc\nd"
        let hardCount = nodes |> List.filter ((=) HardBreakNode) |> List.length
        let softCount = nodes |> List.filter ((=) SoftBreakNode) |> List.length
        Assert.Equal(2, hardCount)
        Assert.Equal(1, softCount)

    [<Fact>]
    let ``inline markup can nest recursively`` () =
        match AsciidocParser.parseInline "*bold _and italic_*" with
        | [ StrongNode [ TextNode "bold "; EmphasisNode [ TextNode "and italic" ] ] ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``CRLF and bare carriage returns are normalized`` () =
        match (AsciidocParser.parse "first\r\nsecond\r\rthird").Children with
        | [ ParagraphNode [ TextNode "first"; SoftBreakNode; TextNode "second" ]; ParagraphNode [ TextNode "third" ] ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``empty delimited blocks produce empty values`` () =
        match (AsciidocParser.parse "----\n----\n....\n....\n++++\n++++").Children with
        | [ CodeBlockNode(None, ""); CodeBlockNode(None, ""); RawBlockNode("html", "") ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``passthrough blocks do not add trailing newlines`` () =
        match (AsciidocParser.parse "++++\nline one\nline two\n++++").Children with
        | [ RawBlockNode("html", "line one\nline two") ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``unterminated passthrough blocks are emitted leniently`` () =
        match (AsciidocParser.parse "++++\nraw").Children with
        | [ RawBlockNode("html", "raw") ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``unterminated quote blocks are parsed recursively`` () =
        match (AsciidocParser.parse "____\n== Inner").Children with
        | [ BlockquoteNode [ HeadingNode _ ] ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``lists interrupt paragraphs without blank lines`` () =
        match (AsciidocParser.parse "paragraph\n* item\n. ordered").Children with
        | [ ParagraphNode _; ListNode(false, _, _, _); ListNode(true, _, _, _) ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``comments and source attributes interrupt paragraphs`` () =
        match (AsciidocParser.parse "paragraph\n// hidden\n[source, go]\n----\ncode\n----").Children with
        | [ ParagraphNode _; CodeBlockNode(Some "go", "code\n") ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``switching list kinds creates separate lists`` () =
        match (AsciidocParser.parse "* unordered\n. ordered\n* unordered again").Children with
        | [ ListNode(false, _, _, _); ListNode(true, _, _, _); ListNode(false, _, _, _) ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``lists support multiple nested levels`` () =
        match (AsciidocParser.parse "* one\n** two\n*** three").Children with
        | [ ListNode(false, _, _, [ ListItemNode [ ParagraphNode _; ListNode(false, _, _, [ ListItemNode [ ParagraphNode _; ListNode _ ] ]) ] ]) ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``source attributes are case-insensitive and trim languages`` () =
        match (AsciidocParser.parse "[SOURCE,  rust ]\n----\nx\n----").Children with
        | [ CodeBlockNode(Some "rust", "x\n") ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``heading and list content trim marker whitespace`` () =
        match (AsciidocParser.parse "==   Heading\n\n*   item  ").Children with
        | [ HeadingNode(2, [ TextNode "Heading" ]); ListNode(false, _, _, [ ListItemNode [ ParagraphNode [ TextNode "item" ] ] ]) ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``long delimiters are accepted`` () =
        match (AsciidocParser.parse "'''''\n-----\nx\n-----").Children with
        | [ ThematicBreakNode; CodeBlockNode(None, "x\n") ] -> ()
        | nodes -> unexpected nodes

    [<Fact>]
    let ``plain inline input coalesces into one text node`` () =
        Assert.Equal<InlineNode list>([ TextNode "just plain text" ], AsciidocParser.parseInline "just plain text")

    [<Theory>]
    [<InlineData("*open")>]
    [<InlineData("_open")>]
    [<InlineData("`open")>]
    [<InlineData("<<open")>]
    let ``unterminated inline constructs remain literal`` input =
        Assert.Equal<InlineNode list>([ TextNode input ], AsciidocParser.parseInline input)

    [<Fact>]
    let ``malformed macros remain literal text`` () =
        Assert.Equal<InlineNode list>([ TextNode "link:target[open" ], AsciidocParser.parseInline "link:target[open")
        Assert.Equal<InlineNode list>([ TextNode "image:cat.png" ], AsciidocParser.parseInline "image:cat.png")
