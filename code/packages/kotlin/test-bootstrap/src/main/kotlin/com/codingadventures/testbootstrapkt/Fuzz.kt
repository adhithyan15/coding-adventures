package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.fuzz.CrashRecord as JCrashRecord
import com.codingadventures.testbootstrap.fuzz.Fuzz as JFuzz
import com.codingadventures.testbootstrap.fuzz.Handle as JHandle
import com.codingadventures.testbootstrap.fuzz.Options as JFuzzOptions
import com.codingadventures.testbootstrap.fuzz.Report as JReport
import com.codingadventures.testbootstrap.fuzz.Signature
import com.codingadventures.testbootstrap.fuzz.Target
import java.util.Collections

/*
 * The Kotlin face of the coverage-guided campaign (VER06g section 2.1).  The
 * engine (VER06f) runs the campaign: mutation, admission, scheduling, dedup,
 * minimisation, budgets.  This file only types what goes in and what comes out.
 */

/**
 * What a target sees: coverage reporting and two deterministic budgets.  The
 * campaign creates it; a target must not retain it.
 *
 * Coverage is **caller-reported** (VER06f 4.2): the JDK has no in-process per-edge
 * API, so call [edge] at the program points you want guided.  A target that never
 * does is reported as `black-box-uninstrumented`.
 */
class Handle internal constructor(@get:JvmSynthetic internal val engine: JHandle) {
    /** Reports that the target reached the program point [id] (reduced mod 65,536). */
    fun edge(id: ULong) = engine.edge(id.toLong())

    /** Reports a program point by name; the id is a stable FNV-1a hash of [label]. */
    fun edge(label: String) = engine.edge(JHandle.edgeId(label))

    /**
     * Counts one unit of work.  After `stepBudget` ticks the execution is a
     * timeout, and every later call keeps throwing, so it cannot be swallowed.
     */
    fun tick() = engine.tick()

    /** Declares an allocation of [n] bytes made on behalf of the input; over `memBudget` is an OOM outcome. */
    fun reserve(n: Long) = engine.reserve(n)

    companion object {
        /** The stable edge id of a name (FNV-1a). */
        fun edgeId(label: String): ULong = JHandle.edgeId(label).toULong()
    }
}

/** The code under test.  Any `Throwable` that escapes (including `TODO()`) is a crash. */
typealias FuzzTarget = (data: ByteArray, h: Handle) -> Unit

/**
 * Bounds a campaign; the engine validates every field against its ceiling
 * (CWE-400) and throws `IllegalArgumentException` for an out-of-range one.
 * [toString] omits the seed.
 */
data class FuzzOptions(
    val maxIterations: Int = 1000,
    val seed: ULong = 42uL,
    val maxInputLen: Int = 4096,
    val baseEnergy: Int = 10,
    val minimizeBudget: Int = JFuzzOptions.DEFAULT_MINIMIZE_BUDGET,
    val stepBudget: Int = JFuzzOptions.DEFAULT_STEP_BUDGET,
    val memBudget: Int = JFuzzOptions.DEFAULT_MEM_BUDGET,
) {
    @JvmSynthetic
    internal fun toEngine(): JFuzzOptions =
        JFuzzOptions(maxIterations, seed.toEngineSeed(), maxInputLen, baseEnergy, minimizeBudget, stepBudget, memBudget)

    override fun toString(): String =
        "FuzzOptions(maxIterations=$maxIterations, maxInputLen=$maxInputLen, baseEnergy=$baseEnergy, " +
            "minimizeBudget=$minimizeBudget, stepBudget=$stepBudget, memBudget=$memBudget)"
}

/** What was actually collected: the report never claims more feedback than it had. */
enum class CoverageTier(val label: String) {
    /** The target reported edges through [Handle.edge]. */
    CALLER_REPORTED(JReport.TIER_REPORTED),

    /** No edge was collected: mutation was blind. */
    BLACK_BOX(JReport.TIER_BLACK_BOX),
}

/**
 * One deduplicated crash.  A regular class on purpose: a `data class` would
 * generate `toString`/`copy` that could reach the input.  [toString] prints the
 * signature, kind, frames and counts, never the bytes; the minimal input is
 * reached only by [minimalInput], which returns a copy.
 */
class CrashRecord internal constructor(@get:JvmSynthetic internal val engine: JCrashRecord) {
    /** The 8-hex-digit signature. */
    val signature: String get() = engine.signature()

    /** The normalised crash kind (a class name, e.g. `kotlin.NotImplementedError`). */
    val kind: String get() = engine.kind()

    /** The normalised frames, innermost first. */
    val frames: List<String> get() = engine.frames()

    /** How many executions hit this signature. */
    val hits: Int get() = engine.hits()

    /** False when minimisation was cut short by a budget. */
    val minimized: Boolean get() = engine.minimized()

    /** True when the retained-bytes cap forced the input to be dropped. */
    val inputDropped: Boolean get() = engine.inputDropped()

