// DevTools.cs — DevTools protocol middleware (v0.1.0 stub).
//
// Per UI33-rewrite §8: every mosaic-flux runtime publishes a uniform
// event stream so the Mosaic DevTools desktop app can attach.  On
// .NET hosts the transport is a named pipe \\.\pipe\mosaic-devtools.
//
// v0.1.0 ships the middleware shape and logs each event to
// Console.Out in a format the future DevTools client will
// recognise.  The named-pipe implementation requires async setup
// and is deferred to v0.2.0.

namespace Mosaic.Flux;

/// <summary>
/// DevTools protocol middleware factory.
/// </summary>
public static class DevTools
{
    /// <summary>
    /// Build a DevTools middleware.  Logs structured events to
    /// Console.Out; v0.2.0 will additionally transmit via the
    /// named pipe \\.\pipe\mosaic-devtools.
    /// </summary>
    /// <param name="storeName">Disambiguator when multiple stores
    /// are active.  Defaults to "default".</param>
    public static Middleware<TState> Create<TState>(
        string storeName = "default")
    {
        return (action, _, _) =>
        {
            var ts = DateTime.UtcNow.ToString("O");
            Console.WriteLine(
                $"[mosaic-flux-devtools] {ts} {storeName}/{action.GetType().Name}");
        };
    }
}
