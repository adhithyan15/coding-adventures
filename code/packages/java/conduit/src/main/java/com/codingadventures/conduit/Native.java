package com.codingadventures.conduit;

/**
 * Native gateway to the Rust {@code conduit_jni} cdylib.
 *
 * <p>Every method here is declared {@code native} and implemented in
 * {@code code/packages/rust/conduit-jni/src/lib.rs}. The JVM resolves each to
 * an exported symbol named
 * {@code Java_com_codingadventures_conduit_Native_<method>}.
 *
 * <p>The static initializer loads {@code libconduit_jni.{so,dylib,dll}}. Tests
 * and programs must pass {@code -Djava.library.path=<dir containing the lib>};
 * {@code build.gradle.kts} points it at the Rust release build directory.
 *
 * <h2>Peer-pointer model</h2>
 *
 * <p>The Rust side allocates a {@code NativeApp} / {@code NativeServer} on the
 * heap and hands Java back a {@code long} pointing at it. Java passes that
 * {@code long} back on every call. {@code nativeDisposeApp} /
 * {@code nativeDisposeServer} free the allocation and release the global
 * references that keep handler lambdas alive. {@link Application} and
 * {@link Server} are {@link AutoCloseable} and call dispose from
 * {@code close()}.
 */
final class Native {

    static {
        System.loadLibrary("conduit_jni");
    }

    private Native() {
    }

    // ── Application construction ────────────────────────────────────────────

    /** Allocate a Rust NativeApp; returns its peer pointer. */
    static native long nativeNewApp();

    /** Register a route handler. */
    static native void nativeAddRoute(long app, String method, String pattern, ConduitHandler handler);

    /** Register a before filter (return a Response to short-circuit, null to continue). */
    static native void nativeAddBefore(long app, ConduitHandler handler);

    /** Register an after filter (return a Response to replace, null to keep the prior one). */
    static native void nativeAddAfter(long app, ConduitHandler handler);

    /** Set the not-found handler. */
    static native void nativeSetNotFound(long app, ConduitHandler handler);

    /** Set the error handler (its Request's {@link Request#error()} carries the message). */
    static native void nativeSetErrorHandler(long app, ConduitHandler handler);

    /** Store a string setting. */
    static native void nativeSetSetting(long app, String key, String value);

    /** Read a string setting, or {@code null} if absent. */
    static native String nativeGetSetting(long app, String key);

    /** Free a NativeApp that was never turned into a server. */
    static native void nativeDisposeApp(long app);

    // ── Server lifecycle ────────────────────────────────────────────────────

    /** Consume the app, bind {@code host:port}, return the server peer pointer. */
    static native long nativeNewServer(long app, String host, int port, int maxConnections);

    /** Run the server on the calling thread, blocking until stopped. */
    static native void nativeServe(long server);

    /** Run the server on a background Rust thread; returns immediately. */
    static native void nativeServeBackground(long server);

    /** Signal the server to stop. */
    static native void nativeStop(long server);

    /** The bound TCP port (useful when port 0 was requested). */
    static native int nativeLocalPort(long server);

    /** Whether the server thread is currently active. */
    static native boolean nativeRunning(long server);

    /** Stop, join, release global refs, and free the NativeServer. */
    static native void nativeDisposeServer(long server);
}
