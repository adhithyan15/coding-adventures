package com.codingadventures.itf

/**
 * Closed enumeration of valid ITF validation error codes in Kotlin.
 */
enum class ItfErrorCode(val errorId: String) {
    INPUT_TOO_LONG(ItfMessages.ERROR_INPUT_TOO_LONG),
    INVALID_LENGTH(ItfMessages.ERROR_INVALID_LENGTH),
    INVALID_CHARACTER(ItfMessages.ERROR_INVALID_CHARACTER)
}
