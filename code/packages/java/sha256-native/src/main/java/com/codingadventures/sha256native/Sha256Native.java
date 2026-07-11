package com.codingadventures.sha256native;

import java.lang.ref.Cleaner;

/**
 * Native-through-Rust SHA-256 for the JVM — the companion to the pure-Java
 * {@code sha256} package. Instead of reimplementing the algorithm, it calls the
 * Rust {@code coding_adventures_sha256} crate through JNI (the
 * {@code sha256_native_jni} cdylib).
 *
 * <p>The one-shot API ({@link #sha256(byte[])} / {@link #sha256Hex(byte[])}) is
 * stateless. The streaming {@link Hasher} owns a native handle and is an
 * {@link AutoCloseable}; it also registers with a {@link Cleaner} so the native
 * memory is reclaimed even if {@code close()} is missed.
 */
public final class Sha256Native {

    private static final Cleaner CLEANER = Cleaner.create();
    private static final char[] HEX = "0123456789abcdef".toCharArray();

    private Sha256Native() {
    }

    /** The 32-byte SHA-256 digest of {@code data} (computed in Rust). */
    public static byte[] sha256(byte[] data) {
        return Native.nativeDigest(data);
    }

    /** The 64-character lowercase hex digest of {@code data} (computed in Rust). */
    public static String sha256Hex(byte[] data) {
        return toHex(sha256(data));
    }

    static String toHex(byte[] bytes) {
        char[] out = new char[bytes.length * 2];
        for (int i = 0; i < bytes.length; i++) {
            int v = bytes[i] & 0xff;
            out[i * 2] = HEX[v >>> 4];
            out[i * 2 + 1] = HEX[v & 0x0f];
        }
        return new String(out);
    }

    /**
     * A streaming SHA-256 hasher backed by a native Rust hasher. Feed data with
     * {@link #update}; {@link #digest} is non-destructive. Call {@link #close}
     * (or use try-with-resources) to free the native handle promptly; a
     * {@link Cleaner} is the safety net if you forget.
     */
    public static final class Hasher implements AutoCloseable {

        // The handle lives in a separate object so the Cleaner action does not
        // capture the Hasher (which would keep it reachable and defeat the GC).
        private static final class State implements Runnable {
            long handle;

            State(long handle) {
                this.handle = handle;
            }

            @Override
            public void run() {
                if (handle != 0) {
                    Native.nativeHasherFree(handle);
                    handle = 0;
                }
            }
        }

        private final State state;
        private final Cleaner.Cleanable cleanable;

        /** Create a new streaming hasher. */
        public Hasher() {
            this.state = new State(Native.nativeHasherNew());
            this.cleanable = CLEANER.register(this, state);
        }

        private Hasher(long handle) {
            this.state = new State(handle);
            this.cleanable = CLEANER.register(this, state);
        }

        private void checkOpen() {
            if (state.handle == 0) {
                throw new IllegalStateException("hasher is closed");
            }
        }

        /** Feed more bytes into the hash. */
        public void update(byte[] data) {
            checkOpen();
            Native.nativeHasherUpdate(state.handle, data);
        }

        /** The 32-byte digest of all data fed so far (non-destructive). */
        public byte[] digest() {
            checkOpen();
            return Native.nativeHasherDigest(state.handle);
        }

        /** The 64-character lowercase hex digest string. */
        public String hexDigest() {
            return toHex(digest());
        }

        /** An independent copy of this hasher (its own native handle). */
        public Hasher copy() {
            checkOpen();
            return new Hasher(Native.nativeHasherClone(state.handle));
        }

        /** Free the native handle. Idempotent; safe to call more than once. */
        @Override
        public void close() {
            cleanable.clean();
        }
    }
}
