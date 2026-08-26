package com.codingadventures.canonicalcbor

/** A stable, payload-blind CBR01 conformance error. */
class CborException(val id: String) : Exception(messageFor(id)) {
    companion object {
        private fun messageFor(id: String): String = when (id) {
            "unexpected-eof" -> "canonical-cbor: unexpected end of input"
            "trailing-bytes" -> "canonical-cbor: trailing bytes after decoded item"
            "reserved" -> "canonical-cbor: reserved additional-info value"
            "indefinite" -> "canonical-cbor: indefinite item rejected"
            "non-minimal-integer" -> "canonical-cbor: argument is not in smallest form"
            "invalid-utf8" -> "canonical-cbor: text is not valid UTF-8"
            "non-canonical-map-order" -> "canonical-cbor: map key order is not canonical"
            "unsupported-simple" -> "canonical-cbor: unsupported simple value"
            "float-not-supported" -> "canonical-cbor: floats are not supported"
            "too-deep" -> "canonical-cbor: decoded nesting is too deep"
            "length-too-large" -> "canonical-cbor: declared length is too large"
            "duplicate-map-key" -> "canonical-cbor: duplicate canonical map key"
            "encode-too-deep" -> "canonical-cbor: encoded nesting is too deep"
            "encode-too-large" -> "canonical-cbor: encoded item is too large"
            else -> throw IllegalArgumentException("unknown canonical CBOR error identifier")
        }
    }
}

