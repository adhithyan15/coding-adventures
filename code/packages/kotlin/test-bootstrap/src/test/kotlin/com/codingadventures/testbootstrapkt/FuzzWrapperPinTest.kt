package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.fuzz.Fuzz as JFuzz
import com.codingadventures.testbootstrap.fuzz.Handle as JHandle
import com.codingadventures.testbootstrap.fuzz.Mutate
import com.codingadventures.testbootstrap.fuzz.Options as JOptions
import com.codingadventures.testbootstrap.fuzz.Target
import com.codingadventures.testbootstrap.prng.SplitMix64
import java.math.BigInteger
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * Pins every Kotlin fuzz wrapper (VER06g section 8).  fuzz-v1 never pins a real
 * signature hash or an energy value, and its mutator cases touch only a few
 * argument values, so a wrapper that silently altered an argument or a result could
 * pass the corpus.  These tests close that gap two ways:
 *
 * - pinned values copied from the Java engine's own tests (signatures, energy, TAP);
 * - differential grids: every mutator overload against the engine's `Mutate` called
 *   directly, over offsets, counts, deltas, widths, endians and values that make
 *   each argument matter.
 */
class FuzzWrapperPinTest {
    private fun big(n: Long) = BigInteger.valueOf(n)

    private fun e(x: Endian) = Mutate.Endian.valueOf(x.name)

    private val data = ByteArray(8) { (it * 37 + 1).toByte() }
    private val offsets = listOf(-3L, 0L, 1L, 7L, 8L, 13L, 1000L)
    private val counts = listOf(-1L, 0L, 1L, 2L, 9L, 100L)

    // ---- pinned values (from BitmapSignatureTest / the reference) ----------------------

    @Test
    fun signaturesArePinnedAgainstTheReference() {
        assertEquals("4eca7ae8", crashSignature("panic", listOf("eval", "step")))
        assertEquals("4e61ea76", crashSignature("timeout", emptyList()))
        assertEquals("7146a410", crashSignature("oom", emptyList()))
        assertEquals("557f548e", crashSignature("x", listOf("a:b")))
        assertEquals("557f548e", crashSignature("x", listOf("a", "b")))
        assertEquals("%08x".format(SplitMix64.fnv1a64("panic:eval:step") and 0xFFFFFFFFL), crashSignature("panic", listOf("eval", "step")))
    }

    @Test
    fun dedupSeparatesKindsFramesAndDepth() {
        assertTrue(isDuplicateCrash("panic", listOf("eval", "step"), "panic", listOf("eval", "step")))
        assertFalse(isDuplicateCrash("panic", listOf("eval", "step"), "segv", listOf("alloc", "write")))
        assertFalse(isDuplicateCrash("panic", listOf("a"), "panic", listOf("a", "b")))
        assertFalse(isDuplicateCrash("panic", listOf("a", "b"), "panic", listOf("b", "a")))
        assertFalse(isDuplicateCrash("panic", listOf("a", "b"), "panic", listOf("a")))
        assertFalse(isDuplicateCrash("a", listOf("x"), "b", listOf("x")))
    }

    @Test
    fun energyScheduleIsPinned() {
        assertEquals(8, calculateEnergy(10, 4, 3, 6))
        assertEquals(3, calculateEnergy(5, 4, 0, 0))
        assertEquals(10, calculateEnergy(10, 1, 5, 0))
        assertEquals(1, calculateEnergy(0, 1_000_000, 1, 1))
        assertEquals(20, calculateEnergy(10, 0, 1, 1))
        assertEquals(MutatorLimits.MAX_ITERATIONS, calculateEnergy(Int.MAX_VALUE, 1, 1, 1))
        assertTrue(calculateEnergy(10, 4, 6, 6) > calculateEnergy(10, 4, 0, 6))
        assertTrue(calculateEnergy(10, 100, 1, 2) < calculateEnergy(10, 1, 1, 2))
    }

