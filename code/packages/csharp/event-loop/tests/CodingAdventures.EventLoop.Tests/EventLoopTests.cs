using CodingAdventures.EventLoop;

namespace CodingAdventures.EventLoop.Tests;

public sealed class EventLoopTests
{
    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", EventLoopPackage.Version);
    }

    [Fact]
    public void DeliversAllEvents()
    {
        var loop = new EventLoop<int>();
        loop.AddSource(new FixedSource<int>([1, 2, 3], [-1]));
        var received = new List<int>();

        loop.OnEvent(eventItem =>
        {
            if (eventItem == -1)
            {
                return ControlFlow.Exit;
            }

            received.Add(eventItem);
            return ControlFlow.Continue;
        });

        loop.Run();

        Assert.Equal([1, 2, 3], received);
    }

    [Fact]
    public void ExitStopsLoopImmediately()
    {
        var loop = new EventLoop<string>();
        loop.AddSource(new FixedSource<string>(["a", "b", "stop", "c", "d"]));
        var seen = new List<string>();

        loop.OnEvent(eventItem =>
        {
            seen.Add(eventItem);
            return eventItem == "stop" ? ControlFlow.Exit : ControlFlow.Continue;
        });

        loop.Run();

        Assert.Equal(["a", "b", "stop"], seen);
    }

    [Fact]
    public void StopFromHandlerTerminatesLoop()
    {
        var loop = new EventLoop<int>();
        loop.AddSource(new InfiniteSource());
        var count = 0;

        loop.OnEvent(_ =>
        {
            count++;
            if (count >= 5)
            {
                loop.Stop();
            }

            return ControlFlow.Continue;
        });

        loop.Run();

        Assert.Equal(5, count);
    }

    [Fact]
    public void MultipleHandlersAllSeeEvent()
    {
        var loop = new EventLoop<int>();
        loop.AddSource(new FixedSource<int>([99], [-1]));
        int? first = null;
        int? second = null;

        loop.OnEvent(eventItem =>
        {
            if (eventItem == 99)
            {
                first = eventItem;
            }

            return eventItem == -1 ? ControlFlow.Exit : ControlFlow.Continue;
        });
        loop.OnEvent(eventItem =>
        {
            if (eventItem == 99)
            {
                second = eventItem;
            }

            return ControlFlow.Continue;
        });

        loop.Run();

        Assert.Equal(99, first);
        Assert.Equal(99, second);
    }

    [Fact]
    public void MultipleSourcesAreMerged()
    {
        var loop = new EventLoop<string>();
        loop.AddSource(new FixedSource<string>(["from-a"]));
        loop.AddSource(new FixedSource<string>(["from-b"]));
        loop.AddSource(new FixedSource<string>([], ["stop"]));
        var seen = new List<string>();

        loop.OnEvent(eventItem =>
        {
            if (eventItem == "stop")
            {
                return ControlFlow.Exit;
            }

            seen.Add(eventItem);
            return ControlFlow.Continue;
        });

        loop.Run();

        Assert.Equal(2, seen.Count);
        Assert.Contains("from-a", seen);
        Assert.Contains("from-b", seen);
    }

    [Fact]
    public async Task StopWhileIdleTerminatesLoop()
    {
        var loop = new EventLoop<int>();
        var called = false;
        loop.OnEvent(_ =>
        {
            called = true;
            return ControlFlow.Continue;
        });

        var stopTask = Task.Run(() =>
        {
            Thread.Sleep(10);
            loop.Stop();
        });

        loop.Run();
        await stopTask;

        Assert.False(called);
    }

    [Fact]
    public async Task StopHandleTerminatesIdleLoop()
    {
        var loop = new EventLoop<int>();
        var handle = loop.GetStopHandle();
        var stopTask = Task.Run(() =>
        {
            Thread.Sleep(10);
            handle.Stop();
        });

        loop.Run();
        await stopTask;
    }

    [Fact]
    public void ControlFlowValuesAreDistinct()
    {
        Assert.NotEqual(ControlFlow.Continue, ControlFlow.Exit);
    }

    [Fact]
    public void AddSourceAndOnEventRejectNull()
    {
        var loop = new EventLoop<int>();

        Assert.Throws<ArgumentNullException>(() => loop.AddSource(null!));
        Assert.Throws<ArgumentNullException>(() => loop.OnEvent(null!));
    }

    [Fact]
    public async Task NoEventsMeansHandlersAreNotCalled()
    {
        var loop = new EventLoop<int>();
        loop.AddSource(new FixedSource<int>());
        var count = 0;
        loop.OnEvent(_ =>
        {
            count++;
            return ControlFlow.Continue;
        });

        var stopTask = Task.Run(() =>
        {
            Thread.Sleep(10);
            loop.Stop();
        });

        loop.Run();
        await stopTask;

        Assert.Equal(0, count);
    }

    [Fact]
    public void HandlerSeesEventsInOrder()
    {
        var loop = new EventLoop<int>();
        loop.AddSource(new FixedSource<int>([3, 1, 4, 1, 5], [-1]));
        var received = new List<int>();

        loop.OnEvent(eventItem =>
        {
            if (eventItem == -1)
            {
                return ControlFlow.Exit;
            }

            received.Add(eventItem);
            return ControlFlow.Continue;
        });

        loop.Run();

        Assert.Equal([3, 1, 4, 1, 5], received);
    }

    private sealed class FixedSource<TEvent>(params IReadOnlyList<TEvent>[] batches) : IEventSource<TEvent>
    {
        private int _index;

        public IReadOnlyList<TEvent> Poll()
        {
            if (_index >= batches.Length)
            {
                return [];
            }

            return batches[_index++];
        }
    }

    private sealed class InfiniteSource : IEventSource<int>
    {
        private int _next;

        public IReadOnlyList<int> Poll()
        {
            _next++;
            return [_next];
        }
    }
}
