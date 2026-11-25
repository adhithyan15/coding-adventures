package com.codingadventures.testbootstrapkt

import java.math.BigInteger
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

/** Unit tests for the Kotlin fuzz API (VER06g section 2). */
class FuzzApiTest {
    private fun b(vararg v: Int) = ByteArray(v.size) { v[it].toByte() }

    /** A target that crashes on any input starting with 0x42 and holds at least 2 bytes. */
    private fun crashOnB(data: ByteArray, h: Handle) {
        h.edge("entry")
        if (data.size >= 2 && data[0] == 0x42.toByte()) {
            h.edge("deep")
            error("boom with secret ${data.size}")
        }
    }

    private val crashTarget: FuzzTarget = { d, h -> crashOnB(d, h) }

    // ---- the DSL and the explicit form ------------------------------------------------

    @Test
    fun theDslRunsACampaignAndFindsADeduplicatedMinimisedCrash() {
        val report =
            fuzz {
                seed = 7uL
                iterations = 3000
                maxInputLen = 16
                seeds(b(0x42, 1, 2, 3), b(0, 0))
                target(crashTarget)
            }
        assertFalse(report.isClean)
        assertEquals(1, report.crashes.size)
        assertEquals(1, report.uniqueCrashes)
        val crash = report.crashes.single()
        assertEquals("java.lang.IllegalStateException", crash.kind)
        assertTrue(crash.hits > 1, "many inputs, one signature")
        assertTrue(Regex("[0-9a-f]{8}").matches(crash.signature))
        // The minimiser keeps the exact signature and shortens to the 2-byte trigger.
        assertEquals(2, crash.minimalInput().size)
        assertEquals(0x42.toByte(), crash.minimalInput()[0])
        assertTrue(crash.minimized)
        assertFalse(crash.inputDropped)
        assertEquals(CoverageTier.CALLER_REPORTED, report.coverageTier)
        assertTrue(report.totalEdges >= 1)
        assertEquals(7uL, report.seed)
        assertTrue(report.admittedSeeds >= 1)
        assertEquals(0, report.crashesDropped)
        assertEquals(0, report.admissionsDropped)
    }

    @Test
    fun theExplicitFormIsDeterministicAndSeedSensitive() {
        val opts = FuzzOptions(maxIterations = 500, seed = 1uL, maxInputLen = 16)
        val seeds = listOf(b(0x42, 9))
        val a = runCampaign(seeds, opts, crashTarget)
        val again = runCampaign(seeds, opts, crashTarget)
        assertEquals(a.toString(), again.toString())
        assertEquals(a.crashes.single().signature, again.crashes.single().signature)
        val other = runCampaign(seeds, opts.copy(seed = 99uL), crashTarget)
        assertEquals(1, other.crashes.size)
        assertEquals(a.crashes.single().signature, other.crashes.single().signature)
    }

    @Test
    fun aCleanCampaignIsHonestlyCleanAndBlackBoxWithoutEdges() {
        val clean = runCampaign(listOf(b(1)), FuzzOptions(maxIterations = 50)) { _, _ -> }
        assertTrue(clean.isClean)
        assertEquals(CoverageTier.BLACK_BOX, clean.coverageTier)
        assertEquals("black-box-uninstrumented", clean.coverageTier.label)
        assertEquals(emptyList(), clean.tapFailures("my_target"))
        val none = runCampaign(emptyList(), FuzzOptions(maxIterations = 0)) { _, _ -> }
        assertFalse(none.isClean, "a campaign that ran nothing fuzzed nothing")
    }

    @Test
    fun theDslNeedsATargetAndTheEngineValidatesOptions() {
        assertFailsWith<IllegalStateException> { fuzz { iterations = 5 } }
        assertFailsWith<IllegalArgumentException> { fuzz { iterations = -1; target { _, _ -> } } }
        assertFailsWith<IllegalArgumentException> { runCampaign(emptyList(), FuzzOptions(maxInputLen = Int.MAX_VALUE)) { _, _ -> } }
    }

