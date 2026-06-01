using Xunit;
using Mosaic.Flux;

namespace Mosaic.Flux.Tests;

public record ActionTestState(int Count);

public sealed record Increment : IMosaicAction<ActionTestState>
{
    public ActionTestState Apply(ActionTestState state) => state with { Count = state.Count + 1 };
}

public sealed record Add(int Amount) : IMosaicAction<ActionTestState>
{
    public ActionTestState Apply(ActionTestState state) => state with { Count = state.Count + Amount };
}

public class ActionTests
{
    [Fact]
    public void ApplyReturnsNextStateWithoutMutatingInput()
    {
        var initial = new ActionTestState(5);
        var next = new Increment().Apply(initial);
        Assert.Equal(6, next.Count);
        Assert.Equal(5, initial.Count);
    }

    [Fact]
    public void PayloadAccessible()
    {
        var action = new Add(7);
        Assert.Equal(7, action.Amount);
        Assert.Equal(10, action.Apply(new ActionTestState(3)).Count);
    }

    [Fact]
    public void Deterministic()
    {
        var state = new ActionTestState(0);
        var action = new Add(5);
        Assert.Equal(action.Apply(state).Count, action.Apply(state).Count);
    }
}
