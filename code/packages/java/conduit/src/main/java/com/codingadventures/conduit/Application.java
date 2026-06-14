package com.codingadventures.conduit;

/**
 * A Conduit application: the place you register routes, lifecycle filters,
 * fallback handlers, and settings before binding a {@link Server}.
 *
 * <pre>{@code
 * try (Application app = new Application()) {
 *     app.before(req -> req.path().equals("/down") ? Responses.halt(503, "Maintenance") : null)
 *        .get("/", req -> Responses.html("<h1>Hello</h1>"))
 *        .get("/hello/:name", req -> Responses.text("Hi " + req.param("name")))
 *        .notFound(req -> Responses.html("<h1>404</h1>", 404));
 *
 *     try (Server server = Server.bind(app, "127.0.0.1", 3000)) {
 *         server.serve();
 *     }
 * }
 * }</pre>
 *
 * <p>An {@code Application} owns a native peer object (a Rust {@code NativeApp}).
 * Binding a {@link Server} <em>consumes</em> the application — after that you
 * can no longer register routes on it, and {@link #close()} becomes a no-op
 * because the {@link Server} now owns the native resources. If you never bind a
 * server, {@link #close()} frees the native peer; using try-with-resources is
 * therefore recommended.
 */
public final class Application implements AutoCloseable {

    private long handle;
    private boolean consumed;

    /** Create an empty application. */
    public Application() {
        this.handle = Native.nativeNewApp();
        if (this.handle == 0) {
            throw new IllegalStateException("conduit: failed to allocate native application");
        }
    }

    // ── HTTP method helpers (chainable) ─────────────────────────────────────

    public Application get(String pattern, ConduitHandler handler) {
        return route("GET", pattern, handler);
    }

    public Application post(String pattern, ConduitHandler handler) {
        return route("POST", pattern, handler);
    }

    public Application put(String pattern, ConduitHandler handler) {
        return route("PUT", pattern, handler);
    }

    public Application delete(String pattern, ConduitHandler handler) {
        return route("DELETE", pattern, handler);
    }

    public Application patch(String pattern, ConduitHandler handler) {
        return route("PATCH", pattern, handler);
    }

    /** Register a handler for an arbitrary HTTP method. */
    public Application route(String method, String pattern, ConduitHandler handler) {
        checkOpen();
        requireHandler(handler);
        Native.nativeAddRoute(handle, method, pattern, handler);
        return this;
    }

    // ── Filters & fallbacks ─────────────────────────────────────────────────

    /** Register a before filter. Return a Response to short-circuit, null to continue. */
    public Application before(ConduitHandler handler) {
        checkOpen();
        requireHandler(handler);
        Native.nativeAddBefore(handle, handler);
        return this;
    }

    /** Register an after filter. Return a Response to replace, null to keep the prior one. */
    public Application after(ConduitHandler handler) {
        checkOpen();
        requireHandler(handler);
        Native.nativeAddAfter(handle, handler);
        return this;
    }

    /** Set the not-found handler (overwrites any previous). */
    public Application notFound(ConduitHandler handler) {
        checkOpen();
        requireHandler(handler);
        Native.nativeSetNotFound(handle, handler);
        return this;
    }

    /**
     * Set the error handler (overwrites any previous). When a handler throws a
     * non-halt exception, the error handler runs with the message available
     * via {@link Request#error()}.
     */
    public Application onError(ConduitHandler handler) {
        checkOpen();
        requireHandler(handler);
        Native.nativeSetErrorHandler(handle, handler);
        return this;
    }

    // ── Settings ────────────────────────────────────────────────────────────

    /** Store a string setting. */
    public Application set(String key, String value) {
        checkOpen();
        Native.nativeSetSetting(handle, key, value);
        return this;
    }

    /** Read a string setting, or {@code null} if absent. */
    public String getSetting(String key) {
        checkOpen();
        return Native.nativeGetSetting(handle, key);
    }

    // ── Lifecycle ───────────────────────────────────────────────────────────

    /** Package-private: the native peer pointer, for {@link Server#bind}. */
    long handle() {
        checkOpen();
        return handle;
    }

    /** Package-private: mark the app as consumed by a Server. */
    void markConsumed() {
        this.consumed = true;
    }

    @Override
    public void close() {
        // If a Server consumed this app, the Server owns the native resources;
        // disposing here would double-free. Only dispose an un-bound app.
        if (!consumed && handle != 0) {
            Native.nativeDisposeApp(handle);
        }
        handle = 0;
    }

    private void checkOpen() {
        if (consumed) {
            throw new IllegalStateException("application has been consumed by a Server");
        }
        if (handle == 0) {
            throw new IllegalStateException("application is closed");
        }
    }

    private static void requireHandler(ConduitHandler handler) {
        if (handler == null) {
            throw new NullPointerException("handler must not be null");
        }
    }
}
