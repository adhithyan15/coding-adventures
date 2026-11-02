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

    /**
     * Return a fresh copy of [left] when [choice] is 1 and of [right] when it
     * is 0, without branching on [choice].
     *
     * Kotlin has no branch-free conversion from `Boolean` to an integer, so a
     * Boolean choice would need `if (choice) 1 else 0`, a branch on the secret
     * bit that CT01 forbids. The bit is therefore an `Int` that must be exactly
     * 0 or 1, as Go's `crypto/subtle` does:
     *
     *     choice = 0  ->  mask = -0 = 0x00000000  ->  result is right
     *     choice = 1  ->  mask = -1 = 0xFFFFFFFF  ->  result is left
     *
     * Validation tests `choice and 1.inv()`, which is 0 for both valid values,
     * so a valid call takes the same path whichever bit is secret.
     *
     * @throws IllegalArgumentException if the lengths differ or [choice] is not
     *     0 or 1, before either input is read
     */
    fun ctSelectBytes(left: ByteArray, right: ByteArray, choice: Int): ByteArray {
        require(left.size == right.size) { "ctSelectBytes requires equal-length inputs" }
        require(choice and 1.inv() == 0) { "ctSelectBytes requires choice to be 0 or 1" }

        val mask = -choice
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
