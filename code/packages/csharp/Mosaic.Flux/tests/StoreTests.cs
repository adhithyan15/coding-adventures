using Xunit;
using System.ComponentModel;
using Mosaic.Flux;

namespace Mosaic.Flux.Tests;

public record StoreState(int Count = 0, string Label = "");

public sealed record StIncrement : IMosaicAction<StoreState>
{
    public StoreState Apply(StoreState state) => state with { Count = state.Count + 1 };
}

public sealed record StSetLabel(string Label) : IMosaicAction<StoreState>
{
    public StoreState Apply(StoreState state) => state with { Label = Label };
}

public sealed record StNoOp : IMosaicAction<StoreState>
{
    // Returns the SAME instance to test the no-op short-circuit.
    public StoreState Apply(StoreState state) => state;
}

public class StoreTests
{
    [Fact]
    public void StartsAtInitialState()
    {
        var store = new MosaicStore<StoreState>(new StoreState());
        Assert.Equal(0, store.State.Count);
        Assert.Equal("", store.State.Label);
    }

    [Fact]
    public void DispatchAppliesAction()
    {
        var store = new MosaicStore<StoreState>(new StoreState());
        store.Dispatch(new StIncrement());
        Assert.Equal(1, store.State.Count);
    }

    [Fact]
    public void PayloadedActionWorks()
    {
        var store = new MosaicStore<StoreState>(new StoreState());
        store.Dispatch(new StSetLabel("hi"));
        Assert.Equal("hi", store.State.Label);
    }

    [Fact]
    public void SelectReturnsProjection()
    {
        var store = new MosaicStore<StoreState>(new StoreState(Count: 5));
        Assert.Equal(5, store.Select(s => s.Count));
    }

    [Fact]
    public void SubscribeFiresOnChangedSlice()
    {
        var store = new MosaicStore<StoreState>(new StoreState());
        var received = new List<int>();
        store.Subscribe(s => s.Count, received.Add);
        store.Dispatch(new StIncrement());
        Assert.Equal(new[] { 1 }, received);
    }

    [Fact]
    public void SubscribeDoesNotFireOnUnrelatedChange()
    {
        var store = new MosaicStore<StoreState>(new StoreState());
        var received = new List<int>();
        store.Subscribe(s => s.Count, received.Add);
        store.Dispatch(new StSetLabel("x"));
        Assert.Empty(received);
    }

    [Fact]
    public void UnsubscribeStopsNotifications()
    {
        var store = new MosaicStore<StoreState>(new StoreState());
        var received = new List<int>();
        var sub = store.Subscribe(s => s.Count, received.Add);
        store.Dispatch(new StIncrement());
        sub.Dispose();
        store.Dispatch(new StIncrement());
        Assert.Equal(new[] { 1 }, received);
    }

    [Fact]
    public void NoOpDispatchSkipsSubscriberButRunsMiddleware()
    {
        var subscriberCalls = 0;
        var middlewareCalls = 0;
        var store = new MosaicStore<StoreState>(
            new StoreState(),
            new Middleware<StoreState>[] { (_, _, _) => middlewareCalls++ });
        store.Subscribe(s => s.Count, _ => subscriberCalls++);
        store.Dispatch(new StNoOp());
        Assert.Equal(0, subscriberCalls);
        Assert.Equal(1, middlewareCalls);
    }

    [Fact]
    public void PropertyChangedFiresOnDispatch()
    {
        var store = new MosaicStore<StoreState>(new StoreState());
        var notifications = new List<string?>();
        store.PropertyChanged += (_, e) => notifications.Add(e.PropertyName);
        store.Dispatch(new StIncrement());
        Assert.Contains("State", notifications);
    }

    [Fact]
    public void PropertyChangedDoesNotFireOnNoOp()
    {
        var store = new MosaicStore<StoreState>(new StoreState());
        var notifications = new List<string?>();
        store.PropertyChanged += (_, e) => notifications.Add(e.PropertyName);
        store.Dispatch(new StNoOp());
        Assert.Empty(notifications);
    }

    [Fact]
    public void MiddlewareSeesTriple()
    {
        var seen = new List<(string Type, int Prev, int Next)>();
        var store = new MosaicStore<StoreState>(
            new StoreState(),
            new Middleware<StoreState>[]
            {
                (action, prev, next) => seen.Add((action.GetType().Name, prev.Count, next.Count))
            });
        store.Dispatch(new StIncrement());
        Assert.Single(seen);
        Assert.Equal(0, seen[0].Prev);
        Assert.Equal(1, seen[0].Next);
    }

    [Fact]
    public void CustomEqualityRespected()
    {
        var store = new MosaicStore<StoreState>(new StoreState());
        var received = new List<int>();
        store.Subscribe(s => s.Count, received.Add, (_, _) => true);
        store.Dispatch(new StIncrement());
        Assert.Empty(received);
    }
}