    @Test
    fun admissionCountsUniqueNovelEdgesOverFull64BitIds() {
        val max = ULong.MAX_VALUE
        assertEquals(Novelty(true, 1), corpusAdmit(listOf(1uL, 2uL, 3uL), listOf(2uL, 4uL)))
        assertEquals(Novelty(true, 1), corpusAdmit(listOf(1uL, 2uL, 3uL), listOf(4uL, 2uL)))
        assertEquals(Novelty(true, 2), corpusAdmit(listOf(1uL), listOf(5uL, 6uL, 5uL, 1uL)))
        assertEquals(Novelty(false, 0), corpusAdmit(listOf(1uL, 2uL, 3uL), listOf(1uL, 2uL)))
        assertEquals(Novelty(false, 0), corpusAdmit(listOf(max), listOf(max)))
        assertEquals(Novelty(true, 1), corpusAdmit(listOf(1uL), listOf(1uL + (1uL shl 32))), "ids are full words, not reduced")
        assertEquals(Novelty(true, 1), corpusAdmit(emptyList(), listOf(max)))
        assertEquals(Novelty(false, 0), corpusAdmit(listOf(1uL), emptyList()))
    }

    @Test
    fun tapLinesAreExactAndSanitised() {
        assertEquals("not ok 3 - t [fuzz-oom:0a0b0c0d]", formatTapFailure(3, "t", FuzzFailureId.OOM, "0a0b0c0d"))
        assertEquals("not ok 1 - a_b_c [fuzz-crash:3a8f19b2]", formatTapFailure(1, "a b\nc", FuzzFailureId.CRASH, "3a8f19b2"))
        assertEquals("not ok 12 - x [fuzz-timeout:00000000]", formatTapFailure(12, "x", FuzzFailureId.TIMEOUT, "00000000"))
        assertFailsWith<IllegalArgumentException> { formatTapFailure(1, "t", FuzzFailureId.CRASH, "3A8F19B2") }
        assertFailsWith<IllegalArgumentException> { formatTapFailure(1, "t", FuzzFailureId.CRASH, "3a8f19b") }
    }

    @Test
    fun strictCorpusTextIsPinned() {
        assertTrue(tfvIsValid("tfv 1\n"))
        assertFalse(tfvIsValid("bad header\n"))
        assertTrue(tfvIsValid("tfv 1\nr R1 Kop S706274\n"))
        assertFalse(tfvIsValid("tfv 1\nr R1 Kop S706274 Kdata XFF\n"))
    }

    @Test
    fun failureVocabularyIsPinnedAndClosed() {
        assertEquals(listOf("fuzz-crash", "fuzz-timeout", "fuzz-oom", "fuzz-leak", "fuzz-panic"), FuzzFailureId.ALL)
        assertEquals("fuzz-crash", FuzzFailureId.CRASH)
        assertEquals("fuzz-timeout", FuzzFailureId.TIMEOUT)
        assertEquals("fuzz-oom", FuzzFailureId.OOM)
        assertEquals("fuzz-leak", FuzzFailureId.LEAK)
        assertEquals("fuzz-panic", FuzzFailureId.PANIC)
        for (bad in listOf("fuzz-arbitrary-error", "", "FUZZ-CRASH", "fuzz-crash ", "property-panic", "fuzz-crash\n")) assertFalse(FuzzFailureId.isValid(bad), bad)
    }

