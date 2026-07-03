package com.codingadventures.ircserver;

import java.util.List;

/**
 * A high-performance IRC server for the JVM, backed by the all-Rust
 * {@code irc-net-reactor} engine (on the home-grown kqueue/epoll reactor).
 *
 * <p>Every line of IRC and TCP logic runs in Rust; this class only launches and
 * controls the server. Usage:
 *
 * <pre>{@code
 * try (IrcServer server = IrcServer.builder().port(6667).build()) {
 *     server.serveBackground();
 *     // ... connect IRC clients to server.localHost():server.localPort() ...
 *     server.stop();
 * }
 * }</pre>
 *
 * <p>The peer is a raw native pointer; like {@code conduit}'s server, this class
 * is single-owner. Do not race {@link #close()} against other methods on the
 * same instance from another thread.
 */
public final class IrcServer implements AutoCloseable {

    /** Peer pointer to the Rust {@code NativeServer}; 0 once closed. */
    private long handle;

    private IrcServer(long handle) {
        this.handle = handle;
    }

    /** Start configuring a server. */
    public static Builder builder() {
        return new Builder();
    }

    /** Run the event loop on the calling thread, blocking until {@link #stop()}. */
    public void serve() {
        checkOpen();
        Native.nativeServe(handle);
    }

    /** Run the event loop on a background Rust thread; returns immediately. */
    public void serveBackground() {
        checkOpen();
        Native.nativeServeBackground(handle);
    }

    /** Signal the server to stop and join the background thread. */
    public void stop() {
        checkOpen();
        Native.nativeStop(handle);
    }

    /** Whether the event loop is currently running. */
    public boolean running() {
        checkOpen();
        return Native.nativeRunning(handle);
    }

    /** The bound IP address. */
    public String localHost() {
        checkOpen();
        return Native.nativeLocalHost(handle);
    }

    /** The bound TCP port (useful when port 0 was requested). */
    public int localPort() {
        checkOpen();
        return Native.nativeLocalPort(handle);
    }

    /** The bound {@code host:port} address. */
    public String localAddr() {
        return localHost() + ":" + localPort();
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

    /** Fluent configuration for an {@link IrcServer}. */
    public static final class Builder {
        private String host = "127.0.0.1";
        private int port = 6667;
        private String serverName = "irc.local";
        private List<String> motd = List.of("Welcome.");
        private String operPassword = "";
        private int maxConnections = 1024;

        private Builder() {
        }

        public Builder host(String host) {
            this.host = host;
            return this;
        }

        public Builder port(int port) {
            this.port = port;
            return this;
        }

        public Builder serverName(String serverName) {
            this.serverName = serverName;
            return this;
        }

        public Builder motd(List<String> motd) {
            this.motd = (motd == null || motd.isEmpty()) ? List.of("Welcome.") : motd;
            return this;
        }

        public Builder operPassword(String operPassword) {
            this.operPassword = operPassword == null ? "" : operPassword;
            return this;
        }

        public Builder maxConnections(int maxConnections) {
            this.maxConnections = maxConnections;
            return this;
        }

        /** Bind the server and return a running-capable handle. */
        public IrcServer build() {
            // MOTD lines are joined with newlines for a single JNI string arg;
            // the Rust side splits them back into lines.
            String joinedMotd = String.join("\n", motd);
            long h = Native.nativeNewServer(
                host, port, serverName, joinedMotd, operPassword, maxConnections);
            if (h == 0) {
                throw new IllegalStateException("irc-server-native: failed to bind server");
            }
            return new IrcServer(h);
        }
    }
}