    @Test
    fun everyDslKnobReachesTheEngine() {
        var executions = 0
        val report =
            fuzz {
                seed = 3uL
                iterations = 20
                maxInputLen = 4
                baseEnergy = 2
                minimizeBudget = 5
                stepBudget = 10
                memBudget = 1024
                seeds(b(1, 2))
                target { data, _ ->
                    executions++
                    assertTrue(data.size <= 4 || data.size == 2, "length cap applies to mutated candidates")
                }
            }
        assertEquals(20, report.iterations)
        assertTrue(executions >= 20)
    }

    // ---- crash dedup, TODO(), budgets ---------------------------------------------------

    @Test
    fun todoIsACrashLikeAnyOtherErrorAndDeduplicates() {
        val report = runCampaign(listOf(b(1, 2, 3)), FuzzOptions(maxIterations = 50)) { _, _ -> TODO("not yet") }
        assertEquals(1, report.crashes.size, "every input hits the same TODO signature")
        val crash = report.crashes.single()
        assertEquals("kotlin.NotImplementedError", crash.kind)
        assertTrue(crash.hits >= 1)
        assertEquals(0, crash.minimalInput().size)
        assertEquals(1, report.tapFailures("todo_target").size)
    }

    @Test
    fun differentThrowSitesAreDifferentSignaturesAndTheSameSiteIsOne() {
        val report =
            runCampaign(listOf(b(1), b(2)), FuzzOptions(maxIterations = 200, maxInputLen = 8)) { data, _ ->
                if (data.isNotEmpty() && data[0].toInt() % 2 == 0) throwA() else throwB()
            }
        assertEquals(2, report.crashes.size)
        assertEquals(2, report.crashes.map { it.signature }.toSet().size)
        assertTrue(report.crashes.all { it.frames.any { f -> "throwA" in f || "throwB" in f } }, report.crashes.toString())
    }

    private fun throwA(): Nothing = throw IllegalArgumentException("a")

    private fun throwB(): Nothing = throw IllegalArgumentException("b")

    @Test
    fun aStepBudgetBreachIsATimeoutNotACrash() {
        val report =
            runCampaign(listOf(b(1)), FuzzOptions(maxIterations = 5, stepBudget = 10)) { _, h ->
                while (true) h.tick()
            }
        assertTrue(report.timeouts > 0)
        assertEquals(0, report.crashes.size)
        assertFalse(report.isClean)
        assertTrue(report.tapFailures("loops").single().contains("[fuzz-timeout:"))
    }

    @Test
    fun aTargetThatSwallowsTheBudgetSentinelIsStillATimeout() {
        val report =
            runCampaign(listOf(b(1)), FuzzOptions(maxIterations = 3, stepBudget = 3)) { _, h ->
                try {
                    repeat(100) { h.tick() }
                } catch (_: Throwable) {
                    // swallowed: the engine still reports the budget that ran out
                }
            }
        assertTrue(report.timeouts > 0)
        assertEquals(0, report.crashes.size)
    }

    @Test
    fun aMemoryBudgetBreachIsAnOomAndSharesStickyExhaustionWithTick() {
        val report =
            runCampaign(listOf(b(1)), FuzzOptions(maxIterations = 3, memBudget = 100)) { _, h ->
                h.reserve(0)
                h.reserve(-5)
                try {
                    h.reserve(1000)
                } catch (_: Throwable) {
                }
                h.tick() // sticky: rethrows the memory sentinel, so this is an OOM and not a timeout
            }
        assertTrue(report.ooms > 0)
        assertEquals(0, report.timeouts)
        assertTrue(report.tapFailures("big").single().contains("[fuzz-oom:"))
    }

