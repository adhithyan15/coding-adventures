package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.pbt.NotFalsifyingException
import com.codingadventures.testbootstrap.prng.SplitMix64
import java.math.BigInteger
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

/** Unit tests for the Kotlin API surface (VER05f section 2). */
class ApiTest {
    private fun b(n: Long) = BigInteger.valueOf(n)

    private val local = Options(100, 1000, CiMode.OFF)

    // ---- unsigned primitives -----------------------------------------------------

    @Test
    fun u64GeneratorIsTheEngineWordAsULong() {
        assertEquals(16294208416658607535uL, u64().sample(SplitMix64(0)))
        assertEquals(listOf(16294208416658607535uL), genU64(0uL, 1))
    }

    @Test
    fun u8GeneratorIsUByteAndShrinksToItsFailureBoundary() {
        assertEquals(175.toUByte(), u8().sample(SplitMix64(0)))
        // falsifies(v) is false for every candidate, so no move is accepted.
        assertEquals(Shrunk(200.toUByte(), 0), u8().shrink(200.toUByte()) { false })
        val failed = assertIs<Verdict.Failed<UByte>>(checkProperty(1uL, u8(), local) { it < 100.toUByte() })
        assertEquals(100.toUByte(), failed.minimal)
    }

    @Test
    fun u64ShrinkRoundTripsThroughTheBijection() {
        val failed = assertIs<Verdict.Failed<ULong>>(checkProperty(3uL, u64(), local) { it < 1000uL })
        assertEquals(1000uL, failed.minimal)
        assertTrue(failed.counterexample!! >= 1000uL)
        assertEquals(ULong.MAX_VALUE.toBigInteger(), BigInteger.ONE.shiftLeft(64).subtract(BigInteger.ONE))
        assertEquals(0uL.toBigInteger(), BigInteger.ZERO)
        assertEquals(BigInteger.valueOf(-1).toULongWrapping(), ULong.MAX_VALUE)
    }

    @Test
    fun otherSamplersMatchTheirGenerators() {
        assertEquals(listOf(true, false, true), genBool(0uL, 3))
        assertEquals(4, genI128(42uL, 4).size)
        assertEquals(3, genBytes(13uL, 4, 8, 3).size)
        assertTrue(genBounded(7uL, b(10), b(20), 8).all { it >= b(10) && it <= b(20) })
        assertEquals(8, genU8(0uL, 8).size)
        assertEquals(175.toUByte(), genU8(0uL, 8).first())
    }

    // ---- generator builders --------------------------------------------------------

    @Test
    fun mapFilterBindAndListBuilders() {
        val g = integer(b(0), b(50)).map { it.toInt() }.filter { it % 2 == 0 }.list(0, 4)
        forAll(g, 9uL) { xs -> xs.all { it % 2 == 0 } && xs.size <= 4 }.assertPassed()

        val dependent = integer(b(1), b(5)).bind { n -> list(constant(7), n.toInt(), n.toInt()) }
        forAll(dependent) { xs -> xs.size in 1..5 && xs.all { it == 7 } }.assertPassed()
        forAll(bool().flatMap { constant(it) }) { true }.assertPassed()
    }

    @Test
    fun oneOfAndFrequencyChooseAmongArms() {
        forAll(oneOf(constant(1), constant(2), constant(3))) { it in 1..3 }.assertPassed()
        val freq = frequency(0L to constant("never"), 1L to constant("a"), 3L to constant("b"))
        val seen = (1..200).map { freq.sample(SplitMix64(it.toLong())) }.toSet()
        assertEquals(setOf("a", "b"), seen)
        assertFailsWith<IllegalArgumentException> { frequency<Int>() }
        assertFailsWith<IllegalArgumentException> { oneOf<Int>() }
    }

    @Test
    fun tuplesShrinkOneCoordinateAtATime() {
        val g = tuple(integer(b(0), b(100)), integer(b(0), b(100)))
        val failed = assertIs<Verdict.Failed<Pair<BigInteger, BigInteger>>>(checkProperty(5uL, g, local) { (x, y) -> x + y <= b(50) })
        val (x, y) = failed.minimal!!
        assertEquals(b(51), x + y)
        val g3 = tuple(bool(), int64(0, 9), u8())
        val f3 = assertIs<Verdict.Failed<Triple<Boolean, Long, UByte>>>(checkProperty(5uL, g3, local) { (_, n, u) -> n < 3L || u < 5.toUByte() })
        assertEquals(Triple(false, 3L, 5.toUByte()), f3.minimal)
    }

    @Test
    fun bytesInt64AndI128Generators() {
        val failed = assertIs<Verdict.Failed<ByteArray>>(checkProperty(2uL, bytes(0, 10), local) { it.size < 5 })
        assertContentEquals(ByteArray(5), failed.minimal)
        assertTrue(int64(-5, 5).sample(SplitMix64(1)) in -5L..5L)
        assertTrue(i128().sample(SplitMix64(1)).bitLength() <= 127)
        assertEquals(b(7), integer(b(7), b(7)).sample(SplitMix64(1)))
    }

