namespace CodingAdventures.TypeCheckerProtocol.Tests

open Xunit
open CodingAdventures.TypeCheckerProtocol.FSharp

type SimpleNode =
    { Kind: string
      mutable Value: string }

type TypedNode =
    { Kind: string
      Value: string
      ResolvedType: string }

type GoodTypeChecker() =
    interface ITypeChecker<SimpleNode, TypedNode> with
        member _.Check ast =
            { TypedAst =
                { Kind = ast.Kind
                  Value = ast.Value
                  ResolvedType = "int" }
              Errors = [] }

type BadTypeChecker() =
    interface ITypeChecker<SimpleNode, TypedNode> with
        member _.Check ast =
            { TypedAst =
                { Kind = ast.Kind
                  Value = ast.Value
                  ResolvedType = "error" }
              Errors = [ { Message = $"Unknown kind: {ast.Kind}"; Line = 1; Column = 1 } ] }

type RuleDrivenTypeChecker() as this =
    inherit GenericTypeChecker<SimpleNode>()

    do
        this.RegisterHook("node", "literal", fun node _ ->
            node.Value <- "checked"
            null)

        this.RegisterHook("node", "broken", fun node _ ->
            this.Error($"bad node: {node.Kind}", node)
            null)

        this.RegisterHook("node", "*", fun _ _ -> NotHandled.Value :> obj)

    override this.Run ast =
        this.Dispatch("node", ast, defaultValue = null) |> ignore

    override _.NodeKind node = Some node.Kind

    override _.Locate _ = 7, 9

module TypeCheckerProtocolTests =
    [<Fact>]
    let ``diagnostics are immutable comparable and hashable`` () =
        let diagnostic = { Message = "Type mismatch"; Line = 3; Column = 7 }

        Assert.Equal({ Message = "Type mismatch"; Line = 3; Column = 7 }, diagnostic)
        Assert.NotEqual({ Message = "Type mismatch"; Line = 4; Column = 7 }, diagnostic)
        Assert.Contains(diagnostic, set [ diagnostic ])
        Assert.Contains("Type mismatch", diagnostic.ToString())

    [<Fact>]
    let ``type check result reports ok state`` () =
        let ok =
            { TypedAst = { Kind = "literal"; Value = ""; ResolvedType = "int" }
              Errors = [] }

        let bad =
            { TypedAst = { Kind = "bad"; Value = ""; ResolvedType = "error" }
              Errors = [ { Message = "bad"; Line = 1; Column = 1 } ] }

        Assert.True ok.Ok
        Assert.Empty ok.Errors
        Assert.False bad.Ok
        Assert.Equal("error", bad.TypedAst.ResolvedType)

    [<Fact>]
    let ``interface accepts different checkers`` () =
        let good = GoodTypeChecker() :> ITypeChecker<SimpleNode, TypedNode>
        let bad = BadTypeChecker() :> ITypeChecker<SimpleNode, TypedNode>

        Assert.True((good.Check { Kind = "literal"; Value = "" }).Ok)
        let result = bad.Check { Kind = "??"; Value = "" }
        Assert.False result.Ok
        Assert.Contains("??", result.Errors.Head.Message)

    [<Fact>]
    let ``generic checker dispatches registered hooks`` () =
        let node = { Kind = "literal"; Value = "before" }

        let result = RuleDrivenTypeChecker().Check node

        Assert.True result.Ok
        Assert.Equal("checked", result.TypedAst.Value)

    [<Fact>]
    let ``generic checker records errors with location`` () =
        let result = RuleDrivenTypeChecker().Check { Kind = "broken"; Value = "" }

        Assert.False result.Ok
        Assert.Equal({ Message = "bad node: broken"; Line = 7; Column = 9 }, result.Errors.Head)

    [<Fact>]
    let ``dispatch falls through to default`` () =
        let node = { Kind = "unknown"; Value = "unchanged" }

        let result = RuleDrivenTypeChecker().Check node

        Assert.True result.Ok
        Assert.Equal("unchanged", result.TypedAst.Value)

    [<Theory>]
    [<InlineData("expr:add", "expr_add")>]
    [<InlineData("  fn decl ", "fn_decl")>]
    [<InlineData(null, "")>]
    let ``normalize kind collapses punctuation`` input expected =
        let normalized =
            if isNull input then
                GenericTypeChecker<SimpleNode>.NormalizeKind None
            else
                GenericTypeChecker<SimpleNode>.NormalizeKind(Some input)

        Assert.Equal(expected, normalized)