    /** Hand-computed values for the corpus' constants (PT-F1: unequal splice splits, widths, endians). */
    @Test
    fun handComputedMutatorValues() {
        val z = ByteArray(4)
        assertContentEquals(byteArrayOf(0, 0, 0, 0x80.toByte()), z.boundaryInsert(0, 4, 0x80000000uL, Endian.LITTLE))
        assertContentEquals(byteArrayOf(0x80.toByte(), 0, 0, 0), z.boundaryInsert(0, 4, 0x80000000uL, Endian.BIG))
        assertContentEquals(byteArrayOf(0, 1), byteArrayOf(0xff.toByte(), 0).byteArithmetic(0, 1, 2, Endian.LITTLE))
        assertContentEquals(byteArrayOf(1, 0), byteArrayOf(0, 0xff.toByte()).byteArithmetic(0, 1, 2, Endian.BIG))
        assertContentEquals(byteArrayOf(1, 2, 3, 4, 7, 8), byteArrayOf(1, 2, 3, 4).splice(byteArrayOf(5, 6, 7, 8), 4, 2))
        assertContentEquals(byteArrayOf(1, 2, 5, 6, 7, 8), byteArrayOf(1, 2, 3, 4).splice(byteArrayOf(5, 6, 7, 8), 2, 0))
        assertContentEquals(byteArrayOf(1, 7, 8), byteArrayOf(1, 2, 3, 4).splice(byteArrayOf(5, 6, 7, 8), 1, 2))
        assertContentEquals(byteArrayOf(1, 2, 3, 4, 5, 6), byteArrayOf(1, 2, 3).blockInsert(3, byteArrayOf(4, 5, 6)))
        assertContentEquals(byteArrayOf(1, 2, 1, 2, 3), byteArrayOf(1, 2, 3).blockDuplicate(0, 2, 2))
        assertContentEquals(byteArrayOf(1, 5), byteArrayOf(1, 2, 3, 4, 5).blockDelete(1, 3))
        assertContentEquals(byteArrayOf(2, 0), byteArrayOf(0, 0).bitFlip(1, 1))
        assertContentEquals(byteArrayOf(0x0f, 0), byteArrayOf(0, 0).bitFlip(0, 4))
        assertContentEquals(byteArrayOf(0xff.toByte(), 0xff.toByte(), 0), byteArrayOf(0, 0, 0).byteFlip(0, 2))
    }

    // ---- differential grids: Kotlin wrapper == engine, argument by argument -------------------

    @Test
    fun bitAndByteFlipsMatchTheEngineForEveryArgument() {
        for (o in offsets) for (c in counts) {
            assertContentEquals(Mutate.bitFlip(data, o, c), data.bitFlip(o, c), "bitFlip($o,$c)")
            assertContentEquals(Mutate.bitFlipBig(data, big(o), big(c)), data.bitFlip(big(o), big(c)), "bitFlipBig($o,$c)")
            assertContentEquals(Mutate.byteFlip(data, o, c), data.byteFlip(o, c), "byteFlip($o,$c)")
            assertContentEquals(Mutate.byteFlipBig(data, big(o), big(c)), data.byteFlip(big(o), big(c)), "byteFlipBig($o,$c)")
        }
        val huge = BigInteger.TEN.pow(40)
        assertContentEquals(Mutate.bitFlipBig(data, huge, huge), data.bitFlip(huge, huge))
        assertContentEquals(Mutate.byteFlipBig(data, huge.negate(), huge), data.byteFlip(huge.negate(), huge))
    }

    @Test
    fun arithmeticAndBoundaryMatchTheEngineForEveryArgument() {
        val deltas = listOf(-35L, -1L, 1L, 300L, 70000L, Long.MAX_VALUE, Long.MIN_VALUE)
        val values = listOf(0uL, 1uL, 255uL, 0x80000000uL, 0xFFFFFFFFuL, ULong.MAX_VALUE)
        for (w in listOf(0, 1, 2, 3, 4, 8, 9)) for (en in Endian.entries) for (o in offsets) {
            for (d in deltas) {
                assertContentEquals(Mutate.byteArithmetic(data, o, d, w, e(en)), data.byteArithmetic(o, d, w, en), "arith($o,$d,$w,$en)")
                assertContentEquals(Mutate.byteArithmeticBig(data, big(o), big(d), w, e(en)), data.byteArithmetic(big(o), big(d), w, en), "arithBig($o,$d,$w,$en)")
            }
            for (v in values) {
                assertContentEquals(Mutate.boundaryInsert(data, o, w, v.toLong(), e(en)), data.boundaryInsert(o, w, v, en), "boundary($o,$w,$v,$en)")
                val bv = v.toBigInteger()
                assertContentEquals(Mutate.boundaryInsertBig(data, big(o), w, bv, e(en)), data.boundaryInsert(big(o), w, bv, en), "boundaryBig($o,$w,$v,$en)")
            }
            val neg = big(-1)
            assertContentEquals(Mutate.boundaryInsertBig(data, big(o), w, neg, e(en)), data.boundaryInsert(big(o), w, neg, en))
            val wide = BigInteger.ONE.shiftLeft(70).add(big(5))
            assertContentEquals(Mutate.boundaryInsertBig(data, big(o), w, wide, e(en)), data.boundaryInsert(big(o), w, wide, en))
            assertContentEquals(Mutate.byteArithmeticBig(data, big(o), wide, w, e(en)), data.byteArithmetic(big(o), wide, w, en))
        }
    }

