package com.codingadventures.sha256native

import java.lang.ref.Cleaner

/**
 * Native-through-Rust SHA-256 for Kotlin — the companion to the pure-Kotlin
 * `sha256` package. Calls the Rust `coding_adventures_sha256` crate through JNI,
 * reusing the same `sha256_native_jni` cdylib as `java/sha256-native`.
 */
object Sha256Native {

    private val cleaner: Cleaner = Cleaner.create()
    private const val HEX = "0123456789abcdef"

    /** The 32-byte SHA-256 digest of [data] (computed in Rust). */
    fun sha256(data: ByteArray): ByteArray = Native.nativeDigest(data)

    /** The 64-character lowercase hex digest of [data] (computed in Rust). */
    fun sha256Hex(data: ByteArray): String = toHex(sha256(data))

    internal fun toHex(bytes: ByteArray): String {
        val out = CharArray(bytes.size * 2)
        for (i in bytes.indices) {
            val v = bytes[i].toInt() and 0xff
            out[i * 2] = HEX[v ushr 4]
            out[i * 2 + 1] = HEX[v and 0x0f]
        }
        return String(out)
    }

    /**
     * A streaming SHA-256 hasher backed by a native Rust hasher. [AutoCloseable];
     * the native handle is freed by [close] (idempotent) with a [Cleaner] net.
     * [digest] is non-destructive.
     */
    class Hasher private constructor(handle: Long) : AutoCloseable {

        // The handle lives in a separate holder so the Cleaner action does not
        // capture the Hasher (which would keep it reachable and defeat the GC).
        private class State(@JvmField var handle: Long) : Runnable {
            override fun run() {
                if (handle != 0L) {
                    Native.nativeHasherFree(handle)
                    handle = 0L
                }
            }
        }

        private val state = State(handle)
        private val cleanable: Cleaner.Cleanable = cleaner.register(this, state)

        constructor() : this(Native.nativeHasherNew())

        private fun checkOpen() {
            check(state.handle != 0L) { "hasher is closed" }
        }

        /** Feed more bytes into the hash. */
        fun update(data: ByteArray) {
            checkOpen()
            Native.nativeHasherUpdate(state.handle, data)
        }

        /** The 32-byte digest of all data fed so far (non-destructive). */
        fun digest(): ByteArray {
            checkOpen()
            return Native.nativeHasherDigest(state.handle)
        }

        /** The 64-character lowercase hex digest string. */
        fun hexDigest(): String = toHex(digest())

        /** An independent copy of this hasher (its own native handle). */
        fun copy(): Hasher {
            checkOpen()
            return Hasher(Native.nativeHasherClone(state.handle))
        }

        /** Free the native handle. Idempotent. */
        override fun close() = cleanable.clean()
    }
}
