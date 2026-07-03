using Xunit;
using Mosaic.Flux;

namespace Mosaic.Flux.Tests;

public record MwState(int V = 0);

public sealed record MwBump : IMosaicAction<MwState>
{
    public MwState Apply(MwState state) => state with { V = state.V + 1 };
}

public class MiddlewareTests
{
    [Fact]
    public void EmptyComposeIsNoOp()
    {
        var m = MiddlewareHelpers.Compose<MwState>(Array.Empty<Middleware<MwState>>());
        m(new MwBump(), new MwState(0), new MwState(1));  // should not throw
    }

    [Fact]
    public void SingleMiddlewareReturnedVerbatim()
    {
        Middleware<MwState> single = (_, _, _) => { };
        var composed = MiddlewareHelpers.Compose(new[] { single });
        Assert.Same(single, composed);
    }

    [Fact]
    public void RunsInOrder()
    {
        var calls = new List<string>();
        var composed = MiddlewareHelpers.Compose(new Middleware<MwState>[]
        {
            (_, _, _) => calls.Add("a"),
            (_, _, _) => calls.Add("b"),
            (_, _, _) => calls.Add("c"),
        });
        composed(new MwBump(), new MwState(0), new MwState(1));
        Assert.Equal(new[] { "a", "b", "c" }, calls);
    }

    [Fact]
    public void IsolatesThrows()
    {
        var calls = new List<string>();
        var composed = MiddlewareHelpers.Compose(new Middleware<MwState>[]
        {
            (_, _, _) => calls.Add("a"),
            (_, _, _) => throw new Exception("boom"),
            (_, _, _) => calls.Add("c"),
        });
        composed(new MwBump(), new MwState(0), new MwState(1));
        Assert.Equal(new[] { "a", "c" }, calls);
    }

    [Fact]
    public void LoggerMiddlewareDoesNotThrow()
    {
        var m = MiddlewareHelpers.Logger<MwState>();
        m(new MwBump(), new MwState(0), new MwState(1));
    }
}