    @Test
    fun blockOperationsAndSpliceMatchTheEngineForEveryArgument() {
        val ins = byteArrayOf(9, 8, 7)
        val donor = byteArrayOf(50, 51, 52, 53, 54)
        for (o in offsets) for (c in counts) {
            assertContentEquals(Mutate.blockDelete(data, o, c), data.blockDelete(o, c), "delete($o,$c)")
            assertContentEquals(Mutate.blockDeleteBig(data, big(o), big(c)), data.blockDelete(big(o), big(c)), "deleteBig($o,$c)")
            assertContentEquals(Mutate.splice(data, donor, o, c), data.splice(donor, o, c), "splice($o,$c)")
            assertContentEquals(Mutate.spliceBig(data, donor, big(o), big(c)), data.splice(donor, big(o), big(c)), "spliceBig($o,$c)")
            for (d in offsets) {
                assertContentEquals(Mutate.blockDuplicate(data, o, c, d), data.blockDuplicate(o, c, d), "dup($o,$c,$d)")
                assertContentEquals(Mutate.blockDuplicateBig(data, big(o), big(c), big(d)), data.blockDuplicate(big(o), big(c), big(d)), "dupBig($o,$c,$d)")
            }
        }
        for (o in offsets) {
            assertContentEquals(Mutate.blockInsert(data, o, ins), data.blockInsert(o, ins), "insert($o)")
            assertContentEquals(Mutate.blockInsertBig(data, big(o), ins), data.blockInsert(big(o), ins), "insertBig($o)")
        }
        val huge = BigInteger.TEN.pow(30)
        assertContentEquals(Mutate.spliceBig(data, donor, huge, huge.negate()), data.splice(donor, huge, huge.negate()))
        assertContentEquals(Mutate.blockDeleteBig(data, huge, huge), data.blockDelete(huge, huge))
    }

    @Test
    fun mutateSequenceMatchesTheEngineForSeedStepsAndMaxLen() {
        for (seed in listOf(0uL, 42uL, ULong.MAX_VALUE)) for (steps in listOf(0, 1, 4, 50, -3)) for (maxLen in listOf(0, 9, 20, MutatorLimits.MAX_INPUT_BYTES)) {
            assertContentEquals(Mutate.mutateSequence(data, seed.toLong(), steps, maxLen), data.mutateSequence(seed, steps, maxLen), "seq($seed,$steps,$maxLen)")
        }
        assertContentEquals(Mutate.mutateSequence(data, 7L, 30, MutatorLimits.MAX_INPUT_BYTES), data.mutateSequence(7uL, 30), "the default maxLen is the engine cap")
        assertContentEquals(Mutate.mutateSequence(ByteArray(0), 7L, 5, 100), ByteArray(0).mutateSequence(7uL, 5, 100))
    }

    @Test
    fun noMutatorChangesItsReceiverOrItsArguments() {
        val copy = data.copyOf()
        val donor = byteArrayOf(1, 2, 3)
        data.bitFlip(1, 3); data.byteFlip(1, 3); data.byteArithmetic(1, 5, 2, Endian.BIG); data.boundaryInsert(1, 2, 255uL, Endian.LITTLE)
        data.blockDelete(1, 2); data.blockInsert(1, donor); data.blockDuplicate(1, 2, 3); data.splice(donor, 3, 1); data.mutateSequence(1uL, 20)
        assertContentEquals(copy, data)
        assertContentEquals(byteArrayOf(1, 2, 3), donor)
    }

