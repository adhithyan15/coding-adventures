using Xunit;
using Mosaic.Flux;

namespace Mosaic.Flux.Tests;

public record SelState(int A = 0, int B = 0, string Label = "");

public class SelectorTests
{
    [Fact]
    public void SingleInputRecomputesOnChange()
    {
        var calls = 0;
        var doubled = Selector.Create<SelState, int, int>(
            s => s.A,
            a => { calls++; return a * 2; });
        Assert.Equal(10, doubled(new SelState(A: 5)));
        Assert.Equal(14, doubled(new SelState(A: 7)));
        Assert.Equal(2, calls);
    }

    [Fact]
    public void SingleInputCachesOnStable()
    {
        var calls = 0;
        var doubled = Selector.Create<SelState, int, int>(
            s => s.A,
            a => { calls++; return a * 2; });
        var s = new SelState(A: 5);
        doubled(s); doubled(s); doubled(s);
        Assert.Equal(1, calls);
    }

    [Fact]
    public void SingleInputCachesAcrossStateRefs()
    {
        var calls = 0;
        var doubled = Selector.Create<SelState, int, int>(
            s => s.A,
            a => { calls++; return a * 2; });
        doubled(new SelState(A: 5, B: 0));
        doubled(new SelState(A: 5, B: 999, Label: "different"));
        Assert.Equal(1, calls);
    }

    [Fact]
    public void TwoInputRecomputesWhenEitherChanges()
    {
        var calls = 0;
        var sum = Selector.Create<SelState, int, int, int>(
            s => s.A,
            s => s.B,
            (a, b) => { calls++; return a + b; });
        Assert.Equal(3, sum(new SelState(A: 1, B: 2)));
        Assert.Equal(6, sum(new SelState(A: 1, B: 5)));
        Assert.Equal(9, sum(new SelState(A: 4, B: 5)));
        Assert.Equal(3, calls);
    }

    [Fact]
    public void TwoInputCachesOnStableInputs()
    {
        var calls = 0;
        var sum = Selector.Create<SelState, int, int, int>(
            s => s.A,
            s => s.B,
            (a, b) => { calls++; return a + b; });
        var s = new SelState(A: 1, B: 2);
        sum(s); sum(s);
        Assert.Equal(1, calls);
    }

    [Fact]
    public void ThreeInputRecomputesWhenAnyChanges()
    {
        var calls = 0;
        var fmt = Selector.Create<SelState, int, int, string, string>(
            s => s.A,
            s => s.B,
            s => s.Label,
            (a, b, lbl) => { calls++; return $"{lbl}:{a + b}"; });
        Assert.Equal("x:3", fmt(new SelState(A: 1, B: 2, Label: "x")));
        Assert.Equal("x:3", fmt(new SelState(A: 1, B: 2, Label: "x")));
        Assert.Equal("y:3", fmt(new SelState(A: 1, B: 2, Label: "y")));
        Assert.Equal(2, calls);
    }
}
