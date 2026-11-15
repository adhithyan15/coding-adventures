package com.codingadventures.itf

/**
 * Diagnostic and error messages for ITF encoding in Kotlin, strictly payload-blind.
 */
object ItfMessages {
    const val ERROR_INPUT_TOO_LONG: String = "input-too-long"
    const val ERROR_INVALID_LENGTH: String = "invalid-length"
    const val ERROR_INVALID_CHARACTER: String = "invalid-character"

    const val MESSAGE_INPUT_TOO_LONG: String = "ITF input must contain at most 4096 characters"
    const val MESSAGE_INVALID_LENGTH: String = "ITF input must contain an even number of digits"
    const val MESSAGE_INVALID_CHARACTER: String = "ITF input must contain digits only"
    const val MESSAGE_INVALID_SCALARS: String = "ITF input must contain valid Unicode scalar values"
    const val MESSAGE_NULL_INPUT: String = "ITF input must not be null"

    const val REDACTED_PLACEHOLDER: String = "<redacted>"
    const val INVALID_PLACEHOLDER: String = "<invalid>"

    fun messageFor(code: ItfErrorCode?): String = when (code) {
        ItfErrorCode.INPUT_TOO_LONG -> MESSAGE_INPUT_TOO_LONG
        ItfErrorCode.INVALID_LENGTH -> MESSAGE_INVALID_LENGTH
        ItfErrorCode.INVALID_CHARACTER, null -> MESSAGE_INVALID_CHARACTER
    }
}
