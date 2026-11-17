package com.codingadventures.itf

/**
 * Thrown when ITF input fails validation against the ITF specification in Kotlin.
 */
class InvalidItfInputException(
    val errorCode: ItfErrorCode
) : IllegalArgumentException(ItfMessages.messageFor(errorCode)) {
    val errorId: String
        get() = errorCode.errorId
}
