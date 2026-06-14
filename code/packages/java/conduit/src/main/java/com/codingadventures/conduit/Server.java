package com.codingadventures.conduit;

/**
 * Binds a Conduit {@link Application} to a TCP port and serves it with the Rust
 * {@code web-core} engine.
 *
 * <pre>{@code
 * try (Server server = Server.bind(app, "127.0.0.1", 3000)) {
 *     server.serve();   // blocks until stop()/close()
 * }
 * }</pre>
 *
 * <p>For tests, use {@link #serveBackground()} to run on a background thread,
 * then {@link #localPort()} to discover the OS-assigned port (when binding to
 * port 0) and {@link #stop()} to shut down.
 *
 * <p>{@code Server} is {@link AutoCloseable}: {@link #close()} stops the server
 * and frees the native peer (releasing the global references that keep handler
 * lambdas alive).
 *
 * <h2>Lifecycle threading</h2>
 *
 * <p>A {@code Server} expects a single-threaded lifecycle: build, bind, then
 * {@code serve()} (which blocks) or {@code serveBackground()}. The one
 * cross-thread operation is {@link #stop()} (safe from any thread — it signals
 * an atomic stop handle). Do <em>not</em> race {@link #close()} against
 * {@code serve()}/other methods from another thread; the native side guards
 * against a zeroed handle but a true data race on the peer pointer is undefined.
 * The typical pattern is try-with-resources, or {@code stop()} from a signal
 * handler followed by {@code close()} on the owning thread.
 */
public final class Server implements AutoCloseable {

    /** Default maximum concurrent connections when not specified. */
    public static final int DEFAULT_MAX_CONNECTIONS = 128;

    private long handle;

    private Server(long handle) {
        this.handle = handle;
    }

    /** Bind {@code app} to {@code host:port} with the default connection limit. */
    public static Server bind(Application app, String host, int port) {
        return bind(app, host, port, DEFAULT_MAX_CONNECTIONS);
    }

    /**
     * Bind {@code app} to {@code host:port}. Pass port {@code 0} to let the OS
     * pick a free port (read it back with {@link #localPort()}). This consumes
     * {@code app}.
     */
    public static Server bind(Application app, String host, int port, int maxConnections) {
        if (app == null) {
            throw new NullPointerException("app must not be null");
        }
        long serverHandle = Native.nativeNewServer(app.handle(), host, port, maxConnections);
        // nativeNewServer consumed the native app box (success or throw). Mark
        // it so the Application's close() doesn't try to dispose it again.
        app.markConsumed();
        if (serverHandle == 0) {
            throw new IllegalStateException("conduit: failed to bind server");
        }
        return new Server(serverHandle);
    }

    /** Run the server on the calling thread, blocking until {@link #stop()}. */
    public void serve() {
        checkOpen();
        Native.nativeServe(handle);
    }

    /** Run the server on a background Rust thread; returns immediately. */
    public void serveBackground() {
        checkOpen();
        Native.nativeServeBackground(handle);
    }

    /** Signal the server to stop. Safe to call from any thread. */
    public void stop() {
        checkOpen();
        Native.nativeStop(handle);
    }

    /** The bound TCP port (useful when port 0 was requested). */
    public int localPort() {
        checkOpen();
        return Native.nativeLocalPort(handle);
    }

    /** Whether the server background thread is currently active. */
    public boolean running() {
        checkOpen();
        return Native.nativeRunning(handle);
    }

    @Override
    public void close() {
        if (handle != 0) {
            Native.nativeDisposeServer(handle);
            handle = 0;
        }
    }

    private void checkOpen() {
        if (handle == 0) {
            throw new IllegalStateException("server is closed");
        }
    }
}
