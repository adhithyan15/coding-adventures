package com.codingadventures.sha512

/**
 * Centralized error and diagnostic messages for SHA-512 (Standing Rule #20).
 */
object Sha512Messages {
    const val ERR_NULL_DATA: String = "sha512: input data must not be null"
    const val ERR_INDEX_OUT_OF_BOUNDS: String = "sha512: offset or length out of bounds"
    const val ERR_INVALID_BYTE: String = "sha512: byte value out of range 0..255"
    const val ERR_MESSAGE_TOO_LONG: String = "sha512: message length must be less than 2^128 bits"
    const val ERR_CORPUS_MALFORMED: String = "sha512: malformed fixture corpus row"
    const val ERR_CORPUS_OVERSIZED: String = "sha512: fixture corpus file exceeds 4096 bytes"
    const val ERR_UNKNOWN: String = "sha512: unknown error"

    fun messageFor(code: Sha512ErrorCode?): String {
        return when (code) {
            Sha512ErrorCode.NULL_DATA -> ERR_NULL_DATA
            Sha512ErrorCode.INDEX_OUT_OF_BOUNDS -> ERR_INDEX_OUT_OF_BOUNDS
            Sha512ErrorCode.INVALID_BYTE -> ERR_INVALID_BYTE
            Sha512ErrorCode.MESSAGE_TOO_LONG -> ERR_MESSAGE_TOO_LONG
            Sha512ErrorCode.CORPUS_MALFORMED -> ERR_CORPUS_MALFORMED
            Sha512ErrorCode.CORPUS_OVERSIZED -> ERR_CORPUS_OVERSIZED
            Sha512ErrorCode.UNKNOWN, null -> ERR_UNKNOWN
        }
    }
}
