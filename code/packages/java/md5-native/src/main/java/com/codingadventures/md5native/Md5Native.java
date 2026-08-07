package com.codingadventures.md5native;

import java.lang.ref.Cleaner;

/**
 * Native-through-Rust MD5 for the JVM — companion to the pure-Java {@code md5}
 * package. Calls the Rust {@code coding_adventures_md5} crate through JNI (the
 * {@code md5_native_jni} cdylib). Mirrors {@code java/sha256-native} (16-byte
 * digest).
 *
 * <p><b>Security:</b> MD5 is cryptographically broken — checksum use only.
 */
public final class Md5Native {

    private static final Cleaner CLEANER = Cleaner.create();
    private static final char[] HEX = "0123456789abcdef".toCharArray();

    private Md5Native() {
    }

    /** The 16-byte MD5 digest of {@code data} (computed in Rust). */
    public static byte[] sumMd5(byte[] data) {
        return Native.nativeDigest(data);
    }

    /** The 32-character lowercase hex digest of {@code data} (computed in Rust). */
    public static String hexString(byte[] data) {
        return toHex(sumMd5(data));
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

    /** A streaming MD5 hasher backed by a native Rust hasher. {@link AutoCloseable}. */
    public static final class Digest implements AutoCloseable {

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

        public Digest() {
            this.state = new State(Native.nativeHasherNew());
            this.cleanable = CLEANER.register(this, state);
        }

        private Digest(long handle) {
            this.state = new State(handle);
            this.cleanable = CLEANER.register(this, state);
        }

        private void checkOpen() {
            if (state.handle == 0) {
                throw new IllegalStateException("digest is closed");
            }
        }

        public void update(byte[] data) {
            checkOpen();
            Native.nativeHasherUpdate(state.handle, data);
        }

        public byte[] digest() {
            checkOpen();
            return Native.nativeHasherDigest(state.handle);
        }

        public String hexDigest() {
            return toHex(digest());
        }

        public Digest copy() {
            checkOpen();
            return new Digest(Native.nativeHasherClone(state.handle));
        }

        @Override
        public void close() {
            cleanable.clean();
        }
    }
}
