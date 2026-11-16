package com.codingadventures.sha512

/**
 * Payload-blind exception thrown by SHA-512 operations with a closed error code (Standing Rule #20).
 */
class Sha512Exception(val code: Sha512ErrorCode) :
    RuntimeException(Sha512Messages.messageFor(code))
