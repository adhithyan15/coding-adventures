namespace CodingAdventures.GfmParser.FSharp.Tests

open CodingAdventures.GfmParser.FSharp
open CodingAdventures.DocumentAst.FSharp
open Xunit

module GfmParserTests =
    [<Fact>]
    let ``parse parses task list items`` () =
        let doc = GfmParser.Parse "- [x] done\n- [ ] todo"

        match doc.Children with
        | [ ListNode (false, None, _, children) ] ->
            Assert.All(
                children,
                fun child ->
                    match child with
                    | TaskItemNode _ -> ()
                    | _ -> Assert.Fail("Expected task item"))
        | _ -> Assert.Fail("Expected a single task list")

    [<Fact>]
    let ``parse parses tables with alignment`` () =
        let doc = GfmParser.Parse "| Left | Right |\n| :--- | ---: |\n| A | B |"

        match doc.Children with
        | [ TableNode (align, rows) ] ->
            Assert.Equal(Some Left, List.item 0 align)
            Assert.Equal(Some Right, List.item 1 align)
            Assert.True((List.item 0 rows).IsHeader)
            Assert.False((List.item 1 rows).IsHeader)
        | _ -> Assert.Fail("Expected table")

    [<Fact>]
    let ``parse parses strikethrough`` () =
        let doc = GfmParser.Parse "~~gone~~"

        match doc.Children with
        | [ ParagraphNode [ StrikethroughNode _ ] ] -> ()
        | _ -> Assert.Fail("Expected paragraph with strikethrough")
