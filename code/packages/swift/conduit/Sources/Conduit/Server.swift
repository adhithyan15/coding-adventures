import CConduit

/// A bound Conduit server. Obtain one from `Application.bind`.
///
/// `serve()` runs the request loop on the calling thread (it blocks). The
/// underlying reactor dispatches handlers inline on that thread.
/// `serveBackground()` runs it on a dedicated OS thread so the caller can keep
/// going (used by tests). Call `stop()` to shut down.
///
/// `@unchecked Sendable`: a `Server` is explicitly designed to be stopped from a
/// different thread than the one serving (the native `StopHandle` is cross-thread
/// and atomic), and `localPort`/`running` read atomics. Sharing it across threads
/// to call `stop()` is the intended pattern.
public final class Server: @unchecked Sendable {
    private var srv: OpaquePointer?

    init(_ srv: OpaquePointer) { self.srv = srv }

    /// Serve in the foreground until stopped. Returns false if serving failed.
    @discardableResult
    public func serve() -> Bool {
        guard let srv else { return false }
        return conduit_server_serve(srv) == 0
    }

    /// Serve on a background thread. Returns false if it could not start.
    @discardableResult
    public func serveBackground() -> Bool {
        guard let srv else { return false }
        return conduit_server_serve_background(srv) == 0
    }

    /// Stop a running server (and join its background thread, if any).
    public func stop() {
        if let srv { conduit_server_stop(srv) }
    }

    /// The bound port (useful after binding to port 0).
    public var localPort: UInt16 {
        guard let srv else { return 0 }
        return conduit_server_local_port(srv)
    }

    /// Whether the server is currently running.
    public var running: Bool {
        guard let srv else { return false }
        return conduit_server_running(srv) != 0
    }

    deinit {
        if let srv { conduit_server_free(srv) }
    }
}
