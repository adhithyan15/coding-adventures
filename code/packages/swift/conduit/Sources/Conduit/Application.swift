import CConduit

// ── Closure boxes ────────────────────────────────────────────────────────────
//
// Swift closures capture context, so they can't be passed directly as C function
// pointers. Instead we box each closure in a reference type, hand the engine a
// pointer to the box as the opaque `ctx`, and register a context-free trampoline
// the engine calls on disposal so the box is released. The trampolines below
// capture nothing, so they are valid `@convention(c)` function pointers.

final class HandlerBox {
    let fn: (Request) throws -> Response?
    init(_ fn: @escaping (Request) throws -> Response?) { self.fn = fn }
}

final class AfterBox {
    let fn: (Request, Response) -> Response
    init(_ fn: @escaping (Request, Response) -> Response) { self.fn = fn }
}

private let handlerTrampoline:
    @convention(c) (UnsafeMutableRawPointer?, OpaquePointer?) -> OpaquePointer? = { ctx, reqPtr in
        guard let ctx, let reqPtr else { return nil }
        let box = Unmanaged<HandlerBox>.fromOpaque(ctx).takeUnretainedValue()
        let req = Request(reqPtr)
        do {
            guard let resp = try box.fn(req) else { return nil } // nil = continue / no response
            return resp.toC()
        } catch let halt as ConduitHalt {
            return halt.response.toC()
        } catch {
            // Non-halt error: stash the message and signal failure so the engine
            // routes through the error handler (or a default 500).
            conduit_capi_report_error("\(error)")
            return nil
        }
    }

private let afterTrampoline:
    @convention(c) (UnsafeMutableRawPointer?, OpaquePointer?, OpaquePointer?) -> OpaquePointer? = {
        ctx, reqPtr, current in
        guard let ctx, let reqPtr, let current else { return current }
        let box = Unmanaged<AfterBox>.fromOpaque(ctx).takeUnretainedValue()
        let req = Request(reqPtr)
        let cur = Response(reading: current)
        conduit_response_free(current) // we own `current`; read it then free it
        return box.fn(req, cur).toC()
    }

private let handlerCtxFree: @convention(c) (UnsafeMutableRawPointer?) -> Void = { ctx in
    if let ctx { Unmanaged<HandlerBox>.fromOpaque(ctx).release() }
}

private let afterCtxFree: @convention(c) (UnsafeMutableRawPointer?) -> Void = { ctx in
    if let ctx { Unmanaged<AfterBox>.fromOpaque(ctx).release() }
}

// ── Application ──────────────────────────────────────────────────────────────

/// A Conduit application: register routes and lifecycle hooks, then `bind` to get
/// a `Server`. Every registration method returns `self`, so calls chain.
///
///     let app = Application()
///     app.get("/") { _ in .html("<h1>Hello</h1>") }
///        .get("/hello/:name") { req in .json("{\"hi\":\"\(req.param("name") ?? "")\"}") }
///     let server = try app.bind(host: "127.0.0.1", port: 3000)
///     server.serve()
public final class Application {
    private var app: OpaquePointer?
    private var consumed = false

    public init() {
        app = conduit_app_new()
    }

    // Routes ------------------------------------------------------------------

    @discardableResult
    public func route(_ method: String, _ pattern: String,
                      _ handler: @escaping (Request) throws -> Response) -> Application {
        let box = HandlerBox { try handler($0) }
        let ctx = Unmanaged.passRetained(box).toOpaque()
        conduit_app_add_route(app, method, pattern, handlerTrampoline, ctx, handlerCtxFree)
        return self
    }

    @discardableResult
    public func get(_ pattern: String, _ handler: @escaping (Request) throws -> Response) -> Application {
        route("GET", pattern, handler)
    }

    @discardableResult
    public func post(_ pattern: String, _ handler: @escaping (Request) throws -> Response) -> Application {
        route("POST", pattern, handler)
    }

    @discardableResult
    public func put(_ pattern: String, _ handler: @escaping (Request) throws -> Response) -> Application {
        route("PUT", pattern, handler)
    }

    @discardableResult
    public func delete(_ pattern: String, _ handler: @escaping (Request) throws -> Response) -> Application {
        route("DELETE", pattern, handler)
    }

    @discardableResult
    public func patch(_ pattern: String, _ handler: @escaping (Request) throws -> Response) -> Application {
        route("PATCH", pattern, handler)
    }

    // Hooks -------------------------------------------------------------------

    /// A before-filter. Return a `Response` to short-circuit, or `nil` to continue.
    /// `try halt(...)` short-circuits as well.
    @discardableResult
    public func before(_ handler: @escaping (Request) throws -> Response?) -> Application {
        let box = HandlerBox(handler)
        let ctx = Unmanaged.passRetained(box).toOpaque()
        conduit_app_add_before(app, handlerTrampoline, ctx, handlerCtxFree)
        return self
    }

    /// A transforming after-hook. Receives the request and the current response,
    /// returns the response to send (return it unchanged to merely observe).
    @discardableResult
    public func after(_ handler: @escaping (Request, Response) -> Response) -> Application {
        let box = AfterBox(handler)
        let ctx = Unmanaged.passRetained(box).toOpaque()
        conduit_app_add_after(app, afterTrampoline, ctx, afterCtxFree)
        return self
    }

    @discardableResult
    public func notFound(_ handler: @escaping (Request) throws -> Response) -> Application {
        let box = HandlerBox { try handler($0) }
        let ctx = Unmanaged.passRetained(box).toOpaque()
        conduit_app_set_not_found(app, handlerTrampoline, ctx, handlerCtxFree)
        return self
    }

    @discardableResult
    public func onError(_ handler: @escaping (Request) throws -> Response) -> Application {
        let box = HandlerBox { try handler($0) }
        let ctx = Unmanaged.passRetained(box).toOpaque()
        conduit_app_set_error_handler(app, handlerTrampoline, ctx, handlerCtxFree)
        return self
    }

    // Settings ----------------------------------------------------------------

    @discardableResult
    public func set(_ key: String, _ value: String) -> Application {
        conduit_app_set_setting(app, key, value)
        return self
    }

    public func getSetting(_ key: String) -> String? {
        guard let p = conduit_app_get_setting(app, key) else { return nil }
        defer { conduit_string_free(p) }
        return String(cString: p)
    }

    // Bind --------------------------------------------------------------------

    /// Bind `host:port` and return a `Server`. Consumes the application (the
    /// native side moves it into the server), so call this last.
    public func bind(host: String = "127.0.0.1", port: UInt16 = 3000) throws -> Server {
        guard let a = app, !consumed else { throw ConduitError.alreadyBound }
        guard let srv = conduit_server_bind(host, port, a) else {
            consumed = true
            app = nil
            throw ConduitError.bindFailed(String(cString: conduit_last_error()))
        }
        consumed = true
        app = nil // the native app was consumed by bind
        return Server(srv)
    }

    deinit {
        if !consumed, let a = app {
            conduit_app_free(a)
        }
    }
}
