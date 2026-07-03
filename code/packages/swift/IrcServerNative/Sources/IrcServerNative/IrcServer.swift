import CIrcServer

// ============================================================================
// IrcServer — a high-performance IRC server for Swift
// ============================================================================
//
// Every line of IRC and TCP logic runs in Rust (the `irc-net-reactor` engine on
// the home-grown kqueue/epoll reactor). Swift only *launches and controls* the
// server through the `irc-server-capi` C ABI — there is no callback into Swift,
// so this type is a pure lifecycle control surface: create, serve, stop.
//
//   let server = try IrcServer(port: 6667)
//   server.serveBackground()                 // runs the loop on a Rust thread
//   // ... connect IRC clients to server.localHost : server.localPort ...
//   server.stop()
//
// `serve()` runs the loop on the calling thread and blocks until `stop()`;
// `serveBackground()` runs it on a dedicated Rust OS thread and returns at once.

/// Thrown when the underlying engine cannot bind the requested address.
public struct IrcServerError: Error, CustomStringConvertible {
    public let message: String
    public var description: String { message }
}

/// A bound IRC server.
///
/// `@unchecked Sendable`: an `IrcServer` is explicitly designed to be stopped
/// from a different thread than the one serving. The C ABI underneath takes only
/// shared references for `stop`/`running`/`localPort`, with all shared state
/// atomic or `Mutex`-guarded, so calling `stop()` from another thread while
/// `serve()` blocks is safe.
///
/// One invariant the ABI requires — that the handle is not freed while a call is
/// still in flight — is upheld automatically here: a thread running `serve()`
/// holds a strong reference to the instance for the call's whole duration, so ARC
/// cannot run `deinit` (and thus `irc_server_free`) until `serve()` returns.
public final class IrcServer: @unchecked Sendable {
    private var srv: OpaquePointer?

    /// The bound IP address (captured at construction, so it is stable for the
    /// life of the server).
    public let localHost: String

    /// The bound TCP port (the OS-assigned port when constructed with `port: 0`).
    public let localPort: UInt16

    /// Bind a new IRC server.
    ///
    /// - Parameters:
    ///   - host: the interface to bind (default loopback).
    ///   - port: the TCP port; `0` lets the OS choose an ephemeral port.
    ///   - serverName: the server name announced to clients.
    ///   - motd: the message-of-the-day lines.
    ///   - operPassword: the `OPER` password (empty disables `OPER`).
    ///   - maxConnections: the connection cap (clamped to at least 1 in Rust).
    /// - Throws: `IrcServerError` if the socket cannot be bound.
    public init(
        host: String = "127.0.0.1",
        port: UInt16 = 6667,
        serverName: String = "irc.local",
        motd: [String] = ["Welcome."],
        operPassword: String = "",
        maxConnections: UInt32 = 1024
    ) throws {
        // MOTD lines are joined with newlines for a single C-string arg; the Rust
        // side splits them back into lines (dropping empties).
        let motdJoined = motd.joined(separator: "\n")
        let handle = host.withCString { hostC in
            serverName.withCString { nameC in
                motdJoined.withCString { motdC in
                    operPassword.withCString { passC in
                        irc_server_new(hostC, port, nameC, motdC, passC, maxConnections)
                    }
                }
            }
        }
        guard let handle else {
            throw IrcServerError(message: "irc_server_new: failed to bind \(host):\(port)")
        }
        self.srv = handle

        // Read the bound address back from the engine (resolves an ephemeral
        // port) and own the returned C string per the ABI contract.
        if let hostPtr = irc_server_local_host(handle) {
            self.localHost = String(cString: hostPtr)
            irc_server_string_free(hostPtr)
        } else {
            self.localHost = host
        }
        self.localPort = irc_server_local_port(handle)
    }

    /// Serve in the foreground until stopped. Returns `false` if serving failed.
    @discardableResult
    public func serve() -> Bool {
        guard let srv else { return false }
        return irc_server_serve(srv) == 0
    }

    /// Serve on a background Rust thread. Returns `false` if it could not start.
    @discardableResult
    public func serveBackground() -> Bool {
        guard let srv else { return false }
        return irc_server_serve_background(srv) == 0
    }

    /// Stop a running server (and join its background thread, if any).
    public func stop() {
        if let srv { irc_server_stop(srv) }
    }

    /// Whether the event loop is currently running.
    public var running: Bool {
        guard let srv else { return false }
        return irc_server_running(srv)
    }

    /// The bound `host:port` address.
    public var localAddr: String { "\(localHost):\(localPort)" }

    deinit {
        // irc_server_free stops + joins before releasing the handle.
        if let srv { irc_server_free(srv) }
    }
}
