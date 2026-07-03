package com.codingadventures.ircserver;

/**
 * JNI bindings to the {@code irc_server_native_jni} Rust cdylib, which embeds the
 * all-Rust {@code irc-net-reactor} IRC engine. Each {@code native} method maps to
 * a {@code Java_com_codingadventures_ircserver_Native_*} function in
 * {@code code/packages/rust/irc-server-native-jni/src/lib.rs}.
 *
 * <p>All IRC and TCP logic runs in Rust; the JVM only launches and controls the
 * server. There is no callback into Java.
 */
final class Native {

    static {
        System.loadLibrary("irc_server_native_jni");
    }

    private Native() {
    }

    /**
     * Bind a server and return its peer pointer (0 on failure, with a Java
     * exception already thrown). {@code motd} is a single newline-joined string.
     */
    static native long nativeNewServer(
        String host,
        int port,
        String serverName,
        String motd,
        String operPassword,
        int maxConnections);

    /** Run the event loop on the calling thread (blocks until stopped). */
    static native void nativeServe(long server);

    /** Run the event loop on a background Rust thread; returns immediately. */
    static native void nativeServeBackground(long server);

    /** Signal the loop to stop and join the background thread. */
    static native void nativeStop(long server);

    /** Whether the loop is currently running. */
    static native boolean nativeRunning(long server);

    /** The bound IP address. */
    static native String nativeLocalHost(long server);

    /** The bound TCP port. */
    static native int nativeLocalPort(long server);

    /** Stop, join, and free the peer. */
    static native void nativeDisposeServer(long server);
}
