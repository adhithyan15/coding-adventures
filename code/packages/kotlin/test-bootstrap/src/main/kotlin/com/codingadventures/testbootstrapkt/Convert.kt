@file:JvmName("Convert")

package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.pbt.Gen as JGen
import com.codingadventures.testbootstrap.pbt.Shrinker
import com.codingadventures.testbootstrap.pbt.Shrunk as JShrunk
import java.math.BigInteger

/*
 * Conversions between Kotlin shapes and Java engine shapes.
 *
 * Everything here is a bijection or a field copy.  Nothing in this file may
 * decide what is drawn, how a value shrinks or how many steps it took: that is
 * the engine's job (VER05f section 1).
 */

private val TWO_64: BigInteger = BigInteger.ONE.shiftLeft(64)

/**
 * A Kotlin `ULong` as a non-negative `BigInteger`:
 *
 * ```
 *   0uL                    -> 0
 *   18446744073709551615uL -> 2^64 - 1   (bit pattern 0xffff...ffff)
 * ```
 */
@JvmSynthetic
internal fun ULong.toBigInteger(): BigInteger {
    val signed = BigInteger.valueOf(toLong())
    return if (signed.signum() < 0) signed.add(TWO_64) else signed
}

/** The low 64 bits of a `BigInteger` as a `ULong` (so a negative corpus seed wraps mod 2^64). */
@JvmSynthetic
internal fun BigInteger.toULongWrapping(): ULong = toLong().toULong()

/** The seed the engine sees: the Kotlin seed's bit pattern, plus the test seam. */
@JvmSynthetic
internal fun ULong.toEngineSeed(): Long = toLong() + Seams.seedOffset

/** An engine shrink result as a Kotlin one (the seam may bend the step count). */
@JvmSynthetic
internal fun <T> JShrunk<T>.toKotlin(): Shrunk<T> = Shrunk(value(), steps() + Seams.shrinkStepDelta)

/**
 * Re-types an engine generator through a bijection.  Sampling maps the sampled
 * value forward; shrinking maps the value back, lets the engine's own shrinker
 * walk, and maps the result forward again.  The step count is the engine's.
 */
@JvmSynthetic
internal fun <A, B> JGen<A>.iso(to: (A) -> B, from: (B) -> A): JGen<B> {
    val inner = this
    return JGen.of(
        { rng -> to(inner.sample(rng)) },
        { value, falsifies, maxSteps ->
            val r = inner.shrink(from(value), { a -> falsifies.test(to(a)) }, maxSteps)
            JShrunk(to(r.value()), r.steps())
        },
    )
}

/** A Kotlin shrinker as an engine shrinker (a field-for-field adapter). */
@JvmSynthetic
internal fun <T> ShrinkFn<T>.toEngine(): Shrinker<T> {
    val fn = this
    return Shrinker { value, falsifies, maxSteps ->
        val r = fn(value, { falsifies.test(it) }, maxSteps)
        JShrunk(r.value, r.steps)
    }
}