    @Test
    fun customGeneratorWithAndWithoutShrinker() {
        val plain = Gen.of({ rng -> rng.bounded(10) })
        assertEquals(Shrunk(5L, 0), plain.shrink(5L) { true })
        // a shrinker that always proposes 0
        val g = Gen.of({ rng -> rng.bounded(1000) }, { v, falsifies, _ -> if (falsifies(0L)) Shrunk(0L, 1) else Shrunk(v, 0) })
        val failed = assertIs<Verdict.Failed<Long>>(checkProperty(1uL, g, local) { false })
        assertEquals(0L, failed.minimal)
        assertEquals(1, failed.shrinkSteps)
    }

    @Test
    fun explicitShrinkerOverridesTheGenerator() {
        val v = checkProperty(1uL, integer(b(0), b(100)), local, { value, _, _ -> Shrunk(value, 0) }) { it < b(0) }
        val failed = assertIs<Verdict.Failed<BigInteger>>(v)
        assertEquals(0, failed.shrinkSteps)
        assertEquals(failed.counterexample, failed.minimal)
    }

    // ---- verdicts --------------------------------------------------------------------

    @Test
    fun passedVerdict() {
        val v = forAll(integer(b(0), b(10)), 1uL, Options(25, 100, CiMode.OFF)) { it <= b(10) }
        assertEquals(Verdict.Passed(25), v)
        assertTrue(v.passed)
        assertFalse(v.failed)
        v.assertPassed()
    }

    @Test
    fun failedVerdictCarriesSeedAndWitnessesWhenNotRedacted() {
        val v = forAll(integer(b(0), b(1000)), 42uL, local) { it <= b(100) }
        val failed = assertIs<Verdict.Failed<BigInteger>>(v)
        assertEquals(FailureId.COUNTEREXAMPLE, failed.failureId)
        assertEquals(42uL, failed.seed)
        assertEquals(b(101), failed.minimal)
        assertNotNull(failed.counterexample)
        assertNull(failed.digest)
        assertTrue(failed.failed)
        assertFalse(failed.passed)
        assertContains(failed.message, "seed=42")
        val e = assertFailsWith<AssertionError> { v.assertPassed() }
        assertContains(e.message!!, "[property-counterexample]")
    }

    @Test
    fun defaultOptionsRedactTheSeedAndWitness() {
        val failed = assertIs<Verdict.Failed<BigInteger>>(forAll(integer(b(0), b(1000)), 42uL) { it <= b(100) })
        assertNull(failed.seed)
        assertNull(failed.minimal)
        assertNull(failed.counterexample)
        assertNotNull(failed.digest)
        assertFalse("seed" in failed.message)
    }

    @Test
    fun honestFailures() {
        val none = forAll(bool(), 1uL, Options(0, 10, CiMode.OFF)) { true }
        assertEquals(FailureId.EXHAUSTED, assertIs<Verdict.Failed<Boolean>>(none).failureId)
        val p = assertIs<Verdict.Failed<Boolean>>(forAll(bool(), 1uL, local) { error("boom") })
        assertEquals(FailureId.PANIC, p.failureId)
        assertEquals(1uL, p.seed)
        val exhausted = forAll(bool().filter(3) { false }, 1uL, local) { true }
        assertEquals(FailureId.EXHAUSTED, assertIs<Verdict.Failed<Boolean>>(exhausted).failureId)
        assertFailsWith<IllegalArgumentException> { forAll(bool(), 1uL, Options(-1, 1, CiMode.OFF)) { true } }
    }

    // ---- shrinking -----------------------------------------------------------------

    @Test
    fun shrinkFunctions() {
        val r = shrinkInt(b(54321), b(0), b(0), b(100000)) { it <= b(100) }
        assertEquals(Shrunk(b(101), r.steps), r)
        val l = shrinkList(listOf(b(10), b(5), b(8), b(4)), b(0)) { xs -> xs.fold(BigInteger.ZERO) { a, c -> a + c } <= b(20) }
        assertEquals(b(21), l.value.fold(BigInteger.ZERO) { a, c -> a + c })
        val by = shrinkBytes(byteArrayOf(1, 2, 3, 4, 5, 6)) { it.size < 5 }
        assertContentEquals(ByteArray(5), by.value)
        assertFailsWith<NotFalsifyingException> { shrinkInt(b(1), b(0), b(0), b(10)) { true } }
        assertEquals(Replay(b(413), b(101)), simulateReplay(42uL) { it <= b(100) })
    }

    // ---- misc ------------------------------------------------------------------------

    @Test
    fun failureIdVocabularyAndSubSeed() {
        assertEquals(4, FailureId.ALL.size)
        assertTrue(FailureId.isValid("property-timeout"))
        assertFalse(FailureId.isValid("nope"))
        assertFalse(FailureId.isValid(null))
        assertEquals(SplitMix64.subSeed(7L, "a/b").toULong(), subSeed(7uL, "a/b"))
        assertTrue(subSeed(7uL, "a") != subSeed(7uL, "b"))
    }

    @Test
    fun ciModeFromAnEnvironmentLookup() {
        assertEquals(CiMode.ON, ciModeFrom { if (it == "CI") "true" else null })
        assertEquals(CiMode.OFF, ciModeFrom { null })
        assertEquals(CiMode.OFF, ciModeFrom { "false" })
        assertEquals(CiMode.UNSET, Options().ci)
    }

    @Test
    fun seamsStartAtProductionValues() {
        assertEquals(0L, Seams.seedOffset)
        assertEquals(0, Seams.shrinkStepDelta)
        assertFalse(Seams.swapBounds)
    }
}