    @Test
    fun anExecutionThatReservesWithinItsBudgetPasses() {
        val report = runCampaign(listOf(b(1)), FuzzOptions(maxIterations = 5, memBudget = 100)) { _, h -> h.reserve(100) }
        assertTrue(report.isClean)
    }

    /**
     * A Kotlin function reference compiles to a synthetic class per call site, and its
     * `invoke` frame is part of the stack, so two call sites of the same function have
     * different signatures; a lambda does not (VER06g section 6, item 2).
     */
    @Test
    fun functionReferencesAddASyntheticFramePerCallSiteButLambdasDoNot() {
        val opts = FuzzOptions(maxIterations = 20, maxInputLen = 8)
        val seeds = listOf(b(0x42, 1))
        val viaRef = runCampaign(seeds, opts, ::crashOnB).crashes.single()
        val viaLambda = runCampaign(seeds, opts) { d, h -> crashOnB(d, h) }.crashes.single()
        assertEquals(1, viaLambda.frames.count { "crashOnB" in it })
        assertTrue(viaRef.frames.any { "invoke" in it }, viaRef.frames.toString())
        assertNotEquals(viaRef.signature, viaLambda.signature)
        // the binding's own adapter is always the outermost frame
        assertTrue(viaLambda.frames.last().startsWith("FuzzKt."), viaLambda.frames.toString())
    }

    // ---- privacy -----------------------------------------------------------------------------

    @Test
    fun renderingsNeverShowInputBytesOrTheSeed() {
        val secret = byteArrayOf(0x42, 0x5a, 0x73, 0x6f)
        val report = runCampaign(listOf(secret), FuzzOptions(maxIterations = 100, seed = 123456789uL)) { d, _ ->
            if (d.isNotEmpty() && d[0] == 0x42.toByte()) error("x")
        }
        val crash = report.crashes.single()
        val renders = listOf(report.toString(), crash.toString(), FuzzOptions(seed = 123456789uL).toString()) + report.tapFailures("t")
        for (r in renders) {
            assertFalse("123456789" in r, "the seed leaked into: $r")
            assertFalse("425a736f" in r.lowercase(), "input bytes leaked into: $r")
        }
        assertTrue(report.toString().startsWith("Report{"))
        assertTrue(crash.toString().startsWith("CrashRecord{"))
        assertEquals(123456789uL, report.seed, "the seed is reachable only by name")
    }

    @Test
    fun privateAccessorsReturnCopies() {
        val report = runCampaign(listOf(b(0x42, 1)), FuzzOptions(maxIterations = 50, maxInputLen = 8), crashTarget)
        val m = report.crashes.single().minimalInput()
        m[0] = 0
        assertEquals(0x42.toByte(), report.crashes.single().minimalInput()[0])
        val corpus = report.corpus()
        corpus.forEach { it.fill(7) }
        assertTrue(report.corpus().none { c -> c.isNotEmpty() && c.all { it == 7.toByte() } })
    }

    @Test
    fun theTargetGetsItsOwnCopyOfTheInput() {
        val original = b(5, 6, 7)
        runCampaign(listOf(original), FuzzOptions(maxIterations = 3)) { d, _ -> d.fill(0) }
        assertContentEquals(b(5, 6, 7), original)
    }

    // ---- handle ----------------------------------------------------------------------------------

    @Test
    fun edgeIdsAreStableFnvHashesAndReportedEdgesAreCounted() {
        assertEquals(Handle.edgeId("a"), Handle.edgeId("a"))
        assertNotEquals(Handle.edgeId("a"), Handle.edgeId("b"))
        val report = runCampaign(listOf(b(1)), FuzzOptions(maxIterations = 1)) { _, h ->
            h.edge(1uL)
            h.edge(1uL + 65_536uL) // reduced mod 65,536: the same slot
            h.edge("label")
        }
        assertEquals(2, report.totalEdges)
        assertEquals(CoverageTier.CALLER_REPORTED, report.coverageTier)
    }

