package com.codingadventures.md5native

import java.lang.ref.Cleaner

/**
 * Native-through-Rust MD5 for Kotlin — companion to the pure-Kotlin `md5`
 * package. Calls the Rust `coding_adventures_md5` crate through JNI, reusing the
 * same `md5_native_jni` cdylib as `java/md5-native`.
 *
 * **Security:** MD5 is cryptographically broken — checksum use only.
 */
object Md5Native {

    private val cleaner: Cleaner = Cleaner.create()
    private const val HEX = "0123456789abcdef"

    /** The 16-byte MD5 digest of [data] (computed in Rust). */
    fun sumMd5(data: ByteArray): ByteArray = Native.nativeDigest(data)

    /** The 32-character lowercase hex digest of [data] (computed in Rust). */
    fun hexString(data: ByteArray): String = toHex(sumMd5(data))

    internal fun toHex(bytes: ByteArray): String {
        val out = CharArray(bytes.size * 2)
        for (i in bytes.indices) {
            val v = bytes[i].toInt() and 0xff
            out[i * 2] = HEX[v ushr 4]
            out[i * 2 + 1] = HEX[v and 0x0f]
        }
        return String(out)
    }

    /** A streaming MD5 hasher backed by a native Rust hasher. [AutoCloseable]. */
    class Digest private constructor(handle: Long) : AutoCloseable {

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
            check(state.handle != 0L) { "digest is closed" }
        }

        fun update(data: ByteArray) {
            checkOpen()
            Native.nativeHasherUpdate(state.handle, data)
        }

        fun digest(): ByteArray {
            checkOpen()
            return Native.nativeHasherDigest(state.handle)
        }

        fun hexDigest(): String = toHex(digest())

        fun copy(): Digest {
            checkOpen()
            return Digest(Native.nativeHasherClone(state.handle))
        }

        override fun close() = cleanable.clean()
    }
}