    // ---- options, handle and views against the engine ---------------------------------------------

    private fun javaReport(o: FuzzOptions, t: Target) =
        JFuzz.run(listOf(byteArrayOf(0x42, 1), byteArrayOf(2)), JOptions(o.maxIterations, o.seed.toLong(), o.maxInputLen, o.baseEnergy, o.minimizeBudget, o.stepBudget, o.memBudget), t)

    /** A target whose behaviour depends on every budget and length, so each option reaches the outcome. */
    private val knobTarget: Target = Target { d, h ->
        h.edge(d.size.toLong())
        if (d.size > 5) h.edge(1_000L + d[0])
        repeat(d.size) { h.tick() }
        h.reserve(d.size.toLong() * 10)
        if (d.isNotEmpty() && d[0] == 0x42.toByte()) throw IllegalStateException("x")
    }

    @Test
    fun everyOptionReachesTheEngineExactlyAsADirectEngineRunWould() {
        val sets =
            listOf(
                FuzzOptions(maxIterations = 300, seed = 5uL, maxInputLen = 12, baseEnergy = 3, minimizeBudget = 7, stepBudget = 4, memBudget = 35),
                FuzzOptions(maxIterations = 120, seed = 99uL, maxInputLen = 40, baseEnergy = 20, minimizeBudget = 0, stepBudget = 30, memBudget = 1000),
                FuzzOptions(maxIterations = 77, seed = ULong.MAX_VALUE, maxInputLen = 3, baseEnergy = 0, minimizeBudget = 50, stepBudget = 100, memBudget = 20),
            )
        for (o in sets) {
            val kotlinSide = runCampaign(listOf(byteArrayOf(0x42, 1), byteArrayOf(2)), o) { d, h -> knobTarget.run(d, h.engine) }
            val engineSide = javaReport(o, knobTarget)
            assertEquals(engineSide.toString(), kotlinSide.toString(), "$o")
            assertEquals(engineSide.seed(), kotlinSide.seed.toLong())
            assertEquals(engineSide.crashes().map { it.minimalInput().size }, kotlinSide.crashes.map { it.minimalInput().size })
            assertEquals(engineSide.corpus().map { it.toList() }, kotlinSide.corpus().map { it.toList() })
        }
        // every knob mattered: the two option sets above give different reports
        assertTrue(javaReport(sets[0], knobTarget).toString() != javaReport(sets[1], knobTarget).toString())
    }

    @Test
    fun theDslFeedsTheSameOptionsAsTheExplicitForm() {
        val viaDsl =
            fuzz {
                seed = 5uL; iterations = 300; maxInputLen = 12; baseEnergy = 3; minimizeBudget = 7; stepBudget = 4; memBudget = 35
                seeds(byteArrayOf(0x42, 1), byteArrayOf(2))
                target { d, h -> knobTarget.run(d, h.engine) }
            }
        val explicit =
            runCampaign(
                listOf(byteArrayOf(0x42, 1), byteArrayOf(2)),
                FuzzOptions(maxIterations = 300, seed = 5uL, maxInputLen = 12, baseEnergy = 3, minimizeBudget = 7, stepBudget = 4, memBudget = 35),
            ) { d, h -> knobTarget.run(d, h.engine) }
        assertEquals(explicit.toString(), viaDsl.toString())
        assertEquals(5uL, viaDsl.seed)
    }

