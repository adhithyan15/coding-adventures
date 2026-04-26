namespace CodingAdventures.EventLoop;

using System.Threading;

public static class EventLoopPackage
{
    public const string Version = "0.1.0";
}

public enum ControlFlow
{
    Continue,
    Exit,
}

public interface IEventSource<TEvent>
{
    IReadOnlyList<TEvent> Poll();
}

public sealed class StopHandle
{
    private readonly StopState _state;

    internal StopHandle(StopState state)
    {
        _state = state;
    }

    public void Stop()
    {
        _state.Stop();
    }
}

public sealed class EventLoop<TEvent>
{
    private readonly List<IEventSource<TEvent>> _sources = [];
    private readonly List<Func<TEvent, ControlFlow>> _handlers = [];
    private readonly StopState _state = new();

    public void AddSource(IEventSource<TEvent> source)
    {
        ArgumentNullException.ThrowIfNull(source);
        _sources.Add(source);
    }

    public void OnEvent(Func<TEvent, ControlFlow> handler)
    {
        ArgumentNullException.ThrowIfNull(handler);
        _handlers.Add(handler);
    }

    public StopHandle GetStopHandle()
    {
        return new StopHandle(_state);
    }

    public void Stop()
    {
        _state.Stop();
    }

    public void Run()
    {
        _state.Reset();

        while (!_state.IsStopped)
        {
            var queue = new List<TEvent>();
            foreach (var source in _sources)
            {
                queue.AddRange(source.Poll());
            }

            var shouldExit = false;
            foreach (var eventItem in queue)
            {
                foreach (var handler in _handlers)
                {
                    if (handler(eventItem) == ControlFlow.Exit)
                    {
                        shouldExit = true;
                        break;
                    }
                }

                if (shouldExit)
                {
                    break;
                }
            }

            if (shouldExit)
            {
                return;
            }

            if (queue.Count == 0)
            {
                Thread.Yield();
            }
        }
    }
}

internal sealed class StopState
{
    private int _stopped;

    public bool IsStopped => Volatile.Read(ref _stopped) != 0;

    public void Stop()
    {
        Volatile.Write(ref _stopped, 1);
    }

    public void Reset()
    {
        Volatile.Write(ref _stopped, 0);
    }
}