    /** A copy of the shortest input known to reproduce the signature.  PRIVATE: never log it. */
    fun minimalInput(): ByteArray = engine.minimalInput()

    override fun toString(): String = engine.toString()
}

/**
 * What a campaign found; every execution lands in exactly one bucket.  A view of
 * the engine's report.  [toString] prints counts only; the seed and the corpus
 * are PRIVATE and reachable only by [seed] and [corpus].
 */
class Report internal constructor(@get:JvmSynthetic internal val engine: JReport) {
    /** Mutation executions run (triage is not counted). */
    val iterations: Int get() = engine.iterations()

    /** Seeds in the final corpus. */
    val admittedSeeds: Int get() = engine.admittedSeeds()

    /** Distinct covered edges. */
    val totalEdges: Int get() = engine.totalEdges()

    /** One record per distinct signature (an unmodifiable list, also to Java callers). */
    val crashes: List<CrashRecord> = Collections.unmodifiableList(engine.crashes().map { CrashRecord(it) })

    /** Executions that exhausted their step budget. */
    val timeouts: Int get() = engine.timeouts()

    /** Executions that exhausted their memory budget. */
    val ooms: Int get() = engine.ooms()

    /** Novel signatures beyond the crash-table cap (a cap never silently loses a failure). */
    val crashesDropped: Int get() = engine.crashesDropped()

    /** Corpus admissions refused by the retained-bytes cap. */
    val admissionsDropped: Int get() = engine.admissionsDropped()

    /** What coverage was really collected. */
    val coverageTier: CoverageTier get() = CoverageTier.entries.first { it.label == engine.coverageTier() }

    /** True when at least one mutation ran and nothing crashed, timed out or ran out of memory. */
    val isClean: Boolean get() = engine.isClean()

    /** Distinct crash signatures seen, including dropped ones. */
    val uniqueCrashes: Int get() = engine.uniqueCrashes()

    /** The campaign seed.  PRIVATE. */
    val seed: ULong get() = engine.seed().toULong()

    /** Copies of the admitted seeds.  PRIVATE: they can contain sensitive input. */
    fun corpus(): List<ByteArray> = engine.corpus()

    /** Opaque public failure lines (`not ok N - target [id:hex8]`), none for a clean report. */
    fun tapFailures(target: String, firstTest: Int = 1): List<String> = Signature.tapFailures(engine, target, firstTest)

    override fun toString(): String = engine.toString()
}

/** The explicit form: run a bounded, deterministic campaign of [target] from [initialCorpus]. */
fun runCampaign(initialCorpus: List<ByteArray>, options: FuzzOptions = FuzzOptions(), target: FuzzTarget): Report {
    // The engine reuses one handle for a whole campaign, so the wrapper is built once.
    var wrapper: Handle? = null
    val adapter = Target { data, jh ->
        val w = wrapper?.takeIf { it.engine === jh } ?: Handle(jh).also { wrapper = it }
        target(data, w)
    }
    return Report(JFuzz.run(initialCorpus, options.toEngine(), adapter))
}

/** Configures a campaign inside [fuzz]. */
class CampaignBuilder {
    /** The campaign seed. */
    var seed: ULong = 42uL

    /** Mutation executions. */
    var iterations: Int = 1000

    /** Longest mutated candidate. */
    var maxInputLen: Int = 4096

    /** Energy base of the power schedule. */
    var baseEnergy: Int = 10

    /** Executions spent minimising one crash. */
    var minimizeBudget: Int = JFuzzOptions.DEFAULT_MINIMIZE_BUDGET

    /** `tick()` calls per execution before a timeout. */
    var stepBudget: Int = JFuzzOptions.DEFAULT_STEP_BUDGET

    /** `reserve()` bytes per execution before an OOM. */
    var memBudget: Int = JFuzzOptions.DEFAULT_MEM_BUDGET

    private val corpus = mutableListOf<ByteArray>()
    private var body: FuzzTarget? = null

    /** Adds initial seeds. */
    fun seeds(vararg inputs: ByteArray) {
        corpus.addAll(inputs)
    }

    /** Sets the code under test. */
    fun target(block: FuzzTarget) {
        body = block
    }

    @JvmSynthetic
    internal fun run(): Report {
        val t = body ?: throw IllegalStateException("fuzz { } needs a target { data, h -> ... }")
        return runCampaign(corpus.toList(), FuzzOptions(iterations, seed, maxInputLen, baseEnergy, minimizeBudget, stepBudget, memBudget), t)
    }
}

/**
 * The campaign DSL:
 *
 * ```
 * val report = fuzz {
 *     seed = 7uL
 *     iterations = 2000
 *     seeds(byteArrayOf(1, 2, 3))
 *     target { data, h ->
 *         h.edge("entry")
 *         if (data.size > 2 && data[0] == 0x42.toByte()) error("boom")   // a crash
 *     }
 * }
 * check(report.isClean)
 * ```
 */
fun fuzz(configure: CampaignBuilder.() -> Unit): Report = CampaignBuilder().apply(configure).run()