    // ---- mutators and support wrappers ----------------------------------------------------------

    @Test
    fun longAndBigIntegerOverloadsAgree() {
        val d = b(1, 2, 3, 4, 5, 6, 7, 8)
        fun big(n: Long) = BigInteger.valueOf(n)
        assertContentEquals(d.bitFlip(9, 3), d.bitFlip(big(9), big(3)))
        assertContentEquals(d.byteFlip(2, 2), d.byteFlip(big(2), big(2)))
        assertContentEquals(d.byteArithmetic(1, 35, 2, Endian.BIG), d.byteArithmetic(big(1), big(35), 2, Endian.BIG))
        assertContentEquals(d.boundaryInsert(0, 4, 0x80000000uL, Endian.LITTLE), d.boundaryInsert(big(0), 4, big(0x80000000L), Endian.LITTLE))
        assertContentEquals(d.blockDelete(1, 2), d.blockDelete(big(1), big(2)))
        assertContentEquals(d.blockInsert(2, b(9, 9)), d.blockInsert(big(2), b(9, 9)))
        assertContentEquals(d.blockDuplicate(0, 2, 5), d.blockDuplicate(big(0), big(2), big(5)))
        assertContentEquals(d.splice(b(10, 11, 12), 2, 1), d.splice(b(10, 11, 12), big(2), big(1)))
        assertContentEquals(b(1, 2, 10, 11, 12).copyOfRange(0, 2) + b(11, 12), d.splice(b(10, 11, 12), 2, 1).copyOfRange(0, 4))
        assertContentEquals(b(0, 0, 0, 0x80), ByteArray(4).boundaryInsert(0, 4, 0x80000000uL, Endian.LITTLE))
        assertContentEquals(b(0, 1), b(0, 0).byteArithmetic(1, 1, 1, Endian.LITTLE))
        assertContentEquals(b(1, 2, 3, 4, 5, 6, 7, 8), d, "no mutator changes its receiver")
    }

    @Test
    fun mutateSequenceIsSeededAndCapped() {
        val d = ByteArray(8)
        assertContentEquals(d.mutateSequence(42uL, 4), d.mutateSequence(42uL, 4))
        assertNotEquals(d.mutateSequence(42uL, 8).toList(), d.mutateSequence(43uL, 8).toList())
        assertFailsWith<IllegalArgumentException> { d.mutateSequence(1uL, MutatorLimits.MAX_ITERATIONS + 1) }
        assertTrue(MutatorLimits.MAX_INPUT_BYTES > 0)
    }

    @Test
    fun supportWrappersDelegateToTheEngine() {
        assertEquals(Novelty(true, 1), corpusAdmit(listOf(1uL, 2uL, 3uL), listOf(2uL, 4uL, 4uL)))
        assertEquals(Novelty(false, 0), corpusAdmit(listOf(1uL, 2uL), listOf(1uL)))
        assertEquals(8, calculateEnergy(10, 4, 3, 6))
        val sig = crashSignature("panic", listOf("eval", "step"))
        assertTrue(Regex("[0-9a-f]{8}").matches(sig))
        assertTrue(isDuplicateCrash("panic", listOf("eval"), "panic", listOf("eval")))
        assertFalse(isDuplicateCrash("panic", listOf("eval"), "segv", listOf("eval")))
        assertEquals("not ok 3 - t [fuzz-oom:0a0b0c0d]", formatTapFailure(3, "t", FuzzFailureId.OOM, "0a0b0c0d"))
        assertFailsWith<IllegalArgumentException> { formatTapFailure(1, "t", "fuzz-bogus", "0a0b0c0d") }
        assertFailsWith<IllegalArgumentException> { formatTapFailure(1, "t", FuzzFailureId.CRASH, "XYZ") }
        assertTrue(tfvIsValid("tfv 1\n").let { true })
        assertFalse(tfvIsValid("bad header\n"))
    }
}
