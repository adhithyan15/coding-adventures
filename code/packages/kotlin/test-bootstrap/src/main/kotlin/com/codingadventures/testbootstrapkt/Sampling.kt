package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.pbt.Gens
import java.math.BigInteger

/*
 * Corpus-style samplers: "give me `count` values from `seed`".  They exist so a
 * seed can be replayed without building a generator, and they are what the
 * pbt-v1 gen_* cases call.  Each delegates to the engine; the Kotlin layer only
 * types the result (ULong seeds, UByte/ULong outputs).
 */

/** [count] booleans from [seed]. */
fun genBool(seed: ULong, count: Int): List<Boolean> =
    Gens.genPrimitive(seed.toEngineSeed(), "bool", count).map { it as Boolean }

/** [count] bytes (as `UByte`) from [seed]. */
fun genU8(seed: ULong, count: Int): List<UByte> =
    Gens.genPrimitive(seed.toEngineSeed(), "u8", count).map { (it as BigInteger).toInt().toUByte() }

/** [count] 64-bit words from [seed]. */
fun genU64(seed: ULong, count: Int): List<ULong> =
    Gens.genPrimitive(seed.toEngineSeed(), "u64", count).map { (it as BigInteger).toULongWrapping() }

/** [count] signed 128-bit integers from [seed]. */
fun genI128(seed: ULong, count: Int): List<BigInteger> =
    Gens.genPrimitive(seed.toEngineSeed(), "i128", count).map { it as BigInteger }

/** [count] uniform integers over `[min, max]` from [seed]. */
fun genBounded(seed: ULong, min: BigInteger, max: BigInteger, count: Int): List<BigInteger> =
    if (Seams.swapBounds) {
        Gens.genBounded(seed.toEngineSeed(), max, min, count)
    } else {
        Gens.genBounded(seed.toEngineSeed(), min, max, count)
    }

/** [count] byte strings with length in `[minLen, maxLen]` from [seed]. */
fun genBytes(seed: ULong, minLen: Int, maxLen: Int, count: Int): List<ByteArray> =
    Gens.genBytes(seed.toEngineSeed(), minLen, maxLen, count)
