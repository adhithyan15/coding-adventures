// Middleware.cs — cross-cutting concern hook.
//
// Middleware sees every dispatched (action, prevState, nextState)
// triple AFTER Apply produces the next state.  Use for loggers,
// analytics, persistence, and effect schedulers.
//
// Errors thrown by middleware are caught and reported via
// Console.Error; subsequent middleware still run (matches the
// TS / Swift / Kotlin / Dart runtimes — one bad middleware can't
// take down the others).

namespace Mosaic.Flux;

/// <summary>
/// Middleware delegate: invoked after each dispatch with the
/// applied action, the state before, and the state after.
/// </summary>
public delegate void Middleware<TState>(
    IMosaicAction<TState> action,
    TState prevState,
    TState nextState);

/// <summary>
/// Static helpers for middleware composition.
/// </summary>
public static class MiddlewareHelpers
{
    /// <summary>
    /// Compose middleware in registration order.  Returns a no-op
    /// when the list is empty.  Errors thrown by one middleware
    /// are caught and reported; subsequent middleware still run.
    /// </summary>
    public static Middleware<TState> Compose<TState>(
        IEnumerable<Middleware<TState>> middleware)
    {
        var list = middleware.ToList();
        if (list.Count == 0)
        {
            return (_, _, _) => { /* no-op */ };
        }
        if (list.Count == 1)
        {
            return list[0];
        }
        return (action, prev, next) =>
        {
            foreach (var m in list)
            {
                try
                {
                    m(action, prev, next);
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine(
                        $"[mosaic-flux] middleware threw: {ex.Message}");
                }
            }
        };
    }

    /// <summary>
    /// Dev logger middleware — writes action type name to
    /// Console.Out on each dispatch.  Production hosts typically
    /// compose their own logger that ships to telemetry.
    /// </summary>
    public static Middleware<TState> Logger<TState>()
    {
        return (action, _, _) =>
        {
            Console.WriteLine($"[mosaic-flux] {action.GetType().Name}");
        };
    }
}
