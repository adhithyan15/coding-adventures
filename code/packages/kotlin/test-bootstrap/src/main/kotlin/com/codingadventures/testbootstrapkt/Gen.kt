package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.pbt.Gen as JGen
import com.codingadventures.testbootstrap.pbt.Gens
import com.codingadventures.testbootstrap.prng.SplitMix64
import java.math.BigInteger
import java.util.function.Predicate

/** The deterministic stream a generator draws from. */
typealias Rng = SplitMix64

/** The result of shrinking: the smallest value found that still falsifies, and the accepted-move count. */
data class Shrunk<T>(val value: T, val steps: Int)

/** A custom shrinker: `falsifies(v)` is true while `v` still makes the property fail. */
typealias ShrinkFn<T> = (value: T, falsifies: (T) -> Boolean, maxSteps: Int) -> Shrunk<T>

/**
 * A generator of `T`: it draws from a PRNG stream and knows how to shrink what
 * it drew.  This is a thin Kotlin face on the Java engine's generator (VER05f);
 * every combinator below delegates to `Gens`, so draw order and shrink order are
 * the engine's, unchanged.
 *
 * ```
 * val evens = integer(0.toBigInteger(), 100.toBigInteger()).filter { !it.testBit(0) }
 * forAll(evens.list(0, 5)) { xs -> xs.size <= 5 }.assertPassed()
 * ```
 */
class Gen<T> private constructor(private val engine: JGen<T>) {
    /** The engine generator.  Hidden from Java: Kotlin `internal` alone would not hide it. */
    @get:JvmSynthetic
    internal val handle: JGen<T> get() = engine

    /** Draws one value. */
    fun sample(rng: Rng): T = engine.sample(rng)

    /** Shrinks [value] while [falsifies] holds; `Shrunk(value, 0)` if this generator cannot shrink. */
    fun shrink(value: T, maxSteps: Int = 1000, falsifies: (T) -> Boolean): Shrunk<T> =
        engine.shrink(value, Predicate { falsifies(it) }, maxSteps).toKotlin()

    /** Transforms generated values.  Drops shrinking: [f] has no inverse. */
    fun <U> map(f: (T) -> U): Gen<U> = wrap(Gens.map(engine) { f(it) })

    /**
     * Keeps values satisfying [pred]; after [maxRetries] consecutive rejections
     * the run is a `property-exhausted` verdict.  Shrinking is preserved.
     */
    fun filter(maxRetries: Int = 100, pred: (T) -> Boolean): Gen<T> =
        wrap(Gens.filter(engine, Predicate { pred(it) }, maxRetries))

    /** Dependent generation: sample, then sample from `f(value)`.  No shrinking. */
    fun <U> bind(f: (T) -> Gen<U>): Gen<U> = wrap(Gens.flatMap(engine) { f(it).handle })

    /** Alias of [bind]. */
    fun <U> flatMap(f: (T) -> Gen<U>): Gen<U> = bind(f)

    /** A list of this generator's values, length in `[minLen, maxLen]`. */
    fun list(minLen: Int, maxLen: Int): Gen<List<T>> = wrap(Gens.list(engine, minLen, maxLen))

    companion object {
        @JvmSynthetic
        internal fun <T> wrap(engine: JGen<T>): Gen<T> = Gen(engine)

        /**
         * Builds a custom generator from a [sampler] and an optional [shrinker]
         * (`null` means values are reported unshrunk).
         */
        @JvmStatic
        fun <T> of(sampler: (Rng) -> T, shrinker: ShrinkFn<T>? = null): Gen<T> {
            return Gen(JGen.of({ rng -> sampler(rng) }, shrinker?.toEngine()))
        }
    }
}

// -----------------------------------------------------------------------------
// Primitive generators
// -----------------------------------------------------------------------------

/** Always [value]; draws nothing. */
fun <T> constant(value: T): Gen<T> = Gen.wrap(Gens.constant(value))

/** `bounded(2) == 1`; shrinks true to false. */
fun bool(): Gen<Boolean> = Gen.wrap(Gens.bool())

/** `[0, 255]` as `UByte`. */
fun u8(): Gen<UByte> = Gen.wrap(Gens.u8().iso({ it.toUByte() }, { it.toInt() }))

/** A full 64-bit word as `ULong`. */
fun u64(): Gen<ULong> = Gen.wrap(Gens.u64().iso({ it.toULongWrapping() }, { it.toBigInteger() }))

/** A signed 128-bit integer (no Kotlin type holds it, so `BigInteger`). */
fun i128(): Gen<BigInteger> = Gen.wrap(Gens.i128())

/** A uniform integer over the inclusive range `[min, max]`, any width. */
fun integer(min: BigInteger, max: BigInteger): Gen<BigInteger> = Gen.wrap(Gens.integer(min, max))

/** A uniform `Long` over the inclusive range `[min, max]`. */
fun int64(min: Long, max: Long): Gen<Long> = Gen.wrap(Gens.int64(min, max))

/** A byte string with length in `[minLen, maxLen]`. */
fun bytes(minLen: Int, maxLen: Int): Gen<ByteArray> = Gen.wrap(Gens.bytes(minLen, maxLen))

// -----------------------------------------------------------------------------
// Combinators
// -----------------------------------------------------------------------------

/** Picks one generator uniformly.  No shrinking. */
fun <T> oneOf(vararg gens: Gen<T>): Gen<T> = Gen.wrap(Gens.oneOf(*gens.map { it.handle }.toTypedArray()))

/**
 * A weighted choice: `frequency(1L to a, 3L to b)` picks `b` three times in four.
 * A zero-weight arm is never chosen; an empty list, a negative weight or a zero
 * or oversized total is rejected by the engine.
 */
fun <T> frequency(vararg arms: Pair<Long, Gen<T>>): Gen<T> =
    Gen.wrap(Gens.frequency(*arms.map { Gens.Arm(it.first, it.second.handle) }.toTypedArray()))

/** A list of [elem] values, length in `[minLen, maxLen]`. */
fun <T> list(elem: Gen<T>, minLen: Int, maxLen: Int): Gen<List<T>> = elem.list(minLen, maxLen)

/** Samples [a] then [b]; shrinks one coordinate at a time. */
fun <A, B> tuple(a: Gen<A>, b: Gen<B>): Gen<Pair<A, B>> =
    Gen.wrap(
        Gens.tuple2(a.handle, b.handle)
            .iso({ Pair(it.first(), it.second()) }, { Gens.Pair(it.first, it.second) }),
    )

/** Samples [a], [b], [c] in order; shrinks one coordinate at a time. */
fun <A, B, C> tuple(a: Gen<A>, b: Gen<B>, c: Gen<C>): Gen<Triple<A, B, C>> =
    Gen.wrap(
        Gens.tuple3(a.handle, b.handle, c.handle)
            .iso({ Triple(it.first(), it.second(), it.third()) }, { Gens.Triple(it.first, it.second, it.third) }),
    )
