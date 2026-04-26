package com.codingadventures.ctcompare

object CtCompare {
    fun ctEq(left: ByteArray, right: ByteArray): Boolean {
        if (left.size != right.size) return false

        var accumulator = 0
        for (index in left.indices) {
            accumulator = accumulator or ((left[index].toInt() xor right[index].toInt()) and 0xFF)
        }
        return accumulator == 0
    }

    fun ctEqFixed(left: ByteArray, right: ByteArray): Boolean = ctEq(left, right)

    fun ctSelectBytes(left: ByteArray, right: ByteArray, choice: Boolean): ByteArray {
        require(left.size == right.size) { "ctSelectBytes requires equal-length inputs" }

        val mask = 0 - if (choice) 1 else 0
        return ByteArray(left.size) { index ->
            (right[index].toInt() xor ((left[index].toInt() xor right[index].toInt()) and mask)).toByte()
        }
    }

    fun ctEqU64(left: Long, right: Long): Boolean {
        val diff = left xor right
        val folded = (diff or -diff).ushr(63)
        return folded == 0L
    }
}