    @Test
    fun theKotlinHandleReportsTheSameEdgesAndBudgetsAsTheEngineHandle() {
        // One campaign, two paths to the same ids: if the wrapper altered an id the engine
        // handle's edge would be a different slot and the edge total would be 2, not 1.
        val report =
            runCampaign(listOf(byteArrayOf(1)), FuzzOptions(maxIterations = 1)) { _, h ->
                h.edge(3uL); h.engine.edge(3L)
                h.edge(ULong.MAX_VALUE); h.engine.edge(-1L)
                h.edge("lbl"); h.engine.edge(JHandle.edgeId("lbl"))
            }
        assertEquals(3, report.totalEdges)
        assertEquals(JHandle.edgeId("lbl").toULong(), Handle.edgeId("lbl"))
        assertEquals(SplitMix64.fnv1a64("lbl").toULong(), Handle.edgeId("lbl"))
        // tick: exactly stepBudget ticks pass, one more is a timeout
        assertTrue(runCampaign(listOf(byteArrayOf(1)), FuzzOptions(maxIterations = 2, stepBudget = 3)) { _, h -> repeat(3) { h.tick() } }.isClean)
        assertEquals(1, runCampaign(listOf(byteArrayOf(1)), FuzzOptions(maxIterations = 2, stepBudget = 3)) { _, h -> repeat(4) { h.tick() } }.timeouts, "the crashing seed is triaged once and never mutated")
        // reserve: totals up to the budget pass, one byte over is an OOM, in the engine's unit (bytes)
        assertTrue(runCampaign(listOf(byteArrayOf(1)), FuzzOptions(maxIterations = 2, memBudget = 100)) { _, h -> h.reserve(60); h.reserve(40) }.isClean)
        assertTrue(runCampaign(listOf(byteArrayOf(1)), FuzzOptions(maxIterations = 2, memBudget = 100)) { _, h -> h.reserve(60); h.reserve(41) }.ooms > 0)
    }

    @Test
    fun reportAndCrashViewsMirrorTheEngineFieldForField() {
        val report =
            runCampaign(listOf(byteArrayOf(0x42, 1), byteArrayOf(2), byteArrayOf(3, 3)), FuzzOptions(maxIterations = 200, maxInputLen = 12, stepBudget = 6)) { d, h -> knobTarget.run(d, h.engine) }
        val j = report.engine
        assertEquals(j.iterations(), report.iterations)
        assertEquals(j.admittedSeeds(), report.admittedSeeds)
        assertEquals(j.totalEdges(), report.totalEdges)
        assertEquals(j.timeouts(), report.timeouts)
        assertEquals(j.ooms(), report.ooms)
        assertEquals(j.crashesDropped(), report.crashesDropped)
        assertEquals(j.admissionsDropped(), report.admissionsDropped)
        assertEquals(j.isClean(), report.isClean)
        assertEquals(j.uniqueCrashes(), report.uniqueCrashes)
        assertEquals(j.coverageTier(), report.coverageTier.label)
        assertEquals(j.seed(), report.seed.toLong())
        assertEquals(j.toString(), report.toString())
        assertEquals(j.crashes().size, report.crashes.size)
        assertTrue(report.crashes.isNotEmpty() && report.timeouts > 0)
        for ((jc, kc) in j.crashes().zip(report.crashes)) {
            assertEquals(jc.signature(), kc.signature)
            assertEquals(jc.kind(), kc.kind)
            assertEquals(jc.frames(), kc.frames)
            assertEquals(jc.hits(), kc.hits)
            assertEquals(jc.minimized(), kc.minimized)
            assertEquals(jc.inputDropped(), kc.inputDropped)
            assertContentEquals(jc.minimalInput(), kc.minimalInput())
            assertEquals(jc.toString(), kc.toString())
            assertEquals(crashSignature(kc.kind, kc.frames), kc.signature, "the signature is the pinned function of kind and frames")
        }
        assertEquals(com.codingadventures.testbootstrap.fuzz.Signature.tapFailures(j, "tgt", 4), report.tapFailures("tgt", 4))
        assertTrue(report.tapFailures("tgt", 4).first().startsWith("not ok 4 - tgt ["))
        assertTrue(report.tapFailures("tgt").first().startsWith("not ok 1 - tgt ["))
        assertFailsWith<UnsupportedOperationException> { (report.crashes as MutableList<CrashRecord>).clear() }
    }
}
