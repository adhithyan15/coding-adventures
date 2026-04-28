namespace CodingAdventures.TypeCheckerProtocol.Tests;

public sealed record SimpleNode(string Kind, string Value = "");

public sealed record TypedNode(string Kind, string Value = "", string ResolvedType = "unknown");

public sealed class GoodTypeChecker : ITypeChecker<SimpleNode, TypedNode>
{
    public TypeCheckResult<TypedNode> Check(SimpleNode ast) =>
        new(new TypedNode(ast.Kind, ast.Value, "int"));
}

public sealed class BadTypeChecker : ITypeChecker<SimpleNode, TypedNode>
{
    public TypeCheckResult<TypedNode> Check(SimpleNode ast) =>
        new(
            new TypedNode(ast.Kind, ast.Value, "error"),
            [new TypeErrorDiagnostic($"Unknown kind: {ast.Kind}", 1, 1)]);
}

public sealed class RuleDrivenTypeChecker : GenericTypeChecker<SimpleNode>
{
    public RuleDrivenTypeChecker()
    {
        RegisterHook("node", "literal", (node, _) => new TypedNode(node.Kind, "checked", "int"));
        RegisterHook("node", "broken", (node, _) =>
        {
            Error($"bad node: {node.Kind}", node);
            return null;
        });
        RegisterHook("node", "*", (_, _) => NotHandled.Value);
    }

    protected override void Run(SimpleNode ast) => Dispatch("node", ast);

    protected override string? NodeKind(SimpleNode node) => node.Kind;

    protected override (int Line, int Column) Locate(object subject) => (7, 9);
}

public sealed class TypeCheckerProtocolTests
{
    [Fact]
    public void DiagnosticsAreImmutableComparableAndHashable()
    {
        var diagnostic = new TypeErrorDiagnostic("Type mismatch", 3, 7);

        Assert.Equal(new TypeErrorDiagnostic("Type mismatch", 3, 7), diagnostic);
        Assert.NotEqual(new TypeErrorDiagnostic("Type mismatch", 4, 7), diagnostic);
        Assert.Contains(diagnostic, new HashSet<TypeErrorDiagnostic> { diagnostic });
        Assert.Contains("Type mismatch", diagnostic.ToString());
    }

    [Fact]
    public void TypeCheckResultReportsOkState()
    {
        var ok = new TypeCheckResult<TypedNode>(new TypedNode("literal"));
        var bad = new TypeCheckResult<TypedNode>(
            new TypedNode("bad", ResolvedType: "error"),
            [new TypeErrorDiagnostic("bad", 1, 1)]);

        Assert.True(ok.Ok);
        Assert.Empty(ok.Errors);
        Assert.False(bad.Ok);
        Assert.Equal("error", bad.TypedAst.ResolvedType);
    }

    [Fact]
    public void InterfaceAcceptsDifferentCheckers()
    {
        ITypeChecker<SimpleNode, TypedNode> good = new GoodTypeChecker();
        ITypeChecker<SimpleNode, TypedNode> bad = new BadTypeChecker();

        Assert.True(good.Check(new SimpleNode("literal")).Ok);
        var result = bad.Check(new SimpleNode("??"));
        Assert.False(result.Ok);
        Assert.Contains("??", result.Errors[0].Message);
    }

    [Fact]
    public void GenericCheckerDispatchesRegisteredHooks()
    {
        var checker = new RuleDrivenTypeChecker();

        var result = checker.Check(new SimpleNode("literal", "before"));

        Assert.True(result.Ok);
        Assert.Equal("literal", result.TypedAst.Kind);
    }

    [Fact]
    public void GenericCheckerRecordsErrorsWithLocation()
    {
        var result = new RuleDrivenTypeChecker().Check(new SimpleNode("broken"));

        Assert.False(result.Ok);
        Assert.Equal(new TypeErrorDiagnostic("bad node: broken", 7, 9), result.Errors[0]);
    }

    [Fact]
    public void DispatchFallsThroughToDefault()
    {
        var checker = new RuleDrivenTypeChecker();

        var result = checker.Check(new SimpleNode("unknown", "unchanged"));

        Assert.True(result.Ok);
        Assert.Equal("unchanged", result.TypedAst.Value);
    }

    [Theory]
    [InlineData("expr:add", "expr_add")]
    [InlineData("  fn decl ", "fn_decl")]
    [InlineData(null, "")]
    public void NormalizeKindCollapsesPunctuation(string? input, string expected)
    {
        Assert.Equal(expected, GenericTypeChecker<SimpleNode>.NormalizeKind(input));
    }
}
